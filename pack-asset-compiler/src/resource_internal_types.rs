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
// Types that are used internally to describe Resources
use pack_common::*;
use std::io::Cursor;

use crate::xml_file::xml_to_res_chunk;

// TODO: Factor common values like name and resource_id into a parent struct with an
//   enum for just the value
/// Represents a part of the `res/` directory within an Android package.
#[derive(Debug, Clone)]
pub enum Resource {
    File(FileResource),
    String(StringResource)
}

/// Represents any non-string resource file
#[derive(Debug, Clone)]
pub struct FileResource {
    /// eg. `drawable`
    pub subdirectory: String,
    /// eg. `preview.png`
    pub name: String,
    /// Starts as 0, populated by the asset complier
    pub resource_id: u32,
    /// Contents of the file in bytes.
    pub contents: Vec<u8>
}

impl FileResource {
    /// Returns the path to this file within an APK or AAB Module, for example `res/drawable/preview.png`.
    pub fn get_path(&self) -> String {
        format!("res/{}/{}", self.subdirectory, self.name)
    }

    /// Returns the name of the resource file without its file extension.
    pub fn get_basename(&self) -> Result<String> {
        Ok(self.name.split('.').next().unwrap_or("").to_string())
    }

    /// Returns a [FileResource] representing the file in question
    ///
    /// If using `pack-api`, you can provide a FileResource for `strings.xml`, and it will
    /// automatically be parsed into a series of [StringResource]s
    pub fn new(subdirectory: String, name: String, contents: Vec<u8>) -> Self {
        FileResource {
            subdirectory,
            name,
            resource_id: 0,
            contents
        }
    }

    /// Returns the `Vec<u8>` to be placed into an APK to represent this file. For most
    /// files, that's just the contents. For files in the XML directory, they are compiled
    /// to a [special format](https://cs.android.com/android/platform/superproject/main/+/main:frameworks/base/libs/androidfw/include/androidfw/ResourceTypes.h;l=244)
    /// unique to AAPT.
    pub fn as_bytes_for_apk(&self, resources: &[Resource]) -> Result<Vec<u8>> {
        if self.subdirectory == "xml" {
            let (parsed_xml_res_chunk, _) =
                xml_to_res_chunk(&mut Cursor::new(self.contents.clone()), resources)?;
            Ok(parsed_xml_res_chunk.to_bytes()?)
        } else {
            // Other files can be dumped in verbatim
            // TODO: Can we just consume this? Cloning is wasteful for large resources
            // TODO: res/drawable resources can be PNG-crushed. AAPT2 does. libimagequant seems perfect.
            Ok(self.contents.clone())
        }
    }
}

/// Represents a key-value pair from `strings.xml`.
#[derive(Debug, Clone)]
pub struct StringResource {
    /// eg. "app_name"
    pub name: String,
    /// eg. "Analogue"
    pub value: String,
    /// Can start as 0, construct_resource_table fills it in
    pub resource_id: u32
}

impl Resource {
    /// Returns the directory after `res/` in which this resource resides, eg. `drawable`.
    pub fn get_subdirectory(&self) -> &str {
        match self {
            Resource::File(file) => &file.subdirectory[..],
            // String resources live in values/strings.xml
            // But they get reported in the APK as "string"
            Resource::String(_) => "string"
        }
    }

    /// Returns the value that needs to be put into the string pool for this resource. For [files](FileResource)
    /// that's relative paths, for [strings](StringResource) that's their actual values.
    pub fn get_string_pool_string(&self) -> String {
        match self {
            Resource::File(file) => file.get_path(),
            Resource::String(sres) => sres.value.clone()
        }
    }

    /// Returns the name of the resource. In the case of a [FileResource], this includes its extension,
    /// eg. `image1.png`. In the case of a [StringResource], this is the name of the string, eg. `confirm_text`.
    pub fn get_name(&self) -> &str {
        match self {
            Resource::File(file) => &file.name[..],
            Resource::String(sres) => &sres.name[..]
        }
    }

    /// Returns the name of the resource without its file extension. For [String Resources](StringResource),
    /// this is equivalent to [get_name](Resource::get_name).
    pub fn get_basename(&self) -> Result<String> {
        match self {
            Resource::File(file) => file.get_basename(),
            Resource::String(sres) => Ok(sres.name.to_string())
        }
    }

    /// Returns the resource's ID ***if*** it has been compiled. This method is not usually useful outside of
    /// internal code unless you are assembling APKs yourself using lower-level APIs.
    ///
    /// If using `pack-api`, this will always be `0` by default because `-api` operates on a _copy_ of the
    /// resource array internally.
    pub fn get_resource_id(&self) -> u32 {
        match self {
            Resource::File(file) => file.resource_id,
            Resource::String(sres) => sres.resource_id
        }
    }

    /// Helper for setting the `resource_id` field regardless of whether you know this is a [FileResource] or
    /// [StringResource].
    pub fn set_resource_id(&mut self, res_id: u32) {
        match self {
            Resource::File(file) => file.resource_id = res_id,
            Resource::String(sres) => sres.resource_id = res_id
        }
    }
}
