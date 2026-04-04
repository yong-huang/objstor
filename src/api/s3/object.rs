use crate::api::s3::handler::S3AppState;
use crate::error::{Error, Result};
use crate::metadata::Object;
use crate::storage::dedup::DedupManager;
use crate::storage::encryption::{decode_sse_c_key, EncryptionInfo};
use crate::storage::tier::StorageTier;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderName, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use chrono::Utc;
use md5::Digest;
use sha2::Sha256;

/// Header names used for SSE responses.
static X_AMZ_SERVER_SIDE_ENCRYPTION: HeaderName =
    HeaderName::from_static("x-amz-server-side-encryption");
static X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM: HeaderName =
    HeaderName::from_static("x-amz-server-side-encryption-customer-algorithm");

pub async fn handle_put_object(
    State(state): State<S3AppState>,
    Path((bucket, key)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response> {
    // Get bucket metadata to check preferred pool
    let bucket_metadata = state.metadata.get_bucket(&bucket)?;
    let _preferred_pool = bucket_metadata.preferred_pool.as_deref();

    // Calculate ETag (MD5 hash) on original (plaintext) data
    let etag = format!("{:x}", md5::Md5::digest(&body));

    // Calculate plaintext hash for dedup (before encryption)
    let mut hasher = Sha256::new();
    hasher.update(&body);
    let plaintext_hash = hex::encode(hasher.finalize());

    // Parse storage class / tier
    let storage_class = headers
        .get("x-amz-storage-class")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("STANDARD");
    let tier = StorageTier::from_s3_storage_class(storage_class);

    // Parse encryption headers
    let sse_type = parse_sse_type(&headers);
    let (data_to_write, encryption_info) = match sse_type {
        SseType::SseS3 => {
            let (ciphertext, info) = state.key_manager.encrypt_sse_s3(&body)?;
            (ciphertext, Some(info))
        }
        SseType::SseC(ref customer_key_b64) => {
            let customer_key = decode_sse_c_key(customer_key_b64)?;
            let (ciphertext, info) = state.key_manager.encrypt_sse_c(&body, &customer_key)?;
            (ciphertext, Some(info))
        }
        SseType::None => (body.to_vec(), None),
    };

    // Select pool based on tier
    let pool = state
        .pool_manager
        .select_pool_for_tier(&tier, data_to_write.len() as u64)
        .await?;

    // Serialize encryption info for storage
    let encryption_info_json = encryption_info.as_ref().map(|i| {
        serde_json::to_string(i).expect("EncryptionInfo is always serializable")
    });

    // Dedup check — only for non-SSE-C objects
    let is_sse_c = matches!(sse_type, SseType::SseC(_));
    if !is_sse_c {
        let already_exists = DedupManager::increment_ref_count(
            &state.metadata,
            &pool.id,
            &plaintext_hash,
            data_to_write.len() as u64,
        )?;

        if already_exists {
            // Dedup hit — data already exists, skip physical write
            let object = Object {
                id: 0,
                bucket: bucket.clone(),
                key: key.clone(),
                version_id: None,
                object_hash: plaintext_hash.clone(),
                size: body.len() as u64,
                content_type: headers
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string()),
                etag: etag.clone(),
                created_at: Utc::now(),
                modified_at: Utc::now(),
                pool_id: pool.id.clone(),
                storage_class: storage_class.to_string(),
                tags: None,
                metadata: None,
                encryption_info: encryption_info_json.clone(),
                tier: Some(tier.to_string()),
            };

            state.metadata.create_object(&object)?;
            state.event_bus.emit_object_created(&bucket, &key, body.len() as u64, &etag);
            return Ok(build_put_response(&etag, &sse_type));
        }
    } else {
        // SSE-C objects still track ref_count
        DedupManager::increment_ref_count(
            &state.metadata,
            &pool.id,
            &plaintext_hash,
            data_to_write.len() as u64,
        )?;
    }

    // Write data to storage pool (with compression for non-Hot tiers)
    let mut pool_mut = pool;
    let location = pool_mut.write_object(&data_to_write, &tier).await?;

    // Create metadata
    let object = Object {
        id: 0,
        bucket: bucket.clone(),
        key: key.clone(),
        version_id: None,
        object_hash: location.object_hash.clone(),
        size: body.len() as u64,
        content_type: headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        etag: etag.clone(),
        created_at: Utc::now(),
        modified_at: Utc::now(),
        pool_id: location.pool_id.clone(),
        storage_class: storage_class.to_string(),
        tags: None,
        metadata: None,
        encryption_info: encryption_info_json.clone(),
        tier: Some(tier.to_string()),
    };

    state.metadata.create_object(&object)?;
    state.event_bus.emit_object_created(&bucket, &key, body.len() as u64, &etag);

    Ok(build_put_response(&etag, &sse_type))
}

pub async fn handle_get_object(
    State(state): State<S3AppState>,
    Path((bucket, key)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response> {
    // Get object metadata
    let object = state.metadata.get_object(&bucket, &key)?;

    // Read encrypted/raw data from pool
    let pool = state.pool_manager.get_pool(&object.pool_id).await?;
    let raw_data = pool.read_object(&object.object_hash).await?;

    // Decrypt if encrypted
    let data = if let Some(ref enc_info_str) = object.encryption_info {
        let enc_info: EncryptionInfo = serde_json::from_str(enc_info_str)
            .map_err(|_| crate::error::Error::ObjectCorrupted(object.key.clone()))?;

        match enc_info.mode.as_str() {
            "SSE-S3" => state.key_manager.decrypt_sse_s3(&raw_data, &enc_info)?,
            "SSE-C" => {
                // Customer must provide their key via header
                let customer_key_b64 = headers
                    .get("x-amz-server-side-encryption-customer-key")
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| {
                        crate::error::Error::MissingHeader(
                            "x-amz-server-side-encryption-customer-key".to_string(),
                        )
                    })?;
                let customer_key = decode_sse_c_key(customer_key_b64)?;
                state
                    .key_manager
                    .decrypt_sse_c(&raw_data, &enc_info, &customer_key)?
            }
            _ => raw_data,
        }
    } else {
        raw_data
    };

    // Handle Range requests
    if let Some(range_header) = headers.get("range").and_then(|v| v.to_str().ok()) {
        return handle_range_response(&data, range_header, &object).await;
    }

    // Build full response
    let mut builder = Response::builder().status(StatusCode::OK);
    builder = builder.header(
        axum::http::header::CONTENT_TYPE,
        object
            .content_type
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string()),
    );
    builder = builder.header(axum::http::header::CONTENT_LENGTH, data.len());
    builder = builder.header(axum::http::header::ETAG, &object.etag);
    builder = builder.header(
        HeaderName::from_static("last-modified"),
        format!("{}", object.modified_at.format("%a, %d %b %Y %H:%M:%S GMT")),
    );
    builder = builder.header(
        HeaderName::from_static("accept-ranges"),
        "bytes",
    );

    // Add encryption response headers
    if let Some(ref enc_info_str) = object.encryption_info {
        let enc_info: EncryptionInfo = serde_json::from_str(enc_info_str)
            .map_err(|_| crate::error::Error::ObjectCorrupted(object.key.clone()))?;
        match enc_info.mode.as_str() {
            "SSE-S3" => {
                builder = builder.header(X_AMZ_SERVER_SIDE_ENCRYPTION.clone(), "AES256");
            }
            "SSE-C" => {
                builder = builder.header(
                    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM.clone(),
                    "AES256",
                );
            }
            _ => {}
        }
    }

    Ok(builder
        .body(axum::body::Body::from(data))
        .expect("response is valid"))
}

pub async fn handle_get_object_version(
    State(state): State<S3AppState>,
    Path((bucket, key, version_id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Response> {
    let object = state
        .metadata
        .get_object_version(&bucket, &key, &version_id)?;

    // Handle delete markers
    if object.object_hash.is_empty() && object.size == 0 {
        return Err(Error::ObjectNotFound(format!("{}/{}", bucket, key)));
    }

    let pool = state.pool_manager.get_pool(&object.pool_id).await?;
    let raw_data = pool.read_object(&object.object_hash).await?;

    let data = if let Some(ref enc_info_str) = object.encryption_info {
        let enc_info: EncryptionInfo = serde_json::from_str(enc_info_str)
            .map_err(|_| crate::error::Error::ObjectCorrupted(object.key.clone()))?;
        match enc_info.mode.as_str() {
            "SSE-S3" => state.key_manager.decrypt_sse_s3(&raw_data, &enc_info)?,
            "SSE-C" => {
                let customer_key_b64 = headers
                    .get("x-amz-server-side-encryption-customer-key")
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| {
                        crate::error::Error::MissingHeader(
                            "x-amz-server-side-encryption-customer-key".to_string(),
                        )
                    })?;
                let customer_key = decode_sse_c_key(customer_key_b64)?;
                state
                    .key_manager
                    .decrypt_sse_c(&raw_data, &enc_info, &customer_key)?
            }
            _ => raw_data,
        }
    } else {
        raw_data
    };

    let mut builder = Response::builder().status(StatusCode::OK);
    builder = builder.header(
        axum::http::header::CONTENT_TYPE,
        object
            .content_type
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string()),
    );
    builder = builder.header(axum::http::header::CONTENT_LENGTH, data.len());
    builder = builder.header(axum::http::header::ETAG, &object.etag);
    builder = builder.header(
        HeaderName::from_static("x-amz-version-id"),
        &version_id,
    );
    if let Some(ref enc_info_str) = object.encryption_info {
        let enc_info: EncryptionInfo = serde_json::from_str(enc_info_str)
            .map_err(|_| crate::error::Error::ObjectCorrupted(object.key.clone()))?;
        match enc_info.mode.as_str() {
            "SSE-S3" => {
                builder = builder.header(X_AMZ_SERVER_SIDE_ENCRYPTION.clone(), "AES256");
            }
            "SSE-C" => {
                builder = builder.header(
                    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM.clone(),
                    "AES256",
                );
            }
            _ => {}
        }
    }

    Ok(builder
        .body(axum::body::Body::from(data))
        .expect("response is valid"))
}

pub async fn handle_head_object(
    State(state): State<S3AppState>,
    Path((bucket, key)): Path<(String, String)>,
) -> Result<Response> {
    let object = state.metadata.get_object(&bucket, &key)?;

    let mut builder = Response::builder().status(StatusCode::OK);
    builder = builder.header(
        axum::http::header::CONTENT_TYPE,
        object
            .content_type
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string()),
    );
    builder = builder.header(axum::http::header::CONTENT_LENGTH, object.size);
    builder = builder.header(axum::http::header::ETAG, &object.etag);
    builder = builder.header(
        HeaderName::from_static("last-modified"),
        format!("{}", object.modified_at.format("%a, %d %b %Y %H:%M:%S GMT")),
    );
    builder = builder.header(
        HeaderName::from_static("accept-ranges"),
        "bytes",
    );

    // Add encryption response headers for HEAD
    if let Some(ref enc_info_str) = object.encryption_info {
        let enc_info: EncryptionInfo = serde_json::from_str(enc_info_str)
            .map_err(|_| crate::error::Error::ObjectCorrupted(object.key.clone()))?;
        match enc_info.mode.as_str() {
            "SSE-S3" => {
                builder = builder.header(X_AMZ_SERVER_SIDE_ENCRYPTION.clone(), "AES256");
            }
            "SSE-C" => {
                builder = builder.header(
                    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM.clone(),
                    "AES256",
                );
            }
            _ => {}
        }
    }

    Ok(builder
        .body(axum::body::Body::empty())
        .expect("response is valid"))
}

pub async fn handle_delete_object(
    State(state): State<S3AppState>,
    Path((bucket, key)): Path<(String, String)>,
) -> Result<Response> {
    // S3 DeleteObject is idempotent — return 204 even if object doesn't exist
    let object = match state.metadata.get_object(&bucket, &key) {
        Ok(obj) => obj,
        Err(_) => return Ok(StatusCode::NO_CONTENT.into_response()),
    };

    // Check dedup ref_count — only delete physical data if no other references
    let should_delete_physical = DedupManager::decrement_ref_count(
        &state.metadata,
        &object.pool_id,
        &object.object_hash,
    )?;

    if should_delete_physical {
        // No more references — delete from storage pool
        let mut pool = state.pool_manager.get_pool(&object.pool_id).await?;
        pool.delete_object(&object.object_hash).await?;
    }

    // Always remove metadata row
    state.metadata.delete_object(&bucket, &key)?;
    state.event_bus.emit_object_deleted(&bucket, &key);

    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn handle_delete_object_version(
    State(state): State<S3AppState>,
    Path((bucket, key, version_id)): Path<(String, String, String)>,
) -> Result<Response> {
    // Get the specific version
    let object = state
        .metadata
        .get_object_version(&bucket, &key, &version_id)?;

    // For delete markers, just remove the row
    if object.object_hash.is_empty() && object.size == 0 {
        state.metadata.delete_object_version(&bucket, &key, &version_id)?;
        return Ok(StatusCode::NO_CONTENT.into_response());
    }

    // Check dedup ref_count — only delete physical data if no other references
    let should_delete_physical = DedupManager::decrement_ref_count(
        &state.metadata,
        &object.pool_id,
        &object.object_hash,
    )?;

    if should_delete_physical {
        let mut pool = state.pool_manager.get_pool(&object.pool_id).await?;
        pool.delete_object(&object.object_hash).await?;
    }

    // Remove the version row
    state.metadata.delete_object_version(&bucket, &key, &version_id)?;
    state.event_bus.emit_object_deleted(&bucket, &key);

    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn handle_list_objects(
    State(state): State<S3AppState>,
    Path(bucket): Path<String>,
) -> Result<Response> {
    state.metadata.get_bucket(&bucket)?;

    let objects = state.metadata.list_objects(&bucket, None, 1000)?;

    // Build XML response
    let mut xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <Name>{}</Name>
    <Prefix></Prefix>
    <KeyCount>{}</KeyCount>
    <MaxKeys>1000</MaxKeys>
    <IsTruncated>false</IsTruncated>
"#,
        bucket,
        objects.len()
    );

    for obj in objects {
        xml.push_str(&format!(
            r#"
    <Contents>
        <Key>{}</Key>
        <LastModified>{}</LastModified>
        <Size>{}</Size>
        <ETag>"{}"</ETag>
        <StorageClass>{}</StorageClass>
    </Contents>
"#,
            obj.key,
            obj.last_modified.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
            obj.size,
            obj.etag,
            obj.storage_class
        ));
    }

    xml.push_str("\n</ListBucketResult>");

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        xml,
    )
        .into_response())
}

