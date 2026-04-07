use crate::api::s3::handler::S3AppState;
use crate::config::Config;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Deserialize;
use sysinfo::{System, Disks};

pub async fn get_system_metrics() -> impl IntoResponse {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_usage = sys.global_cpu_usage();
    let cpu_brand = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let cpu_cores = sys.cpus().len();

    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let mem_percent = if total_mem > 0 {
        used_mem as f64 / total_mem as f64 * 100.0
    } else {
        0.0
    };

    // Find the disk containing the data directory
    let data_dir = std::env::current_dir().unwrap_or_default();
    let disks = Disks::new_with_refreshed_list();
    let mut disk_total: u64 = 0;
    let mut disk_available: u64 = 0;
    let mut disk_name = String::from("Unknown");

    for disk in disks.list() {
        if data_dir.starts_with(disk.mount_point()) {
            disk_total = disk.total_space();
            disk_available = disk.available_space();
            disk_name = disk.name().to_string_lossy().to_string();
            break;
        }
    }

    let disk_used = disk_total.saturating_sub(disk_available);
    let disk_percent = if disk_total > 0 {
        disk_used as f64 / disk_total as f64 * 100.0
    } else {
        0.0
    };

    let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
    let os_name = System::long_os_version().unwrap_or_else(|| "Unknown".to_string());
    let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
    let uptime_secs = System::uptime();

    Json(serde_json::json!({
        "cpu": {
            "usage_percent": cpu_usage,
            "brand": cpu_brand,
            "cores": cpu_cores,
        },
        "memory": {
            "total": total_mem,
            "used": used_mem,
            "percent": mem_percent,
        },
        "disk": {
            "total": disk_total,
            "used": disk_used,
            "available": disk_available,
            "percent": disk_percent,
            "name": disk_name,
        },
        "system": {
            "hostname": hostname,
            "os": os_name,
            "kernel": kernel_version,
            "uptime_secs": uptime_secs,
        }
    }))
}

pub async fn get_health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn get_metrics(State(state): State<S3AppState>) -> impl IntoResponse {
    let pools: Vec<crate::storage::pool::StoragePool> = state.pool_manager.get_all_pools().await;

    // Get buckets list
    let buckets = state.metadata.list_buckets().unwrap_or_default();
    let buckets_data: Vec<serde_json::Value> = buckets
        .iter()
        .map(|b| {
            serde_json::json!({
                "name": b.name,
                "created_at": b.created_at.to_rfc3339(),
                "region": b.region,
                "owner": b.owner,
                "preferred_pool": b.preferred_pool,
            })
        })
        .collect();

    // Count total objects from database
    let total_objects = state
        .metadata
        .conn()
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM objects", [], |row| row.get(0))
        .unwrap_or(0);

    // Query actual pool statistics from database for accuracy
    let pool_metrics: Vec<serde_json::Value> = pools
        .iter()
        .map(|p| {
            // Query actual object count for this pool from database
            let pool_objects: u64 = state.metadata.conn()
                .lock()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM objects WHERE pool_id = ?1",
                    [&p.id],
                    |row| row.get(0)
                )
                .unwrap_or(0);

            // Query actual used space for this pool from database
            let pool_used: u64 = state.metadata.conn()
                .lock()
                .unwrap()
                .query_row(
                    "SELECT COALESCE(SUM(size), 0) FROM objects WHERE pool_id = ?1",
                    [&p.id],
                    |row| row.get(0)
                )
                .unwrap_or(0);

            serde_json::json!({
                "id": p.id,
                "capacity": p.capacity,
                "used": pool_used,
                "objects": pool_objects,
                "status": format!("{:?}", p.status),
                "usage_ratio": if p.capacity > 0 { pool_used as f64 / p.capacity as f64 } else { 0.0 },
            })
        })
        .collect();

    // Calculate total storage from pool metrics (from database, not in-memory values)
    let total_used: u64 = pool_metrics.iter()
        .filter_map(|p| p["used"].as_u64())
        .sum();
    let total_capacity: u64 = pool_metrics.iter()
        .filter_map(|p| p["capacity"].as_u64())
        .sum();

    Json(serde_json::json!({
        "storage": {
            "used": total_used,
            "capacity": total_capacity,
            "usage_ratio": if total_capacity > 0 { total_used as f64 / total_capacity as f64 } else { 0.0 }
        },
        "buckets": buckets_data,
        "pools": pool_metrics,
        "total_objects": total_objects,
    }))
}

