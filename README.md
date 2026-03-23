# ObjStor - S3-Compatible Object Storage Simulator

A high-performance, S3-compatible object storage system written in Rust, featuring multi-pool distributed storage, intelligent load balancing, and a modern web management interface.

![Rust](https://img.shields.io/badge/Rust-1.94+-orange.svg)
![License](https://img.shields.io/badge/License-MIT-blue.svg)

## Features

### Core Features
- **S3-Compatible API**: Full S3 protocol implementation with XML responses
  - Bucket operations: List, Create, Delete, Head
  - Object operations: Put, Get, Delete, Head, Copy
  - Multipart Upload: Create, Upload Part, Complete, Abort, List Parts
  - Versioning: GetBucketVersioning, PutBucketVersioning, ListObjectVersions
  - Tagging: Put/Get/Delete Object Tagging
  - ACLs: Put/Get Object ACL
- **Multi-Pool Storage**: Distributed object storage across multiple storage pools
- **Load Balancing**: Smart scheduling with multiple strategies
  - Least Loaded: Select pool with lowest usage ratio
  - Weighted Round Robin: Distribute based on available space
  - Adaptive: Consider multiple factors (I/O, network, object size)
- **Access Control**: AWS4-HMAC-SHA256 signature authentication

### Web Management Interface
- **Dashboard**: Real-time overview with storage usage charts and metrics
- **Buckets Management**: Create, delete, and browse S3 buckets
- **Objects Browser**: Upload, download, and delete objects with bucket selector
- **Monitoring**: Live system metrics with real-time charts
- **Logs Viewer**: Real-time log streaming via WebSocket
- **Modern UI**: Custom toast notifications and modal dialogs (no native alerts)

### Technical Features
- **Real-time Updates**: WebSocket-based metrics (5s) and logs (2s) streaming
- **SQLite Metadata**: WAL mode for concurrent access and performance
- **SHA256 Hashing**: Content-addressable storage with deduplication
- **ETag Calculation**: MD5-based ETag for S3 compatibility
- **Structured Logging**: tracing-based logging with daily rotation

## Quick Start

### Option 1: Docker Deployment (Recommended)

```bash
# Clone the repository
git clone https://github.com/yong-huang/objstor.git
cd objstor

# Build and run with Docker Compose
docker-compose up -d

# Access the web UI
open http://localhost:8080
```

See [docs/DOCKER.md](docs/DOCKER.md) for detailed Docker deployment options.

### Option 2: Native Installation

#### Prerequisites

- Rust 1.70 or later
- Cargo (comes with Rust)
- macOS / Linux / Windows

#### Installation

```bash
# Clone the repository
git clone https://github.com/yong-huang/objstor.git
cd objstor

# Configure storage pools (optional)
./scripts/configure.sh

# Build the project
cargo build --release

# Run the server
./target/release/objstor
```

### Access Points

The server starts multiple services:

- **Web Management UI**: http://localhost:8080
- **S3 API**: http://localhost:8080/
- **Admin API**: http://localhost:8080/api/v1/
- **WebSocket**: ws://localhost:8080/ws
- **Health Check**: http://localhost:8080/health

### Using Makefile

```bash
# Show all available commands
make help

# Common operations
make build          # Build the project
make run            # Run the server
make docker-build   # Build Docker image
make docker-run     # Run Docker container
make init           # Initialize configuration
```

## S3 API Support

### Bucket Operations
| API | Method | Endpoint | Description |
|-----|--------|----------|-------------|
| ListBuckets | GET | / | Lists all buckets |
| CreateBucket | PUT | /{bucket} | Creates a new bucket |
| DeleteBucket | DELETE | /{bucket} | Deletes a bucket |
| HeadBucket | HEAD | /{bucket} | Checks if bucket exists |
| GetBucketLocation | GET | /{bucket}?location | Gets bucket region |

### Object Operations
| API | Method | Endpoint | Description |
|-----|--------|----------|-------------|
| PutObject | PUT | /{bucket}/{key} | Uploads an object |
| GetObject | GET | /{bucket}/{key} | Downloads an object |
| HeadObject | HEAD | /{bucket}/{key} | Gets object metadata |
| DeleteObject | DELETE | /{bucket}/{key} | Deletes an object |
| CopyObject | PUT | /{bucket}/{key} | Copies an object |

### Multipart Upload
| API | Method | Endpoint | Description |
|-----|--------|----------|-------------|
| CreateMultipartUpload | POST | /{bucket}/{key}?uploads | Initiates multipart upload |
| UploadPart | PUT | /{bucket}/{key}?partNumber&uploadId | Uploads a part |
| CompleteMultipartUpload | POST | /{bucket}/{key}?uploadId | Completes multipart upload |
| AbortMultipartUpload | DELETE | /{bucket}/{key}?uploadId | Aborts multipart upload |
| ListParts | GET | /{bucket}/{key}?uploadId | Lists uploaded parts |
| ListMultipartUploads | GET | /{bucket}?uploads | Lists in-progress uploads |

## Testing

### Default Credentials

The system creates a default access key for testing:
- **Access Key ID**: `test-access-key`
- **Secret Key**: `test-secret-key`
- **Region**: `us-east-1`

### Testing with AWS CLI

```bash
# Set up AWS CLI for local testing
export AWS_ACCESS_KEY_ID=test-access-key
export AWS_SECRET_ACCESS_KEY=test-secret-key
export AWS_DEFAULT_REGION=us-east-1

# List buckets
aws s3 ls --endpoint-url http://localhost:8080

# Create a bucket
aws s3 mb s3://my-bucket --endpoint-url http://localhost:8080

# Upload a file
aws s3 cp file.txt s3://my-bucket/ --endpoint-url http://localhost:8080

# Download a file
aws s3 cp s3://my-bucket/file.txt . --endpoint-url http://localhost:8080

# List objects
aws s3 ls s3://my-bucket --endpoint-url http://localhost:8080

# Delete a file
aws s3 rm s3://my-bucket/file.txt --endpoint-url http://localhost:8080

# Delete a bucket
aws s3 rb s3://my-bucket --endpoint-url http://localhost:8080
```

### Testing with boto3 (Python)

```python
import boto3

# Create S3 client
s3 = boto3.client('s3',
    endpoint_url='http://localhost:8080',
    aws_access_key_id='test-access-key',
    aws_secret_access_key='test-secret-key',
    region_name='us-east-1'
)

# List buckets
response = s3.list_buckets()
print(response['Buckets'])

# Create bucket
s3.create_bucket(Bucket='my-bucket')

# Upload file
s3.upload_file('file.txt', 'my-bucket', 'file.txt')

# Download file
s3.download_file('my-bucket', 'file.txt', 'downloaded.txt')

# List objects
response = s3.list_objects_v2(Bucket='my-bucket')
for obj in response.get('Contents', []):
    print(obj['Key'])
```

## Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         ObjStor System                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌────────────────┐      ┌─────────────────┐                    │
│  │   S3 Client    │──────│  HTTP/REST API  │                    │
│  │ (AWS SDK/boto3)│      │   (Axum Server) │                    │
│  └────────────────┘      └────────┬────────┘                    │
│                                    │                             │
│                           ┌────────▼────────┐                   │
│                           │  S3 Protocol    │                   │
│                           │  Handler Layer  │                   │
│                           └────────┬────────┘                   │
│                                    │                             │
│           ┌────────────────────────┼────────────────────────┐   │
│           │                        │                        │   │
│  ┌────────▼────────┐    ┌─────────▼──────┐    ┌──────────▼──┐  │
│  │ Auth & IAM      │    │ Storage Engine │    │ Scheduler   │  │
│  │ - Access Keys   │    │ - Object CRUD  │    │ - Load      │  │
│  │ - Policies      │    │ - Multipart    │    │   Balancing │  │
│  │ - ACLs          │    │ - Encryption   │    │ - Data      │  │
│  └────────┬────────┘    └─────────┬──────┘    └──────────┬──┘  │
│           │                       │                        │    │
│           │             ┌─────────▼────────────────────────▼──┐ │
│           │             │         Storage Pool Manager         │ │
│           │             │  - Pool allocation                  │ │
│           │             │  - Space management                 │ │
│           │             │  - Health monitoring                │ │
│           │             └─────────────┬───────────────────────┘ │
│           │                           │                          │
│  ┌────────▼────────┐    ┌─────────────▼───────────────────────┐ │
│  │ Metadata Store  │    │      Data Storage Layer             │ │
│  │ (SQLite)        │    │  /data/pools/pool-XXX/objects/     │ │
│  │ - Buckets       │    └─────────────────────────────────────┘ │
│  │ - Objects       │                                            │
│  │ - Users         │    ┌─────────────────┐    ┌────────────┐  │
│  │ - Policies      │    │   Log System    │    │   Web UI   │  │
│  └─────────────────┘    │ (tracing)       │    │  Dashboard │  │
│                         │ - Access logs   │    │ - Monitor  │  │
│                         │ - Error logs    │    │ - Logs     │  │
│                         └─────────────────┘    └────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Project Structure

```
objstor/
├── Cargo.toml              # Project dependencies
├── README.md
├── CLAUDE.md              # Development guidelines
│
├── data/                  # Data storage (created at runtime)
│   ├── pools/            # Storage pools
│   │   ├── pool-001/
│   │   │   ├── objects/ # Object data (content-addressable)
│   │   │   └── metadata/
│   │   └── pool-002/
│   └── metadata.db       # SQLite metadata store
│
├── src/
│   ├── main.rs           # Application entry point
│   ├── lib.rs
│   │
│   ├── config/           # Configuration management
│   │   ├── mod.rs
│   │   ├── server.rs
│   │   └── storage.rs
│   │
│   ├── api/              # API layer
│   │   ├── mod.rs
│   │   ├── s3/          # S3 protocol handlers
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs      # AWS signature verification
│   │   │   ├── handler.rs   # Main request router
│   │   │   ├── bucket.rs    # Bucket operations
│   │   │   ├── object.rs    # Object operations
│   │   │   ├── multipart.rs # Multipart upload
│   │   │   └── error.rs     # S3 error responses
│   │   ├── admin.rs      # Admin API endpoints
│   │   └── middleware.rs # Logging middleware
│   │
│   ├── storage/          # Storage engine
│   │   ├── mod.rs
│   │   ├── pool.rs           # Storage pool implementation
│   │   ├── pool_manager.rs   # Multi-pool coordination
│   │   ├── object.rs         # Object CRUD operations
│   │   ├── multipart.rs      # Multipart upload handling
│   │   ├── version.rs        # Object versioning
│   │   └── layout.rs         # Storage layout
│   │
│   ├── scheduler/        # Load balancing
│   │   ├── mod.rs
│   │   ├── load_balancer.rs  # Scheduling strategies
│   │   ├── placement.rs      # Data placement policies
│   │   └── metrics.rs        # Performance metrics
│   │
│   ├── metadata/         # Metadata storage
│   │   ├── mod.rs
│   │   ├── db.rs             # SQLite connection
│   │   ├── bucket.rs         # Bucket metadata
│   │   ├── object.rs         # Object metadata
│   │   ├── user.rs           # User/access keys
│   │   └── policy.rs         # IAM policies
│   │
│   ├── auth/             # Authentication
│   │   ├── mod.rs
│   │   ├── signer.rs        # AWS signature parsing
│   │   ├── iam.rs           # IAM policy engine
│   │   └── acl.rs           # ACL support
│   │
│   ├── logging/          # Logging system
│   │   ├── mod.rs
│   │   ├── logger.rs        # Log configuration
│   │   ├── access.rs        # Access logging
│   │   ├── audit.rs         # Audit logging
│   │   └── metrics.rs       # Performance metrics
│   │
│   ├── web/              # Web management interface
│   │   ├── mod.rs
│   │   ├── server.rs
│   │   ├── handlers.rs      # Admin API endpoints
│   │   ├── websocket.rs     # Real-time WebSocket
│   │   └── static/
│   │       ├── index.html
│   │       ├── css/style.css
│   │       └── js/app.js
│   │
│   └── error.rs          # Error types
│
├── scripts/             # Utility scripts
│   ├── init_storage.sh  # Initialize storage directories
│   ├── configure.sh     # Quick configuration
│   ├── init_config.sh   # Interactive configuration wizard
│   ├── docker-build.sh  # Build Docker image
│   ├── docker-push.sh   # Push to registry
│   └── benchmark.sh     # Performance benchmarking
│
├── docker-compose.yml              # Standard Docker deployment
├── docker-compose.dev.yml          # Development environment
├── Dockerfile                      # Container image
├── Dockerfile.local                # Local build image
├── Makefile                        # Build automation
│
├── docs/                # Documentation
│   ├── CONFIGURATION.md  # Configuration guide
│   └── POOL_CONFIG_GUIDE.md  # Pool configuration implementation
│
└── examples/            # Example configurations
    ├── storage.example.json
    ├── docker-compose-metrics.yml
    ├── prometheus.yml
    └── grafana-*.yml
```

## Configuration

### Quick Configuration

```bash
# Run the configuration wizard
./scripts/configure.sh

# Or manually edit the config file
vim data/config/objstor.json
```

### Configuration File

The system uses a JSON configuration file located at `data/config/objstor.json`:

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080,
    "log_level": "info",
    "log_dir": "./logs"
  },
  "storage": {
    "data_dir": "./data",
    "pools": [
      {
        "id": "pool-001",
        "path": "./data/pools/pool-001",
        "capacity": 107374182400,
        "max_objects": 1000000,
        "quota_enabled": false
      }
    ],
    "scheduler": {
      "strategy": "least_loaded",
      "rebalance_threshold": 0.2
    }
  }
}
```

For detailed configuration options, see [docs/CONFIGURATION.md](docs/CONFIGURATION.md).

### Default Configuration

- **Data Directory**: `./data`
- **Storage Pools**: 2 pools, 100GB each
- **Scheduling Strategy**: Least Loaded
- **Server Port**: 8080
- **Log Level**: info
- **Log Directory**: `./logs`

### Storage Layout

```
data/
├── config/
│   └── objstor.json              # Main configuration file
├── pools/
│   ├── pool-001/
│   │   ├── objects/
│   │   │   ├── a3/
│   │   │   │   └── a3f5b8c2.../     # SHA256 hash prefix
│   │   │   │       ├── data          # Actual object data
│   │   │   │       └── meta.json     # Object metadata
│   │   │   └── 7c/
│   │   │       └── 7c1d4e9a.../
│   │   │           ├── data
│   │   │           └── meta.json
│   │   └── metadata/
│   │       └── pool.json             # Pool metadata
│   └── pool-002/
└── metadata.db                        # SQLite metadata store
```

### Pool Configuration

Pools can be configured with custom paths and capacities:

```json
{
  "pools": [
    {
      "id": "ssd-hot",
      "path": "/fast/ssd/objstor",
      "capacity": 536870912000,
      "max_objects": 5000000,
      "quota_enabled": false
    },
    {
      "id": "hdd-cold",
      "path": "/slow/hdd/objstor",
      "capacity": 2199023255552,
      "max_objects": 50000000,
      "quota_enabled": true
    }
  ]
}
```

See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for more examples.

## Development

### Quick Development Setup

```bash
# Run with hot reload
RUST_LOG=debug cargo run

