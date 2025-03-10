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

use crate::{generate_res_chunk, resource_external_types::*};
use pack_common::*;

pub fn construct_string_pool(strings: &Vec<String>) -> Result<ResChunk> {
    let mut string_indicies: Vec<u32> = vec![];
    let mut string_data: Vec<u8> = vec![];
    for string in strings {
        let index = string_data.len() as u32;
        string_indicies.push(index);

        if string.len() > 0x7FFF {
            // I think normal AAPT2 would fall back to UTF-16 encoding here, since
            // that format has variable length count encoding, but in this case we
            // want to keep the source simple so we will just bail.
            // TODO: How common are strings that long?
            return Err(PackError::StringPoolStringTooLong(string.clone()));
        }

        let char_count = string.chars().count();
        let byte_count = string.len();
        if string.len() < 128 {
            string_data.push(char_count as u8);
            string_data.push(byte_count as u8);
        } else {
            string_data.push(0x80 | ((char_count >> 8) & 0xFF) as u8);
            string_data.push((char_count & 0b11111111) as u8);
            string_data.push(0x80 | ((byte_count >> 8) & 0xFF) as u8);
            string_data.push((byte_count & 0b11111111) as u8);
        }

        string_data.extend(string.bytes());
        string_data.push(0);
    }

    // String data is a u8 array, but AAPT requires all chunks to fall on
    // 32-bit boundaries. So we need to padd out to an even 4-bytes.
    // TODO: Move this to the generate_res_chunk function, it should apply to all chunks
    let padding = 4 - (string_data.len() % 4);
    string_data.resize(string_data.len() + padding, 0);

    let string_indicies_size_bytes = 4 * strings.len() as u32;
    let string_pool_header = StringPoolHeader {
        string_count: strings.len() as u32,
        style_count: 0,
        flags: STRING_POOL_UTF8_FLAG,
        strings_start: 0x1C + string_indicies_size_bytes,
        styles_start: 0
    };
    let string_pool_chunk = StringPoolChunk {
        string_pool_header,
        string_indicies,
        string_data
    };

    generate_res_chunk(ChunkType::StringPool, string_pool_chunk, 0x1C - 0x08, 0)
}
