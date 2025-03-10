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

// The "XML first pass" is used to determine how many unique "android:" prefixed attributes are
// present in a file. This is so that we can pack the optimal String Pool length into the APK
// and reduce file size.
use std::{collections::HashSet, io::Read};

use xml::{reader::XmlEvent, EventReader};

pub fn count_unique_android_internal_attributes<T: Read>(byte_source: &mut T) -> usize {
    let mut unique_attrs_count = 0;
    let xml_source = EventReader::new(byte_source);
    let mut seen_attr_names = HashSet::new();
    for event in xml_source.into_iter().flatten() {
        if let XmlEvent::StartElement {
            name: _name,
            attributes,
            namespace: _namespace
        } = event
        {
            for attr in attributes {
                if let Some(prefix) = &attr.name.prefix {
                    if prefix == "android" && !seen_attr_names.contains(&attr.name.local_name) {
                        seen_attr_names.insert(attr.name.local_name.clone());
                        unique_attrs_count += 1;
                    }
                }
            }
        }
    }
    unique_attrs_count
}
