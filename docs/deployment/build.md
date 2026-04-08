# ObjStor - Build and Setup Guide

## Prerequisites

### Option 1: Docker Deployment (Recommended)

- Docker 20.10+
- Docker Compose 2.0+ (optional)

### Option 2: Native Build

#### Install Rust

If Rust is not installed on your system, install it using rustup:

```bash
# macOS/Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# After installation, reload your shell
source $HOME/.cargo/env
```

For other platforms, visit https://rustup.rs/

#### Verify Installation

```bash
cargo --version
rustc --version
```

## Building ObjStor

### Method 1: Docker Build (Recommended)

```bash
# Clone the repository
git clone https://github.com/yong-huang/objstor.git
cd objstor

# Build Docker image
docker build -t objstor:latest .

# Or use the build script
./scripts/docker-build.sh

# Run with Docker Compose
docker-compose up -d
```

See [docs/DOCKER.md](docs/DOCKER.md) for more Docker deployment options.

### Method 2: Native Build

### 1. Clone or Navigate to Project

```bash
cd /Users/hyhit/Desktop/workspace/projects/objstor
```

### 2. Initialize Storage (Optional)

```bash
./scripts/configure.sh
```

This will create the default configuration and directory structure.

### 3. Build the Project

```bash
# Debug build (faster compile)
cargo build

# Release build (optimized)
cargo build --release
```

### 4. Run the Server

```bash
# Debug mode
cargo run

# Release mode
./target/release/objstor
```

The server will start on:
- **Web UI**: http://localhost:3020
- **S3 API**: http://localhost:3020/ (same port, different path)

### 5. Using Makefile

```bash
# Show all commands
make help

# Build
make build

# Run
make run

# Initialize config
make init

# Docker operations
make docker-build
make docker-run
make docker-compose-up
```

## Troubleshooting

### Docker Issues

**Issue**: Docker build fails
```bash
# Clear Docker cache
docker system prune -a

# Rebuild without cache
docker build --no-cache -t objstor:latest .
```

**Issue**: Container won't start
```bash
# Check logs
docker logs objstor

# Check volume permissions
docker run --rm -v objstor_data:/data alpine ls -la /data
```

### Port Already in Use

If port 3020 is already in use, you can change it in `src/main.rs`:

```rust
let addr = SocketAddr::from(([0, 0, 0, 0], 3020));  // Change this
```

### Build Errors

If you encounter build errors:

1. **Update Rust**: `rustup update`
2. **Clean build**: `cargo clean && cargo build`
3. **Check dependencies**: `cargo fetch`

### Missing Dependencies

The project requires system libraries for SQLite (bundled) and OpenSSL. On most systems, these are included.

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Code Formatting

```bash
# Format code
cargo fmt

# Check formatting without making changes
cargo fmt --check
```

### Linting

```bash
# Run Clippy linter
cargo clippy

# Fix warnings automatically
cargo clippy --fix
```

## Project Structure After Build

```
objstor/
├── target/                  # Build artifacts
│   ├── debug/              # Debug builds
│   └── release/            # Release builds
├── data/                   # Runtime data
│   ├── pools/
│   ├── config/
│   └── metadata.db
├── logs/                   # Log files
└── src/                    # Source code
```

## Configuration

### Using Configuration File

ObjStor uses a JSON configuration file at `data/config/objstor.json`:

```bash
# Quick configuration wizard
./scripts/configure.sh

# Or manually edit
vim data/config/objstor.json
```

Example configuration:

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 3020,
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
        "max_objects": 1000000
      }
    ],
    "scheduler": {
      "strategy": "least_loaded"
    }
  }
}
```

### Environment Variables

```bash
# Set log level
export RUST_LOG=info  # debug, info, warn, error

# Set custom data directory
export OBJSTOR_DATA_DIR=/path/to/data
```

### Configuration Files

Configuration files are managed in `data/config/`:
- `objstor.json` - Main configuration (server + storage)

For detailed configuration options, see [docs/CONFIGURATION.md](docs/CONFIGURATION.md).

## Performance Tuning

### Release Build Optimizations

The release profile in `Cargo.toml` includes:
- Opt-level 3 (maximum optimization)
- Link-time optimization (LTO)
- Single codegen unit

### For Production

1. Use release builds: `cargo build --release`
2. Adjust pool sizes in configuration
3. Enable system-level tuning (ulimit, etc.)
4. Consider running behind a reverse proxy (nginx)

## Next Steps

After building and running:

### Using Web UI
1. Open http://localhost:3020
2. View dashboard with storage metrics
3. Create buckets and upload objects

### Testing S3 API
```bash
# Set credentials
export AWS_ACCESS_KEY_ID=test-access-key
export AWS_SECRET_ACCESS_KEY=test-secret-key

# Test with AWS CLI
aws s3 ls --endpoint-url http://localhost:3020
aws s3 mb s3://test-bucket --endpoint-url http://localhost:3020
aws s3 cp file.txt s3://test-bucket/ --endpoint-url http://localhost:3020
```

### Monitoring
```bash
# View metrics
curl http://localhost:3020/api/v1/metrics

# Health check
curl http://localhost:3020/health
```

### Advanced Configuration
- See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for pool configuration
- See [docs/DOCKER.md](docs/DOCKER.md) for Docker deployment
- Run `./scripts/benchmark.sh` for performance testing

## Support

For issues or questions:
- Check [docs/](docs/) for detailed documentation
- Review [README.md](README.md) for usage examples
- Check logs in `logs/` directory or Docker logs
- See [CLAUDE.md](CLAUDE.md) for development details
