use crate::api::s3::handler::S3AppState;
use crate::config::Config;
use axum::{
    extract::State,
    response::{IntoResponse, Json},
};

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
