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
use std::borrow::Cow;
use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::{env, fs};
use xml::attribute::{Attribute, OwnedAttribute};
use xml::name::OwnedName;
use xml::namespace::Namespace;
use xml::reader::{EventReader, XmlEvent as ReaderXmlEvent};
use xml::writer::{EmitterConfig, EventWriter, XmlEvent as WriterXmlEvent};

pub mod res_dir;

const ANDROID_NAMESPACE: &str = "http://schemas.android.com/apk/res/android";
const USAGE: &str = "Usage: pack-cli [--version-code CODE] [--version-name NAME] \
    [--rename-manifest-package PACKAGE] <input-dir> <output-path> [keys.pem]";

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
    let args = parse_cli_args(env::args().skip(1))?;
    let out_apk_path = PathBuf::from(&args.out_path).with_extension("apk");
    let out_aab_path = PathBuf::from(&args.out_path).with_extension("aab");

    let signing_keys =
        args.pem_path
            .map_or_else(Keys::generate_random_testing_keys, |pem_path| {
                let key_pem_bytes = fs::read(pem_path)?;
                let key_pem_str = String::from_utf8(key_pem_bytes)
                    .map_err(|_e| PackError::Cli("Key PEM file is not valid UTF-8.".into()))?;
                Keys::from_combined_pem_string(&key_pem_str)
            })?;

    let mut in_path = PathBuf::from(&args.in_dir);

    in_path.push("AndroidManifest.xml");
    let android_manifest =
        apply_manifest_overrides(&fs::read(&in_path)?, &args.manifest_overrides)?;
    in_path.pop();

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

struct CliArgs {
    in_dir: String,
    out_path: String,
    pem_path: Option<String>,
    manifest_overrides: ManifestOverrides,
}

#[derive(Default)]
struct ManifestOverrides {
    package_name: Option<String>,
    version_code: Option<String>,
    version_name: Option<String>,
}

impl ManifestOverrides {
    fn is_empty(&self) -> bool {
        self.package_name.is_none() && self.version_code.is_none() && self.version_name.is_none()
    }
}

fn parse_cli_args(args: impl IntoIterator<Item = String>) -> Result<CliArgs> {
    let mut positional = vec![];
    let mut manifest_overrides = ManifestOverrides::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--version-code=") {
            manifest_overrides.version_code = Some(value.into());
            continue;
        }
        if let Some(value) = arg.strip_prefix("--version-name=") {
            manifest_overrides.version_name = Some(value.into());
            continue;
        }
        if let Some(value) = arg.strip_prefix("--rename-manifest-package=") {
            manifest_overrides.package_name = Some(value.into());
            continue;
        }

        match arg.as_str() {
            "--version-code" => {
                manifest_overrides.version_code = Some(next_option_value(&mut args, &arg)?);
            }
            "--version-name" => {
                manifest_overrides.version_name = Some(next_option_value(&mut args, &arg)?);
            }
            "--rename-manifest-package" => {
                manifest_overrides.package_name = Some(next_option_value(&mut args, &arg)?);
            }
            "--help" | "-h" => return Err(PackError::Cli(USAGE.into())),
            _ if arg.starts_with("--") => {
                return Err(PackError::Cli(format!(
                    "Unknown option \"{arg}\".\n{USAGE}"
                )));
            }
            _ => positional.push(arg),
        }
    }

    match positional.as_slice() {
        [in_dir, out_path] => Ok(CliArgs {
            in_dir: in_dir.into(),
            out_path: out_path.into(),
            pem_path: None,
            manifest_overrides,
        }),
        [in_dir, out_path, pem_path] => Ok(CliArgs {
            in_dir: in_dir.into(),
            out_path: out_path.into(),
            pem_path: Some(pem_path.into()),
            manifest_overrides,
        }),
        _ => Err(PackError::Cli(format!(
            "Expected <input-dir> <output-path> and optional [keys.pem].\n{USAGE}"
        ))),
    }
}

fn next_option_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String> {
    args.next()
        .ok_or_else(|| PackError::Cli(format!("{option} requires a value.\n{USAGE}")))
}

fn apply_manifest_overrides(
    manifest: &[u8],
    manifest_overrides: &ManifestOverrides,
) -> Result<Vec<u8>> {
    if manifest_overrides.is_empty() {
        return Ok(manifest.into());
    }

    let parser = EventReader::new(Cursor::new(manifest));
    let mut output = vec![];
    {
        let mut writer = EmitterConfig::new()
            .write_document_declaration(false)
            .create_writer(&mut output);
        let mut updated_manifest = false;

        for event in parser {
            let event = event.map_err(PackError::XmlParsingFailed)?;
            match event {
                ReaderXmlEvent::StartDocument { .. } | ReaderXmlEvent::EndDocument => {}
                ReaderXmlEvent::StartElement {
                    name,
                    mut attributes,
                    mut namespace,
                } if name.local_name == "manifest" && !updated_manifest => {
                    updated_manifest = true;
                    apply_manifest_attribute_overrides(
                        &mut attributes,
                        &mut namespace,
                        manifest_overrides,
                    );
                    write_start_element(&mut writer, &name, &attributes, &namespace)?;
                }
                _ => {
                    if let Some(event) = event.as_writer_event() {
                        writer.write(event).map_err(manifest_rewrite_error)?;
                    }
                }
            }
        }
    }

    Ok(output)
}

