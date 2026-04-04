use crate::api::events::EventBus;
use crate::api::rate_limit::RateLimiter;
use crate::api::s3::{bucket, object};
use crate::storage::encryption::MasterKeyManager;
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
    pub key_manager: Arc<MasterKeyManager>,
    pub event_bus: Arc<EventBus>,
    pub rate_limiter: Arc<RateLimiter>,
}

impl Clone for S3AppState {
    fn clone(&self) -> Self {
        Self {
            metadata: Arc::clone(&self.metadata),
            pool_manager: Arc::clone(&self.pool_manager),
            multipart_manager: Arc::clone(&self.multipart_manager),
            key_manager: Arc::clone(&self.key_manager),
            event_bus: Arc::clone(&self.event_bus),
            rate_limiter: Arc::clone(&self.rate_limiter),
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
        let query = uri.query().unwrap_or("");
        let parts: Vec<&str> = path
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        match (method.as_str(), parts.len(), parts.first()) {
            // List all buckets (empty path)
            ("GET", 0, _) => bucket::handle_list_buckets(State(state))
                .await
                .unwrap_or_else(|e| e.into_response()),

            // Bucket operations
            ("PUT", 1, Some(bucket_name)) => {
                if query.contains("acl") {
                    bucket::handle_put_bucket_acl(State(state), Path(bucket_name.to_string()))
                        .await
                        .unwrap_or_else(|e| e.into_response())
                } else {
                    bucket::handle_create_bucket(State(state), Path(bucket_name.to_string()), headers)
                        .await
                        .unwrap_or_else(|e| e.into_response())
                }
            }
            ("DELETE", 1, Some(bucket_name)) => {
                bucket::handle_delete_bucket(State(state), Path(bucket_name.to_string()))
                    .await
                    .unwrap_or_else(|e| e.into_response())
            }
            ("HEAD", 1, Some(bucket_name)) => {
                bucket::handle_head_bucket(State(state), Path(bucket_name.to_string()))
                    .await
                    .unwrap_or_else(|e| e.into_response())
            }
            ("GET", 1, Some(bucket_name)) => {
                if query.contains("location") {
                    bucket::handle_get_bucket_location(State(state), Path(bucket_name.to_string()))
                        .await
                        .unwrap_or_else(|e| e.into_response())
                } else if query.contains("acl") {
                    bucket::handle_get_bucket_acl(State(state), Path(bucket_name.to_string()))
                        .await
                        .unwrap_or_else(|e| e.into_response())
                } else if query.contains("versions") && !query.contains("versionId") {
                    object::handle_list_object_versions(State(state), Path(bucket_name.to_string()))
                        .await
                        .unwrap_or_else(|e| e.into_response())
                } else {
                    // Use object handler for listing objects in bucket
                    object::handle_list_objects(State(state), Path(bucket_name.to_string()))
                        .await
                        .unwrap_or_else(|e| e.into_response())
                }
            }
            // Batch delete (POST with ?delete)
            ("POST", 1, Some(bucket_name)) => {
                if query.contains("delete") {
                    object::handle_batch_delete(
                        State(state),
                        Path(bucket_name.to_string()),
                        body,
                    )
                    .await
                    .unwrap_or_else(|e| e.into_response())
                } else {
                    (StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed").into_response()
                }
            }

            // Object operations (2+ parts: bucket + key)
            ("PUT", 2.., Some(bucket_name)) => {
                let key = parts[1..].join("/");
                object::handle_put_object(
                    State(state),
                    Path((bucket_name.to_string(), key)),
                    headers,
                    body,
                )
                .await
                .unwrap_or_else(|e| e.into_response())
            }
            ("GET", 2.., Some(bucket_name)) => {
                let key = parts[1..].join("/");

                // Check for ?versionId query param
                if let Some(version_id) = get_query_param(query, "versionId") {
                    object::handle_get_object_version(
                        State(state),
                        Path((bucket_name.to_string(), key, version_id)),
                        headers,
                    )
                    .await
                    .unwrap_or_else(|e| e.into_response())
                } else {
                    object::handle_get_object(
                        State(state),
                        Path((bucket_name.to_string(), key)),
                        headers,
                    )
                    .await
                    .unwrap_or_else(|e| e.into_response())
                }
            }
            ("HEAD", 2.., Some(bucket_name)) => {
                let key = parts[1..].join("/");
                object::handle_head_object(State(state), Path((bucket_name.to_string(), key)))
                    .await
                    .unwrap_or_else(|e| e.into_response())
            }
            ("DELETE", 2.., Some(bucket_name)) => {
                let key = parts[1..].join("/");

                // Check for ?versionId query param (permanent version delete)
                if let Some(version_id) = get_query_param(query, "versionId") {
                    object::handle_delete_object_version(
                        State(state),
                        Path((bucket_name.to_string(), key, version_id)),
                    )
                    .await
                    .unwrap_or_else(|e| e.into_response())
                } else {
                    object::handle_delete_object(State(state), Path((bucket_name.to_string(), key)))
                        .await
                        .unwrap_or_else(|e| e.into_response())
                }
            }

            _ => (StatusCode::NOT_FOUND, "Not Found").into_response(),
        }
    }
}

/// Extract a query parameter value by key.
fn get_query_param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        if kv.next()? == key {
            return kv.next().map(|v| urlencoding::decode(v).unwrap_or_default().to_string());
        }
    }
    None
}
