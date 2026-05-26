// Copyright 2024 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use pack_api::{compile_and_sign_aab, compile_and_sign_apk, Keys, PackError, Package, Result};
use res_dir::read_res_dir;
use std::io::Cursor;
use std::path::PathBuf;
use std::{env, fs};
use xml::reader::{EventReader, XmlEvent};

pub mod res_dir;

const ANDROID_NAMESPACE: &str = "http://schemas.android.com/apk/res/android";
const MISSING_VERSION_CODE_WARNING: &str = "AndroidManifest.xml is missing android:versionCode \
    on its <manifest> element. Android requires a version code for installable APKs.";

/// Run from a watch face directory to build signed APK and AAB files.
///
/// ```
/// $ ls ./watchface
/// res/ AndroidManifest.xml
/// $ pack-cli ./watchface ./watchface/package
/// $ ls ./watchface
/// res/ AndroidManifest.xml package.apk package.aab
/// ```
///
/// For signing keys, use:
///
/// ```
/// $ pack-cli ./watchface ./watchface/package.apk ./keys.pem
/// ```
///
/// Where `keys.pem` is a PEM-format file containing both a `-----BEGIN CERTIFICATE-----`
/// section and a `-----BEGIN PRIVATE KEY-----` section.
fn main() {
    let result = pack_main();
    if let Err(err) = result {
        eprintln!("Error: {err}");
    }
}

fn pack_main() -> Result<()> {
    let in_dir = env::args()
        .nth(1)
        .ok_or(PackError::Cli("Input directory path not provided.".into()))?;
    let out_path = env::args()
        .nth(2)
        .ok_or(PackError::Cli("Output APK path not provided.".into()))?;
    let out_apk_path = PathBuf::from(&out_path).with_extension("apk");
    let out_aab_path = PathBuf::from(&out_path).with_extension("aab");

    let signing_keys =
        env::args()
            .nth(3)
            .map_or_else(Keys::generate_random_testing_keys, |pem_path| {
                let key_pem_bytes = fs::read(pem_path)?;
                let key_pem_str = String::from_utf8(key_pem_bytes)
                    .map_err(|_e| PackError::Cli("Key PEM file is not valid UTF-8.".into()))?;
                Keys::from_combined_pem_string(&key_pem_str)
            })?;

    let mut in_path = PathBuf::from(&in_dir);

    in_path.push("AndroidManifest.xml");
    let android_manifest = fs::read(&in_path)?;
    in_path.pop();

    for warning in critical_manifest_warnings(&android_manifest)? {
        eprintln!("Warning: {warning}");
    }

    in_path.push("res");
    let resources = read_res_dir(&in_path)?;
    in_path.pop();

    let pkg = Package {
        android_manifest,
        resources
    };

    let apk = compile_and_sign_apk(&pkg, &signing_keys)?;
    fs::write(&out_apk_path, apk)?;
    println!("Wrote {out_apk_path:?} to disk.");
    let aab = compile_and_sign_aab(&pkg, &signing_keys)?;
    fs::write(&out_aab_path, aab)?;
    println!("Wrote {out_aab_path:?} to disk.");

    println!("Compiled, aligned & signed successfully!");

    Ok(())
}

fn critical_manifest_warnings(manifest: &[u8]) -> Result<Vec<&'static str>> {
    let parser = EventReader::new(Cursor::new(manifest));
    for event in parser {
        match event.map_err(PackError::XmlParsingFailed)? {
            XmlEvent::StartElement {
                name, attributes, ..
            } if name.local_name == "manifest" => {
                let has_version_code = attributes.iter().any(|attr| {
                    attr.name.local_name == "versionCode"
                        && attr.name.namespace.as_deref() == Some(ANDROID_NAMESPACE)
                });
                return Ok(if has_version_code {
                    vec![]
                } else {
                    vec![MISSING_VERSION_CODE_WARNING]
                });
            }
            XmlEvent::StartElement { .. } => return Ok(vec![]),
            _ => {}
        }
    }
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST_WITH_VERSION_CODE: &[u8] = br#"
        <manifest
            xmlns:android="http://schemas.android.com/apk/res/android"
            package="com.example.pack"
            android:versionCode="1">
            <application />
        </manifest>
    "#;

    const MANIFEST_WITHOUT_VERSION_CODE: &[u8] = br#"
        <manifest
            xmlns:android="http://schemas.android.com/apk/res/android"
            package="com.example.pack">
            <application />
        </manifest>
    "#;

    #[test]
    fn does_not_warn_when_version_code_is_present() {
        assert_eq!(
            critical_manifest_warnings(MANIFEST_WITH_VERSION_CODE).unwrap(),
            Vec::<&'static str>::new()
        );
    }

    #[test]
    fn warns_when_version_code_is_missing() {
        assert_eq!(
            critical_manifest_warnings(MANIFEST_WITHOUT_VERSION_CODE).unwrap(),
            vec![MISSING_VERSION_CODE_WARNING]
        );
    }
}
