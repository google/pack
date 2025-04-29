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

use core::fmt;
use std::{io, num::ParseIntError, rc::Rc};

use deku::prelude::*;
use rsa::pkcs8;
use zip::result::ZipError;

/// Common error type making it easier to share `Result`s between PACK crates.
///
/// In general designed to avoid needing utilities like `map_err`.
#[derive(Debug, Clone)]
pub enum PackError {
    /// pack-cli encountered an error while processing something specific to the
    /// command line implementation. For example, not enough arguments were
    /// passed via the shell.
    Cli(String),
    /// The bytes passed in `Package.android_manifest` are not valid UTF-8.
    ManifestIsNotUTF8,
    /// The AndroidManifest file doesn't contain a "package" attribute.
    ManifestDoesNotHavePackageName,
    /// PACK only supports UTF-8 encoding for AAPT StringPools. In this format,
    /// string lengths are stored in signed 16-bit integers, meaning the
    /// maximum supported string length is `0x7FFF` bytes.
    StringPoolStringTooLong(String),
    /// Attempted to construct an APK resource table with a package identifier
    /// longer than 128 bytes long.
    PackageNameTooLong(String),
    /// When AssetCompiler was trying to serialise a struct similar to AAPT,
    /// something went wrong. See [DekuError].
    ByteSerialisationFailed(DekuError),
    /// In APK encoding, XML files require a first-pass to figure out how many
    /// `android:`-prefixed attributes they contain. If that code has a mistake
    /// in it, a later part of the process will throw this error.
    ///
    /// **If you experience this, it is considered an internal bug in PACK.
    /// Please report it.**
    TooManyUniqueAndroidInternalAttributes,
    /// PACK needs to know about all possible internal attributes, such as
    /// `android:name`, `android:compileSdkVersion`, etc. If a newer attribute
    /// is introduced and used in a file, this error will be thrown.
    UnknownAndroidInternalAttribute(String),
    /// Parsing failed while reading an XML file (`AndroidManifest.xml`,
    /// `strings.xml`, or any file in `res/xml`). See [xml::reader::Error].
    XmlParsingFailed(xml::reader::Error),
    /// An attribute was persent in an XML file which was expected to be an
    /// integer (eg. `android:minSdkVersion`), but its value was not a valid
    /// integer (eg. `"abc"`).
    IntegerAttributeParsingFailed(ParseIntError),
    /// An XML attribute value began with `@` as though it was a reference
    /// (eg. `@drawable/preview`), but its format didn't fit what was expected
    /// (two strings with one slash separator).
    ReferenceAttributeParsingFailed(String),
    /// An XML attribute value was parsed, but its target wasn't in the APK.
    ReferenceAttributeLookupFailed(String),
    /// PACK's AAB compiler tried to cast a ProtoXML Node to an Element.
    ///
    /// **If you experience this, it is considered an internal bug in PACK.
    /// Please report it.**
    ProtoXmlNodeIsNotAnElement,
    /// An error occurred while a package was writing to disk. Since only
    /// `pack-cli` interacts with the disk, it's likely that one of the file
    /// paths you passed to it is invalid, or the disk was full or similar.
    FileIoError(Rc<io::Error>),
    /// `pack-zip` failed to create a zip file in-memory.
    ZipWritingFailed(Rc<ZipError>),
    /// The APK Signature Scheme v2/v3 code failed to find the ZIP End Of
    /// Central Directory marker within the zip file.
    SignerZipParsingFailed,
    /// An error occurred while trying to instantiate a `Keys` object from a
    /// `.pem` string.
    SignerPemParsingFailed(Rc<pem::PemError>),
    /// The `.pem` file passed to `Keys` was valid, but it was missing either
    /// a certificate or private key.
    SignerNoKeys,
    /// The `PRIVATE KEY` in the `.pem` was present, but it wasn't an RSA
    /// Private Key.
    SignerRsaPrivateKeyParsingFailed(pkcs8::Error),
    /// An error occurred while signing a hash, see [rsa::Error].
    SignerRsaSigningFailed(Rc<rsa::Error>),
    /// An error occurred while serialising the RSA key, see
    /// [pkcs8::spki::Error].
    SignerRsaKeySerialisationFailed(pkcs8::spki::Error),
    /// The signing certificate couldn't be loaded for V1 AAB signing.
    SignerCertificateDecodingFailed(Rc<rasn::error::DecodeError>),
    /// V1 Signing data couldn't be serialised
    SignerPKCS7EncodingFailed(Rc<rasn::error::EncodeError>)
}

