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

// Types involved in the APK Signature Scheme v2
use deku::prelude::*;

use crate::hasher::Sha256Hash;

// Named according to the APK Signature Scheme v2 doc

#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct ApkSigningBlock {
    // Size of this structure MINUS this field!
    // This field appears twice, the 'minus' is only for one of them.
    // So if the structure is 128 bytes, this reads 120, NOT 112.
    pub size_of_self_not_counted: u64,
    pub pairs: SigningBlockPairs,
    pub size_of_self_counted: u64,
    pub magic: [u8; 16]
}

// This is in its own block so that we can determine its size before serialising its parent
#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct SigningBlockPairs {
    pub pairs: Vec<U64LengthPrefixed<SigningBlockIdValuePair>>
}

#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct SigningBlockIdValuePair {
    pub id: u32,
    pub value: Vec<u8>
}

#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct SignatureSchemeV2Block {
    pub signers: U32LengthPrefixed<Vec<U32LengthPrefixed<Signer>>>
}

#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct SignatureSchemeV3Block {
    pub signers: U32LengthPrefixed<Vec<U32LengthPrefixed<V3Signer>>>
}

#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct Signer {
    pub signed_data: U32LengthPrefixed<SignedData>,
    pub signatures: U32LengthPrefixed<Vec<U32LengthPrefixed<Signature>>>,
    // SubjectPublicKeyInfo, ASN.1 DER form
    pub public_key: U32LengthPrefixed<Vec<u8>>
}

#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct V3Signer {
    pub signed_data: U32LengthPrefixed<V3SignedData>,

    pub min_sdk: u32,
    pub max_sdk: u32,

    pub signatures: U32LengthPrefixed<Vec<U32LengthPrefixed<Signature>>>,
    // SubjectPublicKeyInfo, ASN.1 DER form
    pub public_key: U32LengthPrefixed<Vec<u8>>
}

#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct Signature {
    pub signature_algorithm_id: SignatureAlgorithmId,
    pub signature: U32LengthPrefixed<Vec<u8>>
}

#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct SignedData {
    pub digests: U32LengthPrefixed<Vec<U32LengthPrefixed<Digest>>>,
    // Array of X.509 Certificates (ASN.1 DER form) as bytes
    pub certificates: U32LengthPrefixed<Vec<U32LengthPrefixed<Vec<u8>>>>,
    // PACK doesn't need these so we should just write 0 here
    pub additional_attributes: u32
}

#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct V3SignedData {
    pub digests: U32LengthPrefixed<Vec<U32LengthPrefixed<Digest>>>,
    // Array of X.509 Certificates (ASN.1 DER form) as bytes
    pub certificates: U32LengthPrefixed<Vec<U32LengthPrefixed<Vec<u8>>>>,
    pub min_sdk: u32,
    pub max_sdk: u32,
    // PACK doesn't need these so we should just write 0 here
    pub additional_attributes: u32
}

#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct Digest {
    pub signature_algorithm_id: SignatureAlgorithmId,
    pub digest: U32LengthPrefixed<Sha256Hash>
}

#[derive(Debug, PartialEq, DekuWrite, Clone)]
#[deku(id_type = "u32")]
pub enum SignatureAlgorithmId {
    #[deku(id = 0x0103)]
    RsaSsaPkcs1v1_5WithSha2_256
}

// Helper structures

// Outer APK Signing Block structures use u64 lengths
#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct U64LengthPrefixed<T: DekuWriter> {
    pub length: u64,
    pub value: T
}
// The "Integrity-protected contents" block uses u32 lengths
#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct U32LengthPrefixed<T: DekuWriter> {
    pub length: u32,
    pub value: T
}

#[derive(Debug, PartialEq, DekuWrite, Clone)]
pub struct RawWrapper<T: DekuWriter> {
    pub value: T
}

// Constructs length-prefixed things
pub fn len_pfx_u32<T: DekuWriter + Clone>(thing: T) -> U32LengthPrefixed<T> {
    let wrap = RawWrapper {
        value: thing.clone()
    };

    U32LengthPrefixed {
        length: wrap.to_bytes().unwrap().len() as u32,
        value: thing
    }
}

pub fn len_pfx_u64<T: DekuWriter + Clone>(thing: T) -> U64LengthPrefixed<T> {
    let wrap = RawWrapper {
        value: thing.clone()
    };

    U64LengthPrefixed {
        length: wrap.to_bytes().unwrap().len() as u64,
        value: thing
    }
}
