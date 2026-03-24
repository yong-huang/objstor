# ObjStor Docker Deployment Guide

## Quick Deployment

### 1. Using Automated Script (Recommended)

```bash
# Run deployment script
./scripts/docker-deploy.sh
```

The script will automatically:
- ✓ Create storage directory structure
- ✓ Build Docker image
- ✓ Start service
- ✓ Health check

### 2. Manual Deployment

#### Step 1: Create Storage Directories

```bash
# Create storage directories on host machine
mkdir -p /Users/hyhit/Desktop/workspace/storage/pools/pool-001/{objects,metadata}
mkdir -p /Users/hyhit/Desktop/workspace/storage/pools/pool-002/{objects,metadata}
```

#### Step 2: Build Image

```bash
docker build -t objstor:latest .
```

#### Step 3: Start Service

```bash
docker compose up -d
```

## Directory Mapping

### Host → Container Mapping

| Host Path | Container Path | Description |
|-----------|----------------|-------------|
| `/Users/hyhit/Desktop/workspace/storage` | `/app/storage` | Storage pool data |
| `./data` | `/app/data` | Metadata and configuration |
| `./logs` | `/app/logs` | Log files |

### Paths in Configuration Files

Paths in the configuration file `data/config/objstor.json` use **container paths**:

```json
{
  "storage": {
    "pools": [
      {
        "id": "pool-001",
        "path": "./storage/pools/pool-001",  // Container path
        "capacity": 107374182400,
        "max_objects": 1000000
      }
    ]
  }
}
```

## Service Access

### Web Interface and API

- **S3 API**: http://localhost:8080
- **Web UI**: http://localhost:8080/web
- **Health Check**: http://localhost:8080/health
- **Metrics**: http://localhost:8080/api/v1/metrics

### Default Credentials

```
Access Key ID: test-access-key
Secret Key: test-secret-key
```

## Common Commands

### Container Management

```bash
# View logs
docker logs -f objstor

# Stop service
docker compose down

# Restart service
docker compose restart

# Enter container
docker exec -it objstor sh

# Check container status
docker ps
```

### Data Management

```bash
# View storage directory contents
ls -la /Users/hyhit/Desktop/workspace/storage/

# View container storage mapping
docker exec objstor ls -la /app/storage/

# Backup data
tar -czf objstor-backup-$(date +%Y%m%d).tar.gz \
    /Users/hyhit/Desktop/workspace/storage/ \
    ./data/ \
    ./logs/
```

## S3 Client Testing

### Using AWS CLI

```bash
# Configure alias
alias s3='aws s3 --endpoint-url http://localhost:8080'

# List buckets
s3 ls

# Create bucket
s3 mb s3://test-bucket

# Upload file
echo "Hello ObjStor" > test.txt
s3 cp test.txt s3://test-bucket/

# Download file
s3 cp s3://test-bucket/test.txt downloaded.txt

# Delete file
s3 rm s3://test-bucket/test.txt

# Delete bucket
s3 rb s3://test-bucket
```

### Using rclone

```bash
# Configure rclone
rclone config create objstor s3 \
    --provider "Other" \
    --access-key-id "test-access-key" \
    --secret-access-key "test-secret-key" \
    --endpoint "http://localhost:8080" \
    --location "us-east-1"

# List buckets
rclone lsd objstor:

# Upload file
rclone copy /path/to/files objstor:test-bucket/

# Download file
rclone copy objstor:test-bucket/ /path/to/download/
```

## Monitoring and Debugging

### View Real-time Logs

```bash
# View all logs
docker logs -f objstor

# View only error logs
docker logs -f objstor 2>&1 | grep ERROR

# View last 100 lines
docker logs --tail 100 objstor
```

### Health Check

```bash
# Using curl
curl http://localhost:8080/health

# Using wget
wget -qO- http://localhost:8080/health

# View metrics
curl http://localhost:8080/api/v1/metrics | jq
```

### Performance Monitoring

Use docker-compose configuration with monitoring:

```bash
# Start with monitoring configuration
docker compose -f docker-compose.dev.yml up -d

# Access Prometheus
open http://localhost:9090

# Access Grafana (admin/admin)
open http://localhost:3000
```

## Troubleshooting

### Issue 1: Container Fails to Start

```bash
# View detailed logs
docker logs objstor

# Check directory permissions
ls -la /Users/hyhit/Desktop/workspace/storage/
ls -la ./data/

# Fix permissions
chmod -R 755 /Users/hyhit/Desktop/workspace/storage/
chmod -R 755 ./data/
```

### Issue 2: Unable to Access Service

```bash
# Check if port is occupied
lsof -i :8080

# Check container status
docker ps -a

# Check network
docker network ls
docker network inspect objstor_objstor_network
```

### Issue 3: Data Cannot Be Written

```bash
# Check storage directory permissions
docker exec objstor ls -la /app/storage/

# Check disk space
df -h /Users/hyhit/Desktop/workspace/storage/

# Recreate directory structure
rm -rf /Users/hyhit/Desktop/workspace/storage/*
mkdir -p /Users/hyhit/Desktop/workspace/storage/pools/pool-001/{objects,metadata}
```

### Issue 4: Configuration File Error

```bash
# Validate JSON format
cat data/config/objstor.json | jq

# Regenerate default configuration
rm data/config/objstor.json
docker compose up -d
```

## Advanced Configuration

### Modify Storage Path

Edit `docker-compose.yml`:

```yaml
volumes:
  # Change to your path
  - /your/custom/path:/app/storage
```

Edit `data/config/objstor.json`:

```json
{
  "storage": {
    "pools": [
      {
        "path": "./storage/pools/pool-001"
      }
    ]
  }
}
```

### Add More Storage Pools

```bash
# Create new pool directory
mkdir -p /Users/hyhit/Desktop/workspace/storage/pools/pool-003/{objects,metadata}

# Update configuration file
vim data/config/objstor.json

# Restart service
docker compose restart
```

### Change Port

Edit `docker-compose.yml`:

```yaml
ports:
  - "9000:8080"  # Host port:Container port
```

### Enable Debug Logging

Edit `docker-compose.yml`:

```yaml
environment:
  - RUST_LOG=debug  # Change to debug level
```

## Backup and Restore

### Backup

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="./backups"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p $BACKUP_DIR

# Stop service (optional, ensures data consistency)
docker compose stop

# Create backup
tar -czf $BACKUP_DIR/objstor-$DATE.tar.gz \
    /Users/hyhit/Desktop/workspace/storage/ \
    ./data/ \
    ./logs/

# Restart service
docker compose start

echo "Backup completed: $BACKUP_DIR/objstor-$DATE.tar.gz"
```

### Restore

```bash
#!/bin/bash
# restore.sh

BACKUP_FILE=$1

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: ./restore.sh <backup_file>"
    exit 1
fi

# Stop service
docker compose down

# Extract backup
tar -xzf $BACKUP_FILE -C /

# Start service
docker compose up -d

echo "Restore completed"
```

## Upgrade

```bash
# 1. Backup data
./backup.sh

# 2. Pull new code
git pull

# 3. Rebuild image
docker build -t objstor:latest .

# 4. Restart service
docker compose up -d

# 5. Verify service
curl http://localhost:8080/health
```

## Uninstall

```bash
# Stop and delete container
docker compose down

# Delete image
docker rmi objstor:latest

# Delete data (caution!)
# rm -rf /Users/hyhit/Desktop/workspace/storage/
# rm -rf ./data/
# rm -rf ./logs/
```
