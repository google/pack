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

use crate::crypto_keys::Keys;
use deku::DekuContainerWrite;
use pack_common::*;
use rsa::Pkcs1v15Sign;
use sha2::{Digest, Sha256};

pub fn get_signature_for_signed_data<T: DekuContainerWrite>(
    signed_data: &T,
    keys: &Keys
) -> Result<Vec<u8>> {
    let digest = Sha256::digest(signed_data.to_bytes()?);
    let padding = Pkcs1v15Sign::new::<Sha256>();
    Ok(keys.private_key.sign(padding, &digest)?)
}