# Or use Make
make run
```

```bash
# Debug build (faster compilation)
cargo build

# Release build (optimized)
cargo build --release

# Check compilation without building
cargo check
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_bucket_creation
```

### Development Server

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run with trace logging
RUST_LOG=trace cargo run

# Run specific module logs
RUST_LOG=objstor::api=debug,objstor::storage=trace cargo run
```

### Code Quality

```bash
# Format code
cargo fmt

# Check code style
cargo fmt --check

# Run clippy lints
cargo clippy

# Run clippy with all features
cargo clippy --all-features
```

## Web UI Features

### Dashboard
- Storage usage summary cards
- Real-time storage usage chart (Chart.js)
- Storage pool status with usage bars
- Auto-refresh via WebSocket (5s interval)

### Buckets Management
- List all buckets with metadata
- Create new buckets (with validation)
- Delete buckets (with confirmation modal)
- View bucket details (name, region, creation date)

### Objects Browser
- Bucket selector dropdown
- Upload objects via file picker
- Download objects
- Delete objects (with confirmation)
- Real-time object count updates

### Monitoring
- Live storage usage percentage
- Total objects count (from database)
- Active pools count
- Real-time metrics chart
- Update timestamp

### Logs Viewer
- Real-time log streaming via WebSocket
- Color-coded log levels (info, warn, error)
- Timestamp display
- Auto-scroll to latest logs
- 2-second update interval

