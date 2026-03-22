# ObjStor - Project Guide

## Project Overview

ObjStor is an S3-compatible object storage simulator built with Rust. It provides a complete S3 API implementation, intelligent load balancing, and a modern web management interface.

## Key Technical Details

### Tech Stack
- **Language**: Rust 1.70+
- **Web Framework**: Axum 0.7
- **Database**: SQLite (rusqlite)
- **Authentication**: AWS4-HMAC-SHA256 signatures
- **Real-time**: WebSocket
- **Frontend**: Vanilla JavaScript + Chart.js

### File Organization

#### Core Modules
- `src/main.rs` - Entry point, server setup
- `src/lib.rs` - Library exports
- `src/error.rs` - Error types and conversions

#### Storage Layer (`src/storage/`)
- `pool.rs` - Individual storage pool implementation
- `pool_manager.rs` - Multi-pool coordination
- `object.rs` - Object read/write operations
- `multipart.rs` - Multipart upload handling
- `layout.rs` - Storage directory structure

#### Scheduler (`src/scheduler/`)
- `load_balancer.rs` - Scheduling strategies (LeastLoaded, WeightedRoundRobin, Adaptive)
- `metrics.rs` - Performance metrics collection
- `placement.rs` - Data placement strategies

#### Metadata (`src/metadata/`)
- `db.rs` - SQLite schema and connection
- `bucket.rs` - Bucket metadata operations
- `object.rs` - Object metadata operations
- `user.rs` - Access key management

#### API Layer (`src/api/`)
- `s3/handler.rs` - Main S3 request router
- `s3/bucket.rs` - Bucket operations (Create, Delete, List)
- `s3/object.rs` - Object operations (Put, Get, Delete, List)
- `s3/multipart.rs` - Multipart upload operations
- `admin.rs` - Admin API for metrics

#### Web UI (`src/web/`)
- `server.rs` - Web server routes
- `websocket.rs` - Real-time WebSocket updates
- `static/` - HTML/CSS/JS frontend

### Important Design Decisions

1. **Storage Pool Architecture**: Objects are stored in hash-based directories within pools, enabling efficient distribution
2. **Load Balancing**: Multiple strategies available (LeastLoaded is default)
3. **Metadata Separation**: SQLite stores metadata separately from data for faster queries
4. **Real-time Updates**: WebSocket pushes metrics every 5 seconds

### Testing Access Keys

Default test credentials (created on first run):
- Access Key ID: `test-access-key`
- Secret Key: `test-secret-key`

### Development Commands

```bash
# Build
cargo build

# Run
cargo run

# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Run with Clippy
cargo clippy

# Format code
cargo fmt
```

### Common Issues

1. **Port already in use**: Change ports in `src/main.rs`
2. **Permission errors**: Ensure write access to `./data` directory
3. **Database locked**: SQLite uses WAL mode, but only one process should access at a time

### Adding New Features

1. **New S3 API**: Add handler in `src/api/s3/`, update router in `handler.rs`
2. **New scheduling strategy**: Add to `src/scheduler/load_balancer.rs`
3. **New metrics**: Add to `src/scheduler/metrics.rs`, update WebSocket messages
4. **Web UI pages**: Add to `src/web/static/index.html`, handle in `src/web/static/js/app.js`
