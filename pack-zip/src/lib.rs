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

// Android requires native libraries to be stored uncompressed and aligned to a page
// boundary before it will `mmap` them straight out of the APK/AAB instead of extracting a
// copy to disk (`android:extractNativeLibs="false"`). 16 KiB covers both the traditional
// 4 KiB page size and the 16 KiB page size newer devices require.
const NATIVE_LIB_ALIGNMENT: u16 = 16384;

// Matches `lib/<abi>/*.so` (APK) and `base/lib/<abi>/*.so` (AAB base module) — the two
// layouts native libraries are staged under, without depending on any particular ABI name.
fn is_native_lib(path: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    segments.len() >= 3
        && segments[segments.len() - 3] == "lib"
        && segments[segments.len() - 1].ends_with(".so")
}

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
    let native_lib_options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .with_alignment(NATIVE_LIB_ALIGNMENT);

    for file in files {
        let options = if is_native_lib(&file.path) {
            native_lib_options
        } else if UNCOMPRESSED_FILES.contains(&&file.path[..]) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn packed(files: &[File]) -> zip::ZipArchive<Cursor<Vec<u8>>> {
        let mut buf = Cursor::new(Vec::new());
        zip_apk(files, &mut buf).unwrap();
        zip::ZipArchive::new(buf).unwrap()
    }

    #[test]
    fn native_libs_are_stored_uncompressed_and_page_aligned() {
        let files = [
            File {
                path: "lib/arm64-v8a/libkite.so".into(),
                data: b"not really an elf".to_vec()
            },
            File {
                path: "base/lib/x86_64/libkite.so".into(),
                data: b"nor this one".to_vec()
            }
        ];
        let mut zip = packed(&files);
        for i in 0..zip.len() {
            let entry = zip.by_index(i).unwrap();
            assert_eq!(
                entry.compression(),
                CompressionMethod::Stored,
                "{}",
                entry.name()
            );
            assert_eq!(
                entry.data_start() % u64::from(NATIVE_LIB_ALIGNMENT),
                0,
                "{} not aligned to {NATIVE_LIB_ALIGNMENT}",
                entry.name()
            );
        }
    }

    #[test]
    fn resources_arsc_still_stored_uncompressed_and_native_libs_stay_out_of_it() {
        let files = [File {
            path: "resources.arsc".into(),
            data: b"arsc".to_vec()
        }];
        let mut zip = packed(&files);
        assert_eq!(
            zip.by_index(0).unwrap().compression(),
            CompressionMethod::Stored
        );
    }

    #[test]
    fn ordinary_files_are_still_deflated() {
        let files = [File {
            path: "AndroidManifest.xml".into(),
            data: b"<manifest/>".to_vec()
        }];
        let mut zip = packed(&files);
        assert_eq!(
            zip.by_index(0).unwrap().compression(),
            CompressionMethod::Deflated
        );
    }

    #[test]
    fn is_native_lib_matches_apk_and_aab_layouts_only() {
        assert!(is_native_lib("lib/arm64-v8a/libkite.so"));
        assert!(is_native_lib("base/lib/x86_64/libpython3.14.so"));
        // right extension, but "lib" isn't the ABI directory's parent
        assert!(!is_native_lib("assets/notlib/arm64-v8a/libfoo.so"));
        // "lib" appears, but not three segments from the end
        assert!(!is_native_lib("some/lib/deep/path/file.so"));
        // right directory shape, wrong extension
        assert!(!is_native_lib("lib/arm64-v8a/readme.txt"));
        assert!(!is_native_lib("resources.arsc"));
        assert!(!is_native_lib("classes.dex"));
    }
}
