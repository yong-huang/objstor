use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use objstor::{
    api::{admin, s3::handler::S3AppState, middleware::logging_middleware},
    logging::init_logging,
    scheduler::SchedulingStrategy,
    storage::{pool::PoolConfig, PoolManager},
    web::websocket,
};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    let log_config = objstor::config::ServerConfig::default();
    let _guard = init_logging(&log_config)?;
    tracing::info!("Starting ObjStor v0.1.0");

    // Initialize storage layout
    let data_dir = PathBuf::from("./data");
    let config_path = data_dir.join("config");

    // Create storage directory structure
    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(&config_path)?;
    std::fs::create_dir_all(data_dir.join("pools"))?;

    // Initialize storage pools
    let pool_configs = vec![
        PoolConfig {
            id: "pool-001".to_string(),
            path: data_dir.join("pools/pool-001"),
            capacity: 100 * 1024 * 1024 * 1024, // 100GB
            max_objects: 1_000_000,
            quota_enabled: false,
        },
        PoolConfig {
            id: "pool-002".to_string(),
            path: data_dir.join("pools/pool-002"),
            capacity: 100 * 1024 * 1024 * 1024, // 100GB
            max_objects: 1_000_000,
            quota_enabled: false,
        },
    ];

    let pool_manager = Arc::new(
        PoolManager::new(pool_configs, SchedulingStrategy::LeastLoaded).await?,
    );

    // Initialize metadata store
    let metadata_path = data_dir.join("metadata.db");
    let metadata_store = Arc::new(objstor::metadata::MetadataStore::new(&metadata_path)?);

    // TODO: Create default access key for testing
    tracing::info!("Skipping default access key creation for now");

    // Initialize multipart upload manager
    let multipart_manager = Arc::new(tokio::sync::Mutex::new(
        objstor::storage::multipart::MultipartUploadManager::new(),
    ));

    // Create application state
    let state = S3AppState {
        metadata: Arc::clone(&metadata_store),
        pool_manager: Arc::clone(&pool_manager),
        multipart_manager: Arc::clone(&multipart_manager),
    };

    // Build router - order matters!
    let app = Router::new()
        // Health check
        .route("/health", axum::routing::get(admin::get_health))
        .route("/api/v1/health", axum::routing::get(admin::get_health))
        .route("/api/v1/metrics", axum::routing::get(admin::get_metrics))
        .route("/api/v1/buckets", axum::routing::get(admin::get_buckets_api))
        // WebSocket endpoint
        .route("/ws", axum::routing::get(websocket::websocket_handler))
        // Web UI routes (at /web)
        .route("/web", axum::routing::get(web_handler))
        // Static assets (CSS, JS)
        .nest_service("/static", ServeDir::new("src/web/static"))
        // S3 API fallback (handles all other paths including "/" for ListBuckets)
        .fallback(s3_handler_wrap)
        // CORS
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        // Middleware
        .layer(axum::middleware::from_fn(logging_middleware))
        .with_state(state.clone());

    // Bind address
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));

    tracing::info!("Server listening on http://{}", addr);
    tracing::info!("  S3 API: http://{}", addr);
    tracing::info!("  Web UI: http://{}", addr);
    tracing::info!("Default access key: test-access-key / test-secret-key");
    tracing::info!("Ready to accept requests!");

    // Create TCP listener and start server
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;

    Ok(())
}

async fn s3_handler_wrap(
    State(state): State<S3AppState>,
    req: Request,
) -> Response {
    tracing::info!("S3 Request: {} {}", req.method(), req.uri());

    let (parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, 16 * 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => return Response::builder()
            .status(StatusCode::PAYLOAD_TOO_LARGE)
            .body(axum::body::Body::from("Request too large"))
            .unwrap(),
    };

    objstor::api::s3::handler::S3Handler::handle_request(
        Method::from(parts.method),
        parts.uri,
        parts.headers,
        body_bytes,
        state,
    )
    .await
}

async fn web_handler() -> Response {
    let html = include_str!("web/static/index.html");
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(axum::body::Body::from(html))
        .unwrap()
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
