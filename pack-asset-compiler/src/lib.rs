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
use resource_external_types::{ChunkType, ResChunk, ResChunkHeader};

pub mod internal_android_attributes;
pub mod resource_external_types;
pub mod resource_internal_types;
pub mod resource_table;
pub mod string_pool;
pub mod strings_xml_parser;
pub mod xml_file;
pub mod xml_first_pass;

pub fn generate_res_chunk<T: DekuContainerWrite>(
    chunk_type: ChunkType,
    data: T,
    extra_header_size: u16,
    extra_chunk_size: u16
) -> Result<ResChunk> {
    let data_bytes = data.to_bytes()?;
    let data = ResChunk {
        header: ResChunkHeader {
            chunk_type,
            header_size: 0x08 + extra_header_size,
            chunk_size: 0x08 + extra_chunk_size as u32 + data_bytes.len() as u32
        },
        data: data_bytes
    };
    if data.header.chunk_size % 4 != 0 {
        unimplemented!("Generic chunk alignment ({:?})", data);
    }
    Ok(data)
}
