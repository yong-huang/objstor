use crate::api::s3::handler::S3AppState;
use crate::error::Result;
use crate::metadata::Object;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use chrono::Utc;
use md5::{Digest, Md5};

pub async fn handle_put_object(
    State(state): State<S3AppState>,
    Path((bucket, key)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response> {
    // Verify bucket exists
    state.metadata.get_bucket(&bucket)?;

    // Calculate ETag (MD5 hash)
    let etag = format!("{:x}", md5::Md5::digest(&body));

    // Store object
    let mut pool = state.pool_manager.select_pool_for_object(body.len() as u64).await?;
    let location = pool.write_object(&body).await?;

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
        storage_class: "STANDARD".to_string(),
        tags: None,
        metadata: None,
    };

    state.metadata.create_object(&object)?;

    Ok((
        StatusCode::OK,
        [
            (axum::http::header::ETAG, etag),
            ("x-amz-version-id".parse::<axum::http::HeaderName>().unwrap(), "null".to_string()),
        ],
        "",
    )
        .into_response())
}

pub async fn handle_get_object(
    State(state): State<S3AppState>,
    Path((bucket, key)): Path<(String, String)>,
) -> Result<Response> {
    // Get object metadata
    let object = state.metadata.get_object(&bucket, &key)?;

    // Read object data from pool
    let pool = state.pool_manager.get_pool(&object.pool_id).await?;
    let data = pool.read_object(&object.object_hash).await?;

    Ok((
        StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, object.content_type.unwrap_or_else(|| "application/octet-stream".to_string())),
            (axum::http::header::CONTENT_LENGTH, object.size.to_string()),
            (axum::http::header::ETAG, object.etag.clone()),
            ("last-modified".parse::<axum::http::HeaderName>().unwrap(), format!("{}", object.modified_at.format("%a, %d %b %Y %H:%M:%S GMT"))),
        ],
        data,
    )
        .into_response())
}

pub async fn handle_head_object(
    State(state): State<S3AppState>,
    Path((bucket, key)): Path<(String, String)>,
) -> Result<Response> {
    let object = state.metadata.get_object(&bucket, &key)?;

    Ok((
        StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, object.content_type.unwrap_or_else(|| "application/octet-stream".to_string())),
            (axum::http::header::CONTENT_LENGTH, object.size.to_string()),
            (axum::http::header::ETAG, object.etag),
            ("last-modified".parse::<axum::http::HeaderName>().unwrap(), format!("{}", object.modified_at.format("%a, %d %b %Y %H:%M:%S GMT"))),
        ],
        (),
    )
        .into_response())
}

pub async fn handle_delete_object(
    State(state): State<S3AppState>,
    Path((bucket, key)): Path<(String, String)>,
) -> Result<Response> {
    // Get object metadata first
    let object = state.metadata.get_object(&bucket, &key)?;

    // Delete from storage pool
    let mut pool = state.pool_manager.get_pool(&object.pool_id).await?;
    pool.delete_object(&object.object_hash).await?;

    // Delete from metadata
    state.metadata.delete_object(&bucket, &key)?;

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
