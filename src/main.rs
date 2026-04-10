use axum::{
    extract::{Query, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Router,
};
use objstor::{
    api::{
        admin, events::EventBus, middleware::logging_middleware, rate_limit::RateLimiter,
        s3::handler::S3AppState,
    },
    config::Config,
    logging::init_logging,
    scheduler::SchedulingStrategy,
    storage::{
        encryption::MasterKeyManager,
        integrity::IntegrityChecker,
        lifecycle::{LifecycleEngine, LifecycleRule},
        pool_manager::PoolManager,
        tier::StorageTier,
    },
    web::websocket,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::{net::SocketAddr, sync::Arc, sync::Mutex as StdMutex};
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load or create configuration
    let config =
        Config::load_or_create().map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

    // Initialize logging with config from file
    let _guard = init_logging(&config.server)?;
    tracing::info!("Starting ObjStor v0.1.0");
    tracing::info!("Loaded configuration from data/config/objstor.json");

    // Initialize storage directories
    config
        .storage
        .init_directories()
        .map_err(|e| anyhow::anyhow!("Failed to initialize directories: {}", e))?;
    tracing::info!("Storage directories initialized");

    // Get data directory from config
    let data_dir = &config.storage.data_dir;

    // Convert storage pool configs to pool configs
    let pool_configs = config.storage.to_pool_configs();
    tracing::info!("Loaded {} storage pools", pool_configs.len());

    // Log pool information
    for pool in &pool_configs {
        tracing::info!(
            "  Pool: {} - Path: {:?}, Capacity: {} GB, Max Objects: {}",
            pool.id,
            pool.path,
            pool.capacity / (1024 * 1024 * 1024),
            pool.max_objects
        );
    }

    let pool_manager =
        Arc::new(PoolManager::new(pool_configs, SchedulingStrategy::LeastLoaded).await?);

    // Initialize encryption master key manager
    let key_manager = Arc::new(MasterKeyManager::load_or_create(data_dir)?);
    tracing::info!("Encryption master key loaded");

    // Initialize metadata store
    let metadata_path = data_dir.join("metadata.db");
    let metadata_store = Arc::new(objstor::metadata::MetadataStore::new(&metadata_path)?);

    // Initialize multipart upload manager
    let multipart_manager = Arc::new(tokio::sync::Mutex::new(
        objstor::storage::multipart::MultipartUploadManager::new(),
    ));

    // Initialize event bus
    let event_bus = Arc::new(EventBus::new(256));
    tracing::info!("Event bus initialized");

    // Initialize rate limiter (default: 100 rps, burst of 200)
    let rate_limiter = Arc::new(RateLimiter::new(100.0, 200));

    // Create application state
    let state = S3AppState {
        metadata: Arc::clone(&metadata_store),
        pool_manager: Arc::clone(&pool_manager),
        multipart_manager: Arc::clone(&multipart_manager),
        key_manager: Arc::clone(&key_manager),
        event_bus: Arc::clone(&event_bus),
        rate_limiter: Arc::clone(&rate_limiter),
    };

    // Spawn lifecycle engine background task
    let lifecycle_rules = vec![
        LifecycleRule {
            prefix: String::new(),
            source_tier: StorageTier::Hot,
            destination_tier: StorageTier::Warm,
            transition_days: 30,
        },
        LifecycleRule {
            prefix: String::new(),
            source_tier: StorageTier::Warm,
            destination_tier: StorageTier::Cold,
            transition_days: 90,
        },
    ];
    let lifecycle_engine = Arc::new(LifecycleEngine::new(lifecycle_rules));
    let lifecycle_db = Arc::clone(&metadata_store);
    let lifecycle_pools = Arc::clone(&pool_manager);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            if let Err(e) = lifecycle_engine
                .run_cycle(&lifecycle_db, &lifecycle_pools)
                .await
            {
                tracing::error!("Lifecycle cycle error: {}", e);
            }
        }
    });
    tracing::info!("Lifecycle engine started (1-hour interval)");

    // Spawn integrity checker background task (30-min interval)
    let integrity_event_bus = Arc::clone(&event_bus);
    let integrity_pools = Arc::clone(&pool_manager);
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(30 * 60));
        loop {
            interval.tick().await;
            if let Err(e) = IntegrityChecker::run_check(&integrity_pools, &integrity_event_bus).await
            {
                tracing::error!("Integrity check error: {}", e);
            }
        }
    });
    tracing::info!("Integrity checker started (30-minute interval)");

    // Store last integrity result for the API endpoint
    let _last_integrity: Arc<StdMutex<Option<serde_json::Value>>> =
        Arc::new(StdMutex::new(None));

    // Build router - order matters!
    let app = Router::new()
        // Health check
        .route("/health", axum::routing::get(admin::get_health))
        .route("/api/v1/health", axum::routing::get(admin::get_health))
        .route("/api/v1/metrics", axum::routing::get(admin::get_metrics))
        .route("/api/v1/system", axum::routing::get(admin::get_system_metrics))
        .route(
            "/api/v1/buckets",
            axum::routing::get(admin::get_buckets_api),
        )
        .route("/api/v1/config", axum::routing::get(admin::get_config))
        .route("/api/v1/config", axum::routing::put(admin::update_config))
        // Audit logs
        .route(
            "/api/v1/audit-logs",
            axum::routing::get(handle_audit_logs),
        )
        // Pre-signed URL generation
        .route(
            "/api/v1/presign",
            axum::routing::get(handle_presign),
        )
        // Object search
        .route(
            "/api/v1/search",
            axum::routing::get(handle_search),
        )
        // Integrity status
        .route(
            "/api/v1/integrity",
            axum::routing::get(handle_integrity),
        )
        // Access Keys
        .route("/api/v1/access-keys", axum::routing::get(admin::list_access_keys))
        // Object listing (admin API, no S3 auth required)
        .route("/api/v1/objects", axum::routing::get(admin::list_objects_api))
        .route("/api/v1/access-keys", axum::routing::post(admin::create_access_key))
        .route("/api/v1/access-keys/:key_id", axum::routing::put(admin::update_access_key))
        .route("/api/v1/access-keys/:key_id", axum::routing::delete(admin::delete_access_key))
        // AI Search
        .route("/api/v1/ai/models", axum::routing::get(admin::get_ai_models))
        .route("/api/v1/ai/search", axum::routing::post(admin::ai_search))
        // AI Auto-Tagging
        .route("/api/v1/ai/tags", axum::routing::post(admin::ai_generate_tags))
        .route("/api/v1/ai/tags/bulk", axum::routing::post(admin::ai_bulk_generate_tags))
        // AI Summarization
        .route("/api/v1/ai/summarize", axum::routing::post(admin::ai_summarize_object))
        .route("/api/v1/ai/summarize/bulk", axum::routing::post(admin::ai_bulk_summarize))
        // AI Chat
        .route("/api/v1/ai/chat", axum::routing::post(admin::ai_chat))
        // AI Lifecycle Suggestions
        .route("/api/v1/ai/lifecycle-suggestions", axum::routing::get(admin::ai_lifecycle_suggestions))
        // Bucket & Request Stats
        .route(
            "/api/v1/bucket-stats",
            axum::routing::get(admin::get_bucket_stats),
        )
        .route(
            "/api/v1/request-stats",
            axum::routing::get(admin::get_request_stats),
        )
        // WebSocket endpoint
        .route("/ws", axum::routing::get(websocket::websocket_handler))
        // Web UI routes (at /web)
        .route("/web", axum::routing::get(web_handler))
        // Static assets (CSS, JS)
        .nest_service("/static", ServeDir::new("src/web/static"))
        // S3 API fallback (handles all other paths including "/" for ListBuckets)
        .fallback(s3_handler_wrap)
        // CORS
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        // Middleware
        .layer(axum::middleware::from_fn(logging_middleware))
        .with_state(state.clone());

    // Bind address from config
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));

    tracing::info!("Server listening on http://{}", addr);
    tracing::info!("  S3 API: http://{}", addr);
    tracing::info!("  Web UI: http://{}", addr);
    tracing::info!("Default access key: test-access-key / test-secret-key");
    tracing::info!("Ready to accept requests!");

    // Create TCP listener and start server
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn s3_handler_wrap(State(state): State<S3AppState>, req: Request) -> Response {
    // Redirect browsers to /web for non-S3 requests on "/"
    let path = req.uri().path().to_string();
    if path == "/" && req.headers().get("authorization").is_none() {
        return redirect_to_web().await;
    }

    let start = std::time::Instant::now();
    tracing::info!("S3 Request: {} {}", req.method(), req.uri());

    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();

    // Extract source_ip from headers (fallback to unknown)
    let source_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Extract user_agent
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Parse bucket from path for rate limiting
    let path_parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let bucket_name = path_parts.first().copied().unwrap_or("");

    // Rate limiting check
    if !bucket_name.is_empty() && !state.rate_limiter.try_consume(bucket_name) {
        let duration_ms = start.elapsed().as_millis() as i64;
        let _ = state.metadata.insert_audit_log(
            method.as_str(),
            &path,
            503,
            Some(bucket_name),
            path_parts.get(1).copied(),
            None,
            Some(&source_ip),
            user_agent.as_deref(),
            Some("Rate limit exceeded"),
            duration_ms,
        );
        return objstor::error::Error::SlowDown.into_response();
    }

    // Pre-signed URL validation (for GET requests)
    if method.as_str() == "GET" && uri.query().is_some_and(|q| q.contains("X-Amz-Signature")) {
        let host = headers
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("localhost:8080");
        let key_bytes = state.key_manager.get_key_bytes();
        match objstor::api::s3::presign::validate_presigned_request(
            &uri.to_string(),
            host,
            &key_bytes,
        ) {
            Ok((bucket, key, _method)) => {
                let bucket_copy = bucket.clone();
                let key_copy = key.clone();
                // Serve the object directly (bypass auth)
                let response = objstor::api::s3::object::handle_get_object(
                    axum::extract::State(state.clone()),
                    axum::extract::Path((bucket, key)),
                    headers.clone(),
                )
                .await;

                let status = match &response {
                    Ok(resp) => resp.status().as_u16(),
                    Err(_) => 403,
                };
                let _ = state.metadata.insert_audit_log(
                    method.as_str(),
                    &path,
                    status,
                    Some(&bucket_copy),
                    Some(&key_copy),
                    None,
                    Some(&source_ip),
                    user_agent.as_deref(),
                    None,
                    start.elapsed().as_millis() as i64,
                );
                return response.unwrap_or_else(|e| e.into_response());
            }
            Err(_) => {
                return objstor::error::Error::PreSignedUrlExpired.into_response();
            }
        }
    }

    let (parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, 16 * 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::PAYLOAD_TOO_LARGE)
                .body(axum::body::Body::from("Request too large"))
                .unwrap()
        }
    };

    // AWS4 signature authentication
    let headers_map: HashMap<String, String> = parts
        .headers
        .iter()
        .filter_map(|(k, v)| {
            Some((k.to_string().to_lowercase(), v.to_str().ok()?.to_string()))
        })
        .collect();
    let signer = objstor::auth::Signer::new(Arc::clone(&state.metadata));
    let full_uri = format!(
        "{}{}",
        parts.uri.path(),
        parts
            .uri
            .query()
            .map(|q| format!("?{}", q))
            .unwrap_or_default()
    );
    if let Err(e) = signer.verify_request(
        parts.method.as_str(),
        &full_uri,
        &headers_map,
        &body_bytes,
    ) {
        let duration_ms = start.elapsed().as_millis() as i64;
        let _ = state.metadata.insert_audit_log(
            parts.method.as_str(),
            &path,
            403,
            Some(bucket_name),
            path_parts.get(1).copied(),
            None,
            Some(&source_ip),
            user_agent.as_deref(),
            None,
            duration_ms,
        );
        return e.into_response();
    }

    let response = objstor::api::s3::handler::S3Handler::handle_request(
        parts.method,
        parts.uri,
        parts.headers,
        body_bytes,
        state.clone(),
    )
    .await;

    let status = response.status().as_u16();
    let _ = state.metadata.insert_audit_log(
        method.as_str(),
        &path,
        status,
        Some(bucket_name),
        path_parts.get(1).copied(),
        None,
        Some(&source_ip),
        user_agent.as_deref(),
        None,
        start.elapsed().as_millis() as i64,
    );

    response
}

