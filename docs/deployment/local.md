# ObjStor Local Deployment Guide

## Deployment Status

✅ **Deployment Successful!** ObjStor has been successfully deployed and is running.

### Service Information

- **Running Mode**: Local execution (non-Docker)
- **S3 API**: http://localhost:8080
- **Web UI**: http://localhost:8080/web
- **Health Check**: http://localhost:8080/health
- **Storage Directory**: `/Users/hyhit/Desktop/workspace/storage`

### Storage Configuration

Storage pools have been mapped to the specified directory:
```
/Users/hyhit/Desktop/workspace/storage/
└── pools/
    ├── pool-001/
    │   ├── objects/     # Object data
    │   └── metadata/    # Pool metadata
    └── pool-002/
        ├── objects/
        └── metadata/
```

### Default Credentials

```
Access Key ID: test-access-key
Secret Key: test-secret-key
```

## Usage

### Start Service

```bash
# Foreground execution
./target/release/objstor

# Background execution
nohup ./target/release/objstor > logs/objstor.log 2>&1 &

# Use deployment script
./scripts/local-deploy.sh
```

### Stop Service

```bash
# Find process
ps aux | grep objstor

# Stop service
pkill -f "objstor"
```

### View Logs

```bash
# Real-time logs
tail -f logs/objstor.log

# Last 100 lines
tail -100 logs/objstor.log
```

## S3 Client Usage

### Using AWS CLI

```bash
# List all buckets
aws s3 ls --endpoint-url http://localhost:8080

# Create bucket
aws s3 mb s3://my-bucket --endpoint-url http://localhost:8080

# Upload file
aws s3 cp file.txt s3://my-bucket/ --endpoint-url http://localhost:8080

# Download file
aws s3 cp s3://my-bucket/file.txt ./downloaded.txt --endpoint-url http://localhost:8080

# List bucket contents
aws s3 ls s3://my-bucket/ --endpoint-url http://localhost:8080

# Delete file
aws s3 rm s3://my-bucket/file.txt --endpoint-url http://localhost:8080

# Delete bucket
aws s3 rb s3://my-bucket --endpoint-url http://localhost:8080
```

### Testing and Verification

```bash
# Health check
curl http://localhost:8080/health

# List buckets (XML format)
curl http://localhost:8080/

# View metrics
curl http://localhost:8080/api/v1/metrics | jq
```

## Configuration File

Configuration file location: `data/config/objstor.json`

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080,
    "log_level": "info"
  },
  "storage": {
    "data_dir": "./data",
    "pools": [
      {
        "id": "pool-001",
        "path": "/Users/hyhit/Desktop/workspace/storage/pools/pool-001",
        "capacity": 107374182400,
        "max_objects": 1000000
      }
    ]
  }
}
```

## Data Verification

Verify files are stored in the mapped directory:

```bash
# Find stored files
find /Users/hyhit/Desktop/workspace/storage/pools/ -type f -name "data"

# View file contents
cat /Users/hyhit/Desktop/workspace/storage/pools/pool-001/objects/*/data
```

## Troubleshooting

### Issue 1: Port Already in Use

```bash
# Check port usage
lsof -i :8080

# Stop process occupying the port
kill -9 <PID>
```

### Issue 2: Service Fails to Start

```bash
# View detailed logs
cat logs/objstor.log

# Check configuration file
cat data/config/objstor.json | jq
```

### Issue 3: Unable to Connect to Service

```bash
# Check if service is running
ps aux | grep objstor

# Test port
telnet localhost 8080
```

## Performance Monitoring

```bash
# View process resource usage
top -p $(pgrep objstor)

# View storage usage
du -sh /Users/hyhit/Desktop/workspace/storage/

# View object count
find /Users/hyhit/Desktop/workspace/storage/pools/ -type f -name "data" | wc -l
```

## Next Steps

1. **Configure Authentication**: Create production access keys
2. **Setup Monitoring**: Configure Prometheus + Grafana
3. **Backup Strategy**: Set up regular backups
4. **Performance Tuning**: Adjust configuration based on load

## Related Documentation

- [Docker Deployment Guide](docker.md)
- [Configuration Guide](../CONFIGURATION.md)
- [API Documentation](../API.md)
