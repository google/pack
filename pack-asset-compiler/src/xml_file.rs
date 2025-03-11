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
use std::{
    collections::{HashMap, HashSet},
    io::{Read, Seek, SeekFrom},
    path::PathBuf
};

use crate::{
    generate_res_chunk,
    internal_android_attributes::{get_internal_attribute_id, internal_attribute_type},
    resource_external_types::*,
    resource_internal_types::Resource,
    string_pool::construct_string_pool,
    xml_first_pass::count_unique_android_internal_attributes
};
use deku::DekuContainerWrite;
use xml::{
    attribute::OwnedAttribute,
    name::OwnedName,
    reader::{EventReader, XmlEvent}
};

const ANDROID_NAMESPACE: &str = "http://schemas.android.com/apk/res/android";
const ANDROID_PREFIX: &str = "android";
// Version of AAPT2 we are emulating
const ANDROID_COMPILE_VERSION: &str = "34";
const ANDROID_COMPILE_CODENAME: &str = "14";
pub const ANDROID_INTERNAL_ATTRIBUTE_MAGIC: u32 = 0x0101_0000;

// Accounts for android:compileSdkVersion and android:compileSdkCodename, which
// we add ourselves.
const ANDROID_UNIQUE_ATTR_PADDING: usize = 2;

fn generate_xml_chunk<T: DekuContainerWrite>(chunk_type: ChunkType, chunk: T) -> Result<Vec<u8>> {
    let chunk_bytes = chunk.to_bytes()?;
    let node_header = XmlNodeChunk {
        line_number: 1,
        comment: UINT32_MINUS_ONE,
        node_data: chunk_bytes
    };
    Ok(generate_res_chunk(chunk_type, node_header, 8, 0)?.to_bytes()?)
}

fn generate_namspace_chunk(start: bool, prefix: u32, uri: u32) -> Result<Vec<u8>> {
    generate_xml_chunk(
        if start {
            ChunkType::XmlStartNamespace
        } else {
            ChunkType::XmlEndNamespace
        },
        XmlNamespaceChunk { prefix, uri }
    )
}

// If the XML file was a manifest, we can bubble some useful information up to the caller,
// such as the package name
pub struct ManifestInfo {
    pub package_name: Option<String>,
    // This is only required for AAB packaging
    pub label: Option<String>
}

