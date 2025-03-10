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

//! # PACK API
//!
//! This crate exposes the main public API through which other projects can use
//! PACK's APK and AAB compilation features.
//!
//! ## Creating an APK
//!
//! The following API compiles and signs an APK in memory.
//!
//! ```
//! let pkg = Package {
//!     android_manifest: "<?xml version...".as_bytes(),
//!     resources: vec![
//!         FileResource::new("xml".into(), "strings.xml".into(), "<resource>...".as_bytes()),
//!         FileResource::new("drawable".into(), "image.png".into(), fs::read(...))
//!     ]
//! }
//!
//! // Use placeholder keys for simplicity
//! let signing_keys = crypto_keys::Keys::generate_random_testing_keys();
//! let apk_bytes = compile_and_sign_apk(pkg, signing_keys)?;
//! ```
//!
//! ## Creating an AAB
//!
//! The API is exactly the same for the more complex Google Play publishing format.
//!
//! ```
//! let aab_bytes = compile_and_sign_aab(pkg, signing_keys)?;
//! ```

use std::io::{BufReader, Cursor};

use deku::DekuContainerWrite;
use pack_asset_compiler::{
    resource_external_types::ResChunk, resource_internal_types::Resource,
    resource_table::construct_resource_table, strings_xml_parser::parse_strings_xml,
    xml_file::xml_to_res_chunk
};
use pack_sign::v1_signing::add_v1_signature_files;

pub use pack_asset_compiler::resource_internal_types::FileResource;
pub use pack_common::{PackError, Result};
pub use pack_sign::crypto_keys::Keys;

/// Represents an Android package before compilation.
pub struct Package {
    /// The package's AndroidManifest.xml file as a series of UTF-8 bytes.
    pub android_manifest: Vec<u8>,
    /// The package's associated files from the res/ directories.
    pub resources: Vec<FileResource>
}

/// Performs all the steps in packaging an APK.
///
/// This includes:
///
///  - Compiling resources into `aapt2`'s ResourceChunk format
///  - Constructing a 4-byte aligned Zip file with the right compression settings
///  - Signing the resultant APK with APK Signature Scheme v2 & v3
///
/// Returns: A vector of bytes representing the final APK zip file. For example,
/// you could flush these to disk or download them from a webpage if called from WASM.
///
/// The APK is built and signed in-memory without using the local filesystem.
pub fn compile_and_sign_apk(package: &Package, keys: &Keys) -> Result<Vec<u8>> {
    let mut resources = vec![];
    // Look for strings.xml and parse it if present
    for res in &package.resources {
        if res.subdirectory == "values" && res.name == "strings.xml" {
            let mut string_cur = Cursor::new(&res.contents);
            resources.extend(parse_strings_xml(&mut string_cur));
        } else {
            resources.push(Resource::File(res.clone()));
        }
    }
    // Sort resources alphabetically so that all sub-types are grouped and binary-searchable
    resources.sort_by(|a, b| a.get_subdirectory().cmp(b.get_subdirectory()));

    let (manifest_res_chunk, package_name, _label) =
        parse_manifest(&package.android_manifest, &resources)?;
    let mut apk_files: Vec<pack_zip::File> = vec![];

    apk_files.push(res_to_apk_file(
        "AndroidManifest.xml".into(),
        &manifest_res_chunk
    )?);

    // Generate the resources.arsc file
    let resource_table_res_chunk = construct_resource_table(&package_name, &mut resources)?;
    // Add it to the APK
    apk_files.push(res_to_apk_file(
        "resources.arsc".into(),
        &resource_table_res_chunk
    )?);

    // Add the resource files themselves to the APK
    for res in &resources {
        if let Resource::File(file) = res {
            let res_bytes = file.as_bytes_for_apk(&resources)?;
            apk_files.push(pack_zip::File {
                path: format!("res/{}/{}", file.subdirectory, file.name),
                data: res_bytes
            })
        }
    }

    let mut zip_buf = vec![];
    let zip_buf_cursor = Cursor::new(&mut zip_buf);
    pack_zip::zip_apk(&apk_files, zip_buf_cursor)?;

    pack_sign::sign_apk_buffer(&mut zip_buf, keys)
}

/// Performs all the steps in packaging an AAB (Android App Bundle).
///
/// This includes:
///
///  - Compiling resources into `bundletool`'s ProtoXML format
///  - Setting up a base resource module and resource table
///  - Constructing a 4-byte aligned Zip file with the right compression settings
///  - Signing the resultant AAB with APK Signature Scheme v1, v2 & v3
///
/// Returns: A vector of bytes representing the final AAB zip file.
///
/// The AAB is built and signed in-memory without using the local filesystem.
///
/// ### Why are AABs signed with Signature Scheme v1 but APKs aren't?
///
/// From Android 7 (Nougat) and up, APKs are not required to be signed using Scheme v1.
/// However, Google Play's backend has not implemented support for signing v2
/// so bundles intended for publishing must be signed using the old format.
pub fn compile_and_sign_aab(package: &Package, keys: &Keys) -> Result<Vec<u8>> {
    let mut resources = vec![];
    // Look for strings.xml and parse it if present
    for res in &package.resources {
        if res.subdirectory == "values" && res.name == "strings.xml" {
            let mut string_cur = Cursor::new(&res.contents);
            resources.extend(parse_strings_xml(&mut string_cur));
        } else {
            resources.push(Resource::File(res.clone()));
        }
    }
    // Sort resources alphabetically so that all sub-types are grouped and binary-searchable
    resources.sort_by(|a, b| a.get_subdirectory().cmp(b.get_subdirectory()));

    let (_, package_name, label) = parse_manifest(&package.android_manifest, &resources)?;

    let mut aab_files = pack_aab::construct_aab(
        &package_name,
        &label,
        String::from_utf8(package.android_manifest.clone())
            .map_err(|_e| PackError::NotAManifest)?,
        &mut resources
    )?;

    // Sign the AAB with Scheme v1 (pre-zip)
    add_v1_signature_files(&mut aab_files, keys)?;

    // Zip up the AAB
    let mut aab_buf = vec![];
    let aab_buf_cursor = Cursor::new(&mut aab_buf);
    pack_zip::zip_apk(&aab_files, aab_buf_cursor)?;

    // Sign the AAB with Scheme v2 and v3 (post-zip)
    pack_sign::sign_apk_buffer(&mut aab_buf, keys)
}

fn parse_manifest(
    manifest: &[u8],
    resources: &[Resource]
) -> Result<(ResChunk, String, Option<String>)> {
    let manifest_cursor = Cursor::new(manifest);
    let mut reader = BufReader::new(manifest_cursor);
    let (manifest_res_chunk, manifest_info) = xml_to_res_chunk(&mut reader, resources)?;
    Ok((
        manifest_res_chunk,
        manifest_info.package_name.ok_or(PackError::NotAManifest)?,
        manifest_info.label
    ))
}

fn res_to_apk_file(path: String, chunk: &ResChunk) -> Result<pack_zip::File> {
    Ok(pack_zip::File {
        path,
        data: chunk.to_bytes()?
    })
}
