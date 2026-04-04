use crate::api::s3::handler::S3AppState;
use crate::error::Result;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
pub async fn handle_list_buckets(State(state): State<S3AppState>) -> Result<Response> {
    let buckets = state.metadata.list_buckets()?;

    // Manually construct XML to avoid serde-xml-rs issues
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    xml.push_str("<ListAllMyBucketsResult>");
    xml.push_str("<Owner><ID>owner-id</ID><DisplayName>Owner</DisplayName></Owner>");
    xml.push_str("<Buckets>");

    for bucket in buckets {
        xml.push_str("<Bucket>");
        xml.push_str(&format!("<Name>{}</Name>", escape_xml(&bucket.name)));
        xml.push_str(&format!(
            "<CreationDate>{}</CreationDate>",
            bucket.created_at.to_rfc3339()
        ));
        xml.push_str("</Bucket>");
    }

    xml.push_str("</Buckets>");
    xml.push_str("</ListAllMyBucketsResult>");

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        xml,
    )
        .into_response())
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub async fn handle_create_bucket(
    State(state): State<S3AppState>,
    Path(bucket_name): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    // Validate bucket name
    if !is_valid_bucket_name(&bucket_name) {
        return Err(crate::error::Error::InvalidRequest(
            "Invalid bucket name".to_string(),
        ));
    }

    // Check if bucket exists
    if state.metadata.bucket_exists(&bucket_name)? {
        return Err(crate::error::Error::BucketAlreadyExists(bucket_name));
    }

    // Get region from headers or default
    let region = headers
        .get("x-amz-bucket-region")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("us-east-1");

    // Get preferred pool from headers (optional)
    let preferred_pool = headers
        .get("x-amz-bucket-pool")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Validate pool if specified
    if let Some(ref pool_id) = preferred_pool {
        let pools = state.pool_manager.get_all_pools().await;
        if !pools.iter().any(|p| &p.id == pool_id) {
            return Err(crate::error::Error::InvalidRequest(format!(
                "Pool '{}' not found",
                pool_id
            )));
        }
    }

    // Create bucket
    state.metadata.create_bucket(
        &bucket_name,
        "owner",
        Some(region.to_string()),
        preferred_pool,
    )?;

    state.event_bus.emit_bucket_created(&bucket_name);

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        "",
    )
        .into_response())
}

pub async fn handle_delete_bucket(
    State(state): State<S3AppState>,
    Path(bucket_name): Path<String>,
) -> Result<Response> {
    state.metadata.delete_bucket(&bucket_name)?;
    state.event_bus.emit_bucket_deleted(&bucket_name);

    Ok((
        StatusCode::NO_CONTENT,
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        "",
    )
        .into_response())
}

pub async fn handle_head_bucket(
    State(state): State<S3AppState>,
    Path(bucket_name): Path<String>,
) -> Result<Response> {
    state.metadata.get_bucket(&bucket_name)?;

    Ok(StatusCode::OK.into_response())
}

pub async fn handle_get_bucket_location(
    State(state): State<S3AppState>,
    Path(bucket_name): Path<String>,
) -> Result<Response> {
    let bucket = state.metadata.get_bucket(&bucket_name)?;

    let location = if bucket.region == "us-east-1" || bucket.region.is_empty() {
        String::new()
    } else {
        bucket.region
    };

    // S3 returns empty content for us-east-1, or region string for others
    let xml = if location.is_empty() {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
            <LocationConstraint xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\"/>"
        )
    } else {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
            <LocationConstraint xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
            <LocationConstraint>{}</LocationConstraint>\
            </LocationConstraint>",
            escape_xml(&location)
        )
    };

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        xml,
    )
        .into_response())
}

pub async fn handle_get_bucket_acl(
    State(state): State<S3AppState>,
    Path(bucket_name): Path<String>,
) -> Result<Response> {
    state.metadata.get_bucket(&bucket_name)?;

    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
        <AccessControlPolicy xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
        <Owner>\
        <ID>owner-id</ID>\
        <DisplayName>Owner</DisplayName>\
        </Owner>\
        <AccessControlList>\
        <Grant>\
        <Grantee xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:type=\"CanonicalUser\">\
        <ID>owner-id</ID>\
        <DisplayName>Owner</DisplayName>\
        </Grantee>\
        <Permission>FULL_CONTROL</Permission>\
        </Grant>\
        </AccessControlList>\
        </AccessControlPolicy>"
    );

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        xml,
    )
        .into_response())
}

pub async fn handle_put_bucket_acl(
    State(_state): State<S3AppState>,
    Path(_bucket_name): Path<String>,
) -> Result<Response> {
    // Accept ACL changes silently (stub implementation)
    Ok(StatusCode::OK.into_response())
}

fn is_valid_bucket_name(name: &str) -> bool {
    if name.len() < 3 || name.len() > 63 {
        return false;
    }

    // Must start and end with alphanumeric
    if !name.chars().next().unwrap().is_alphanumeric()
        || !name.chars().last().unwrap().is_alphanumeric()
    {
        return false;
    }

    // Only contain lowercase letters, numbers, hyphens
    for ch in name.chars() {
        if !ch.is_alphanumeric() && ch != '-' {
            return false;
        }
    }

    true
}
