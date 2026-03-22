use crate::api::s3::handler::S3AppState;
use crate::error::Result;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct ListAllMyBucketsResult {
    #[serde(rename = "Owner")]
    owner: Owner,
    #[serde(rename = "Buckets")]
    buckets: Buckets,
}

#[derive(Debug, Serialize)]
struct Owner {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "DisplayName")]
    display_name: String,
}

#[derive(Debug, Serialize)]
struct Buckets {
    #[serde(rename = "Bucket", default)]
    bucket: Vec<Bucket>,
}

#[derive(Debug, Serialize)]
struct Bucket {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "CreationDate")]
    creation_date: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateBucketConfiguration {
    #[serde(rename = "LocationConstraint")]
    location_constraint: Option<String>,
}

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
        xml.push_str(&format!("<CreationDate>{}</CreationDate>", bucket.created_at.to_rfc3339()));
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
        return Err(crate::error::Error::InvalidRequest("Invalid bucket name".to_string()));
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

    // Create bucket
    state.metadata.create_bucket(&bucket_name, "owner", Some(region.to_string()))?;

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