pub async fn handle_list_object_versions(
    State(state): State<S3AppState>,
    Path(bucket): Path<String>,
) -> Result<Response> {
    state.metadata.get_bucket(&bucket)?;

    let versions = state.metadata.list_object_versions(&bucket, 1000)?;

    let mut xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListObjectVersionsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <Name>{}</Name>
    <Prefix></Prefix>
    <MaxKeys>1000</MaxKeys>
    <IsTruncated>false</IsTruncated>
"#,
        bucket
    );

    for obj in versions {
        if obj.object_hash.is_empty() && obj.size == 0 {
            // Delete marker
            xml.push_str(&format!(
                r#"
    <DeleteMarker>
        <Key>{}</Key>
        <VersionId>{}</VersionId>
        <IsLatest>{}</IsLatest>
        <LastModified>{}</LastModified>
        <Owner><ID>owner-id</ID></Owner>
    </DeleteMarker>
"#,
                escape_xml(&obj.key),
                obj.version_id.as_deref().unwrap_or("null"),
                if obj.version_id.is_none() { "true" } else { "false" },
                obj.modified_at.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
            ));
        } else {
            xml.push_str(&format!(
                r#"
    <Version>
        <Key>{}</Key>
        <VersionId>{}</VersionId>
        <IsLatest>{}</IsLatest>
        <LastModified>{}</LastModified>
        <ETag>"{}"</ETag>
        <Size>{}</Size>
        <StorageClass>{}</StorageClass>
        <Owner><ID>owner-id</ID></Owner>
    </Version>
"#,
                escape_xml(&obj.key),
                obj.version_id.as_deref().unwrap_or("null"),
                if obj.version_id.is_none() { "true" } else { "false" },
                obj.modified_at.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                obj.etag,
                obj.size,
                obj.storage_class,
            ));
        }
    }

    xml.push_str("\n</ListObjectVersionsResult>");

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        xml,
    )
        .into_response())
}

