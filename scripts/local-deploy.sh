#!/bin/bash
# Local deployment script - uses pre-built binary

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

STORAGE_DIR="/Users/hyhit/Desktop/workspace/storage"

echo -e "${GREEN}=== ObjStor 本地部署脚本 ===${NC}"
echo ""

# 1. 检查本地编译的二进制
if [ ! -f "target/release/objstor" ]; then
    echo -e "${YELLOW}本地二进制文件不存在，正在编译...${NC}"
    ~/.cargo/bin/cargo build --release
    echo -e "${GREEN}✓ 编译完成${NC}"
else
    echo -e "${GREEN}✓ 找到本地二进制文件${NC}"
fi

# 2. 创建存储目录
echo ""
echo -e "${YELLOW}创建存储目录结构...${NC}"
mkdir -p "$STORAGE_DIR/pools/pool-001"/{objects,metadata}
mkdir -p "$STORAGE_DIR/pools/pool-002"/{objects,metadata}
mkdir -p ./data/config
mkdir -p ./logs
echo -e "${GREEN}✓ 目录结构已创建${NC}"

# 3. 创建配置文件
if [ ! -f "./data/config/objstor.json" ]; then
    echo ""
    echo -e "${YELLOW}创建配置文件...${NC}"
    cat > ./data/config/objstor.json <<'EOF'
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
        "path": "./storage/pools/pool-001",
        "capacity": 107374182400,
        "max_objects": 1000000,
        "quota_enabled": false
      },
      {
        "id": "pool-002",
        "path": "./storage/pools/pool-002",
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
    echo -e "${GREEN}✓ 配置文件已创建${NC}"
fi

# 4. 尝试 Docker 部署
echo ""
echo -e "${YELLOW}尝试 Docker 部署...${NC}"

if ! command -v docker &> /dev/null || ! docker ps &> /dev/null; then
    echo -e "${RED}✗ Docker 不可用，尝试本地运行${NC}"
    echo ""
    echo "使用本地运行方式："
    echo "  ./target/release/objstor"
    exit 0
fi

# 检查是否可以拉取基础镜像
echo -e "${YELLOW}测试 Docker 连接...${NC}"
if timeout 10 docker pull alpine:3.18 &> /dev/null; then
    echo -e "${GREEN}✓ Docker 连接正常${NC}"

    # 使用本地 Dockerfile
    if [ -f "Dockerfile.local" ]; then
        echo -e "${YELLOW}使用本地 Dockerfile 构建镜像...${NC}"
        docker build -f Dockerfile.local -t objstor:latest .

        echo ""
        echo -e "${YELLOW}启动容器...${NC}"

        # 停止旧容器
        docker stop objstor 2>/dev/null || true
        docker rm objstor 2>/dev/null || true

        # 启动新容器
        docker run -d \
            --name objstor \
            --restart unless-stopped \
            -p 8080:8080 \
            -v "$STORAGE_DIR:/app/storage" \
            -v "$(pwd)/data:/app/data" \
            -v "$(pwd)/logs:/app/logs" \
            -e RUST_LOG=info \
            objstor:latest

        echo -e "${GREEN}✓ 容器已启动${NC}"

        # 等待服务就绪
        sleep 3

        if docker ps | grep -q "objstor"; then
            echo ""
            echo -e "${GREEN}=== 部署成功 ===${NC}"
            echo ""
            echo "服务地址:"
            echo "  - S3 API:     http://localhost:8080"
            echo "  - Web UI:     http://localhost:8080/web"
            echo "  - Health:     http://localhost:8080/health"
            echo ""
            echo "存储映射:"
            echo "  - $STORAGE_DIR"
            echo ""
            echo "查看日志:"
            echo "  docker logs -f objstor"
        fi
    fi
else
    echo -e "${RED}✗ Docker 连接失败${NC}"
    echo ""
    echo -e "${YELLOW}使用本地运行方式：${NC}"
    echo "  RUST_LOG=info ./target/release/objstor"
fi

echo ""