## Admin API

### Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/metrics` | GET | System metrics |
| `/api/v1/buckets` | GET | List buckets |
| `/api/v1/health` | GET | Health check |

### Metrics Response Example

```json
{
  "storage": {
    "used": 1073741824,
    "capacity": 214748364800,
    "usage_ratio": 0.005
  },
  "buckets": [
    {
      "name": "my-bucket",
      "created_at": "2024-01-15T10:30:00Z",
      "region": "us-east-1",
      "owner": "test-user"
    }
  ],
  "pools": [
    {
      "id": "pool-001",
      "capacity": 107374182400,
      "used": 536870912,
      "objects": 5,
      "status": "Healthy",
      "usage_ratio": 0.005
    }
  ],
  "total_objects": 10
}
```

## Performance

### Benchmarks

Run the included benchmark script:

```bash
./scripts/benchmark.sh
```

Expected performance (on modern hardware):
- Small file uploads (<1MB): ~1000 ops/sec
- Large file downloads (>100MB): ~500 MB/sec
- Concurrent connections: 1000+

### Optimization Tips

1. **Use Release Mode**: Always run `cargo run --release` for production
2. **Enable WAL Mode**: SQLite WAL mode is enabled by default
3. **Pool Sizing**: Create pools sized for your workload
4. **Monitoring**: Use the Monitoring page to track performance