pub async fn handle_batch_delete(
    State(state): State<S3AppState>,
    Path(bucket): Path<String>,
    body: Bytes,
) -> Result<Response> {
    state.metadata.get_bucket(&bucket)?;

    // Parse XML body: <Delete><Object><Key>...</Key></Object>...</Delete>
    let body_str = String::from_utf8(body.to_vec())
        .map_err(|_| Error::InvalidRequest("Invalid XML body".to_string()))?;

    let mut keys = Vec::new();
    // Simple XML parsing for <Key>...</Key>
    for part in body_str.split("<Key>") {
        if let Some(end) = part.find("</Key>") {
            keys.push(part[..end].to_string());
        }
    }

    let mut xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<DeleteResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
"#.to_string();

    for key in &keys {
        match state.metadata.get_object(&bucket, key) {
            Ok(object) => {
                let should_delete_physical = DedupManager::decrement_ref_count(
                    &state.metadata,
                    &object.pool_id,
                    &object.object_hash,
                )
                .unwrap_or(false);

                if should_delete_physical {
                    if let Ok(mut pool) = state.pool_manager.get_pool(&object.pool_id).await {
                        let _ = pool.delete_object(&object.object_hash).await;
                    }
                }

                if state.metadata.delete_object(&bucket, key).is_ok() {
                    state.event_bus.emit_object_deleted(&bucket, key);
                    xml.push_str(&format!(
                        r#"<Deleted><Key>{}</Key></Deleted>
"#,
                        escape_xml(key)
                    ));
                } else {
                    xml.push_str(&format!(
                        r#"<Error><Key>{}</Key><Code>InternalError</Code></Error>
"#,
                        escape_xml(key)
                    ));
                }
            }
            Err(_) => {
                xml.push_str(&format!(
                    r#"<Error><Key>{}</Key><Code>NoSuchKey</Code></Error>
"#,
                    escape_xml(key)
                ));
            }
        }
    }

    xml.push_str("</DeleteResult>");

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        xml,
    )
        .into_response())
}

