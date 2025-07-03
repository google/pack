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

use pack_common::{PackError, Result};

use crate::resource_external_types::AttributeDataType;

// See get_internal_attribute_id
include!(concat!(env!("OUT_DIR"), "/internal_attributes_map.rs"));

pub fn infer_attribute_type(value: &String) -> AttributeDataType {
    if value.parse::<u32>().is_ok() {
        AttributeDataType::DecimalInteger
    } else if value == "true" || value == "false" {
        AttributeDataType::BooleanInteger
    } else if value.starts_with("@") {
        AttributeDataType::Reference
    } else {
        AttributeDataType::String
    }
}

/// The Android Internal Attributes (android:name, android:compileSdkVersion
/// etc.) all have internal IDs which are important to know and look up.
/// Since there are over 1,400 of them, an indexOf() style look up is incredibly
/// inefficient, and since they are not sorted, binary search is unhelpful.
/// Therefore, we build a lookup table at compile-time and read it here.
pub fn get_internal_attribute_id(attr: &str) -> Result<u32> {
    INTERNAL_ATTRIBUTES_MAP
        .get(attr)
        .ok_or(PackError::UnknownAndroidInternalAttribute(attr.into()))
        .copied()
}
