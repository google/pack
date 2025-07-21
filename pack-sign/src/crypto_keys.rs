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

use std::collections::HashMap;

use pack_common::*;
use rsa::{
    pkcs8::{DecodePrivateKey, EncodePublicKey},
    RsaPrivateKey, RsaPublicKey
};

/// Holds the certificate and RSA Private Key used for signing.
pub struct Keys {
    /// X.509 Signing Certificate in ASN.1 DER form
    pub certificate: Vec<u8>,
    /// RSA Public Key
    pub public_key: RsaPublicKey,
    /// RSA Private Key
    pub private_key: RsaPrivateKey
}

impl Keys {
    /// Parses and creates an instance of [Keys] from a `.pem` file.
    ///
    /// "Combined" in this case means that the one file has both a `BEGIN
    /// CERTIFICATE` and a `BEGIN PRIVATE KEY` section as one long UTF-8 string.
    ///
    /// If you don't have one of these, use [generate_random_testing_keys](Keys::generate_random_testing_keys).
    pub fn from_combined_pem_string(combined_pem: &str) -> Result<Keys> {
        let pem_map = parse_pem_map_by_tags(combined_pem)?;
        let certificate = pem_map
            .get("CERTIFICATE")
            .ok_or(PackError::SignerNoKeys)?
            .clone();

        let priv_key_bytes = pem_map.get("PRIVATE KEY").ok_or(PackError::SignerNoKeys)?;
        let private_key = RsaPrivateKey::from_pkcs8_der(priv_key_bytes)?;
        let public_key = RsaPublicKey::from(private_key.clone());

        Ok(Keys {
            public_key,
            private_key,
            certificate
        })
    }

    /// Randomly generates RSA signing keys and an accompanying certificate.
    ///
    /// This API is only enabled when the optional "cert-gen" feature is enabled
    /// for pack-sign (it's on by default). It introduces a non-trivial amount of
    /// extra dependencies and includes ASM/C code. For that reason is only enabled
    /// on the desktop CLI and not on the web by default.
    ///
    /// It is also very slow. ~150ms. Which on an M1 Pro is 10x pack-cli's entire
    /// run time without it. For that reason, it's recommended that you generate
    /// keys with OpenSSL and pass them in to [Keys::from_combined_pem_string].
    ///
    /// # Why is this ok for local APK testing?
    ///
    /// For testing APKs on your local device, you aren't concerned about the
    /// app's origin or whether the developer is who they say they are (they're you).
    ///
    /// # Why is this still ok for AABs on Google Play?
    ///
    /// When you upload a signed `.aab` file to Google Play Console, you have the
    /// option of using "Google-managed keys", in which case Google will re-sign
    /// your app with keys unique to your Google Developer account.
    ///
    /// That makes this signing key less important. However, it does come with the
    /// large caveat below, which you should pay attention to.
    ///
    /// # Why shouldn't we always do this?
    ///
    /// This will hinder you publishing updates. Google Play Console updates need
    /// to be uploaded using the same signing key as the initial version. So if your
    /// keys are randomly generated, you'll be stuck on v1 forever. Similarly for
    /// updating locally-tested APKs, you'll have to `adb uninstall package.name`
    /// before you install an updated APK, since they'll come from different
    /// publishers and Android will reject the update while the old version is still
    /// installed.
    #[cfg(feature = "cert-gen")]
    pub fn generate_random_testing_keys() -> Result<Keys> {
        // These dependencies only exist when compiled with cert-gen
        use rand::prelude::*;
        use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
        use rsa::pkcs8::{EncodePrivateKey, LineEnding};

        eprintln!("Warning: Randomly generating a placeholder signing key. This is slow!");
        eprintln!("    It's recommended to generate your own keys first and pass them in.");

        // Randomly generate an RSA Private Key, derive its Public Key,
        // and prepare it for passing over to the rcgen library.
        let private_key = RsaPrivateKey::new(&mut thread_rng(), 2048)?;
        let public_key = RsaPublicKey::from(private_key.clone());
        let private_key_pem = private_key.to_pkcs8_pem(LineEnding::LF)?.to_string();

        // Self-sign an X.509 certificate using the random keys
        let key_pair = KeyPair::from_pem(&private_key_pem).unwrap();
        // We sign all testing certificates as our crate name
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, env!("CARGO_PKG_NAME"));
        let mut cert_params = CertificateParams::new(vec![]).unwrap();
        cert_params.distinguished_name = distinguished_name;
        let cert = cert_params.self_signed(&key_pair).unwrap();

        Ok(Self {
            certificate: cert.der().to_vec(),
            private_key,
            public_key
        })
    }

    /// Returns the RSA Private Key encoded in ASN.1 DER format.
    pub fn pub_key_as_der(&self) -> Result<Vec<u8>> {
        Ok(self.public_key.to_public_key_der()?.as_ref().to_vec())
    }
}

/// Parses a .pem file and returns a map of Tag -> Contents
fn parse_pem_map_by_tags(combined_pem: &str) -> Result<HashMap<String, Vec<u8>>> {
    let parsed = pem::parse_many(combined_pem)?;
    let mut map = HashMap::new();
    for pem_part in parsed {
        map.insert(pem_part.tag().into(), pem_part.into_contents());
    }
    Ok(map)
}
