use crate::config::Config;
use axum::http::StatusCode;
use axum::response::Response;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request;
use hyper_util::rt::TokioIo;

use super::admin::json_error;

/// Load config and verify AI is enabled with a model set.
pub fn ensure_ai_enabled() -> Result<Config, Response> {
    let config = Config::from_file("data/config/objstor.json").map_err(|_| {
        json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to load configuration",
        )
    })?;

    if !config.ai.enabled {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "AI features are not enabled",
        ));
    }

    if config.ai.model.is_empty() {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "No AI model selected. Please configure a model in Settings.",
        ));
    }

    Ok(config)
}

/// Strip markdown code fences from LLM output.
fn strip_fences(content: &str) -> String {
    let content = content.trim();
    let content = content
        .strip_prefix("```json")
        .or_else(|| content.strip_prefix("```"))
        .unwrap_or(content);
    let content = content.strip_suffix("```").unwrap_or(content);
    content.trim().to_string()
}

/// Low-level HTTP request helper using hyper directly (bypasses reqwest).
pub async fn http_request(
    api_endpoint: &str,
    api_key: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<Bytes, Response> {
    let base = api_endpoint.trim_end_matches('/');
    let host_port = base
        .trim_start_matches("http://")
        .trim_start_matches("https://");

    let tcp_stream = match tokio::net::TcpStream::connect(host_port).await {
        Ok(s) => s,
        Err(e) => {
            return Err(json_error(
                StatusCode::BAD_GATEWAY,
                &format!("TCP connect to {} failed: {}", host_port, e),
            ))
        }
    };

    let io = TokioIo::new(tcp_stream);
    let (mut sender, conn): (
        hyper::client::conn::http1::SendRequest<Full<Bytes>>,
        _,
    ) = match hyper::client::conn::http1::handshake(io).await {
        Ok(r) => r,
        Err(e) => {
            return Err(json_error(
                StatusCode::BAD_GATEWAY,
                &format!("HTTP handshake failed: {}", e),
            ))
        }
    };

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::error!("HTTP connection background error: {}", e);
        }
    });

    let mut req_builder = Request::builder()
        .method(method)
        .uri(path)
        .header("Host", host_port);

    if body.is_some() {
        req_builder = req_builder.header("Content-Type", "application/json");
    }
    if !api_key.is_empty() {
        req_builder = req_builder.header(
            "Authorization",
            format!("Bearer {}", api_key),
        );
    }

    let request_body = match body {
        Some(b) => Full::new(Bytes::from(b.to_string())),
        None => Full::new(Bytes::new()),
    };

    let request = match req_builder.body(request_body) {
        Ok(r) => r,
        Err(e) => {
            return Err(json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to build request: {}", e),
            ))
        }
    };

    let response = match sender.send_request(request).await {
        Ok(r) => r,
        Err(e) => {
            return Err(json_error(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to send request: {}", e),
            ))
        }
    };

    let status = response.status();
    let body_bytes: Bytes = match response.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return Err(json_error(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to read response body: {}", e),
            ))
        }
    };

    if !status.is_success() {
        let body_str = String::from_utf8_lossy(&body_bytes);
        return Err(json_error(
            StatusCode::BAD_GATEWAY,
            &format!("LLM API error {}: {}", status, body_str),
        ));
    }

    Ok(body_bytes)
}

/// Call the OpenAI-compatible chat completions API.
pub async fn call_llm(
    config: &Config,
    system_prompt: &str,
    user_message: &str,
) -> Result<String, Response> {
    let req_body = serde_json::json!({
        "model": config.ai.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_message }
        ],
        "temperature": 0.0,
        "max_tokens": config.ai.max_tokens,
    });

    let body_str = serde_json::to_string(&req_body).unwrap_or_default();
    tracing::info!(
        "call_llm: endpoint={}, model={}, key_len={}",
        config.ai.api_endpoint,
        config.ai.model,
        config.ai.api_key.len()
    );

    let body_bytes = http_request(
        &config.ai.api_endpoint,
        &config.ai.api_key,
        "POST",
        "/v1/chat/completions",
        Some(&body_str),
    )
    .await?;

    let body: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return Err(json_error(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to parse LLM response: {}", e),
            ))
        }
    };

    let content = body
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();

    Ok(strip_fences(&content))
}
