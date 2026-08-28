use crate::error::{Error, Result};
use crate::metadata::db::MetadataStore;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

struct SigningParams {
    date: String,
    region: String,
}

struct RequestContext<'a> {
    method: &'a str,
    uri: &'a str,
    headers: &'a HashMap<String, String>,
    signed_headers: &'a str,
    body: &'a [u8],
}

pub struct Signer {
    metadata_store: Arc<MetadataStore>,
}

impl Signer {
    pub fn new(metadata_store: Arc<MetadataStore>) -> Self {
        Self { metadata_store }
    }

    pub fn verify_request(
        &self,
        method: &str,
        uri: &str,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<String> {
        let auth_header = headers
            .get("authorization")
            .ok_or_else(|| Error::MissingHeader("authorization".to_string()))?;

        if !auth_header.starts_with("AWS4-HMAC-SHA256") {
            return Err(Error::SignatureMismatch);
        }

        // Strip the algorithm prefix so we can parse "Credential=..., SignedHeaders=..., Signature=..."
        let auth_value = auth_header.strip_prefix("AWS4-HMAC-SHA256 ").unwrap_or("");
        // SDKs differ in separator style ("a, b" vs "a,b"); split on comma and trim
        let parts: Vec<String> = auth_value
            .split(',')
            .map(|p| p.trim().to_string())
            .collect();
        let mut credential = "";
        let mut signed_headers = "";
        let mut signature = "";

        for part in &parts {
            if let Some(value) = part.strip_prefix("Credential=") {
                credential = value;
            } else if let Some(value) = part.strip_prefix("SignedHeaders=") {
                signed_headers = value;
            } else if let Some(value) = part.strip_prefix("Signature=") {
                signature = value;
            }
        }

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

        let access_key = self.metadata_store.get_access_key(access_key_id)?;
        let secret_key = &access_key.secret_key;

        let (canonical_uri, canonical_querystring) = match uri.split_once('?') {
            Some((path, query)) => (path, Self::sort_query_string(query)),
            None => (uri, String::new()),
        };

        let signing_params = SigningParams {
            date: date.to_string(),
            region: region.to_string(),
        };
        let request_ctx = RequestContext {
            method,
            uri: canonical_uri,
            headers,
            signed_headers,
            body,
        };
        let expected_signature = self.calculate_signature(
            &request_ctx,
            secret_key,
            &signing_params,
            &canonical_querystring,
        )?;

        if expected_signature != signature {
            eprintln!(
                "[sig-debug] method={} uri={} expected={} actual={} signed_headers={} x-amz-content-sha256={:?} body_len={}",
                method,
                uri,
                expected_signature,
                signature,
                signed_headers,
                headers.get("x-amz-content-sha256"),
                body.len()
            );
            return Err(Error::SignatureMismatch);
        }

        Ok(access_key.owner.clone())
    }

    fn calculate_signature(
        &self,
        request_ctx: &RequestContext,
        secret_key: &str,
        signing_params: &SigningParams,
        canonical_querystring: &str,
    ) -> Result<String> {
        let canonical_headers =
            self.create_canonical_headers(request_ctx.headers, request_ctx.signed_headers)?;
        let signed_headers_list = request_ctx.signed_headers;

        let payload_hash = match request_ctx.headers.get("x-amz-content-sha256") {
            Some(hash) => hash.clone(),
            None => hex::encode(sha2::Sha256::digest(request_ctx.body)),
        };

        // AWS SigV4 canonical request format:
        // HTTPRequestMethod\nCanonicalURI\nCanonicalQueryString\nCanonicalHeaders\nSignedHeaders\nHashedPayload
        // Note: CanonicalHeaders ends with '\n', so the '\n' before SignedHeaders creates the required blank line
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            request_ctx.method,
            request_ctx.uri,
            canonical_querystring,
            canonical_headers,
            signed_headers_list,
            payload_hash
        );

        let algorithm = "AWS4-HMAC-SHA256";
        let datetime = request_ctx
            .headers
            .get("x-amz-date")
            .ok_or_else(|| Error::MissingHeader("x-amz-date".to_string()))?;
        let credential_scope = format!(
            "{}/{}/{}/aws4_request",
            signing_params.date, signing_params.region, "s3"
        );
        let hashed_canonical_request =
            hex::encode(sha2::Sha256::digest(canonical_request.as_bytes()));

        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm, datetime, credential_scope, hashed_canonical_request
        );

        let k_date = Self::hmac_sha256(
            format!("AWS4{}", secret_key).as_bytes(),
            signing_params.date.as_bytes(),
        )?;
        let k_region = Self::hmac_sha256(&k_date, signing_params.region.as_bytes())?;
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

    fn sort_query_string(query: &str) -> String {
        let mut params: Vec<(&str, &str)> = query
            .split('&')
            .filter(|s| !s.is_empty())
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = parts.next()?;
                let value = parts.next().unwrap_or("");
                Some((key, value))
            })
            .collect();
        params.sort_by_key(|(k, _)| *k);
        params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&")
    }
}