pub async fn get_buckets_api(State(state): State<S3AppState>) -> impl IntoResponse {
    let buckets = state.metadata.list_buckets().unwrap_or_default();

    let buckets_data: Vec<serde_json::Value> = buckets
        .iter()
        .map(|b| {
            serde_json::json!({
                "name": b.name,
                "created_at": b.created_at.to_rfc3339(),
                "region": b.region,
                "owner": b.owner,
                "versioning_enabled": b.versioning_enabled,
                "preferred_pool": b.preferred_pool,
            })
        })
        .collect();

    Json(serde_json::json!({
        "buckets": buckets_data
    }))
}

pub async fn get_config() -> impl IntoResponse {
    match Config::from_file("data/config/objstor.json") {
        Ok(config) => Json(serde_json::json!({
            "server": {
                "host": config.server.host,
                "port": config.server.port,
                "s3_port": config.server.s3_port,
                "log_level": config.server.log_level,
                "log_dir": config.server.log_dir.to_string_lossy().to_string(),
                "max_request_size": config.server.max_request_size,
            },
            "storage": {
                "data_dir": config.storage.data_dir.to_string_lossy().to_string(),
                "scheduler": {
                    "strategy": config.storage.scheduler.strategy,
                    "rebalance_threshold": config.storage.scheduler.rebalance_threshold,
                },
            },
            "ai": {
                "enabled": config.ai.enabled,
                "api_endpoint": config.ai.api_endpoint,
                "api_key": config.ai.api_key,
                "model": config.ai.model,
                "max_tokens": config.ai.max_tokens,
                "timeout_secs": config.ai.timeout_secs,
                "auto_tag": config.ai.auto_tag,
            }
        })),
        Err(_) => {
            Json(serde_json::json!({
                "error": "Failed to load configuration"
            }))
        }
    }
}

pub async fn update_config(
    State(_state): State<S3AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Load current config
    let mut config = match Config::from_file("data/config/objstor.json") {
        Ok(cfg) => cfg,
        Err(_) => {
            return Json(serde_json::json!({
                "success": false,
                "error": "Failed to load current configuration"
            }))
        }
    };

    // Update server config
    if let Some(server) = payload.get("server") {
        if let Some(host) = server.get("host").and_then(|v| v.as_str()) {
            config.server.host = host.to_string();
        }
        if let Some(port) = server.get("port").and_then(|v| v.as_u64()) {
            config.server.port = port as u16;
        }
        if let Some(s3_port) = server.get("s3_port").and_then(|v| v.as_u64()) {
            config.server.s3_port = s3_port as u16;
        }
        if let Some(log_level) = server.get("log_level").and_then(|v| v.as_str()) {
            config.server.log_level = log_level.to_string();
        }
        if let Some(log_dir) = server.get("log_dir").and_then(|v| v.as_str()) {
            config.server.log_dir = log_dir.into();
        }
        if let Some(max_request_size) = server.get("max_request_size").and_then(|v| v.as_u64()) {
            config.server.max_request_size = max_request_size as usize;
        }
    }

    // Update storage config
    if let Some(storage) = payload.get("storage") {
        if let Some(data_dir) = storage.get("data_dir").and_then(|v| v.as_str()) {
            config.storage.data_dir = data_dir.into();
        }
        if let Some(scheduler) = storage.get("scheduler") {
            if let Some(strategy) = scheduler.get("strategy").and_then(|v| v.as_str()) {
                config.storage.scheduler.strategy = strategy.to_string();
            }
            if let Some(threshold) = scheduler.get("rebalance_threshold").and_then(|v| v.as_f64()) {
                config.storage.scheduler.rebalance_threshold = threshold;
            }
        }
    }

    // Update AI config
    if let Some(ai) = payload.get("ai") {
        if let Some(enabled) = ai.get("enabled").and_then(|v| v.as_bool()) {
            config.ai.enabled = enabled;
        }
        if let Some(endpoint) = ai.get("api_endpoint").and_then(|v| v.as_str()) {
            config.ai.api_endpoint = endpoint.to_string();
        }
        if let Some(key) = ai.get("api_key").and_then(|v| v.as_str()) {
            config.ai.api_key = key.to_string();
        }
        if let Some(model) = ai.get("model").and_then(|v| v.as_str()) {
            config.ai.model = model.to_string();
        }
        if let Some(max_tokens) = ai.get("max_tokens").and_then(|v| v.as_u64()) {
            config.ai.max_tokens = max_tokens as u32;
        }
        if let Some(timeout) = ai.get("timeout_secs").and_then(|v| v.as_u64()) {
            config.ai.timeout_secs = timeout;
        }
        if let Some(auto_tag) = ai.get("auto_tag").and_then(|v| v.as_bool()) {
            config.ai.auto_tag = auto_tag;
        }
    }

    // Save updated config
    match config.save_to_default_path() {
        Ok(_) => {
            Json(serde_json::json!({
                "success": true,
                "message": "Configuration updated successfully. Please restart the service for changes to take effect."
            }))
        }
        Err(e) => {
            Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to save configuration: {}", e)
            }))
        }
    }
}

