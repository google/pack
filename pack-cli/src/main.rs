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
use std::path::PathBuf;
use std::{env, fs};

pub mod res_dir;

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
fn main() -> Result<()> {
    let in_dir = env::args()
        .nth(1)
        .ok_or(PackError::Cli("Input directory path not provided".into()))?;
    let out_path = env::args()
        .nth(2)
        .ok_or(PackError::Cli("Output APK path not provided".into()))?;
    let out_apk_path = PathBuf::from(&out_path).with_extension("apk");
    let out_aab_path = PathBuf::from(&out_path).with_extension("aab");

    let signing_keys =
        env::args()
            .nth(3)
            .map_or_else(Keys::generate_random_testing_keys, |pem_path| {
                let key_pem_bytes = fs::read(pem_path)?;
                let key_pem_str = String::from_utf8(key_pem_bytes)
                    .map_err(|_e| PackError::Cli("Key PEM file is not valid UTF-8".into()))?;
                Keys::from_combined_pem_string(&key_pem_str)
            })?;

    let mut in_path = PathBuf::from(&in_dir);

    in_path.push("AndroidManifest.xml");
    let android_manifest = fs::read(&in_path)?;
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
    println!("Wrote {:?} to disk", out_apk_path);
    let aab = compile_and_sign_aab(&pkg, &signing_keys)?;
    fs::write(&out_aab_path, aab)?;
    println!("Wrote {:?} to disk", out_aab_path);

    println!("Compiled, aligned & signed successfully!");

    Ok(())
}
