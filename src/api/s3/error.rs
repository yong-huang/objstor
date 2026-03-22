use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug)]
pub enum S3Error {
    NoSuchBucket(String),
    NoSuchKey(String),
    BucketAlreadyExists(String),
    InvalidBucketName(String),
    AccessDenied,
    SignatureDoesNotMatch,
    InvalidArgument(String),
    InternalError,
}

impl IntoResponse for S3Error {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            S3Error::NoSuchBucket(b) => (StatusCode::NOT_FOUND, "NoSuchBucket", b),
            S3Error::NoSuchKey(k) => (StatusCode::NOT_FOUND, "NoSuchKey", k),
            S3Error::BucketAlreadyExists(b) => (StatusCode::CONFLICT, "BucketAlreadyExists", b),
            S3Error::InvalidBucketName(b) => (StatusCode::BAD_REQUEST, "InvalidBucketName", b),
            S3Error::AccessDenied => (StatusCode::FORBIDDEN, "AccessDenied", "Access Denied".to_string()),
            S3Error::SignatureDoesNotMatch => (StatusCode::FORBIDDEN, "SignatureDoesNotMatch", "Signature mismatch".to_string()),
            S3Error::InvalidArgument(m) => (StatusCode::BAD_REQUEST, "InvalidArgument", m),
            S3Error::InternalError => (StatusCode::INTERNAL_SERVER_ERROR, "InternalError", "Internal error".to_string()),
        };

        let error_body = S3ErrorResponse {
            error: S3ErrorDetail {
                code,
                message,
                request_id: uuid::Uuid::new_v4().to_string(),
            },
        };

        let xml = serde_xml_rs::to_string(&error_body).unwrap_or_default();

        (status, xml).into_response()
    }
}

#[derive(Debug, serde::Serialize)]
struct S3ErrorResponse {
    #[serde(rename = "Error")]
    error: S3ErrorDetail,
}

#[derive(Debug, serde::Serialize)]
struct S3ErrorDetail {
    #[serde(rename = "Code")]
    code: &'static str,
    #[serde(rename = "Message")]
    message: String,
    #[serde(rename = "RequestId")]
    request_id: String,
}