// ===== Access Key Management =====

pub async fn list_access_keys(State(state): State<S3AppState>) -> Response {
    match state.metadata.list_access_keys() {
        Ok(keys) => {
            let data: Vec<serde_json::Value> = keys
                .iter()
                .map(|k| {
                    serde_json::json!({
                        "access_key_id": k.access_key_id,
                        "owner": k.owner,
                        "created_at": k.created_at.to_rfc3339(),
                        "status": k.status,
                    })
                })
                .collect();
            let json = serde_json::json!({ "keys": data }).to_string();
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(json))
                .unwrap()
        }
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", e)),
    }
}

#[derive(Deserialize)]
pub struct CreateAccessKeyRequest {
    pub access_key_id: String,
    pub secret_key: String,
}

pub async fn create_access_key(
    State(state): State<S3AppState>,
    Json(body): Json<CreateAccessKeyRequest>,
) -> Response {
    if body.access_key_id.is_empty() || body.secret_key.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "access_key_id and secret_key are required");
    }
    match state.metadata.create_access_key(&body.access_key_id, &body.secret_key, "admin") {
        Ok(_) => json_ok(StatusCode::CREATED, "Access key created"),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", e)),
    }
}

#[derive(Deserialize)]
pub struct UpdateAccessKeyRequest {
    pub secret_key: String,
}

pub async fn update_access_key(
    State(state): State<S3AppState>,
    axum::extract::Path(key_id): axum::extract::Path<String>,
    Json(body): Json<UpdateAccessKeyRequest>,
) -> Response {
    if body.secret_key.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "secret_key is required");
    }
    match state.metadata.update_access_key(&key_id, &body.secret_key) {
        Ok(_) => json_ok(StatusCode::OK, "Access key updated"),
        Err(e) => json_error(StatusCode::NOT_FOUND, &format!("{}", e)),
    }
}

pub async fn delete_access_key(
    State(state): State<S3AppState>,
    axum::extract::Path(key_id): axum::extract::Path<String>,
) -> Response {
    match state.metadata.delete_access_key(&key_id) {
        Ok(_) => json_ok(StatusCode::OK, "Access key deleted"),
        Err(e) => json_error(StatusCode::NOT_FOUND, &format!("{}", e)),
    }
}

pub async fn get_bucket_stats(State(state): State<S3AppState>) -> impl IntoResponse {
    let conn = state.metadata.conn().lock().unwrap();

    let mut stmt = conn
        .prepare(
            "SELECT bucket, COUNT(*) as object_count, COALESCE(SUM(size), 0) as total_size FROM objects GROUP BY bucket",
        )
        .unwrap();

    let bucket_stats: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "name": row.get::<_, String>(0)?,
                "object_count": row.get::<_, u64>(1)?,
                "total_size": row.get::<_, u64>(2)?,
            }))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    Json(serde_json::json!({
        "buckets": bucket_stats,
    }))
}

