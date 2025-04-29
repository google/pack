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

use deku::prelude::*;
use pack_common::*;
use std::collections::HashMap;

use crate::{
    generate_res_chunk,
    resource_external_types::{
        AttributeDataType, ChunkType, RawBytes, ResChunk, TableConfigChunk, TableEntry,
        TableHeaderChunk, TablePackageChunk, TableTypeChunk, TableTypeSpecChunk,
        XmlAttributeDataChunk
    },
    resource_internal_types::Resource,
    string_pool::construct_string_pool
};

const USER_PACKAGE_MAGIC: u32 = 0x7F;

pub fn construct_resource_table(
    package_name: &str,
    resources: &mut [Resource]
) -> Result<ResChunk> {
    let res_types = get_unique_res_types(resources);
    let res_buckets = get_res_type_buckets(resources);
    let res_basenames: Vec<String> = resources
        .iter()
        .map(|res| res.get_basename())
        .collect::<Result<Vec<String>>>()?;

    let mut data: Vec<u8> = vec![];

    // Add a header for the table we're about to construct
    data.extend(TableHeaderChunk { package_count: 1 }.to_bytes()?);

    let path_strings: Vec<String> = resources
        .iter()
        .map(|res| res.get_string_pool_string())
        .collect();
    let path_string_pool = construct_string_pool(&path_strings)?.to_bytes()?;
    data.extend(path_string_pool);

    let res_types_string_pool = construct_string_pool(&res_types)?.to_bytes()?;
    let res_basenames_string_pool = construct_string_pool(&res_basenames)?.to_bytes()?;

    let mut res_type_data: Vec<u8> = vec![];
    let mut absolute_entry = 0;
    for (i, res_type) in res_types.iter().enumerate() {
        // This is 1-based
        let res_type_id = i as u8 + 1;
        let entry_count = res_buckets.get(res_type).unwrap().len() as u32;
        // Generate a TableTypeSpec for each resouce type
        let type_spec = TableTypeSpecChunk {
            id: res_type_id,
            res0: 0,
            // Reserved 0
            types_count: 0,
            entry_count,
            configuration_change_flags: vec![0; entry_count as usize]
        };
        res_type_data
            .extend(generate_res_chunk(ChunkType::TableTypeSpec, type_spec, 8, 0)?.to_bytes()?);

        // Generate a TableType for each resource type
        let mut entry_data: Vec<u8> = vec![];
        let mut offsets: Vec<u32> = vec![];
        for j in 0..entry_count {
            offsets.push(16 * j);
            resources[absolute_entry as usize]
                .set_resource_id(0x7F00_0000 | ((res_type_id as u32) << 16) | j);
            let entry = TableEntry {
                size: 8,
                flags: 0,
                key: absolute_entry,
                value: XmlAttributeDataChunk {
                    size: 8,
                    res0: 0,
                    data_type: AttributeDataType::String,
                    // TODO: Not sure if this is right
                    data: absolute_entry
                }
            };
            entry_data.extend(entry.to_bytes()?);
            absolute_entry += 1;
        }
        let type_chunk = TableTypeChunk {
            id: res_type_id,
            flags: 0,
            reserved: 0,
            entry_count,
            entries_start: 0x54 + offsets.len() as u32 * 4,
            config: TableConfigChunk {
                size: 64,
                data: [0; 60]
            },
            offsets
        };
        res_type_data.extend(
            generate_res_chunk(
                ChunkType::TableType,
                type_chunk,
                0x54 - 8,
                entry_data.len() as u16
            )?
            .to_bytes()?
        );
        res_type_data.extend(entry_data);
    }

    let table_package_chunk = generate_res_chunk(
        ChunkType::TablePackage,
        TablePackageChunk {
            id: USER_PACKAGE_MAGIC,
            name: get_padded_package_name(package_name)?,
            // This is the same as the header size, means type_strings begins immediately
            type_string_offset: 0x120,
            last_public_type: 0,
            key_string_offset: 0x120 + res_types_string_pool.len() as u32,
            last_public_key: 0,
            type_id_offset: 0
        },
        // The whole chunk before the string pools is considered "header"
        0x120 - 8,
        (res_types_string_pool.len() + res_basenames_string_pool.len() + res_type_data.len())
            as u16
    )?;
    data.extend(table_package_chunk.to_bytes()?);
    data.extend(res_types_string_pool);
    data.extend(res_basenames_string_pool);

    data.extend(res_type_data);

    generate_res_chunk(ChunkType::Table, RawBytes { data }, 4, 0)
}

// Returns the package name in zero-padded 128 UTF-16 characters
fn get_padded_package_name(package_name: &str) -> Result<Vec<u16>> {
    if package_name.len() > 128 {
        return Err(PackError::PackageNameTooLong(package_name.into()));
    }
    let mut out_vec = vec![0; 128];
    let utf16str: Vec<u16> = package_name.encode_utf16().collect();
    out_vec[..utf16str.len()].copy_from_slice(&utf16str[..]);
    Ok(out_vec)
}

pub fn get_unique_res_types(resources: &[Resource]) -> Vec<String> {
    let mut unique_vec = vec![];
    for res in resources {
        let subdir = res.get_subdirectory().to_string();
        if !unique_vec.contains(&subdir) {
            unique_vec.push(subdir);
        }
    }
    unique_vec
}

fn get_res_type_buckets(resources: &[Resource]) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    for res in resources {
        let subdir = res.get_subdirectory().to_string();
        if !map.contains_key(&subdir) {
            map.insert(subdir.clone(), vec![]);
        }
        map.get_mut(&subdir)
            .unwrap()
            .push(res.get_name().to_string());
    }
    map
}
