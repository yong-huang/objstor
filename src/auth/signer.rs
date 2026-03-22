use crate::error::{Error, Result};
use crate::metadata::db::MetadataStore;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

type HmacSha256 = Hmac<Sha256>;

pub struct Signer {
    metadata_store: MetadataStore,
}

impl Signer {
    pub fn new(metadata_store: MetadataStore) -> Self {
        Self { metadata_store }
    }

    pub fn verify_request(
        &self,
        method: &str,
        uri: &str,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<String> {
        // Extract authorization header
        let auth_header = headers
            .get("authorization")
            .ok_or_else(|| Error::MissingHeader("authorization".to_string()))?;

        // Parse AWS signature format
        // Authorization: AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/s3/aws4_request, SignedHeaders=host;x-amz-date, Signature=...

        if !auth_header.starts_with("AWS4-HMAC-SHA256") {
            return Err(Error::SignatureMismatch);
        }

        let parts: Vec<&str> = auth_header.split(", ").collect();
        let mut credential = "";
        let mut signed_headers = "";
        let mut signature = "";

        for part in parts {
            if part.starts_with("Credential=") {
                credential = &part["Credential=".len()..];
            } else if part.starts_with("SignedHeaders=") {
                signed_headers = &part["SignedHeaders=".len()..];
            } else if part.starts_with("Signature=") {
                signature = &part["Signature=".len()..];
            }
        }

        // Parse credential: access_key_id/date/region/service/aws4_request
        let cred_parts: Vec<&str> = credential.split('/').collect();
        if cred_parts.len() != 5 {
            return Err(Error::SignatureMismatch);
        }

        let access_key_id = cred_parts[0];
        let date = cred_parts[1];
        let region = cred_parts[2];
        let service = cred_parts[3];

        if service != "s3" {
            return Err(Error::SignatureMismatch);
        }

        // Get secret key
        let access_key = self.metadata_store.get_access_key(access_key_id)?;
        let secret_key = &access_key.secret_key;

        // Calculate expected signature
        let expected_signature = self.calculate_signature(
            method,
            uri,
            headers,
            signed_headers,
            body,
            secret_key,
            date,
            region,
        )?;

        if expected_signature != signature {
            return Err(Error::SignatureMismatch);
        }

        Ok(access_key.owner.clone())
    }

    fn calculate_signature(
        &self,
        method: &str,
        uri: &str,
        headers: &HashMap<String, String>,
        signed_headers: &str,
        body: &[u8],
        secret_key: &str,
        date: &str,
        region: &str,
    ) -> Result<String> {
        // 1. Create canonical request
        let canonical_uri = uri;
        let canonical_querystring = "";
        let canonical_headers = self.create_canonical_headers(headers, signed_headers)?;
        let signed_headers_list = signed_headers;

        // Hash payload
        let payload_hash = hex::encode(sha2::Sha256::digest(body));

        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method, canonical_uri, canonical_querystring, canonical_headers, signed_headers_list, payload_hash
        );

        // 2. Create string to sign
        let algorithm = "AWS4-HMAC-SHA256";
        let datetime = headers
            .get("x-amz-date")
            .ok_or_else(|| Error::MissingHeader("x-amz-date".to_string()))?;
        let credential_scope = format!("{}/{}/{}/aws4_request", date, region, "s3");
        let hashed_canonical_request = hex::encode(sha2::Sha256::digest(canonical_request.as_bytes()));

        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm, datetime, credential_scope, hashed_canonical_request
        );

        // 3. Calculate signature
        let k_date = Self::hmac_sha256(format!("AWS4{}", secret_key).as_bytes(), date.as_bytes())?;
        let k_region = Self::hmac_sha256(&k_date, region.as_bytes())?;
        let k_service = Self::hmac_sha256(&k_region, "s3".as_bytes())?;
        let k_signing = Self::hmac_sha256(&k_service, b"aws4_request")?;

        let signature = Self::hmac_sha256(&k_signing, string_to_sign.as_bytes())?;
        Ok(hex::encode(signature))
    }

    fn create_canonical_headers(
        &self,
        headers: &HashMap<String, String>,
        signed_headers: &str,
    ) -> Result<String> {
        let header_names: Vec<&str> = signed_headers.split(';').collect();
        let mut canonical_headers = String::new();

        for name in header_names {
            let key = name.to_lowercase();
            if let Some(value) = headers.get(&key) {
                canonical_headers.push_str(&key);
                canonical_headers.push(':');
                canonical_headers.push_str(value.trim());
                canonical_headers.push('\n');
            }
        }

        Ok(canonical_headers)
    }

    fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
        let mut mac = HmacSha256::new_from_slice(key).map_err(|_| Error::SignatureMismatch)?;
        mac.update(data);
        Ok(mac.finalize().into_bytes().to_vec())
    }
}