pub async fn get_request_stats(State(state): State<S3AppState>) -> impl IntoResponse {
    let conn = state.metadata.conn().lock().unwrap();

    // Latency buckets (5-min intervals for last 30 min)
    let mut latency_stmt = conn
        .prepare(
            "SELECT (strftime('%s', timestamp) / 300) * 300 as bucket, AVG(duration_ms) as avg_latency \
             FROM audit_logs \
             WHERE timestamp >= datetime('now', '-30 minutes') AND duration_ms IS NOT NULL \
             GROUP BY bucket ORDER BY bucket",
        )
        .unwrap();

    let latency: Vec<serde_json::Value> = latency_stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "bucket_ts": row.get::<_, i64>(0)?,
                "avg_ms": row.get::<_, f64>(1)?,
            }))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Status code distribution (last 1000 logs)
    let mut status_stmt = conn
        .prepare(
            "SELECT CASE \
                WHEN status_code >= 200 AND status_code < 300 THEN '2xx' \
                WHEN status_code >= 300 AND status_code < 400 THEN '3xx' \
                WHEN status_code >= 400 AND status_code < 500 THEN '4xx' \
                ELSE '5xx' \
              END as status_class, COUNT(*) as count \
              FROM (SELECT status_code FROM audit_logs ORDER BY id DESC LIMIT 1000) \
              GROUP BY status_class",
        )
        .unwrap();

    let status_codes: Vec<serde_json::Value> = status_stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "class": row.get::<_, String>(0)?,
                "count": row.get::<_, u64>(1)?,
            }))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Method distribution (last 1000 logs)
    let mut method_stmt = conn
        .prepare(
            "SELECT method, COUNT(*) as count \
              FROM (SELECT method FROM audit_logs ORDER BY id DESC LIMIT 1000) \
              GROUP BY method",
        )
        .unwrap();

    let methods: Vec<serde_json::Value> = method_stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "method": row.get::<_, String>(0)?,
                "count": row.get::<_, u64>(1)?,
            }))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    Json(serde_json::json!({
        "latency": latency,
        "status_codes": status_codes,
        "methods": methods,
    }))
}

// ===== AI Search =====

pub async fn get_ai_models() -> Response {
    let config = match Config::from_file("data/config/objstor.json") {
        Ok(cfg) => cfg,
        Err(_) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load configuration",
            );
        }
    };

    if !config.ai.enabled {
        return json_error(StatusCode::BAD_REQUEST, "AI features are not enabled");
    }

    let body_bytes = match super::ai_utils::http_request(
        &config.ai.api_endpoint,
        &config.ai.api_key,
        "GET",
        "/v1/models",
        None,
    )
    .await
    {
        Ok(b) => b,
        Err(r) => return r,
    };

    let body: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to parse models response: {}", e),
            )
        }
    };

    let models: Vec<String> = body
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    item.get("id")
                        .and_then(|id| id.as_str())
                        .map(String::from)
                })
                .collect()
        })
        .unwrap_or_default();

    let json = serde_json::json!({ "success": true, "models": models }).to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}

#[derive(Deserialize)]
pub struct AiSearchRequest {
    pub query: String,
    pub bucket: Option<String>,
}

pub async fn ai_search(
    State(state): State<S3AppState>,
    Json(body): Json<AiSearchRequest>,
) -> Response {
    let config = match super::ai_utils::ensure_ai_enabled() {
        Ok(c) => c,
        Err(r) => return r,
    };

    let system_prompt = r#"You are a search query translator for an S3-compatible object storage system. The user will provide a natural language search query about stored objects. You must convert it into a JSON filter object.

The filter object can contain these optional fields:
- "bucket": string — exact bucket name
- "prefix": string — key prefix (e.g. "photos/")
- "key_contains": string — substring the key must contain
- "min_size": number — minimum size in bytes
- "max_size": number — maximum size in bytes
- "content_type": string — content type to match (e.g. "image/png", "application/pdf")
- "min_age_days": number — objects at least this many days old
- "max_age_days": number — objects at most this many days old

Common mappings:
- "PDF files" → {"content_type": "application/pdf"}
- "images" → {"content_type": "image/"}
- "videos" → {"content_type": "video/"}
- "larger than X MB" → {"min_size": X * 1024 * 1024}
- "smaller than X GB" → {"max_size": X * 1024 * 1024 * 1024}
- "uploaded last week" → {"max_age_days": 7}
- "older than a month" → {"min_age_days": 30}

Respond ONLY with the JSON object, no explanation, no markdown code fences."#;

    let content = match super::ai_utils::call_llm(&config, system_prompt, &body.query).await {
        Ok(c) => c,
        Err(r) => return r,
    };

    let filter: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to parse LLM filter JSON: {}. LLM returned: {}", e, content),
            );
        }
    };

    let objects = state
        .metadata
        .search_objects_advanced(&filter, body.bucket.as_deref(), 100);

    match objects {
        Ok(objects) => {
            let json = serde_json::json!({
                "success": true,
                "filter": filter,
                "objects": objects,
                "count": objects.len()
            })
            .to_string();
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(json))
                .unwrap()
        }
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("Search failed: {}", e)),
    }
}

