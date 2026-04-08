# ObjStor Project Structure

## Directory Structure

```
objstor/
├── src/                      # Source code
│   ├── main.rs              # Program entry point
│   ├── lib.rs               # Library exports
│   ├── api/                 # API layer
│   │   ├── s3/             # S3 protocol implementation
│   │   ├── admin.rs        # Admin & AI API endpoints
│   │   ├── ai_utils.rs     # LLM integration utilities
│   │   ├── events.rs       # Event bus
│   │   ├── middleware.rs   # Middleware
│   │   └── rate_limit.rs   # Rate limiting
│   ├── storage/             # Storage engine
│   │   ├── pool.rs         # Storage pool
│   │   ├── pool_manager.rs # Pool manager
│   │   ├── dedup.rs        # Deduplication
│   │   ├── encryption.rs   # Encryption (SSE-S3, SSE-C)
│   │   ├── tier.rs         # Storage tiers
│   │   ├── lifecycle.rs    # Lifecycle management
│   │   └── multipart.rs    # Multipart upload
│   ├── scheduler/           # Scheduler system
│   │   ├── load_balancer.rs # Load balancing
│   │   ├── placement.rs     # Data placement
│   │   └── metrics.rs       # Performance metrics
│   ├── metadata/            # Metadata storage
│   ├── auth/               # Authentication & authorization
│   ├── logging/            # Logging system
│   ├── web/                # Web interface
│   └── config/             # Configuration management
│
├── tests/                   # Tests
│   ├── integration_test.rs # Integration tests
│   └── api_test.rs        # API tests
│
├── scripts/                 # Utility scripts
│   ├── benchmark.sh       # Performance tests
│   ├── configure.sh       # Quick configuration
│   ├── docker-build.sh    # Docker build
│   ├── docker-deploy.sh   # Docker deployment
│   ├── docker-push.sh     # Docker push
│   ├── init_config.sh     # Configuration wizard
│   └── local-deploy.sh    # Local deployment
│
├── examples/               # Example configurations
│   ├── storage.example.json       # Storage configuration example
│   ├── prometheus.yml             # Prometheus configuration
│   ├── docker-compose-metrics.yml # Monitoring stack
│   ├── grafana-datasources.yml    # Grafana datasources
│   └── alertmanager.yml           # Alert configuration
│
├── docs/                   # Documentation
│   ├── CONFIGURATION.md   # Configuration guide
│   └── POOL_CONFIG_GUIDE.md # Pool configuration guide
│
├── data/                   # Runtime data (gitignore)
│   ├── config/           # Configuration files
│   └── metadata.db       # Metadata database
│
├── logs/                   # Log files (gitignore)
│
├── Dockerfile             # Docker build file
├── Dockerfile.local       # Local build Dockerfile
├── docker-compose.yml     # Docker Compose configuration
├── docker-compose.dev.yml # Development environment configuration
├── README.md              # Main project documentation
├── CLAUDE.md              # Claude project guide
├── Makefile              # Build scripts
├── Dockerfile            # Docker build file
├── Dockerfile.local      # Local build Dockerfile
├── docker-compose.yml    # Docker Compose configuration
├── docker-compose.dev.yml # Development environment configuration
│
└── Cargo.toml             # Rust project configuration
```

## Core Module Overview

### API Layer (`src/api/`)
- **S3 Protocol**: Complete S3 API implementation
- **Authentication**: AWS4-HMAC-SHA256 signature verification
- **Admin API**: Health checks, metrics, bucket management

### Storage Layer (`src/storage/`)
- **Storage Pools**: Multi-pool management with individual capacity configs
- **Object Storage**: Content-addressable storage (SHA256)
- **Multipart Upload**: Support for large file chunked uploads

### Scheduler System (`src/scheduler/`)
- **Load Balancing**: LeastLoaded, WeightedRoundRobin, Adaptive
- **Data Placement**: Intelligent storage pool selection
- **Metrics Collection**: Real-time performance monitoring

### Metadata (`src/metadata/`)
- **SQLite Storage**: Buckets, Objects, user information
- **Index Optimization**: Fast queries
- **Transaction Support**: Data consistency

### Web Interface (`src/web/`)
- **Dashboard**: Real-time monitoring and statistics
- **Bucket Management**: Create, delete, browse buckets
- **WebSocket**: Real-time logs and metrics push

