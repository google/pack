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

use crypto_keys::Keys;
use deku::DekuContainerWrite;
use hasher::compute_top_level_hash;
use pack_common::Result;
use signing_block::compute_signing_block;
use zip_parser::find_offsets;
use zip_rebuilder::rebuild_zip_with_signing_block;

mod crypto;
pub mod crypto_keys;
mod hasher;
mod signed_data_block;
mod signing_block;
mod signing_types;
pub mod v1_signing;
mod zip_parser;
mod zip_rebuilder;

// APK Signature Scheme v2 based on https://source.android.com/docs/security/features/apksigning/v2
// APK Signature Scheme v3 based on https://source.android.com/docs/security/features/apksigning/v3
/// Signs a ZIP file buffer, adding an APK Signature Block before its Central Directory.
/// Can be used for both APK and AAB files.
pub fn sign_apk_buffer(apk_buf: &mut [u8], keys: &Keys) -> Result<Vec<u8>> {
    // Dry-run the block to figure out how long it will be given our key
    let dry_run = compute_signing_block([0; 32], keys)?;
    let signing_block_size = dry_run.to_bytes()?.len();
    // Read ZIP file to find central directory
    let offsets = find_offsets(apk_buf)?;
    // SHA-256 hash of ZIP contents (accounting for APK Signing Block)
    let top_level_hash = compute_top_level_hash(apk_buf, &offsets, signing_block_size)?;
    // Compute again using the real hash this time
    let signing_block = compute_signing_block(top_level_hash, keys)?;
    // Build up the final zip file again
    rebuild_zip_with_signing_block(&offsets, apk_buf, signing_block)
}