async fn redirect_to_web() -> Response {
    Response::builder()
        .status(StatusCode::MOVED_PERMANENTLY)
        .header("Location", "/web")
        .body(axum::body::Body::empty())
        .unwrap()
}

async fn web_handler() -> Response {
    let html = include_str!("web/static/index.html");
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(axum::body::Body::from(html))
        .unwrap()
}

// === Admin API handlers ===

#[derive(Deserialize)]
struct AuditLogsQuery {
    limit: Option<usize>,
    bucket: Option<String>,
}

async fn handle_audit_logs(
    State(state): State<S3AppState>,
    Query(params): Query<AuditLogsQuery>,
) -> Response {
    let limit = params.limit.unwrap_or(100);
    let bucket = params.bucket.as_deref();

    match state.metadata.query_audit_logs(limit, bucket) {
        Ok(logs) => {
            let json = serde_json::to_string(&logs).unwrap_or_else(|_| "[]".to_string());
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(json))
                .unwrap()
        }
        Err(e) => {
            let json = serde_json::json!({"error": format!("{}", e)}).to_string();
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(json))
                .unwrap()
        }
    }
}

#[derive(Deserialize)]
struct PresignQuery {
    bucket: String,
    key: String,
    expires: Option<u64>,
}

async fn handle_presign(
    State(state): State<S3AppState>,
    Query(params): Query<PresignQuery>,
) -> Response {
    let expires = params.expires.unwrap_or(3600);
    let host = "localhost:8080"; // Default host
    let key_bytes = state.key_manager.get_key_bytes();
    let url = objstor::api::s3::presign::generate_presigned_url(
        host,
        &params.bucket,
        &params.key,
        expires,
        "GET",
        &key_bytes,
    );

    let json = serde_json::json!({"url": url}).to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    bucket: Option<String>,
    limit: Option<usize>,
}

async fn handle_search(
    State(state): State<S3AppState>,
    Query(params): Query<SearchQuery>,
) -> Response {
    let limit = params.limit.unwrap_or(50);
    let bucket = params.bucket.as_deref();

    match state.metadata.search_objects(&params.q, bucket, limit) {
        Ok(objects) => {
            let json = serde_json::to_string(&objects).unwrap_or_else(|_| "[]".to_string());
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(json))
                .unwrap()
        }
        Err(e) => {
            let json = serde_json::json!({"error": format!("{}", e)}).to_string();
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(json))
                .unwrap()
        }
    }
}

async fn handle_integrity() -> Response {
    // Returns a static "no check run yet" response.
    // In production, this would read from a shared state updated by the background task.
    let json = serde_json::json!({
        "status": "ok",
        "message": "Integrity checker runs every 30 minutes in the background"
    })
    .to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(json))
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
