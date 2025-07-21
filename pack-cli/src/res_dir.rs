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

use pack_api::{FileResource, Result};
use std::{fs, io::Read, path::PathBuf};

pub fn read_res_dir(res_path: &PathBuf) -> Result<Vec<FileResource>> {
    let mut resources = vec![];
    let res_types = fs::read_dir(res_path)?;
    for res_type in res_types {
        // TODO: Use a better pattern here
        if let Ok(entry) = &res_type {
            if let Ok(metadata) = &entry.metadata() {
                if metadata.is_dir() {
                    collect_resources(&entry.path(), &mut resources);
                    continue;
                }
            }
        }
        eprintln!("Warning: Ignoring unusable res/ entry {res_type:?}")
    }
    Ok(resources)
}

fn collect_resources(path: &PathBuf, resources: &mut Vec<FileResource>) {
    let res_name = path.file_name().unwrap().to_string_lossy();
    let maybe_resource_files = fs::read_dir(path);
    if let Err(err) = maybe_resource_files {
        eprintln!("Warning: Failed to read res/ subdirectory {res_name} {err:?}");
        return;
    }
    let resource_files = maybe_resource_files.unwrap();
    for file in resource_files {
        if let Ok(entry) = &file {
            if let Ok(metadata) = &entry.metadata() {
                if !metadata.is_dir() {
                    if let Ok(mut file) = fs::File::open(entry.path()) {
                        let mut file_buf = vec![0; metadata.len() as usize];
                        if let Ok(_read_length) = file.read(&mut file_buf) {
                            resources.push(FileResource {
                                subdirectory: res_name.clone().into(),
                                name: entry.file_name().to_string_lossy().into(),
                                resource_id: 0,
                                contents: file_buf
                            });
                            continue;
                        }
                    }
                }
            }
        }
        eprintln!("Warning: Ignoring unusable {res_name} resource entry {file:?}")
    }
}
