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

// The use of the proto! macro causes some extraneous Default::default() calls,
// but they are harmless and unavoidable with the current design of the macro.
#![allow(clippy::needless_update)]

pub mod android {
    // Generated code from Protobufs does not follow Clippy documentation convention
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(rustdoc::invalid_html_tags)]
    pub mod bundle {
        include!(concat!(env!("OUT_DIR"), "/android.bundle.rs"));
    }
}
pub mod aapt {
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(rustdoc::invalid_html_tags)]
    pub mod pb {
        include!(concat!(env!("OUT_DIR"), "/aapt.pb.rs"));
    }
}
mod proto_util;
mod proto_xml;

use std::io::Cursor;

use aapt::pb::{
    file_reference, item, value, ConfigValue, Configuration, Entry, EntryId, FileReference, Item,
    Package, PackageId, ResourceTable, Source, StringPool, ToolFingerprint, Type, TypeId, Value,
    Visibility
};
use android::bundle::{BundleConfig, Bundletool};
use deku::prelude::*;
use pack_asset_compiler::{resource_internal_types::Resource, string_pool::construct_string_pool};
use pack_common::{PackError, Result};
use prost::Message;
use proto_xml::xml_string_to_proto_xml;

/// We will lie and claim to be this version of BundleTool
const BUNDLETOOL_SPOOF_VERSION: &str = "1.15.6";
const USER_PACKAGE_ID: u32 = 0x7F;

/// Creates a proto object for the `BundleConfig.pb` file which is required at the root
/// of an App Bundle.
///
/// Luckily, DWF uses very few of the available fields for this file.
fn construct_bundle_config() -> BundleConfig {
    inner_proto! {BundleConfig,
        bundletool: proto! {Bundletool,
            version: BUNDLETOOL_SPOOF_VERSION.into()
        }
    }
}

// TODO: Share this from somewhere common in asset-compiler
fn construct_resource_string_pool(
    resources: &mut [Resource],
    raw_application_label: &Option<String>
) -> Result<Vec<u8>> {
    // bundletool appears to prepend the app's android:label here, but I've
    // tested and it works with any arbitrary string.
    // For that reason, we'll follow what they do, but it's not a fatal
    // error if your app doesn't define a label.
    let application_label = if let Some(label) = raw_application_label {
        get_application_label(label, resources)?
    } else {
        "app"
    };
    let path_strings: Vec<String> = resources
        .iter()
        .map(|res| format!("{}/{}", application_label, res.get_string_pool_string()))
        .collect();
    // The real bundletool always adds a "" string at position 0, I would guess
    // for returning a ResourceID for any empty attribute, but it's unnecessary.
    Ok(construct_string_pool(&path_strings)?.to_bytes()?)
}

fn construct_tool_fingerprint() -> Vec<ToolFingerprint> {
    vec![ToolFingerprint {
        tool: "pack-aab".into(),
        version: env!("CARGO_PKG_VERSION").into()
    }]
}

