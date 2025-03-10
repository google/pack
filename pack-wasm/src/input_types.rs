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

use serde::{Deserialize, Serialize};

// This comment can't be inside the struct due to
// https://github.com/rust-lang/rustfmt/issues/3379?issue=rust-lang%7Crustfmt%7C6347
// TODO: Optionally pass in custom PEM signing keys
#[derive(Debug, Serialize, Deserialize)]
pub struct PackWasmResource {
    pub subdirectory: String,
    pub name: String,
    pub contents_b64: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackWasmInput {
    pub resources: Vec<PackWasmResource>,
    pub manifest_b64: String,
    /// Contents of a `.pem` file containing both a `BEGIN CERTIFICATE` and `BEGIN PRIVATE KEY` section
    pub combined_pem_string: String,
    /// If `false`: Generates an APK file for local device testing.
    ///
    /// if `true`: Generates an Android App Bundle for Google Play.
    pub generate_aab: bool
}
