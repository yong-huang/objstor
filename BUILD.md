# ObjStor - Build and Setup Guide

## Prerequisites

### Install Rust

If Rust is not installed on your system, install it using rustup:

```bash
# macOS/Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# After installation, reload your shell
source $HOME/.cargo/env
```

For other platforms, visit https://rustup.rs/

### Verify Installation

```bash
cargo --version
rustc --version
```

## Building ObjStor

### 1. Clone or Navigate to Project

```bash
cd /Users/hyhit/Desktop/workspace/projects/objstor
```

### 2. Initialize Storage (Optional)

```bash
./scripts/init_storage.sh
```

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
- **Web UI**: http://localhost:8080
- **S3 API**: http://localhost:9000

## Troubleshooting

### Port Already in Use

If ports 8080 or 9000 are already in use, you can change them in `src/main.rs`:

```rust
let web_addr = SocketAddr::from(([0, 0, 0, 0], 8080));  // Change this
let s3_addr = SocketAddr::from(([0, 0, 0, 0], 9000));  // And this
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

### Environment Variables

```bash
# Set log level
export RUST_LOG=info  # debug, info, warn, error

# Set custom data directory
export OBJSTOR_DATA_DIR=/path/to/data
```

### Configuration Files

Configuration files are created in `data/config/`:
- `server.json` - Server settings
- `storage.json` - Storage pool settings

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

1. **Access Web UI**: Open http://localhost:8080
2. **Test S3 API**: See README.md for AWS CLI examples
3. **Run Benchmarks**: `./scripts/benchmark.sh`

## Support

For issues or questions:
- Check CLAUDE.md for development details
- Review README.md for usage examples
- Check logs in `logs/` directory