// ===== AI Auto-Tagging =====

#[derive(Deserialize)]
pub struct AiTagRequest {
    pub bucket: String,
    pub key: String,
}

pub async fn ai_generate_tags(
    State(state): State<S3AppState>,
    Json(body): Json<AiTagRequest>,
) -> Response {
    let config = match super::ai_utils::ensure_ai_enabled() {
        Ok(c) => c,
        Err(r) => return r,
    };

    let obj = match state.metadata.get_object(&body.bucket, &body.key) {
        Ok(o) => o,
        Err(_) => return json_error(StatusCode::NOT_FOUND, "Object not found"),
    };

    let user_msg = format!(
        "Filename: {}\nContent-Type: {}\nSize: {} bytes",
        obj.key,
        obj.content_type.as_deref().unwrap_or("unknown"),
        obj.size,
    );

    let system_prompt = r#"Generate 3-8 relevant tags for this file as a JSON object where keys are tag names and values are empty strings. Example: {"document": "","invoice": "","finance": ""}
Respond ONLY with the JSON object, no explanation, no markdown code fences."#;

    let content = match super::ai_utils::call_llm(&config, system_prompt, &user_msg).await {
        Ok(c) => c,
        Err(r) => return r,
    };

    let tags: std::collections::HashMap<String, String> = match serde_json::from_str(&content) {
        Ok(t) => t,
        Err(e) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to parse tags JSON: {}. LLM returned: {}", e, content),
            );
        }
    };

    if let Err(e) = state.metadata.update_object_tags(&body.bucket, &body.key, &tags) {
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to save tags: {}", e),
        );
    }

    let json = serde_json::json!({ "success": true, "tags": tags }).to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}

#[derive(Deserialize)]
pub struct AiBulkTagRequest {
    pub bucket: String,
}

pub async fn ai_bulk_generate_tags(
    State(state): State<S3AppState>,
    Json(body): Json<AiBulkTagRequest>,
) -> Response {
    let config = match super::ai_utils::ensure_ai_enabled() {
        Ok(c) => c,
        Err(r) => return r,
    };

    let objects = match state.metadata.search_objects(
        "",
        Some(&body.bucket),
        50,
    ) {
        Ok(o) => o,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", e)),
    };

    if objects.is_empty() {
        return json_ok(StatusCode::OK, "No objects to tag");
    }

    let mut all_tags: std::collections::HashMap<String, std::collections::HashMap<String, String>> =
        std::collections::HashMap::new();
    let mut tagged_count: usize = 0;

    // Process in batches of 20
    for chunk in objects.chunks(20) {
        let mut entries = Vec::new();
        for obj in chunk {
            entries.push(format!(
                "- Filename: {}, Content-Type: {}, Size: {}",
                obj.key,
                obj.content_type.as_deref().unwrap_or("unknown"),
                obj.size
            ));
        }
        let user_msg = entries.join("\n");

        let system_prompt = r#"For each file listed below, generate 3-6 relevant tags. Respond with a JSON object where keys are filenames and values are objects with tag names as keys. Example: {"file.txt": {"document": "","text": ""}}
Respond ONLY with the JSON object, no explanation, no markdown code fences."#;

        let content = match super::ai_utils::call_llm(&config, system_prompt, &user_msg).await {
            Ok(c) => c,
            Err(r) => return r,
        };

        let batch_tags: std::collections::HashMap<String, std::collections::HashMap<String, String>> =
            match serde_json::from_str(&content) {
                Ok(t) => t,
                Err(_) => continue, // Skip failed batch
            };

        for obj in chunk {
            if let Some(tags) = batch_tags.get(&obj.key) {
                if !tags.is_empty() {
                    let _ = state.metadata.update_object_tags(&obj.bucket, &obj.key, tags);
                    all_tags.insert(obj.key.clone(), tags.clone());
                    tagged_count += 1;
                }
            }
        }
    }

    let json = serde_json::json!({
        "success": true,
        "tagged_count": tagged_count,
        "tags": all_tags
    })
    .to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}

