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

use crate::{
    crypto::get_signature_for_signed_data,
    crypto_keys::Keys,
    signing_types::{
        ApkSigningBlock, SignatureSchemeV2Block, SignatureSchemeV3Block, SignedData, V3SignedData
    }
};
use pack_common::Result;

pub fn compute_signing_block(top_level_hash: [u8; 32], keys: &Keys) -> Result<ApkSigningBlock> {
    // TODO: Allow the user to customise this
    // NOTE: Must be 24 or higher. 23 does not support our hash algorithm.
    let min_sdk = 24;
    // We deal with this unsigned, but it seems Android parses it as signed, hence the 7F.
    let max_sdk = 0x7FFFFFFF;
    // Construct the data block that we're going to sign
    // NOTE: The signature does NOT include the length prefix
    let signed_data = SignedData::new(top_level_hash, keys);
    // Prepare the V3 block simultaneously
    let v3_signed_data = V3SignedData::from(&signed_data, min_sdk, max_sdk);
    // Sign them with RSA
    let signature = get_signature_for_signed_data(&signed_data, keys)?;
    let v3_signature = get_signature_for_signed_data(&v3_signed_data, keys)?;
    // Create the whole APK Signature Scheme block
    let scheme_block = SignatureSchemeV2Block::new(signed_data, signature, keys)?;
    let v3_scheme_block =
        SignatureSchemeV3Block::new(v3_signed_data, v3_signature, keys, min_sdk, max_sdk)?;
    // Create and serialise the entire APK Signing Block that goes straight into the zip file
    let signing_block = ApkSigningBlock::new(scheme_block, v3_scheme_block)?;
    Ok(signing_block)
}