// Encodes an XML file into an XmlFileType ResChunk
// Useful for AndroidManifest, but also things like strings and watch_face_info
// TODO: Refactor this massive function into some kind of struct with members and whatnot
pub fn xml_to_res_chunk<T: Read + Seek>(
    byte_source: &mut T,
    resources: &[Resource]
) -> Result<(ResChunk, ManifestInfo)> {
    let mut strings: Vec<String> = vec![];
    let mut string_ids: HashMap<String, u32> = HashMap::new();
    let mut seen_namespaces = HashSet::new();
    let mut namespace_stack: Vec<Vec<usize>> = vec![];
    let mut xml_resource_map: Vec<u32> = vec![];

    let unique_android_attrs =
        count_unique_android_internal_attributes(byte_source) + ANDROID_UNIQUE_ATTR_PADDING;
    // Send ptr back to the start for second pass over XML
    byte_source.seek(SeekFrom::Start(0)).unwrap();

    // These will all get replaced
    for _ in 0..unique_android_attrs {
        strings.push(String::from("TMP"));
    }

    // If the string already exists in the pool, return the existing ID
    // If not, add it to the pool and return the newly-created ID
    macro_rules! add_or_use_string {
        ($stringexpr:expr) => {{
            if let Some(id) = string_ids.get(&$stringexpr) {
                *id
            } else {
                let new_id = strings.len() as u32;
                strings.push($stringexpr);
                string_ids.insert($stringexpr, new_id);
                new_id
            }
        }};
    }

    macro_rules! add_or_use_android_string {
        ($stringexpr:expr) => {{
            if let Some(id) = string_ids.get(&$stringexpr) {
                *id
            } else {
                let next_android_string = xml_resource_map.len();
                // This should be impossible unless there's a mistake when we calculate
                // exactly how many we're gonna use
                if next_android_string >= unique_android_attrs {
                    return Err(PackError::TooManyUniqueAndroidInternalAttributes);
                }

                let internal_id = get_internal_attribute_id(&$stringexpr)?;
                let id_with_magic = ANDROID_INTERNAL_ATTRIBUTE_MAGIC | internal_id;
                xml_resource_map.push(id_with_magic);

                let new_id = next_android_string as u32;
                strings[next_android_string] = $stringexpr;
                string_ids.insert($stringexpr, new_id);
                new_id
            }
        }};
    }

    let mut manifest_info = ManifestInfo {
        package_name: None,
        label: None
    };
    let xml_source = EventReader::new(byte_source);
    let mut chunks: Vec<u8> = vec![];
    for event in xml_source {
        match event {
            // No Binary XML representation for this
            Ok(XmlEvent::StartDocument {
                version: _,
                encoding: _,
                standalone: _
            }) => {}
            Ok(XmlEvent::StartElement {
                name,
                attributes: imm_attributes,
                namespace
            }) => {
                let mut namespaces_defined_this_element = vec![];
                for ns in namespace.iter() {
                    // These are kind of fake namespaces, runtime Android doesn't
                    // care about these.
                    if ns.0.is_empty() || ns.0 == "tools" || ns.0 == "xml" || ns.0 == "xmlns" {
                        continue;
                    }
                    if seen_namespaces.contains(ns.0) {
                        continue;
                    }
                    seen_namespaces.insert(ns.0.to_string());
                    let prefix_id = add_or_use_string!(ns.0.to_string());
                    let uri_id = add_or_use_string!(ns.1.to_string());
                    chunks.extend(generate_namspace_chunk(true, prefix_id, uri_id)?);
                    namespaces_defined_this_element.push(prefix_id as usize);
                    namespaces_defined_this_element.push(uri_id as usize);
                }
                namespace_stack.push(namespaces_defined_this_element);

                let elem_name = name.local_name.to_string();
                let name_id = add_or_use_string!(elem_name.clone());
                let mut elem = XmlStartElementChunk {
                    name: name_id,
                    namespace: UINT32_MINUS_ONE,
                    // The size of this containing struct
                    attribute_start: 0x14,
                    // The size of XmlAttributeChunk (only coincidentally the same as the above)
                    attribute_size: 0x14,
                    attribute_count: 0,
                    id_index: 0,
                    class_index: 0,
                    style_index: 0,
                    attribute_data: vec![]
                };
                if let Some(ns) = name.namespace {
                    elem.namespace = add_or_use_string!(ns.to_string());
                }

                let mut attributes = imm_attributes.to_vec();
                if elem_name == "manifest" {
                    // Inject some values that AAPT itself injects
                    attributes.push(OwnedAttribute::new(
                        OwnedName::qualified(
                            "compileSdkVersion",
                            ANDROID_NAMESPACE,
                            Some(ANDROID_PREFIX)
                        ),
                        ANDROID_COMPILE_VERSION
                    ));
                    attributes.push(OwnedAttribute::new(
                        OwnedName::qualified(
                            "compileSdkCodename",
                            ANDROID_NAMESPACE,
                            Some(ANDROID_PREFIX)
                        ),
                        ANDROID_COMPILE_CODENAME
                    ));
                    attributes.push(OwnedAttribute::new(
                        OwnedName::local("platformBuildVersionCode"),
                        ANDROID_COMPILE_VERSION
                    ));
                    attributes.push(OwnedAttribute::new(
                        OwnedName::local("platformBuildVersionName"),
                        ANDROID_COMPILE_CODENAME
                    ));
                }

                for attr in attributes {
                    if let Some(ns) = &attr.name.prefix {
                        if ns == "tools" {
                            // Not a runtime-visible attribute
                            continue;
                        }
                    }

                    if elem_name == "manifest"
                        && attr.name.local_name == "package"
                        && attr.name.namespace.is_none()
                    {
                        manifest_info.package_name = Some(attr.value.clone());
                    }
                    if elem_name == "application"
                        && attr.name.local_name == "label"
                        && attr.name.namespace == Some(ANDROID_NAMESPACE.into())
                    {
                        manifest_info.label = Some(attr.value.clone());
                    }

                    let mut attr_type = AttributeDataType::String;
                    if attr.name.local_name == "platformBuildVersionCode"
                        || attr.name.local_name == "platformBuildVersionName"
                    {
                        attr_type = AttributeDataType::DecimalInteger;
                    }
                    if attr.value.starts_with("@") {
                        attr_type = AttributeDataType::Reference;
                    }
                    let name_id = if let Some(prefix) = &attr.name.prefix {
                        if prefix == "android" {
                            // Don't overwrite this in this case
                            if attr_type != AttributeDataType::Reference {
                                attr_type = internal_attribute_type(&attr.name.local_name);
                            }
                            add_or_use_android_string!(attr.name.local_name.clone())
                        } else {
                            add_or_use_string!(attr.name.local_name.clone())
                        }
                    } else {
                        add_or_use_string!(attr.name.local_name.clone())
                    };
                    let namespace_id = if let Some(ns) = attr.name.namespace {
                        add_or_use_string!(ns.clone())
                    } else {
                        UINT32_MINUS_ONE
                    };

                    let value_id = if attr_type == AttributeDataType::String {
                        add_or_use_string!(attr.value.clone())
                    } else {
                        0xFFFFFFFF
                    };
                    let typed_value = XmlAttributeDataChunk {
                        size: 8,
                        res0: 0,
                        data_type: attr_type.clone(),
                        data: match attr_type {
                            AttributeDataType::Reference => {
                                lookup_resource_id(&attr.value, resources)?
                            }
                            AttributeDataType::String => value_id,
                            AttributeDataType::DecimalInteger => attr.value.parse::<u32>()?,
                            AttributeDataType::BooleanInteger => {
                                if attr.value == "true" {
                                    1
                                } else {
                                    0
                                }
                            }
                        }
                    };

                    let attr_chunk = XmlAttributeChunk {
                        namespace: namespace_id,
                        name: name_id,
                        raw_value: value_id,
                        typed_value
                    };
                    elem.attribute_data.extend(attr_chunk.to_bytes()?);
                    elem.attribute_count += 1;
                }

                chunks.extend(generate_xml_chunk(ChunkType::XmlStartElement, elem)?);
            }
            Ok(XmlEvent::Whitespace(_)) => {}
            Ok(XmlEvent::EndElement { name }) => {
                let mut elem = XmlEndElementChunk {
                    name: *string_ids.get(&name.local_name.to_string()).unwrap(),
                    namespace: UINT32_MINUS_ONE
                };
                if let Some(ns) = &name.namespace {
                    elem.namespace = *string_ids.get(&ns.to_string()).unwrap();
                }
                chunks.extend(generate_xml_chunk(ChunkType::XmlEndElement, elem)?);
                let namepsaces_to_close = namespace_stack.pop().unwrap();
                for i in (0..namepsaces_to_close.len()).step_by(2) {
                    chunks.extend(generate_namspace_chunk(
                        false,
                        namepsaces_to_close[i] as u32,
                        namepsaces_to_close[i + 1] as u32
                    )?);
                }
            }
            Ok(XmlEvent::EndDocument) => {}
            Err(e) => return Err(PackError::XmlParsingFailed(e)),
            // TODO: Don't println from within this library crate, consumers might not want that
            _ => eprintln!("Warning: Unknown XML part: {:?}", event.unwrap())
        }
    }

    while xml_resource_map.len() < unique_android_attrs {
        xml_resource_map.push(UINT32_MINUS_ONE);
    }

    let xml_resource_map_chunk = generate_res_chunk(
        ChunkType::XmlResourceMap,
        XmlResourceMap {
            resources: xml_resource_map
        },
        0,
        0
    )?
    .to_bytes()?;

    let string_pool = construct_string_pool(&strings)?;
    let mut string_pool_bytes = string_pool.to_bytes()?;
    string_pool_bytes.extend(xml_resource_map_chunk);
    string_pool_bytes.extend(chunks);

    Ok((
        generate_res_chunk(
            ChunkType::XmlFile,
            RawBytes {
                data: string_pool_bytes
            },
            0,
            0
        )?,
        manifest_info
    ))
}

