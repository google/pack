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

// AAB uses a different format for representing XML files, it is not the same as
// APK. It's again a set of opaque bytes with the same ".xml" extension.
//
// In this case, it's ProtoXML, which seems to have been invented for bundletool.
// This is different to the ResChunkXML which was invented for AAPT.

use std::{collections::HashSet, io::Read};

use pack_asset_compiler::{
    internal_android_attributes::{get_internal_attribute_id, infer_attribute_type},
    resource_external_types::AttributeDataType,
    resource_internal_types::Resource,
    xml_file::{lookup_resource_id, ANDROID_INTERNAL_ATTRIBUTE_MAGIC}
};
use pack_common::{PackError, Result};
use xml::{attribute::OwnedAttribute, common::Position, reader::XmlEvent, EventReader};

use crate::aapt::pb::{
    item, primitive, reference, xml_node::Node, Item, Primitive, Reference, SourcePosition,
    XmlAttribute, XmlElement, XmlNamespace, XmlNode
};

// NOTE: This is very, VERY similar to xml_to_res_chunk. In future could
//   generalise this. They are two ways to define very similar data.
// TODO: Inject compileSdkVersion and friends
pub fn xml_string_to_proto_xml<T: Read>(
    byte_source: &mut T,
    resources: &[Resource]
) -> Result<XmlNode> {
    let mut xml_source = EventReader::new(byte_source);
    let mut xml_out = XmlNode::default();
    let mut child_idx_stack: Vec<usize> = vec![];
    let mut seen_namespaces = HashSet::new();

    loop {
        let event = xml_source.next();
        let source_position = Some(SourcePosition {
            line_number: xml_source.position().row as u32,
            column_number: xml_source.position().column as u32
        });
        match event {
            Ok(XmlEvent::StartElement {
                name,
                attributes,
                namespace
            }) => {
                let mut namespaces_defined_in_this_element = vec![];
                for ns in namespace.iter() {
                    // These are kind of fake namespaces, runtime Android doesn't
                    // care about these.
                    // NOTE: This is subtly different to the ones used for ResChunk XML,
                    //   because bundletool *does* care about "tools"
                    if ns.0.is_empty() || ns.0 == "xml" || ns.0 == "xmlns" {
                        continue;
                    }
                    if seen_namespaces.contains(ns.0) {
                        continue;
                    }
                    seen_namespaces.insert(ns.0.to_string());
                    namespaces_defined_in_this_element.push(XmlNamespace {
                        prefix: ns.0.to_string(),
                        uri: ns.1.to_string(),
                        source: source_position
                    });
                }

                let new_element = Node::Element(XmlElement {
                    name: name.local_name,
                    namespace_uri: name.namespace.unwrap_or("".into()),
                    namespace_declaration: namespaces_defined_in_this_element,
                    attribute: attributes
                        .iter()
                        .map(|attr| parser_attr_to_proto_attr(attr, resources))
                        .collect::<Result<Vec<_>>>()?,
                    child: vec![]
                });

                if xml_out.node.is_none() {
                    // First element
                    xml_out.node = Some(new_element);
                } else {
                    let new_node = XmlNode {
                        node: Some(new_element),
                        source: source_position
                    };
                    let mut parent = node_to_elem(&mut xml_out)?;
                    for child_idx in &child_idx_stack {
                        parent = node_to_elem(&mut parent.child[*child_idx])?;
                    }
                    child_idx_stack.push(parent.child.len());
                    parent.child.push(new_node);
                }
            }
            Ok(XmlEvent::EndElement { .. }) => {
                child_idx_stack.pop();
            }
            Ok(XmlEvent::EndDocument) => break,
            Err(e) => return Err(PackError::XmlParsingFailed(e)),
            _ => {}
        }
    }

    Ok(xml_out)
}

fn parser_attr_to_proto_attr(
    p_attr: &OwnedAttribute,
    resources: &[Resource]
) -> Result<XmlAttribute> {
    let mut compiled_value: Option<item::Value> = None;
    let resource_id = if p_attr.name.prefix.clone().unwrap_or("".into()) == "android" {
        // This is an internal attribute
        let attr_type = infer_attribute_type(&p_attr.name.local_name);
        compiled_value = match attr_type {
            AttributeDataType::DecimalInteger => Some(item::Value::Prim(Primitive {
                oneof_value: Some(primitive::OneofValue::IntDecimalValue(
                    p_attr.value.parse::<i32>()?
                ))
            })),
            AttributeDataType::BooleanInteger => Some(item::Value::Prim(Primitive {
                oneof_value: Some(primitive::OneofValue::BooleanValue(p_attr.value == "true"))
            })),
            // References will be caught anyway when they begin with @
            // And internal strings don't get a type wrapper
            _ => None
        };

        let internal_id = get_internal_attribute_id(&p_attr.name.local_name)?;
        ANDROID_INTERNAL_ATTRIBUTE_MAGIC | internal_id
    } else {
        0
    };

    if p_attr.value.starts_with("@") {
        // This is a reference
        let res_id = lookup_resource_id(&p_attr.value, resources)?;
        compiled_value = Some(item::Value::Ref(Reference {
            r#type: reference::Type::Reference as i32,
            id: res_id,
            // Trim the @
            name: String::from(&p_attr.value[1..]),
            // I don't know why. Saw this in real bundletool output.
            type_flags: 0xFFFF,
            ..Reference::default()
        }));
    }

    Ok(XmlAttribute {
        namespace_uri: p_attr.name.namespace.clone().unwrap_or("".into()),
        name: p_attr.name.local_name.clone(),
        value: p_attr.value.clone(),
        source: None,
        resource_id,
        compiled_item: compiled_value.map(|val| Item {
            value: Some(val),
            ..Item::default()
        })
    })
}

fn node_to_elem(node: &mut XmlNode) -> Result<&mut XmlElement> {
    match &mut node.node {
        Some(Node::Element(elem)) => Ok(elem),
        _ => Err(PackError::ProtoXmlNodeIsNotAnElement)
    }
}
