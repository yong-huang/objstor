# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ObjStor is an S3-compatible object storage simulator written in Rust. It provides a full S3 API, multi-pool distributed storage with intelligent load balancing, and a web management dashboard.

## Development Commands

```bash
# Build (debug)
cargo build

# Build (release)
cargo build --release

# Run server (default port 8080)
cargo run

# Run with debug logging
RUST_LOG=debug cargo run

# Run module-specific logs
RUST_LOG=objstor::api=debug,objstor::storage=trace cargo run

# Run all tests
cargo test

# Run a single test
cargo test test_bucket_creation

# Run tests with output
cargo test -- --nocapture

# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Lint
cargo clippy

# Full check (fmt + clippy + test)
make check

# Docker
make docker-compose-up
```

## Architecture

### Request Flow

Axum receives HTTP requests in `main.rs`. Admin/health/WebSocket routes are matched explicitly; everything else falls through to the S3 handler via `.fallback(s3_handler_wrap)`. The `S3Handler` in `src/api/s3/handler.rs` dispatches based on HTTP method and URI path segments (bucket vs bucket+key).

### Shared State (`S3AppState`)

Three `Arc`-wrapped pieces of state are shared across all handlers:
- **`MetadataStore`** (`src/metadata/db.rs`) — SQLite connection (WAL mode, wrapped in `Arc<Mutex<Connection>>`). Schema is auto-created on first run. Stores buckets, objects, multipart uploads, upload parts, and access keys.
- **`PoolManager`** (`src/storage/pool_manager.rs`) — Coordinates multiple `StoragePool` instances. Uses a `LoadBalancer` to select pools based on the configured `SchedulingStrategy` (LeastLoaded, WeightedRoundRobin, Adaptive, ConsistentHash).
- **`MultipartUploadManager`** (`src/storage/multipart.rs`) — In-memory (wrapped in `tokio::sync::Mutex`), tracks active multipart uploads.

### Storage Layout

Objects are content-addressable: data is SHA256-hashed, stored under `data/pools/{pool-id}/objects/{hash[0:2]}/{hash}/data`, with per-object `meta.json`. Each pool tracks used space and object count in `metadata/pool.json`.

### Key Modules

- `src/config/` — JSON config (`data/config/objstor.json`). Auto-created with defaults on first run.
- `src/error.rs` — Unified `Error` enum that maps to S3 error codes and HTTP status via `IntoResponse`.
- `src/api/s3/auth.rs` — AWS4 signature detection only; full verification is stubbed.
- `src/scheduler/` — Pool selection strategies. `LoadBalancer` selects pools based on health, capacity, and I/O metrics.
- `src/web/` — Dashboard UI served as static files from `src/web/static/`. WebSocket at `/ws` pushes metrics every 5s.
- `src/logging/` — tracing-based logging with daily rotation.

### Database Schema

SQLite at `data/metadata.db` with tables: `buckets`, `objects` (with versioning via `version_id`), `multipart_uploads`, `upload_parts`, `access_keys`. Objects are unique on `(bucket, key, version_id)`.

## Configuration

Config file: `data/config/objstor.json`. Created with defaults on first run. Key settings:
- Server port (default 8080)
- Storage data directory (default `./data`)
- Pool definitions (id, path, capacity, max_objects, quota_enabled)
- Scheduler strategy (default `least_loaded`)

## Testing with S3 Clients

Default credentials: `test-access-key` / `test-secret-key`, region `us-east-1`.

```bash
export AWS_ACCESS_KEY_ID=test-access-key
export AWS_SECRET_ACCESS_KEY=test-secret-key
aws s3 ls --endpoint-url http://localhost:8080
```

## Common Issues

- **Port in use**: Kill process on 8080 or change port in config
- **Database locked**: Only one process should access `data/metadata.db` at a time (WAL mode)
- **Permission errors**: Ensure write access to `./data` directory
