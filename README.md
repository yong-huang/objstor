# ObjStor - S3-Compatible Object Storage Simulator

A high-performance, S3-compatible object storage system written in Rust, featuring load balancing, multi-pool storage, and a modern web management interface.

## Features

- **S3-Compatible API**: Full S3 protocol support for buckets and objects
- **Load Balancing**: Smart data distribution across multiple storage pools
- **Multi-Pool Storage**: Support for multiple storage pools with health monitoring
- **Web UI**: Modern dashboard for monitoring and management
- **Real-time Metrics**: WebSocket-based real-time system monitoring
- **Multipart Upload**: Support for large file uploads
- **Access Control**: AWS-style signature authentication (AWS4-HMAC-SHA256)

## Quick Start

### Prerequisites

- Rust 1.70 or later
- Cargo

### Installation

```bash
# Clone the repository
cd objstor

# Build the project
cargo build --release

# Run the server
cargo run --release
```

### Usage

The server starts two endpoints:

- **Web UI**: http://localhost:8080
- **S3 API**: http://localhost:9000

### Default Access Key

For testing, the system creates a default access key:
- Access Key ID: `test-access-key`
- Secret Key: `test-secret-key`

### Testing with AWS CLI

```bash
# Set up AWS CLI for local testing
export AWS_ACCESS_KEY_ID=test-access-key
export AWS_SECRET_ACCESS_KEY=test-secret-key
export AWS_DEFAULT_REGION=us-east-1

# List buckets
aws s3 ls --endpoint-url http://localhost:9000

# Create a bucket
aws s3 mb s3://my-bucket --endpoint-url http://localhost:9000

# Upload a file
aws s3 cp file.txt s3://my-bucket/ --endpoint-url http://localhost:9000

# Download a file
aws s3 cp s3://my-bucket/file.txt . --endpoint-url http://localhost:9000

# List objects
aws s3 ls s3://my-bucket --endpoint-url http://localhost:9000

# Delete a file
aws s3 rm s3://my-bucket/file.txt --endpoint-url http://localhost:9000

# Delete a bucket
aws s3 rb s3://my-bucket --endpoint-url http://localhost:9000
```

### Testing with boto3 (Python)

```python
import boto3

# Create S3 client
s3 = boto3.client('s3',
    endpoint_url='http://localhost:9000',
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

```
┌─────────────────────────────────────────────────────────────────┐
│                         ObjStor System                          │
├─────────────────────────────────────────────────────────────────┤
│  S3 Client (AWS SDK/boto3)  →  HTTP/REST API  →  S3 Handler    │
│                                                                   │
│  ┌──────────────┐  ┌─────────────┐  ┌──────────┐              │
│  │  Load        │  │  Storage    │  │ Metadata │              │
│  │  Balancer    │  │  Pool       │  │  Store   │              │
│  └──────────────┘  └─────────────┘  └──────────┘              │
│                                                                   │
│  Web UI ←→ WebSocket ←→ Real-time Metrics                       │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

- `src/config/` - Configuration management
- `src/storage/` - Storage engine and pool management
- `src/scheduler/` - Load balancing algorithms
- `src/metadata/` - SQLite metadata storage
- `src/auth/` - AWS signature authentication
- `src/api/s3/` - S3 protocol handlers
- `src/web/` - Web UI and WebSocket server
- `src/logging/` - Structured logging

## Configuration

The system uses sensible defaults but can be configured:

- Data directory: `./data`
- Pools: 2 default pools, 100GB each
- Scheduling: Least-loaded strategy
- Web UI port: 8080
- S3 API port: 9000

## Storage Layout

```
data/
├── pools/
│   ├── pool-001/
│   │   ├── objects/
│   │   │   └── [hash]/data
│   │   └── metadata/pool.json
│   └── pool-002/
└── metadata.db
```

## License

MIT License - see LICENSE file for details

## Contributing

Contributions are welcome! Please submit pull requests or open issues for bugs and feature requests.
