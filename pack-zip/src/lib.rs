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

use pack_common::*;
use std::io::{Seek, Write};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

pub struct File {
    pub path: String,
    pub data: Vec<u8>
}

const UNCOMPRESSED_FILES: &[&str] = &["resources.arsc"];

// Output can be a file *or* a buffer in memory
pub fn zip_apk<T: Write + Seek>(files: &[File], output: T) -> Result<()> {
    let mut zip = ZipWriter::new(output);
    let compressed_options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .with_alignment(4);
    // Some files in APKs are not allowed to be compressed
    // TODO: AAPT2 doesn't compress drawable PNGs, but maybe it could?
    let uncompressed_options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .with_alignment(4);

    for file in files {
        let options = if UNCOMPRESSED_FILES.contains(&&file.path[..]) {
            uncompressed_options
        } else {
            compressed_options
        };
        zip.start_file_from_path(&file.path, options).unwrap();
        zip.write_all(&file.data)?;
    }

    zip.finish()?;
    Ok(())
}