## Documentation

### User Guides
- [BUILD.md](BUILD.md) - Build and setup guide
- [DOCKER_DEPLOY.md](DOCKER_DEPLOY.md) - Docker deployment guide
- [LOCAL_DEPLOYMENT.md](LOCAL_DEPLOYMENT.md) - Local deployment guide
- [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) - Project structure and architecture

### Configuration
- [docs/CONFIGURATION.md](docs/CONFIGURATION.md) - Configuration guide
- [docs/POOL_CONFIG_GUIDE.md](docs/POOL_CONFIG_GUIDE.md) - Pool configuration
- [CLAUDE.md](CLAUDE.md) - Development guidelines

## Troubleshooting

**Issue**: Port 8080 already in use
```bash
# Kill existing process
lsof -ti:8080 | xargs kill -9

# Or use a different port (modify src/main.rs)
```

**Issue**: Database locked
```bash
# Remove WAL files
rm -f data/metadata.db-shm data/metadata.db-wal
```

**Issue**: Permission denied on data directory
```bash
# Fix permissions
chmod -R 755 data/
```

**Issue**: WebSocket connection failed
- Check browser console for errors
- Verify `/ws` route is accessible
- Check firewall settings

## Roadmap

### Planned Features

- [ ] Distributed multi-node support
- [ ] Erasure coding for data redundancy
- [ ] Lifecycle management (auto-delete expired objects)
- [ ] Cross-region replication
- [ ] Event notifications (SNS-like)
- [ ] Requester pays functionality
- [ ] Presigned URLs
- [ ] Bucket policies
- [ ] CORS configuration
- [ ] Website hosting
- [ ] Request metrics and analytics

## Contributing

Contributions are welcome! Please follow these guidelines:

1. **Code Style**: Run `cargo fmt` before committing
2. **Testing**: Add tests for new features
3. **Documentation**: Update README and code comments
4. **Commits**: Use clear commit messages

### Development Workflow

```bash
# 1. Fork and clone the repository
git clone <your-fork>

# 2. Create a feature branch
git checkout -b feature/my-feature

# 3. Make changes and test
cargo test
cargo clippy

# 4. Commit changes
git commit -m "Add my feature"

# 5. Push and create PR
git push origin feature/my-feature
```

## License

MIT License - see LICENSE file for details

## Acknowledgments

- Built with [Axum](https://github.com/tokio-rs/axum) web framework
- S3 protocol compatibility with [AWS SDK](https://aws.amazon.com/sdk/)
- UI inspired by modern design patterns
- Chart visualization with [Chart.js](https://www.chartjs.org/)

---

**Made with ❤️ in Rust**
