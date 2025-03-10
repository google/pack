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

use pack_api::{compile_and_sign_aab, compile_and_sign_apk, FileResource, Keys, Package};

use base64::{engine::general_purpose, Engine};
use input_types::PackWasmInput;
use wasm_bindgen::prelude::*;

mod input_types;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[allow(unused_macros)]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

// Builds and signs an APK in-memory and returns it in Base64
#[wasm_bindgen]
pub fn build(input: JsValue) -> std::result::Result<String, String> {
    let input: PackWasmInput = serde_wasm_bindgen::from_value(input)
        .map_err(|e| format!("JS object input did not match expected format\n{:?}", e))?;

    let android_manifest = b64_to_bytes(&input.manifest_b64[..])?;

    // Turn the input resources into api::Resources
    let resources: Vec<FileResource> = input
        .resources
        .iter()
        .map(|wasm_res| {
            Ok::<FileResource, String>(FileResource::new(
                wasm_res.subdirectory.clone(),
                wasm_res.name.clone(),
                b64_to_bytes(&wasm_res.contents_b64)?
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let signing_keys = Keys::from_combined_pem_string(&input.combined_pem_string)?;

    let pkg = Package {
        android_manifest,
        resources
    };

    if input.generate_aab {
        Ok(bytes_to_b64(&compile_and_sign_aab(&pkg, &signing_keys)?))
    } else {
        Ok(bytes_to_b64(&compile_and_sign_apk(&pkg, &signing_keys)?))
    }
}

fn b64_to_bytes(b64: &str) -> std::result::Result<Vec<u8>, String> {
    // Slightly unusual API
    general_purpose::STANDARD
        .decode(b64.as_bytes())
        .map_err(|e| format!("Failed to decode Base64\n{:?}", e))
}

fn bytes_to_b64(bytes: &Vec<u8>) -> String {
    general_purpose::STANDARD.encode(bytes)
}
