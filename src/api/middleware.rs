use axum::{extract::Request, middleware::Next, response::Response};

pub async fn logging_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();

    let start = std::time::Instant::now();
    let response = next.run(req).await;
    let duration = start.elapsed();

    tracing::info!(
        method = %method,
        uri = %uri,
        status = response.status().as_u16(),
        duration_ms = duration.as_millis(),
        "HTTP request"
    );

    response
}
