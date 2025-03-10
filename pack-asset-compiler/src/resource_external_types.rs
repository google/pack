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

// Types that Android/APKs themselves use to describe resources
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuWrite)]
pub struct ResChunk {
    pub header: ResChunkHeader,
    pub data: Vec<u8>
}

pub const RES_CHUNK_HEADER_SIZE: u32 = 8;
pub const UINT32_MINUS_ONE: u32 = 0xFFFFFFFF;
// Either a string index or UINT32_MINUS_ONE if empty
pub type ResStringPoolRef = u32;

#[derive(Debug, PartialEq, DekuWrite)]
pub struct ResChunkHeader {
    pub chunk_type: ChunkType,
    pub header_size: u16,
    // Includes both this header and the data that follows
    pub chunk_size: u32
}

#[derive(Debug, PartialEq, DekuWrite)]
pub struct XmlNodeChunk {
    // Where this node appeared in the original document
    // Not important for on-device parsing, only debugging and logs
    pub line_number: u32,
    // The XML comment that originally appeared above this note
    pub comment: ResStringPoolRef,

    // TODO: Don't like this structure
    pub node_data: Vec<u8>
}

#[derive(Debug, PartialEq, DekuWrite)]
pub struct XmlResourceMap {
    pub resources: Vec<u32>
}

#[derive(Debug, PartialEq, DekuWrite)]
pub struct RawBytes {
    pub data: Vec<u8>
}

// Used for both the start and end of a namespace
#[derive(Debug, PartialEq, DekuWrite)]
pub struct XmlNamespaceChunk {
    pub prefix: ResStringPoolRef,
    pub uri: ResStringPoolRef
}

// Used for both the start and end of an element
#[derive(Debug, PartialEq, DekuWrite)]
pub struct XmlStartElementChunk {
    pub namespace: ResStringPoolRef,
    pub name: ResStringPoolRef,
    pub attribute_start: u16,
    pub attribute_size: u16,
    pub attribute_count: u16,
    // Index (1-based) of the "id" attribute, 0 if none
    pub id_index: u16,
    // Index (1-based) of the "class" attribute, 0 if none
    pub class_index: u16,
    // Index (1-based) of the "style" attribute, 0 if none
    pub style_index: u16,
    // TODO: Better pattern?
    pub attribute_data: Vec<u8>
}

#[derive(Debug, PartialEq, DekuWrite)]
pub struct XmlEndElementChunk {
    pub namespace: ResStringPoolRef,
    pub name: ResStringPoolRef
}

#[derive(Debug, PartialEq, DekuWrite)]
pub struct XmlAttributeChunk {
    pub namespace: ResStringPoolRef,
    pub name: ResStringPoolRef,
    pub raw_value: ResStringPoolRef,
    pub typed_value: XmlAttributeDataChunk
}

#[derive(Debug, PartialEq, DekuWrite)]
pub struct XmlAttributeDataChunk {
    pub size: u16,
    // TODO: This always being 0 is the same as AttributeDataType just being u16 and including padding
    pub res0: u8,
    pub data_type: AttributeDataType,
    pub data: u32
}

#[derive(Debug, PartialEq, DekuWrite, Clone)]
#[deku(id_type = "u8")]
pub enum AttributeDataType {
    // Others ommitted
    #[deku(id = 0x01)]
    Reference,
    #[deku(id = 0x03)]
    String,
    #[deku(id = 0x10)]
    DecimalInteger,
    #[deku(id = 0x12)]
    BooleanInteger
}

#[derive(Debug, PartialEq, DekuWrite)]
pub struct XmlNamepsaceChunk {
    pub prefix: u32,
    pub uri: u32
}

#[derive(Debug, PartialEq, DekuWrite)]
pub struct StringPoolChunk {
    // Not the same thing as a ResChunkHeader,
    // the format has headers within headers
    pub string_pool_header: StringPoolHeader,
    pub string_indicies: Vec<u32>,
    pub string_data: Vec<u8>
}

#[derive(Debug, PartialEq, DekuWrite)]
pub struct TableHeaderChunk {
    pub package_count: u32
}

#[derive(Debug, PartialEq, DekuWrite)]
pub struct TablePackageChunk {
    pub id: u32,
    // This is always 128 u16s (256 bytes) long.
    // TODO: Should maybe be a slice u16[128]. That's what it is in C++
    pub name: Vec<u16>,
    pub type_string_offset: u32,
    pub last_public_type: u32,
    pub key_string_offset: u32,
    pub last_public_key: u32,
    pub type_id_offset: u32
}

#[derive(Debug, PartialEq, DekuWrite)]
pub struct TableTypeSpecChunk {
    pub id: u8,
    // This is always 0
    pub res0: u8,
    pub types_count: u16,
    pub entry_count: u32,
    pub configuration_change_flags: Vec<u32>
}

#[derive(Debug, PartialEq, DekuWrite)]
pub struct TableTypeChunk {
    pub id: u8,
    pub flags: u8,
    // Must be 0
    pub reserved: u16,
    pub entry_count: u32,
    pub entries_start: u32,
    pub config: TableConfigChunk,
    pub offsets: Vec<u32>
}

#[derive(Debug, PartialEq, DekuWrite)]
pub struct TableEntry {
    pub size: u16,
    pub flags: u16,
    pub key: ResStringPoolRef,
    pub value: XmlAttributeDataChunk
}

// This struct is the number 64 followed by 60 zeroes
// Luckily, we don't care about any of the data for watch faces.
// TODO: Can we report size as 4 and not include any zeroes?
//   Would save disk space for resources.arsc
#[derive(Debug, PartialEq, DekuWrite)]
pub struct TableConfigChunk {
    pub size: u32,
    pub data: [u8; 60]
}

#[derive(Debug, PartialEq, DekuWrite)]
#[deku(id_type = "u16")]
pub enum ChunkType {
    #[deku(id = 0x0000)]
    Null,
    #[deku(id = 0x0001)]
    StringPool,
    #[deku(id = 0x0002)]
    Table,
    #[deku(id = 0x0003)]
    XmlFile,

    // Types within an XmlFile
    #[deku(id = 0x0100)]
    XmlStartNamespace,
    #[deku(id = 0x0101)]
    XmlEndNamespace,
    #[deku(id = 0x0102)]
    XmlStartElement,
    #[deku(id = 0x0103)]
    XmlEndElement,
    #[deku(id = 0x180)]
    XmlResourceMap,

    // Types within a Table
    #[deku(id = 0x0200)]
    TablePackage,
    #[deku(id = 0x0201)]
    TableType,
    #[deku(id = 0x0202)]
    TableTypeSpec
}

pub const STRING_POOL_UTF8_FLAG: u32 = 1 << 8;
#[derive(Debug, PartialEq, DekuWrite)]
pub struct StringPoolHeader {
    pub string_count: u32,
    pub style_count: u32,
    pub flags: u32,
    pub strings_start: u32,
    pub styles_start: u32
}
