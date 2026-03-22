#!/bin/bash
# ObjStor Configuration Initialization Script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "🔧 ObjStor Configuration Initialization"
echo "======================================="
echo ""

# Check if config directory exists
CONFIG_DIR="$PROJECT_DIR/data/config"
if [ ! -d "$CONFIG_DIR" ]; then
    echo "📁 Creating config directory..."
    mkdir -p "$CONFIG_DIR"
fi

# Check if config file exists
CONFIG_FILE="$CONFIG_DIR/objstor.json"
if [ -f "$CONFIG_FILE" ]; then
    echo "⚠️  Configuration file already exists: $CONFIG_FILE"
    read -p "Do you want to overwrite it? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "❌ Aborted."
        exit 1
    fi
fi

# Interactive configuration
echo ""
echo "📋 Configuration Wizard"
echo "======================="
echo ""

# Server configuration
echo "Server Settings:"
read -p "  Host [0.0.0.0]: " HOST
HOST=${HOST:-0.0.0.0}

read -p "  Port [8080]: " PORT
PORT=${PORT:-8080}

read -p "  Log Level (debug/info/warn/error) [info]: " LOG_LEVEL
LOG_LEVEL=${LOG_LEVEL:-info}

# Storage configuration
echo ""
echo "Storage Settings:"
read -p "  Data Directory [./data]: " DATA_DIR
DATA_DIR=${DATA_DIR:-./data}

# Pool configuration
echo ""
echo "Pool Configuration:"
read -p "  Number of pools [2]: " NUM_POOLS
NUM_POOLS=${NUM_POOLS:-2}

POOLS_ARRAY="[]"
POOL_CAPACITY=107374182400  # 100GB default
POOL_MAX_OBJECTS=1000000

for ((i=1; i<=NUM_POOLS; i++)); do
    echo ""
    echo "  Pool $i:"
    read -p "    Pool ID [pool-00$i]: " POOL_ID
    POOL_ID=${POOL_ID:-pool-00$i}

    read -p "    Path [$DATA_DIR/pools/$POOL_ID]: " POOL_PATH
    POOL_PATH=${POOL_PATH:-$DATA_DIR/pools/$POOL_ID}

    read -p "    Capacity in GB [100]: " POOL_CAPACITY_GB
    POOL_CAPACITY_GB=${POOL_CAPACITY_GB:-100}
    POOL_CAPACITY=$((POOL_CAPACITY_GB * 1024 * 1024 * 1024))

    # Add pool to array
    POOL_JSON="{
      \"id\": \"$POOL_ID\",
      \"path\": \"$POOL_PATH\",
      \"capacity\": $POOL_CAPACITY,
      \"max_objects\": $POOL_MAX_OBJECTS,
      \"quota_enabled\": false
    }"

    if [ "$i" -eq 1 ]; then
        POOLS_ARRAY="[$POOL_JSON]"
    else
        POOLS_ARRAY="${POOLS_ARRAY%,}]${POOLS_ARRAY##*[}, $POOL_JSON]}"
    fi
done

# Generate configuration file
cat > "$CONFIG_FILE" <<EOF
{
  "server": {
    "host": "$HOST",
    "port": $PORT,
    "s3_port": $PORT,
    "enable_tls": false,
    "tls_cert": "",
    "tls_key": "",
    "max_request_size": 5368709120,
    "log_level": "$LOG_LEVEL",
    "log_dir": "./logs"
  },
  "storage": {
    "data_dir": "$DATA_DIR",
    "pools": [
EOF

# Add pools to config
for ((i=1; i<=NUM_POOLS; i++)); do
    echo "      {" >> "$CONFIG_FILE"
    echo "        \"id\": \"pool-00$i\"," >> "$CONFIG_FILE"
    echo "        \"path\": \"$DATA_DIR/pools/pool-00$i\"," >> "$CONFIG_FILE"
    echo "        \"capacity\": 107374182400," >> "$CONFIG_FILE"
    echo "        \"max_objects\": 1000000," >> "$CONFIG_FILE"
    echo "        \"quota_enabled\": false" >> "$CONFIG_FILE"
    if [ $i -eq $NUM_POOLS ]; then
        echo "      }" >> "$CONFIG_FILE"
    else
        echo "      }," >> "$CONFIG_FILE"
    fi
done

cat >> "$CONFIG_FILE" <<EOF
    ],
    "scheduler": {
      "strategy": "least_loaded",
      "rebalance_threshold": 0.2
    }
  }
}
EOF

echo ""
echo "✅ Configuration file created: $CONFIG_FILE"
echo ""
echo "📝 Configuration Summary:"
echo "  Host: $HOST:$PORT"
echo "  Log Level: $LOG_LEVEL"
echo "  Data Directory: $DATA_DIR"
echo "  Pools: $NUM_POOLS"
echo ""
echo "🚀 You can now start ObjStor with: cargo run"
