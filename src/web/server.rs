use crate::api::s3::handler::S3AppState;
use axum::{body::Body, extract::State, response::Response, routing::get, Router};
use http::{header, StatusCode};

pub fn web_routes() -> Router<S3AppState> {
    Router::new()
        .route("/", get(index_handler))
        .route("/ws", get(crate::web::websocket::websocket_handler))
        .nest("/api/v1", api_routes())
}

fn api_routes() -> Router<S3AppState> {
    Router::new()
        .route("/health", get(crate::api::admin::get_health))
        .route("/metrics", get(crate::api::admin::get_metrics))
        .route("/buckets", get(list_buckets_api))
        .route("/pools", get(list_pools_api))
}

async fn index_handler() -> Response {
    let html = include_str!("static/index.html");
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(html))
        .unwrap()
}

async fn list_buckets_api(State(state): State<S3AppState>) -> axum::Json<serde_json::Value> {
    let buckets = state.metadata.list_buckets().unwrap_or_default();
    axum::Json(serde_json::json!({
        "buckets": buckets
    }))
}

async fn list_pools_api(State(state): State<S3AppState>) -> axum::Json<serde_json::Value> {
    let pools: Vec<crate::storage::pool::StoragePool> = state.pool_manager.get_all_pools().await;
    axum::Json(serde_json::json!({
        "pools": pools
    }))
}
