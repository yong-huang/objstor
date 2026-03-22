use crate::api::s3::{bucket, multipart, object};
use crate::storage::multipart::MultipartUploadManager;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

pub struct S3AppState {
    pub metadata: Arc<crate::metadata::db::MetadataStore>,
    pub pool_manager: Arc<crate::storage::PoolManager>,
    pub multipart_manager: Arc<tokio::sync::Mutex<MultipartUploadManager>>,
}

impl Clone for S3AppState {
    fn clone(&self) -> Self {
        Self {
            metadata: Arc::clone(&self.metadata),
            pool_manager: Arc::clone(&self.pool_manager),
            multipart_manager: Arc::clone(&self.multipart_manager),
        }
    }
}

pub struct S3Handler;

impl S3Handler {
    pub async fn handle_request(
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: axum::body::Bytes,
        state: S3AppState,
    ) -> Response {
        // Parse URI to extract bucket and key
        let path = uri.path();
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).collect();

        match (method.as_str(), parts.len(), parts.first()) {
            // List all buckets (empty path)
            ("GET", 0, _) => {
                bucket::handle_list_buckets(State(state)).await.unwrap_or_else(|e| e.into_response())
            }

            // Bucket operations
            ("PUT", 1, Some(bucket)) => {
                bucket::handle_create_bucket(State(state), Path(bucket.to_string()), headers)
                    .await
                    .unwrap_or_else(|e| e.into_response())
            }
            ("DELETE", 1, Some(bucket)) => {
                bucket::handle_delete_bucket(State(state), Path(bucket.to_string()))
                    .await
                    .unwrap_or_else(|e| e.into_response())
            }
            ("HEAD", 1, Some(bucket)) => {
                bucket::handle_head_bucket(State(state), Path(bucket.to_string()))
                    .await
                    .unwrap_or_else(|e| e.into_response())
            }
            ("GET", 1, Some(bucket)) => {
                // Use object handler for listing objects in bucket
                object::handle_list_objects(State(state), Path(bucket.to_string()))
                    .await
                    .unwrap_or_else(|e| e.into_response())
            }

            // Object operations (2+ parts: bucket + key)
            ("PUT", 2.., Some(bucket)) => {
                let key = parts[1..].join("/");
                object::handle_put_object(State(state), Path((bucket.to_string(), key)), headers, body)
                    .await
                    .unwrap_or_else(|e| e.into_response())
            }
            ("GET", 2.., Some(bucket)) => {
                let key = parts[1..].join("/");
                object::handle_get_object(State(state), Path((bucket.to_string(), key)))
                    .await
                    .unwrap_or_else(|e| e.into_response())
            }
            ("HEAD", 2.., Some(bucket)) => {
                let key = parts[1..].join("/");
                object::handle_head_object(State(state), Path((bucket.to_string(), key)))
                    .await
                    .unwrap_or_else(|e| e.into_response())
            }
            ("DELETE", 2.., Some(bucket)) => {
                let key = parts[1..].join("/");
                object::handle_delete_object(State(state), Path((bucket.to_string(), key)))
                    .await
                    .unwrap_or_else(|e| e.into_response())
            }

            _ => (StatusCode::NOT_FOUND, "Not Found").into_response(),
        }
    }
}
