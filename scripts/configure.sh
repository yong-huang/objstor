#!/bin/bash
# Simple configuration script for ObjStor

set -e

echo "🚀 ObjStor Quick Configuration"
echo "==============================="
echo ""

# Check if config file exists
if [ -f "data/config/objstor.json" ]; then
    echo "⚠️  Configuration file already exists"
    echo "   Current config: data/config/objstor.json"
    echo ""
    read -p "Overwrite? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Exiting..."
        exit 0
    fi
fi

# Create directories
echo "📁 Creating directories..."
mkdir -p data/config
mkdir -p data/pools/pool-001/objects
mkdir -p data/pools/pool-001/metadata
mkdir -p data/pools/pool-002/objects
mkdir -p data/pools/pool-002/metadata
mkdir -p logs

# Create default configuration
cat > data/config/objstor.json << 'EOF'
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080,
    "s3_port": 8080,
    "enable_tls": false,
    "tls_cert": "",
    "tls_key": "",
    "max_request_size": 5368709120,
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
      },
      {
        "id": "pool-002",
        "path": "./data/pools/pool-002",
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
EOF

echo "✅ Configuration created: data/config/objstor.json"
echo ""
echo "📝 Configuration Summary:"
echo "  - Server: 0.0.0.0:8080"
echo "  - Pools: 2 (pool-001, pool-002)"
echo "  - Capacity per pool: 100 GB"
echo "  - Log level: info"
echo ""
echo "🎯 Next steps:"
echo "  1. Review and edit: data/config/objstor.json"
echo "  2. Start server: cargo run"
echo "  3. Access Web UI: http://localhost:8080"
echo ""
echo "📖 For more information, see: docs/CONFIGURATION.md"
