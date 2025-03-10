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

// The res/values/strings.xml file is parsed separately and specially.
// It's not a path-referenced resource like drawables, the strings all
// go *directly* into resources.arsc
use std::io::Read;

use xml::{reader::XmlEvent, EventReader};

use crate::resource_internal_types::{Resource, StringResource};

pub fn parse_strings_xml<T: Read>(byte_source: &mut T) -> Vec<Resource> {
    let xml_source = EventReader::new(byte_source);
    let mut strings = vec![];
    let mut next_string_name: Option<String> = None;

    for event in xml_source {
        match event {
            Ok(XmlEvent::StartElement {
                name,
                attributes,
                namespace: _namespace
            }) => {
                if name.local_name == "string" {
                    for attr in attributes {
                        if attr.name.local_name == "name" {
                            next_string_name = Some(attr.value);
                        }
                    }
                }
            }
            Ok(XmlEvent::Characters(chars)) => {
                if let Some(string_name) = &next_string_name {
                    strings.push(Resource::String(StringResource {
                        resource_id: 0,
                        name: string_name.clone(),
                        value: chars
                    }))
                }
                // Else this was some other random text in the file, not in a <string /> tag
                // Ignore this for resilience
            }
            // Don't care about most structural elements
            _ => {}
        }
    }

    strings
}
