use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    // Storage errors
    StorageNotFound(String),
    StorageFull(String),
    StorageUnavailable(String),
    NoAvailablePool,
    ObjectNotFound(String),
    ObjectCorrupted(String),

    // Bucket errors
    BucketNotFound(String),
    BucketAlreadyExists(String),
    BucketNotEmpty(String),

    // Database errors
    DatabaseError(rusqlite::Error),
    DatabaseMigrationError(String),

    // Authentication errors
    AuthenticationFailed(String),
    SignatureMismatch,
    AccessDenied,
    InvalidAccessKey,

    // Request errors
    InvalidRequest(String),
    InvalidHeaderValue(String),
    MissingHeader(String),
    ContentLengthMismatch,

    // Multipart upload errors
    InvalidPartOrder,
    InvalidPartSize,
    UploadNotFound(String),
    PartNotFound(String),

    // Configuration errors
    ConfigurationError(String),

    // I/O errors
    IoError(std::io::Error),

    // Serialization errors
    SerializationError(serde_json::Error),

    // Generic internal error
    InternalError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::StorageNotFound(p) => write!(f, "Storage pool not found: {}", p),
            Error::StorageFull(p) => write!(f, "Storage pool full: {}", p),
            Error::StorageUnavailable(p) => write!(f, "Storage pool unavailable: {}", p),
            Error::NoAvailablePool => write!(f, "No available storage pool"),
            Error::ObjectNotFound(k) => write!(f, "Object not found: {}", k),
            Error::ObjectCorrupted(k) => write!(f, "Object corrupted: {}", k),
            Error::BucketNotFound(b) => write!(f, "Bucket not found: {}", b),
            Error::BucketAlreadyExists(b) => write!(f, "Bucket already exists: {}", b),
            Error::BucketNotEmpty(b) => write!(f, "Bucket not empty: {}", b),
            Error::DatabaseError(e) => write!(f, "Database error: {}", e),
            Error::DatabaseMigrationError(e) => write!(f, "Migration error: {}", e),
            Error::AuthenticationFailed(e) => write!(f, "Authentication failed: {}", e),
            Error::SignatureMismatch => write!(f, "Signature mismatch"),
            Error::AccessDenied => write!(f, "Access denied"),
            Error::InvalidAccessKey => write!(f, "Invalid access key"),
            Error::InvalidRequest(e) => write!(f, "Invalid request: {}", e),
            Error::InvalidHeaderValue(e) => write!(f, "Invalid header value: {}", e),
            Error::MissingHeader(h) => write!(f, "Missing header: {}", h),
            Error::ContentLengthMismatch => write!(f, "Content length mismatch"),
            Error::InvalidPartOrder => write!(f, "Invalid part order"),
            Error::InvalidPartSize => write!(f, "Invalid part size"),
            Error::UploadNotFound(id) => write!(f, "Upload not found: {}", id),
            Error::PartNotFound(id) => write!(f, "Part not found: {}", id),
            Error::ConfigurationError(e) => write!(f, "Configuration error: {}", e),
            Error::IoError(e) => write!(f, "I/O error: {}", e),
            Error::SerializationError(e) => write!(f, "Serialization error: {}", e),
            Error::InternalError(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            Error::StorageNotFound(_) | Error::ObjectNotFound(_) | Error::BucketNotFound(_) => {
                (StatusCode::NOT_FOUND, "NoSuchKey", self.to_string())
            }
            Error::BucketAlreadyExists(_) => (
                StatusCode::CONFLICT,
                "BucketAlreadyExists",
                self.to_string(),
            ),
            Error::BucketNotEmpty(_) => (StatusCode::CONFLICT, "BucketNotEmpty", self.to_string()),
            Error::StorageFull(_) | Error::NoAvailablePool => (
                StatusCode::SERVICE_UNAVAILABLE,
                "StorageFull",
                self.to_string(),
            ),
            Error::AuthenticationFailed(_) => (
                StatusCode::UNAUTHORIZED,
                "InvalidAccessKeyId",
                self.to_string(),
            ),
            Error::SignatureMismatch | Error::InvalidAccessKey => (
                StatusCode::FORBIDDEN,
                "SignatureDoesNotMatch",
                self.to_string(),
            ),
            Error::AccessDenied => (StatusCode::FORBIDDEN, "AccessDenied", self.to_string()),
            Error::InvalidRequest(_) | Error::InvalidHeaderValue(_) | Error::MissingHeader(_) => {
                (StatusCode::BAD_REQUEST, "InvalidRequest", self.to_string())
            }
            Error::ContentLengthMismatch => {
                (StatusCode::BAD_REQUEST, "IncompleteBody", self.to_string())
            }
            Error::InvalidPartOrder => (
                StatusCode::BAD_REQUEST,
                "InvalidPartOrder",
                self.to_string(),
            ),
            Error::InvalidPartSize => (StatusCode::BAD_REQUEST, "InvalidPart", self.to_string()),
            Error::UploadNotFound(_) => (StatusCode::NOT_FOUND, "NoSuchUpload", self.to_string()),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "InternalError",
                "An internal error occurred".to_string(),
            ),
        };

        let body = json!({
            "Error": {
                "Code": error_code,
                "Message": message
            }
        });

        (status, Json(body)).into_response()
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::DatabaseError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::SerializationError(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