pub fn lookup_resource_id(reference: &str, resources: &[Resource]) -> Result<u32> {
    // Reference format: "@drawable/preview"
    // Trim @ and split
    let trimmed = String::from(&reference[1..]);
    let subdir_and_name: Vec<&str> = trimmed.split("/").collect();
    if subdir_and_name.len() != 2 {
        return Err(PackError::ReferenceAttributeParsingFailed(
            reference.to_string()
        ));
    }

    let mut res_type = 0;
    let mut res_id = 0;
    let mut subdir = String::new();
    for res in resources {
        if res.get_subdirectory() != subdir {
            subdir = res.get_subdirectory().into();
            res_type += 1;
            res_id = 0;
        }

        let res_name_path = PathBuf::from(res.get_name());
        let res_stem = res_name_path.file_stem();
        if res_stem.is_none() {
            res_id += 1;
            continue;
        }
        if res.get_subdirectory() == subdir_and_name[0] && res_stem.unwrap() == subdir_and_name[1] {
            // At this stage, we may be parsing an AndroidManifest.xml, in which case
            // we may not have built the resource table yet and we hit a chicken-and-egg
            // problem.
            // To avoid a circular dependency, we *predict* which ID the resource table
            // code will assign to the referenced resource.
            let predicted_res_id = 0x7F00_0000 | (res_type << 16) | res_id;
            return Ok(predicted_res_id);
        }
        res_id += 1;
    }

    Err(PackError::ReferenceAttributeLookupFailed(
        reference.to_string()
    ))
}
