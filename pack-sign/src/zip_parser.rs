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

use byteorder::{LittleEndian, ReadBytesExt};
use pack_common::*;
use std::io::Cursor;

#[derive(Default, Debug)]
pub struct ZipOffsets {
    // Central Directory (from start of file)
    pub cd_start: usize,
    // End of Central Directory (from start of file)
    pub eocd_start: usize
}

pub const EOCD_MAGIC: &[u8; 4] = &[0x50, 0x4B, 0x05, 0x06];

pub fn find_offsets(zip_buf: &[u8]) -> Result<ZipOffsets> {
    let mut offsets = ZipOffsets::default();
    for i in (0..=(zip_buf.len() - 4)).rev() {
        let magic = &zip_buf[i..(i + 4)];
        if magic == EOCD_MAGIC {
            // Found the end of central directory!
            offsets.eocd_start = i;

            // The EOCD also tells us where the central directories start
            let mut eocd_cd_start_field = Cursor::new(&zip_buf[(i + 16)..(i + 20)]);
            let cd_start = eocd_cd_start_field.read_u32::<LittleEndian>()?;
            offsets.cd_start = cd_start as usize;
            break;
        }
    }

    match offsets.cd_start {
        // Couldn't find the central directory
        0 => Err(PackError::SignerZipParsingFailed),
        _ => Ok(offsets)
    }
}
