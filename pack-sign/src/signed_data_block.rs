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
    crypto_keys::Keys,
    hasher::Sha256Hash,
    signing_types::{
        len_pfx_u32, len_pfx_u64, ApkSigningBlock, Digest, Signature, SignatureAlgorithmId::*,
        SignatureSchemeV2Block, SignatureSchemeV3Block, SignedData, Signer,
        SigningBlockIdValuePair, SigningBlockPairs, V3SignedData, V3Signer
    }
};
use deku::DekuContainerWrite;
use pack_common::*;

// Constructs the Signed Data block for the V2 Scheme
// This is the data that gets signed by the crypto module
// It does not, itself, contain a cryptographic signature
impl SignedData {
    pub fn new(top_level_hash: Sha256Hash, keys: &Keys) -> SignedData {
        SignedData {
            // TODO: len_vec macro that makes a length-prefixed list of length-prefixed T
            digests: len_pfx_u32(vec![len_pfx_u32(Digest {
                digest: len_pfx_u32(top_level_hash),
                signature_algorithm_id: RsaSsaPkcs1v1_5WithSha2_256
            })]),
            certificates: len_pfx_u32(vec![len_pfx_u32(keys.certificate.clone())]),
            additional_attributes: 0
        }
    }
}

impl V3SignedData {
    pub fn from(v2_data: &SignedData, min_sdk: u32, max_sdk: u32) -> V3SignedData {
        V3SignedData {
            digests: v2_data.digests.clone(),
            certificates: v2_data.certificates.clone(),
            min_sdk,
            max_sdk,
            additional_attributes: v2_data.additional_attributes
        }
    }
}

impl SignatureSchemeV2Block {
    pub fn new(
        signed_data: SignedData,
        signature: Vec<u8>,
        keys: &Keys
    ) -> Result<SignatureSchemeV2Block> {
        Ok(SignatureSchemeV2Block {
            signers: len_pfx_u32(vec![len_pfx_u32(Signer {
                signed_data: len_pfx_u32(signed_data),
                signatures: len_pfx_u32(vec![len_pfx_u32(Signature {
                    signature_algorithm_id: RsaSsaPkcs1v1_5WithSha2_256,
                    signature: len_pfx_u32(signature)
                })]),
                public_key: len_pfx_u32(keys.pub_key_as_der()?)
            })])
        })
    }
}

impl SignatureSchemeV3Block {
    pub fn new(
        signed_data: V3SignedData,
        signature: Vec<u8>,
        keys: &Keys,
        min_sdk: u32,
        max_sdk: u32
    ) -> Result<SignatureSchemeV3Block> {
        Ok(SignatureSchemeV3Block {
            signers: len_pfx_u32(vec![len_pfx_u32(V3Signer {
                signed_data: len_pfx_u32(signed_data),
                min_sdk,
                max_sdk,
                signatures: len_pfx_u32(vec![len_pfx_u32(Signature {
                    signature_algorithm_id: RsaSsaPkcs1v1_5WithSha2_256,
                    signature: len_pfx_u32(signature)
                })]),
                public_key: len_pfx_u32(keys.pub_key_as_der()?)
            })])
        })
    }
}

pub const SIGNATURE_SCHEME_V2_BLOCK_ID: u32 = 0x7109871A;
pub const SIGNATURE_SCHEME_V3_BLOCK_ID: u32 = 0xF05368C0;
pub const APK_SIGNING_BLOCK_MAGIC: &[u8; 16] = b"APK Sig Block 42";
impl ApkSigningBlock {
    pub fn new(
        v2_sig_block: SignatureSchemeV2Block,
        v3_sig_block: SignatureSchemeV3Block
    ) -> Result<ApkSigningBlock> {
        let pairs = SigningBlockPairs {
            pairs: vec![
                len_pfx_u64(SigningBlockIdValuePair {
                    id: SIGNATURE_SCHEME_V2_BLOCK_ID,
                    value: v2_sig_block.to_bytes()?
                }),
                len_pfx_u64(SigningBlockIdValuePair {
                    id: SIGNATURE_SCHEME_V3_BLOCK_ID,
                    value: v3_sig_block.to_bytes()?
                }),
            ]
        };

        let pairs_length = pairs.to_bytes()?.len();
        // Plus size_of_self_counted plus magic
        let sig_block_size = (pairs_length + 8 + 16) as u64;

        Ok(ApkSigningBlock {
            size_of_self_not_counted: sig_block_size,
            pairs,
            size_of_self_counted: sig_block_size,
            magic: *APK_SIGNING_BLOCK_MAGIC
        })
    }
}