// ===== AI Content Summarization =====

#[derive(Deserialize)]
pub struct AiSummarizeRequest {
    pub bucket: String,
    pub key: String,
}

pub async fn ai_summarize_object(
    State(state): State<S3AppState>,
    Json(body): Json<AiSummarizeRequest>,
) -> Response {
    let config = match super::ai_utils::ensure_ai_enabled() {
        Ok(c) => c,
        Err(r) => return r,
    };

    let obj = match state.metadata.get_object(&body.bucket, &body.key) {
        Ok(o) => o,
        Err(_) => return json_error(StatusCode::NOT_FOUND, "Object not found"),
    };

    let ct = obj.content_type.as_deref().unwrap_or("");
    let is_text = ct.starts_with("text/")
        || ct == "application/json"
        || ct == "application/xml"
        || ct == "application/yaml"
        || ct == "application/javascript";

    if !is_text {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Content summarization is only supported for text-based files",
        );
    }

    // Read object data
    let pool = match state.pool_manager.get_pool(&obj.pool_id).await {
        Ok(p) => p,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", e)),
    };

    let raw_data = match pool.read_object(&obj.object_hash).await {
        Ok(d) => d,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", e)),
    };

    let data = if let Some(ref enc_info_str) = obj.encryption_info {
        let enc_info: crate::storage::encryption::EncryptionInfo =
            serde_json::from_str(enc_info_str).unwrap_or_else(|_| {
                crate::storage::encryption::EncryptionInfo {
                    mode: String::new(),
                    iv: String::new(),
                    key_hmac: String::new(),
                }
            });
        match enc_info.mode.as_str() {
            "SSE-S3" => state.key_manager.decrypt_sse_s3(&raw_data, &enc_info).unwrap_or(raw_data),
            _ => raw_data,
        }
    } else {
        raw_data
    };

    let text = String::from_utf8_lossy(&data);
    let truncated = if text.len() > 4000 {
        &text[..4000]
    } else {
        &text
    };

    let user_msg = format!("Filename: {}\nContent:\n{}", obj.key, truncated);
    let system_prompt = "Summarize this file content in 2-3 sentences. Be concise and factual.";

    let summary = match super::ai_utils::call_llm(&config, system_prompt, &user_msg).await {
        Ok(s) => s,
        Err(r) => return r,
    };

    let meta = serde_json::json!({ "ai_summary": summary });
    if let Err(e) = state
        .metadata
        .update_object_metadata(&body.bucket, &body.key, &meta)
    {
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to save summary: {}", e),
        );
    }

    let json = serde_json::json!({ "success": true, "summary": summary }).to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}

#[derive(Deserialize)]
pub struct AiBulkSummarizeRequest {
    pub bucket: String,
}

