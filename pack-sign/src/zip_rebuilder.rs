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

use deku::DekuContainerWrite;
use pack_common::*;

use crate::{signing_types::ApkSigningBlock, zip_parser::ZipOffsets};

pub fn rebuild_zip_with_signing_block(
    offsets: &ZipOffsets,
    zip_buf: &[u8],
    signing_block: ApkSigningBlock
) -> Result<Vec<u8>> {
    let chunk1_range = 0..offsets.cd_start;
    let chunk3_range = offsets.cd_start..offsets.eocd_start;
    let chunk4_range = offsets.eocd_start..zip_buf.len();

    let mut final_apk: Vec<u8> = vec![];
    let signing_block_bytes = signing_block.to_bytes()?;

    final_apk.extend(&zip_buf[chunk1_range]);
    final_apk.extend(&signing_block_bytes);
    final_apk.extend(&zip_buf[chunk3_range]);
    final_apk.extend(&zip_buf[chunk4_range]);

    // Et voila
    Ok(final_apk)
}