/// Result type where the error is always [PackError].
pub type Result<T> = std::result::Result<T, PackError>;

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use PackError::*;
        match self {
            Cli(msg) => write!(f, "{msg}"),
            ManifestIsNotUTF8 => write!(f, "AndroidManifest.xml file is not valid UTF-8."),
            ManifestDoesNotHavePackageName => write!(f, "AndroidManifest.xml file does not define a 'package' attribute on its <manifest /> element."),
            StringPoolStringTooLong(_) => write!(f, "XML file contained a string longer than 32,767 (0x7FFF) characters. Pack does not support arbitrary-size string pools."),
            PackageNameTooLong(pkg) => write!(f, "Package name \"{pkg}\" is too long. Maximum length is 128 characters."),
            ByteSerialisationFailed(deku_error) => write!(f, "Failed to get byte representation of an object.\nInternal error: {deku_error:?}"),
            TooManyUniqueAndroidInternalAttributes => write!(f, "Internal Pack bug: Too many unique Android Internal Attributes. This shouldn't be possible, please file a bug in the Pack repo."),
            UnknownAndroidInternalAttribute(attr) => write!(f, "Unknown Android Internal Attribute \"{attr}\". This may be because the attribute is not valid, or because Pack is not up-to-date on the latest added attributes. If you believe the latter, please file a bug in the Pack repo."),
            XmlParsingFailed(xml_error) => write!(f, "XML parsing error.\nInternal error: {xml_error:?}"),
            IntegerAttributeParsingFailed(err) => write!(f, "Encountered a non-integer value in an attribute that was expected to be an integer.\nInternal error: {err:?}"),
            ReferenceAttributeParsingFailed(attr) => write!(f, "Failed to parse attribute reference \"{attr}\". Expected a format like \"@drawable/preview\" since the value begins with \"@\"."),
            ReferenceAttributeLookupFailed(attr) => write!(f, "Failed to lookup attribute reference \"{attr}\". Does it exist in the input files?"),
            ProtoXmlNodeIsNotAnElement => write!(f, "Internal Pack bug: Failed to cast ProtoXml Node to Element. This shouldn't be possible, please file a bug in the Pack repo."),
            FileIoError(io_err) => write!(f, "File I/O failed. Did you specify a valid input/output path?\nInternal error: {io_err:?}"),
            ZipWritingFailed(zip_error) => write!(f, "Failed to create in-memory Zip archive.\nInternal error: {zip_error:?}"),
            SignerZipParsingFailed => write!(f, "Signer failed to find the Zip End of Central Directory Marker."),
            SignerPemParsingFailed(pem_error) => write!(f, "A signing .pem was provided, but it didn't parse as valid syntax.\nInternal error: {pem_error:?}"),
            SignerNoKeys => write!(f, "A signing .pem was provided, but it didn't contain one usable PRIVATE KEY and CERTIFICATE.\nEnsure keys are not protected with passwords, as Pack does not support parsing these. Else, ensure your .pem is formatted correctly so as not to trip up the parser."),
            SignerRsaPrivateKeyParsingFailed(pkcs_error) => write!(f, "RSA Private Key parsing failed.\nInternal error: {pkcs_error:?}"),
            SignerRsaSigningFailed(rsa_error) => write!(f, "RSA signing failed.\nInternal error: {rsa_error:?}"),
            SignerRsaKeySerialisationFailed(pkcs_error) => write!(f, "Failed to serialise RSA key for APK Signing Scheme v1.\nInternal error: {pkcs_error:?}"),
            SignerCertificateDecodingFailed(decode_error) => write!(f, "Failed to decode certificate from .pem.\nInternal error: {decode_error:?}"),
            SignerPKCS7EncodingFailed(encode_error) => write!(f, "Failed to write PKCS7 signature for APK Signature Scheme v1.\nInternal error: {encode_error:?}"),
        }
    }
}

/// This makes it easier for Result<Something, PackError> to be returned from WASM functions
impl From<PackError> for String {
    fn from(value: PackError) -> Self {
        format!("{value}")
    }
}

// Automatic conversion from other types of error to PackError makes the rest of the code cleaner
impl From<io::Error> for PackError {
    fn from(value: io::Error) -> Self {
        PackError::FileIoError(value.into())
    }
}

impl From<DekuError> for PackError {
    fn from(value: DekuError) -> Self {
        PackError::ByteSerialisationFailed(value)
    }
}

impl From<ParseIntError> for PackError {
    fn from(value: ParseIntError) -> Self {
        PackError::IntegerAttributeParsingFailed(value)
    }
}

impl From<ZipError> for PackError {
    fn from(value: ZipError) -> Self {
        PackError::ZipWritingFailed(value.into())
    }
}

impl From<pem::PemError> for PackError {
    fn from(value: pem::PemError) -> Self {
        PackError::SignerPemParsingFailed(value.into())
    }
}

impl From<pkcs8::Error> for PackError {
    fn from(value: pkcs8::Error) -> Self {
        PackError::SignerRsaPrivateKeyParsingFailed(value)
    }
}

impl From<rsa::Error> for PackError {
    fn from(value: rsa::Error) -> Self {
        PackError::SignerRsaSigningFailed(value.into())
    }
}

impl From<pkcs8::spki::Error> for PackError {
    fn from(value: pkcs8::spki::Error) -> Self {
        PackError::SignerRsaKeySerialisationFailed(value)
    }
}

impl From<rasn::error::DecodeError> for PackError {
    fn from(value: rasn::error::DecodeError) -> Self {
        PackError::SignerCertificateDecodingFailed(value.into())
    }
}

impl From<rasn::error::EncodeError> for PackError {
    fn from(value: rasn::error::EncodeError) -> Self {
        PackError::SignerPKCS7EncodingFailed(value.into())
    }
}