pub async fn ai_bulk_summarize(
    State(state): State<S3AppState>,
    Json(body): Json<AiBulkSummarizeRequest>,
) -> Response {
    let config = match super::ai_utils::ensure_ai_enabled() {
        Ok(c) => c,
        Err(r) => return r,
    };

    let objects = match state.metadata.search_objects(
        "",
        Some(&body.bucket),
        50,
    ) {
        Ok(o) => o,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", e)),
    };

    if objects.is_empty() {
        return json_ok(StatusCode::OK, "No objects to summarize");
    }

    let mut results: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut summarized_count: usize = 0;

    for obj in &objects {
        let ct = obj.content_type.as_deref().unwrap_or("");
        let is_text = ct.starts_with("text/")
            || ct == "application/json"
            || ct == "application/xml"
            || ct == "application/yaml"
            || ct == "application/javascript";

        if !is_text {
            continue;
        }

        let pool = match state.pool_manager.get_pool(&obj.pool_id).await {
            Ok(p) => p,
            Err(_) => continue,
        };

        let raw_data = match pool.read_object(&obj.object_hash).await {
            Ok(d) => d,
            Err(_) => continue,
        };

        let data = if let Some(ref enc_info_str) = obj.encryption_info {
            let enc_info: crate::storage::encryption::EncryptionInfo =
                serde_json::from_str(enc_info_str).unwrap_or_else(|_| {
                    crate::storage::encryption::EncryptionInfo {
                        mode: String::new(),
                        iv: String::new(),
                        key_hmac: String::new(),
                    }
                });
            match enc_info.mode.as_str() {
                "SSE-S3" => state.key_manager.decrypt_sse_s3(&raw_data, &enc_info).unwrap_or(raw_data),
                _ => raw_data,
            }
        } else {
            raw_data
        };

        let text = String::from_utf8_lossy(&data);
        let truncated = if text.len() > 4000 { &text[..4000] } else { &text };

        let user_msg = format!("Filename: {}\nContent:\n{}", obj.key, truncated);
        let system_prompt = "Summarize this file content in 2-3 sentences. Be concise and factual.";

        match super::ai_utils::call_llm(&config, system_prompt, &user_msg).await {
            Ok(summary) => {
                let meta = serde_json::json!({ "ai_summary": summary });
                let _ = state.metadata.update_object_metadata(&obj.bucket, &obj.key, &meta);
                results.insert(obj.key.clone(), summary);
                summarized_count += 1;
            }
            Err(_) => continue,
        }
    }

    let json = serde_json::json!({
        "success": true,
        "summarized_count": summarized_count,
        "results": results
    })
    .to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}

// ===== AI Chat Assistant =====

#[derive(Deserialize)]
pub struct AiChatRequest {
    pub message: String,
}