// === Helper functions ===

/// Parsed SSE encryption type from request headers.
#[derive(Debug)]
enum SseType {
    None,
    SseS3,
    SseC(String), // base64-encoded customer key
}

/// Parse the SSE type from request headers.
fn parse_sse_type(headers: &HeaderMap) -> SseType {
    // Check for SSE-S3
    if let Some(v) = headers.get("x-amz-server-side-encryption") {
        if let Ok(val) = v.to_str() {
            if val == "AES256" {
                return SseType::SseS3;
            }
        }
    }

    // Check for SSE-C
    if let Some(v) = headers.get("x-amz-server-side-encryption-customer-algorithm") {
        if let Ok(val) = v.to_str() {
            if val == "AES256" {
                if let Some(key_val) = headers.get("x-amz-server-side-encryption-customer-key") {
                    if let Ok(key_b64) = key_val.to_str() {
                        return SseType::SseC(key_b64.to_string());
                    }
                }
            }
        }
    }

    SseType::None
}

/// Build the PUT response with appropriate SSE headers.
fn build_put_response(etag: &str, sse_type: &SseType) -> Response {
    let mut builder = Response::builder().status(StatusCode::OK);
    builder = builder.header(axum::http::header::ETAG, etag);
    builder = builder.header(
        HeaderName::from_static("x-amz-version-id"),
        "null",
    );

    match sse_type {
        SseType::SseS3 => {
            builder = builder.header(X_AMZ_SERVER_SIDE_ENCRYPTION.clone(), "AES256");
        }
        SseType::SseC(_) => {
            builder = builder.header(
                X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM.clone(),
                "AES256",
            );
        }
        SseType::None => {}
    }

    builder
        .body(axum::body::Body::empty())
        .expect("response is valid")
}