fn apply_manifest_attribute_overrides(
    attributes: &mut Vec<OwnedAttribute>,
    namespace: &mut Namespace,
    manifest_overrides: &ManifestOverrides,
) {
    if let Some(package_name) = &manifest_overrides.package_name {
        upsert_attribute(attributes, OwnedName::local("package"), package_name);
    }

    if manifest_overrides.version_code.is_some() || manifest_overrides.version_name.is_some() {
        namespace.force_put("android", ANDROID_NAMESPACE);
    }

    if let Some(version_code) = &manifest_overrides.version_code {
        upsert_attribute(
            attributes,
            OwnedName::qualified("versionCode", ANDROID_NAMESPACE, Some("android")),
            version_code,
        );
    }

    if let Some(version_name) = &manifest_overrides.version_name {
        upsert_attribute(
            attributes,
            OwnedName::qualified("versionName", ANDROID_NAMESPACE, Some("android")),
            version_name,
        );
    }
}

fn upsert_attribute(attributes: &mut Vec<OwnedAttribute>, name: OwnedName, value: &str) {
    if let Some(attribute) = attributes.iter_mut().find(|attribute| {
        attribute.name.local_name == name.local_name && attribute.name.namespace == name.namespace
    }) {
        attribute.name = name;
        attribute.value = value.into();
    } else {
        attributes.push(OwnedAttribute::new(name, value));
    }
}

fn write_start_element(
    writer: &mut EventWriter<impl Write>,
    name: &OwnedName,
    attributes: &[OwnedAttribute],
    namespace: &Namespace,
) -> Result<()> {
    let attributes: Vec<Attribute<'_>> = attributes
        .iter()
        .map(|attribute| attribute.borrow())
        .collect();
    writer
        .write(WriterXmlEvent::StartElement {
            name: name.borrow(),
            attributes: Cow::Owned(attributes),
            namespace: Cow::Owned(namespace.clone()),
        })
        .map_err(manifest_rewrite_error)
}

fn manifest_rewrite_error(err: xml::writer::Error) -> PackError {
    PackError::Cli(format!("Failed to rewrite AndroidManifest.xml: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &[u8] = br#"
        <manifest xmlns:android="http://schemas.android.com/apk/res/android"
            package="com.example.old">
            <application android:label="Example" />
        </manifest>
    "#;

    const MANIFEST_WITH_VALUES: &[u8] = br#"
        <manifest xmlns:android="http://schemas.android.com/apk/res/android"
            package="com.example.old"
            android:versionCode="1"
            android:versionName="1.0">
            <application android:label="Example" />
        </manifest>
    "#;

    #[test]
    fn parses_manifest_override_options() {
        let args = parse_cli_args(
            [
                "--version-code",
                "42",
                "--version-name=2.0",
                "--rename-manifest-package",
                "com.example.new",
                "input",
                "output",
                "keys.pem",
            ]
            .into_iter()
            .map(String::from),
        )
        .unwrap();

        assert_eq!(args.in_dir, "input");
        assert_eq!(args.out_path, "output");
        assert_eq!(args.pem_path.as_deref(), Some("keys.pem"));
        assert_eq!(args.manifest_overrides.version_code.as_deref(), Some("42"));
        assert_eq!(args.manifest_overrides.version_name.as_deref(), Some("2.0"));
        assert_eq!(
            args.manifest_overrides.package_name.as_deref(),
            Some("com.example.new")
        );
    }

    #[test]
    fn injects_missing_manifest_attributes() {
        let output = apply_manifest_overrides(
            MANIFEST,
            &ManifestOverrides {
                package_name: Some("com.example.new".into()),
                version_code: Some("42".into()),
                version_name: Some("2.0".into()),
            },
        )
        .unwrap();

        assert_eq!(
            manifest_attribute(&output, None, "package").as_deref(),
            Some("com.example.new")
        );
        assert_eq!(
            manifest_attribute(&output, Some(ANDROID_NAMESPACE), "versionCode").as_deref(),
            Some("42")
        );
        assert_eq!(
            manifest_attribute(&output, Some(ANDROID_NAMESPACE), "versionName").as_deref(),
            Some("2.0")
        );
    }

    #[test]
    fn replaces_existing_manifest_attributes() {
        let output = apply_manifest_overrides(
            MANIFEST_WITH_VALUES,
            &ManifestOverrides {
                package_name: None,
                version_code: Some("7".into()),
                version_name: Some("7.1".into()),
            },
        )
        .unwrap();

        assert_eq!(
            manifest_attribute(&output, None, "package").as_deref(),
            Some("com.example.old")
        );
        assert_eq!(
            manifest_attribute(&output, Some(ANDROID_NAMESPACE), "versionCode").as_deref(),
            Some("7")
        );
        assert_eq!(
            manifest_attribute(&output, Some(ANDROID_NAMESPACE), "versionName").as_deref(),
            Some("7.1")
        );
    }

    fn manifest_attribute(manifest: &[u8], namespace: Option<&str>, name: &str) -> Option<String> {
        let parser = EventReader::new(Cursor::new(manifest));
        for event in parser {
            if let ReaderXmlEvent::StartElement {
                name: element_name,
                attributes,
                ..
            } = event.unwrap()
            {
                if element_name.local_name != "manifest" {
                    continue;
                }
                return attributes
                    .iter()
                    .find(|attribute| {
                        attribute.name.local_name == name
                            && attribute.name.namespace.as_deref() == namespace
                    })
                    .map(|attribute| attribute.value.clone());
            }
        }
        None
    }
}