pub async fn ai_chat(
    State(state): State<S3AppState>,
    Json(body): Json<AiChatRequest>,
) -> Response {
    let config = match super::ai_utils::ensure_ai_enabled() {
        Ok(c) => c,
        Err(r) => return r,
    };

    // Gather system context
    let buckets = state.metadata.list_buckets().unwrap_or_default();
    let total_objects: u64 = state
        .metadata
        .conn()
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM objects WHERE version_id IS NULL", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    let total_used: u64 = state
        .metadata
        .conn()
        .lock()
        .unwrap()
        .query_row(
            "SELECT COALESCE(SUM(size), 0) FROM objects WHERE version_id IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let bucket_list: Vec<String> = buckets.iter().map(|b| b.name.clone()).collect();

    let pools = state.pool_manager.get_all_pools().await;
    let pool_stats: Vec<String> = pools
        .iter()
        .map(|p| format!("{}: available", p.id))
        .collect();

    let context = format!(
        "Buckets: [{}]\nTotal objects: {}\nTotal storage used: {} bytes\nPools: [{}]",
        bucket_list.join(", "),
        total_objects,
        total_used,
        pool_stats.join(", "),
    );

    let system_prompt = format!(
        "You are an ObjStor assistant, an S3-compatible object storage system. \
Current state:\n{}\n\nAnswer questions about the storage concisely. \
If asked about things outside storage, politely redirect.",
        context
    );

    let response = match super::ai_utils::call_llm(&config, &system_prompt, &body.message).await {
        Ok(r) => r,
        Err(r) => return r,
    };

    let json = serde_json::json!({ "success": true, "response": response }).to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}

// ===== AI Lifecycle Suggestions =====

pub async fn ai_lifecycle_suggestions(State(state): State<S3AppState>) -> Response {
    let config = match super::ai_utils::ensure_ai_enabled() {
        Ok(c) => c,
        Err(r) => return r,
    };

    // Gather all analytics in a synchronous block (before any .await)
    let analytics = {
        let conn = state.metadata.conn().lock().unwrap();

        // Object age distribution
        let age_sql = r#"
            SELECT
                CASE
                    WHEN created_at >= (strftime('%s', 'now') - 7*86400) THEN '<7d'
                    WHEN created_at >= (strftime('%s', 'now') - 30*86400) THEN '7-30d'
                    WHEN created_at >= (strftime('%s', 'now') - 90*86400) THEN '30-90d'
                    ELSE '>90d'
                END as age_bucket,
                COUNT(*) as count,
                COALESCE(SUM(size), 0) as total_size
            FROM objects
            WHERE version_id IS NULL AND object_hash != ''
            GROUP BY age_bucket
        "#;

        let age_dist: Vec<serde_json::Value> = conn
            .prepare(age_sql)
            .and_then(|mut stmt| {
                stmt.query_map([], |row| {
                    Ok(serde_json::json!({
                        "age": row.get::<_, String>(0)?,
                        "count": row.get::<_, u64>(1)?,
                        "size": row.get::<_, u64>(2)?,
                    }))
                })
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
            })
            .unwrap_or_default();

        let size_dist: Vec<serde_json::Value> = conn
            .prepare(r#"SELECT CASE WHEN size < 1048576 THEN '<1MB' WHEN size < 104857600 THEN '1-100MB' ELSE '>100MB' END as size_bucket, COUNT(*) as count FROM objects WHERE version_id IS NULL AND object_hash != '' GROUP BY size_bucket"#)
            .and_then(|mut stmt| {
                stmt.query_map([], |row| {
                    Ok(serde_json::json!({
                        "size": row.get::<_, String>(0)?,
                        "count": row.get::<_, u64>(1)?,
                    }))
                })
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
            })
            .unwrap_or_default();

        let ct_dist: Vec<serde_json::Value> = conn
            .prepare("SELECT COALESCE(content_type, 'unknown'), COUNT(*) FROM objects WHERE version_id IS NULL AND object_hash != '' GROUP BY content_type ORDER BY COUNT(*) DESC LIMIT 10")
            .and_then(|mut stmt| {
                stmt.query_map([], |row| {
                    Ok(serde_json::json!({
                        "type": row.get::<_, String>(0)?,
                        "count": row.get::<_, u64>(1)?,
                    }))
                })
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
            })
            .unwrap_or_default();

        let tier_dist: Vec<serde_json::Value> = conn
            .prepare("SELECT COALESCE(tier, 'hot'), COUNT(*) FROM objects WHERE version_id IS NULL AND object_hash != '' GROUP BY tier")
            .and_then(|mut stmt| {
                stmt.query_map([], |row| {
                    Ok(serde_json::json!({
                        "tier": row.get::<_, String>(0)?,
                        "count": row.get::<_, u64>(1)?,
                    }))
                })
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
            })
            .unwrap_or_default();

        let total_objects: u64 = conn
            .query_row("SELECT COUNT(*) FROM objects WHERE version_id IS NULL AND object_hash != ''", [], |row| row.get(0))
            .unwrap_or(0);

        let total_storage: u64 = conn
            .query_row("SELECT COALESCE(SUM(size), 0) FROM objects WHERE version_id IS NULL AND object_hash != ''", [], |row| row.get(0))
            .unwrap_or(0);

        // conn is dropped here when the block ends
        serde_json::json!({
            "total_objects": total_objects,
            "total_storage_bytes": total_storage,
            "age_distribution": age_dist,
            "size_distribution": size_dist,
            "content_type_distribution": ct_dist,
            "tier_distribution": tier_dist,
        })
    };

    let system_prompt = r#"You are a lifecycle policy advisor for an S3-compatible object storage system (ObjStor). Tiers available: hot, warm, cold.
Based on the analytics data provided, suggest lifecycle rules to optimize storage.
Respond ONLY with a JSON array of objects, each with: prefix, source_tier, destination_tier, transition_days, reasoning.
Example: [{"prefix": "logs/", "source_tier": "hot", "destination_tier": "cold", "transition_days": 7, "reasoning": "Log files are rarely accessed after 7 days"}]
If no suggestions are needed, return an empty array. No explanation, no markdown code fences."#;

    let user_msg = format!("Storage analytics:\n{}", serde_json::to_string_pretty(&analytics).unwrap_or_default());

    let content = match super::ai_utils::call_llm(&config, system_prompt, &user_msg).await {
        Ok(c) => c,
        Err(r) => return r,
    };

    let suggestions: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap_or_default();

    let json = serde_json::json!({
        "success": true,
        "suggestions": suggestions,
        "generated_at": chrono::Utc::now().to_rfc3339()
    })
    .to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}

fn json_ok(status: StatusCode, message: &str) -> Response {
    let json = serde_json::json!({ "success": true, "message": message }).to_string();
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}

pub fn json_error(status: StatusCode, message: &str) -> Response {
    let json = serde_json::json!({ "success": false, "error": message }).to_string();
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}
