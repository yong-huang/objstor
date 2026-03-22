#!/bin/bash
# Initialize ObjStor storage directories

set -e

echo "Initializing ObjStor storage..."

# Create base directories
mkdir -p data/pools/pool-001/objects
mkdir -p data/pools/pool-001/metadata
mkdir -p data/pools/pool-002/objects
mkdir -p data/pools/pool-002/metadata
mkdir -p data/config
mkdir -p logs

# Create pool metadata files
cat > data/pools/pool-001/metadata/pool.json <<EOF
{
  "id": "pool-001",
  "capacity": 107374182400,
  "used": 0,
  "objects_count": 0,
  "status": "Healthy"
}
EOF

cat > data/pools/pool-002/metadata/pool.json <<EOF
{
  "id": "pool-002",
  "capacity": 107374182400,
  "used": 0,
  "objects_count": 0,
  "status": "Healthy"
}
EOF

# Create server config
cat > data/config/server.json <<EOF
{
  "host": "0.0.0.0",
  "port": 8080,
  "s3_port": 9000,
  "enable_tls": false,
  "log_level": "info",
  "log_dir": "./logs"
}
EOF

# Create storage config
cat > data/config/storage.json <<EOF
{
  "data_dir": "./data",
  "pools": [
    {
      "id": "pool-001",
      "path": "./data/pools/pool-001",
      "capacity": 107374182400,
      "max_objects": 1000000
    },
    {
      "id": "pool-002",
      "path": "./data/pools/pool-002",
      "capacity": 107374182400,
      "max_objects": 1000000
    }
  ],
  "scheduler": {
    "strategy": "least_loaded",
    "rebalance_threshold": 0.2
  }
}
EOF

echo "Storage initialized successfully!"
echo ""
echo "Pool 1: data/pools/pool-001 (100GB)"
echo "Pool 2: data/pools/pool-002 (100GB)"
echo ""
echo "Run 'cargo run' to start the server."
