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

//! Most of this package is concerned with APK Signature Scheme v2 and v3,
//! but this module handles Signature Scheme v1, aka. Signed JAR File format.

use base64::{prelude::BASE64_STANDARD, Engine};
use pack_common::Result;
use rasn::types::Integer::Primitive;
use rasn::types::Oid;
use rasn::{Decode, Encode};
use rasn_cms::algorithms::RSA;
use rasn_cms::ContentInfo;
use rasn_cms::{
    pkcs7_compat::SignedData, Certificate, CertificateChoices, IssuerAndSerialNumber,
    SignerIdentifier, SignerInfo
};
use rsa::Pkcs1v15Sign;
use sha2::{Digest, Sha256};

use crate::crypto_keys::Keys;

const OID_SHA256: &Oid =
    rasn::types::Oid::JOINT_ISO_ITU_T_COUNTRY_US_ORGANIZATION_GOV_CSOR_NIST_ALGORITHMS_HASH_SHA256;
const OID_PKCS7_DATA: &Oid = rasn::types::Oid::ISO_MEMBER_BODY_US_RSADSI_PKCS7_DATA;
const OID_PKCS7_SIGNED_DATA: &Oid = rasn::types::Oid::ISO_MEMBER_BODY_US_RSADSI_PKCS7_SIGNED_DATA;

// TODO: It would seem that AAPT sorts these files before creating the manifest,
//   This doesn't seem to be required but might be good for consistent output.
pub fn add_v1_signature_files(zip_contents: &mut Vec<pack_zip::File>, keys: &Keys) -> Result<()> {
    // Create all META-INF files first so they don't hash themselves
    let manifest = create_manifest(zip_contents);
    let sig_file = create_signature_file(zip_contents, &manifest);
    let pkcs7_file = create_pkcs7_file(sig_file.clone(), keys)?;
    // Then add them
    zip_contents.push(pack_zip::File {
        path: "META-INF/MANIFEST.MF".to_string(),
        data: manifest.into()
    });
    zip_contents.push(pack_zip::File {
        path: "META-INF/ALIAS.SF".to_string(),
        data: sig_file.into()
    });
    zip_contents.push(pack_zip::File {
        path: "META-INF/ALIAS.RSA".to_string(),
        data: pkcs7_file
    });
    Ok(())
}

fn create_pkcs7_file(sig_file: String, keys: &Keys) -> Result<Vec<u8>> {
    let digest = Sha256::digest(sig_file.clone());
    let padding = Pkcs1v15Sign::new::<Sha256>();
    let signature = keys.private_key.sign(padding, &digest)?;

    let cert = Certificate::decode(&mut rasn::ber::de::Decoder::new(
        &keys.certificate,
        rasn::ber::de::DecoderOptions::der()
    ))?;

    let signer_info = SignerInfo {
        version: Primitive(1),
        sid: SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
            issuer: cert.tbs_certificate.issuer.clone(),
            serial_number: cert.tbs_certificate.serial_number.clone()
        }),
        digest_algorithm: rasn_cms::AlgorithmIdentifier {
            algorithm: OID_SHA256.into(),
            parameters: None
        },
        signed_attrs: None,
        signature_algorithm: rasn_cms::AlgorithmIdentifier {
            algorithm: RSA.into(),
            parameters: None
        },
        signature: signature.into(),
        unsigned_attrs: None
    };

    let signed_data = SignedData {
        version: Primitive(1),
        digest_algorithms: vec![rasn_cms::AlgorithmIdentifier {
            algorithm: OID_SHA256.into(),
            parameters: None
        }]
        .into(),
        encap_content_info: rasn_cms::pkcs7_compat::EncapsulatedContentInfo {
            content_type: OID_PKCS7_DATA.into(),
            content: None
        },
        certificates: Some(vec![CertificateChoices::Certificate(Box::new(cert))].into()),
        crls: None,
        signer_infos: vec![signer_info].into()
    };

    let mut inner_encoder = rasn::der::enc::Encoder::new(rasn::der::enc::EncoderOptions::der());
    signed_data.encode(&mut inner_encoder)?;
    let inner_vec = inner_encoder.output();

    let wrapper = ContentInfo {
        content_type: OID_PKCS7_SIGNED_DATA.into(),
        content: rasn::types::Any::new(inner_vec.clone())
    };

    let mut outer_encoder = rasn::der::enc::Encoder::new(rasn::der::enc::EncoderOptions::der());
    wrapper.encode(&mut outer_encoder)?;

    Ok(outer_encoder.output())
}

fn create_signature_file(files: &Vec<pack_zip::File>, manifest: &String) -> String {
    let mut output_sig = "Signature-Version: 1.0\r\nCreated-By: 1.0 (Android)\r\n".to_string();
    let manifest_digest = b64_digest(manifest);
    output_sig = format!("{output_sig}SHA-256-Digest-Manifest: {manifest_digest}\r\nX-Android-APK-Signed: 2, 3\r\n\r\n");

    for file in files {
        let file_name = &file.path;
        let entry = create_manifest_entry(file);
        let digest = b64_digest(entry);
        output_sig = format!("{output_sig}Name: {file_name}\r\nSHA-256-Digest: {digest}\r\n\r\n");
    }

    output_sig
}

fn create_manifest(files: &Vec<pack_zip::File>) -> String {
    let mut output_manifest = "Manifest-Version: 1.0\r\n\r\n".to_string();

    for file in files {
        let entry = create_manifest_entry(file);
        output_manifest = format!("{output_manifest}{entry}");
    }

    output_manifest
}

// Also used in the generation of ALIAS.SF
fn create_manifest_entry(file: &pack_zip::File) -> String {
    let file_name = &file.path;
    let b64_digest = b64_digest(&file.data);
    format!("Name: {file_name}\r\nSHA-256-Digest: {b64_digest}\r\n\r\n")
}

fn b64_digest(input: impl AsRef<[u8]>) -> String {
    let digest = Sha256::digest(input);
    BASE64_STANDARD.encode(digest)
}