## Deployment Methods

### 1. Local Deployment
- Run compiled binary directly
- No Docker required
- Storage mapped to specified directory

### 2. Docker Deployment
- Containerized deployment
- Requires Docker registry mirrors configuration
- Supports data volume persistence

## Configuration Files

### Main Configuration
**Location**: `data/config/objstor.json`

**Structure**:
- Server settings (host, port, TLS)
- Storage configuration (data directory, pools)
- Scheduler settings (strategy, thresholds)

### Storage Configuration Example
**Location**: `examples/storage.example.json`

Contains various storage configuration scenarios:
- Single storage pool
- Multiple pools (SSD+HDD hybrid)
- NFS network storage
- Multi-disk independent mounts

## Development Guide

### Build Project
```bash
# Development version
cargo build

# Release version
cargo build --release

# Run tests
cargo test

# Code linting
cargo clippy
```

### Add New Features
1. **S3 API**: Add handlers in `src/api/s3/`
2. **Scheduler Strategy**: Add algorithm in `src/scheduler/load_balancer.rs`
3. **Storage Pool**: Update `src/storage/pool.rs`
4. **Web Pages**: Modify `src/web/static/`

### Testing
```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration_test

# API tests (requires running service)
./tests/api_test.sh
```

## Documentation Overview

### User Documentation
- **README.md**: Project overview, quick start
- **docs/deployment/build.md**: Detailed build instructions
- **docs/deployment/docker.md**: Complete Docker deployment guide
- **docs/deployment/local.md**: Local deployment guide

### Developer Documentation
- **CLAUDE.md**: Claude AI project guide
- **docs/CONFIGURATION.md**: Detailed configuration parameters
- **docs/POOL_CONFIG_GUIDE.md**: Storage pool configuration guide

### Example Configurations
- **examples/**: Configuration examples for various scenarios
- **scripts/**: Automated deployment scripts

## Runtime Directories

### Data Directory (`data/`)
- **config/**: Configuration files
  - `objstor.json` - Main configuration file
  - `storage.json` - Storage configuration
- `metadata.db`: Metadata database

### Logs Directory (`logs/`)
- `objstor.log`: Main log file
- `objstor.log.1`, `objstor.log.2`, etc.: Log rotation files

### Storage Directory (External)
- **pool-001/**, **pool-002/**, etc.: Storage pools
  - **objects/**: Object data
    - **[hash]/**: SHA256 hash shards
      - **data**: Actual data
      - **meta.json**: Metadata
  - **metadata/**: Pool metadata
    - **pool.json**: Pool metadata

## Performance Optimization

### Build Optimization
```bash
# Use release configuration
cargo build --release

# Enable LTO (Link-Time Optimization)
# In Cargo.toml: lto = true
```

### Runtime Optimization
- Adjust `max_objects` parameter
- Choose appropriate scheduler strategy
- Use multiple storage pools to distribute load
- Enable log rotation to prevent overly large log files

### Storage Optimization
- Use SSD for pool-001 (hot data)
- Use HDD for pool-002 (cold data)
- Regularly clean unused objects
- Monitor storage usage rate

## Monitoring and Alerting

### Prometheus Metrics
- Request rate
- Storage usage
- Object count
- Error rate

### Grafana Dashboards
Use `examples/grafana-dashboards.yml` for configuration

### Alert Rules
Use `examples/alerts/objstor.yml` for configuration

## Troubleshooting

### Common Issues
1. **Port Conflict**: Check if port 3020 is occupied
2. **Permission Issues**: Ensure storage directory has write permissions
3. **Configuration Errors**: Verify JSON format is correct
4. **Database Lock**: SQLite uses WAL mode

### Log Levels
```bash
# Debug mode
RUST_LOG=debug cargo run

# Verbose logging
RUST_LOG=trace cargo run
```

## Contributing Guidelines

1. Fork the project
2. Create a feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request

### Code Standards
- Format code with `cargo fmt`
- Pass `cargo clippy` checks
- Write unit tests
- Update relevant documentation

## License

MIT License - See LICENSE file for details

## Contact

- GitHub Issues: Report bugs
- Discussions: Feature discussions
