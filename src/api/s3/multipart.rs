use crate::api::s3::handler::S3AppState;
use crate::error::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct UploadQuery {
    uploadId: Option<String>,
}

#[derive(Debug, Serialize)]
struct InitiateMultipartUploadResult {
    #[serde(rename = "Bucket")]
    bucket: String,
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "UploadId")]
    upload_id: String,
}

pub async fn handle_create_multipart_upload(
    State(state): State<S3AppState>,
    Path((bucket, key)): Path<(String, String)>,
) -> Result<Response> {
    state.metadata.get_bucket(&bucket)?;

    let upload = state.multipart_manager.lock().await.create_upload(bucket, key, "owner".to_string())?;

    let result = InitiateMultipartUploadResult {
        bucket: upload.bucket.clone(),
        key: upload.key.clone(),
        upload_id: upload.upload_id.clone(),
    };

    let xml = serde_xml_rs::to_string(&result).map_err(|e| crate::error::Error::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        xml,
    )
        .into_response())
}

pub async fn handle_upload_part() -> Result<Response> {
    // TODO: Implement part upload
    Ok(StatusCode::OK.into_response())
}

pub async fn handle_complete_multipart_upload() -> Result<Response> {
    // TODO: Implement complete multipart
    Ok(StatusCode::OK.into_response())
}

pub async fn handle_abort_multipart_upload() -> Result<Response> {
    // TODO: Implement abort multipart
    Ok(StatusCode::OK.into_response())
}

pub async fn handle_list_parts() -> Result<Response> {
    // TODO: Implement list parts
    Ok(StatusCode::OK.into_response())
}

pub async fn handle_list_multipart_uploads() -> Result<Response> {
    // TODO: Implement list uploads
    Ok(StatusCode::OK.into_response())
}