fn construct_types_table(sorted_resources: &mut Vec<Resource>) -> Result<Vec<Type>> {
    let mut res_types = vec![];

    let mut previous_type = "".to_string();
    let mut type_id = 0;
    let mut current_type: Option<Type> = None;
    let mut entry_id = 0;
    // path_idx appears to be one-based
    let mut path_idx = 1;
    for res in sorted_resources {
        if res.get_subdirectory() != previous_type {
            type_id += 1;
            previous_type = res.get_subdirectory().into();

            if let Some(c_type) = &current_type {
                res_types.push(c_type.clone());
            }
            current_type = proto! {Type,
                type_id: proto!{TypeId, id: type_id },
                name: res.get_subdirectory().into()
            };
            entry_id = 0;
        }

        let value = match res {
            Resource::File(file) => {
                let path = file.get_path();
                let extension = match res.get_subdirectory() {
                    "xml" => file_reference::Type::ProtoXml,
                    "drawable" => file_reference::Type::Png,
                    _ => file_reference::Type::Unknown
                };

                item::Value::File(FileReference {
                    path,
                    r#type: extension as i32
                })
            }
            Resource::String(string) => item::Value::Str(aapt::pb::String {
                value: string.value.clone()
            })
        };

        let c_type = current_type.as_mut().unwrap();
        c_type.entry.push(inner_proto! {Entry,
            entry_id: proto! {EntryId,
              id: entry_id
            },
            name: res.get_basename()?,
            visibility: empty_proto!(Visibility),
            config_value: vec![ConfigValue {
                config: empty_proto!(Configuration),
                value: proto! {Value,
                    source: proto! {Source,
                        path_idx: path_idx
                    },
                    value: Some(value::Value::Item(inner_proto! {Item,
                        value: Some(value)
                    }))
                }
            }]
        });

        entry_id += 1;
        path_idx += 1;
    }
    if let Some(c_type) = &current_type {
        res_types.push(c_type.clone());
    }

    Ok(res_types)
}

fn construct_resource_table(
    package_name: &str,
    application_label: &Option<String>,
    resources: &mut Vec<Resource>
) -> Result<ResourceTable> {
    let string_pool = construct_resource_string_pool(resources, application_label)?;

    Ok(inner_proto! { ResourceTable,
        source_pool: proto! {StringPool, data: string_pool },
        package: vec![Package {
            package_id: proto! {PackageId, id: USER_PACKAGE_ID },
            package_name: package_name.into(),
            r#type: construct_types_table(resources)?
        }],
        tool_fingerprint: construct_tool_fingerprint()
    })
}

pub fn construct_aab(
    package_name: &str,
    application_label: &Option<String>,
    android_manifest: String,
    resources: &mut Vec<Resource>
) -> Result<Vec<pack_zip::File>> {
    let bundle_config = construct_bundle_config();
    let resource_table = construct_resource_table(package_name, application_label, resources)?;

    let mut files = vec![
        pack_zip::File {
            path: "BundleConfig.pb".into(),
            data: bundle_config.encode_to_vec()
        },
        pack_zip::File {
            path: "base/resources.pb".into(),
            data: resource_table.encode_to_vec()
        },
        pack_zip::File {
            path: "base/manifest/AndroidManifest.xml".into(),
            data: xml_string_to_proto_xml(&mut Cursor::new(android_manifest), resources)?
                .encode_to_vec()
        },
    ];

    let res_clone = resources.clone();
    for res in resources {
        if let Resource::File(res_file) = res {
            let res_bytes = if res_file.subdirectory == "xml" {
                let xml_node = xml_string_to_proto_xml(
                    &mut Cursor::new(res_file.contents.clone()),
                    &res_clone
                )?;
                xml_node.encode_to_vec()
            } else {
                // Other files can be dumped in verbatim
                res_file.contents.clone()
            };
            files.push(pack_zip::File {
                path: format!("base/{}", res_file.get_path()),
                data: res_bytes
            })
        }
    }

    Ok(files)
}

/// We have the string that was in the android:label="" attribute, but it might
/// be a reference to a resource ("@string/blah"), so we have to dereference it.
fn get_application_label<'a>(label_literal: &'a str, resources: &'a [Resource]) -> Result<&'a str> {
    if !label_literal.starts_with("@") {
        return Ok(label_literal);
    }

    let subdir_and_name: Vec<&str> = label_literal.split("/").collect();
    if subdir_and_name.len() != 2 {
        return Err(PackError::ReferenceAttributeParsingFailed(
            label_literal.to_string()
        ));
    }
    let name = subdir_and_name[1];

    for res in resources {
        if let Resource::String(str_res) = res {
            if str_res.name == name {
                return Ok(&str_res.value);
            }
        }
    }

    Err(PackError::ReferenceAttributeParsingFailed(
        label_literal.to_string()
    ))
}
