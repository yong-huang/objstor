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

fn json_ok(status: StatusCode, message: &str) -> Response {
    let json = serde_json::json!({ "success": true, "message": message }).to_string();
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}

fn json_error(status: StatusCode, message: &str) -> Response {
    let json = serde_json::json!({ "success": false, "error": message }).to_string();
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}
