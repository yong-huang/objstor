use crate::api::s3::handler::S3AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::broadcast;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<S3AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: S3AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Create a per-connection broadcast channel for metrics/logs
    let (local_tx, _rx) = broadcast::channel::<serde_json::Value>(100);

    // Send initial connection message
    let msg = json!({"type": "connected", "message": "Connected to ObjStor WebSocket"});
    if sender.send(Message::Text(msg.to_string())).await.is_err() {
        return;
    }

    // Spawn a task to send periodic metrics updates
    let local_tx_clone = local_tx.clone();
    let state_metrics = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;

            // Get all pools
            let pools: Vec<crate::storage::pool::StoragePool> =
                state_metrics.pool_manager.get_all_pools().await;

            // Get buckets list
            let buckets = state_metrics.metadata.list_buckets().unwrap_or_default();
            let buckets_data: Vec<serde_json::Value> = buckets
                .iter()
                .map(|b| {
                    serde_json::json!({
                        "name": b.name,
                        "created_at": b.created_at.to_rfc3339(),
                        "region": b.region,
                        "owner": b.owner,
                    })
                })
                .collect();

            // Count total objects from database
            let total_objects = state_metrics
                .metadata
                .conn()
                .lock()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM objects", [], |row| row.get(0))
                .unwrap_or(0);

            // Count objects and used space per pool from database
            let pool_metrics: Vec<serde_json::Value> = pools.iter().map(|p| {
                let pool_objects: u64 = state_metrics.metadata.conn()
                    .lock()
                    .unwrap()
                    .query_row(
                        "SELECT COUNT(*) FROM objects WHERE pool_id = ?1",
                        [&p.id],
                        |row| row.get(0)
                    )
                    .unwrap_or(0);

                let pool_used: u64 = state_metrics.metadata.conn()
                    .lock()
                    .unwrap()
                    .query_row(
                        "SELECT COALESCE(SUM(size), 0) FROM objects WHERE pool_id = ?1",
                        [&p.id],
                        |row| row.get(0)
                    )
                    .unwrap_or(0);

                json!({
                    "id": p.id,
                    "capacity": p.capacity,
                    "used": pool_used,
                    "objects": pool_objects,
                    "status": format!("{:?}", p.status),
                    "usage_ratio": if p.capacity > 0 { pool_used as f64 / p.capacity as f64 } else { 0.0 },
                })
            }).collect();

            let total_used: u64 = pool_metrics.iter()
                .filter_map(|p| p["used"].as_u64())
                .sum();
            let total_capacity: u64 = pool_metrics.iter()
                .filter_map(|p| p["capacity"].as_u64())
                .sum();

            let metrics = json!({
                "type": "metrics",
                "data": {
                    "storage": {
                        "used": total_used,
                        "capacity": total_capacity,
                        "usage_ratio": if total_capacity > 0 { total_used as f64 / total_capacity as f64 } else { 0.0 }
                    },
                    "buckets": buckets_data,
                    "pools": pool_metrics,
                    "total_objects": total_objects,
                }
            });

            let _ = local_tx_clone.send(metrics);
        }
    });

    // Spawn a task to send periodic log entries
    let local_tx_log = local_tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        let mut counter = 0u64;

        loop {
            interval.tick().await;
            counter += 1;

            let (level, message) = match counter % 10 {
                0 => ("info", "System health check completed".to_string()),
                1 => ("info", format!("Processing request #{}", counter)),
                2 => ("info", format!("Storage usage: {}%", counter % 100)),
                3 => (
                    "warn",
                    format!("High memory usage: {}%", 80 + (counter % 20)),
                ),
                4 => (
                    "info",
                    format!("Active connections: {}", 10 + (counter % 50)),
                ),
                5 => ("info", "Bucket access: test-bucket".to_string()),
                6 => ("info", format!("Object uploaded: file-{}.txt", counter)),
                7 => ("info", "GET /api/v1/metrics - 200 OK".to_string()),
                8 => ("info", "WebSocket client connected".to_string()),
                _ => ("info", format!("Background task #{} completed", counter)),
            };

            let log = json!({
                "type": "log",
                "data": {
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "level": level,
                    "message": message
                }
            });

            let _ = local_tx_log.send(log);
        }
    });

    // Subscribe to local channel
    let mut local_rx = local_tx.subscribe();

    // Subscribe to global event bus
    let mut event_rx = state.event_bus.subscribe();

    // Task to send messages to client (from both local and global sources)
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = local_rx.recv() => {
                    if let Ok(val) = msg {
                        if sender.send(Message::Text(val.to_string())).await.is_err() {
                            break;
                        }
                    }
                }
                msg = event_rx.recv() => {
                    if let Ok(val) = msg {
                        if sender.send(Message::Text(val.to_string())).await.is_err() {
                            break;
                        }
                    }
                }
                else => {
                    break;
                }
            }
        }
    });

    // Task to handle messages from client
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(req) = serde_json::from_str::<serde_json::Value>(&text) {
                        tracing::info!("Received WebSocket message: {:?}", req);
                    }
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}
