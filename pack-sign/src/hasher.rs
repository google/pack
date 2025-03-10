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

use std::io::{Cursor, Seek, SeekFrom, Write};

use sha2::{Digest, Sha256};

use crate::zip_parser::ZipOffsets;
use pack_common::*;

pub type Sha256Hash = [u8; 32];

pub const BYTES_IN_1MB: u32 = 1024 * 1024;
pub const FIRST_LEVEL_CHUNK_MAGIC: &[u8] = &[0xa5];
pub const SECOND_LEVEL_CHUNK_MAGIC: &[u8] = &[0x5a];

pub fn compute_top_level_hash(
    apk_buf: &mut [u8],
    offsets: &ZipOffsets,
    signing_block_length: usize
) -> Result<Sha256Hash> {
    let first_level_hashes = compute_first_level_hashes(apk_buf, offsets, signing_block_length)?;

    let mut hasher = Sha256::new();
    hasher.update(SECOND_LEVEL_CHUNK_MAGIC);
    hasher.update((first_level_hashes.len() as u32).to_le_bytes());
    for hash in &first_level_hashes {
        hasher.update(hash);
    }
    let second_level_hash: Sha256Hash = hasher.finalize_reset().into();

    Ok(second_level_hash)
}

fn compute_first_level_hashes(
    apk_buf: &mut [u8],
    offsets: &ZipOffsets,
    signing_block_length: usize
) -> Result<Vec<Sha256Hash>> {
    // The Android Developer documentation calls these chunks 1, 3 and 4 because the
    //   APK Signing Block is chunk 2.
    let chunk1_range = 0..offsets.cd_start;
    let chunk3_range = offsets.cd_start..offsets.eocd_start;
    let chunk4_range = offsets.eocd_start..apk_buf.len();

    let mut first_level_hashes = vec![];

    // Chunk 1: APK contents before the central directory
    let chunk1 = &apk_buf[chunk1_range];
    first_level_hashes.extend(hash_chunk(chunk1));

    // Chunk 3: Central directories
    let chunk3 = &apk_buf[chunk3_range];
    first_level_hashes.extend(hash_chunk(chunk3));

    // Chunk 4 is more complex because we need to modify the EOCD offset to account
    //   for the APK Signing Block, BUT WE HASH BEFORE WRITING THE UPDATED OFFSET!
    //   From my reading of the docs, this is the opposite to what they say. Perhaps
    //   the wording is unclear or the doc needs to be updated.
    let chunk4 = &apk_buf[chunk4_range.clone()];
    first_level_hashes.extend(hash_chunk(chunk4));

    let new_cd_start = offsets.cd_start + signing_block_length;
    let mut cursor = Cursor::new(&mut apk_buf[chunk4_range]);
    cursor.seek(SeekFrom::Start(16))?;
    cursor.write_all(&(new_cd_start as u32).to_le_bytes())?;

    Ok(first_level_hashes)
}

fn hash_chunk(chunk: &[u8]) -> Vec<Sha256Hash> {
    // TODO: Is it more performant or something to share this as a singleton?
    let mut hasher = Sha256::new();
    let mut chunk_hashes = vec![];
    let mut pos = 0;

    while pos < chunk.len() {
        // Each chunk is 1MB OR whatever's left in the buffer
        let end = (pos + BYTES_IN_1MB as usize).min(chunk.len());
        let chunk_size = end - pos;
        hasher.update(FIRST_LEVEL_CHUNK_MAGIC);
        hasher.update((chunk_size as u32).to_le_bytes());
        hasher.update(&chunk[pos..end]);
        chunk_hashes.push(hasher.finalize_reset().into());
        pos = end;
    }

    chunk_hashes
}
