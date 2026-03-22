use crate::api::s3::handler::S3AppState;
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
    let (used, capacity) = state.pool_manager.get_total_usage().await;

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

    Json(serde_json::json!({
        "storage": {
            "used": used,
            "capacity": capacity,
            "usage_ratio": if capacity > 0 { used as f64 / capacity as f64 } else { 0.0 }
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