// === Helper functions ===

/// Parse and handle HTTP Range header for partial content responses.
async fn handle_range_response(
    data: &[u8],
    range_header: &str,
    object: &Object,
) -> Result<Response> {
    let total_len = data.len() as u64;

    // Parse range: "bytes=start-end", "bytes=start-", "bytes=-suffix"
    let range_spec = range_header.strip_prefix("bytes=").ok_or(Error::InvalidRange)?;

    let (start, end) = if let Some(suffix) = range_spec.strip_prefix('-') {
        // bytes=-N (last N bytes)
        let suffix_len: u64 = suffix.parse().map_err(|_| Error::InvalidRange)?;
        if suffix_len == 0 || suffix_len > total_len {
            return Err(Error::InvalidRange);
        }
        (total_len - suffix_len, total_len - 1)
    } else {
        let parts: Vec<&str> = range_spec.splitn(2, '-').collect();
        if parts.len() != 2 {
            return Err(Error::InvalidRange);
        }
        let start: u64 = parts[0].parse().map_err(|_| Error::InvalidRange)?;
        let end = if parts[1].is_empty() {
            total_len - 1
        } else {
            parts[1].parse().map_err(|_| Error::InvalidRange)?
        };

        if start > end || start >= total_len {
            return Err(Error::InvalidRange);
        }

        let end = end.min(total_len - 1);
        (start, end)
    };

    let slice_data = &data[start as usize..=end as usize];
    let content_range = format!("bytes {}-{}/{}", start, end, total_len);

    let mut builder = Response::builder().status(StatusCode::PARTIAL_CONTENT);
    builder = builder.header(
        axum::http::header::CONTENT_TYPE,
        object
            .content_type
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string()),
    );
    builder = builder.header(axum::http::header::CONTENT_LENGTH, slice_data.len());
    builder = builder.header(axum::http::header::ETAG, &object.etag);
    builder = builder.header(HeaderName::from_static("content-range"), content_range);
    builder = builder.header(HeaderName::from_static("accept-ranges"), "bytes");

    Ok(builder
        .body(axum::body::Body::from(slice_data.to_vec()))
        .expect("response is valid"))
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
