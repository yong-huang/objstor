use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Generate a pre-signed URL for GET access to an object.
///
/// Uses HMAC-SHA256 signing derived from the server master key.
pub fn generate_presigned_url(
    host: &str,
    bucket: &str,
    key: &str,
    expires_in_secs: u64,
    _method: &str,
    master_key: &[u8],
) -> String {
    let now = Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();

    // Build canonical request
    let canonical_uri = format!("/{}/{}", bucket, key);
    let canonical_qs = format!(
        "X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential={}%2F{}&X-Amz-Date={}&X-Amz-Expires={}&X-Amz-SignedHeaders=host",
        "objstor-presign",
        "us-east-1",
        amz_date,
        expires_in_secs,
    );

    let string_to_sign = format!(
        "{}\n{}\n{}\n{}",
        "GET",
        &canonical_uri,
        &canonical_qs,
        hex::encode(hash_sha256(&format!("{}\n{}\n{}", "GET", canonical_uri, canonical_qs))),
    );

    let signature = compute_signature(master_key, &string_to_sign);

    format!(
        "http://{}{}?{}&X-Amz-Signature={}",
        host, canonical_uri, canonical_qs, signature
    )
}

/// Validate a pre-signed request URI.
///
/// Returns Ok(bucket, key, method) if valid, Err if expired or signature mismatch.
pub fn validate_presigned_request(
    uri: &str,
    _host: &str,
    master_key: &[u8],
) -> std::result::Result<(String, String, String), &'static str> {
    let parsed: url::Url = url::Url::parse(uri).map_err(|_| "Invalid URL")?;

    let signature = parsed
        .query_pairs()
        .find(|(k, _)| k == "X-Amz-Signature")
        .map(|(_, v)| v.to_string())
        .ok_or("Missing signature")?;

    let amz_date = parsed
        .query_pairs()
        .find(|(k, _)| k == "X-Amz-Date")
        .map(|(_, v)| v.to_string())
        .ok_or("Missing X-Amz-Date")?;

    let amz_expires = parsed
        .query_pairs()
        .find(|(k, _)| k == "X-Amz-Expires")
        .and_then(|(_, v)| v.parse::<u64>().ok())
        .ok_or("Missing or invalid X-Amz-Expires")?;

    let algorithm = parsed
        .query_pairs()
        .find(|(k, _)| k == "X-Amz-Algorithm")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();

    if algorithm != "AWS4-HMAC-SHA256" {
        return Err("Unsupported algorithm");
    }

    // Check expiry
    let date_time =
        chrono::NaiveDateTime::parse_from_str(&amz_date, "%Y%m%dT%H%M%SZ").map_err(|_| "Invalid date")?;
    let expires_at = date_time.and_utc().timestamp() + amz_expires as i64;
    if Utc::now().timestamp() > expires_at {
        return Err("Pre-signed URL expired");
    }

    // Extract path (bucket/key)
    let path = parsed.path().trim_start_matches('/');
    let mut parts = path.splitn(2, '/');
    let bucket = parts.next().unwrap_or("").to_string();
    let key = parts.next().unwrap_or("").to_string();

    if bucket.is_empty() || key.is_empty() {
        return Err("Invalid bucket or key");
    }

    // Rebuild canonical query string without signature
    let canonical_qs: String = parsed
        .query_pairs()
        .filter(|(k, _)| k != "X-Amz-Signature")
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    let canonical_uri = format!("/{}", path);
    let string_to_sign = format!(
        "GET\n{}\n{}\n{}",
        canonical_uri,
        canonical_qs,
        hex::encode(hash_sha256(&format!(
            "GET\n{}\n{}",
            canonical_uri, canonical_qs
        )))
    );

    let expected = compute_signature(master_key, &string_to_sign);
    if expected != signature {
        return Err("Signature mismatch");
    }

    Ok((bucket, key, "GET".to_string()))
}

fn compute_signature(key: &[u8], data: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key is valid");
    mac.update(data.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn hash_sha256(data: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hasher.finalize().into()
}
