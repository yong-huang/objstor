#!/bin/bash
# Docker部署脚本 - 将Pool映射到本地存储目录

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 存储目录路径
STORAGE_DIR="/Users/hyhit/Desktop/workspace/storage"

echo -e "${GREEN}=== ObjStor Docker 部署脚本 ===${NC}"
echo ""

# 1. 检查Docker是否安装
if ! command -v docker &> /dev/null; then
    echo -e "${RED}错误: Docker未安装，请先安装Docker${NC}"
    exit 1
fi

if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    echo -e "${RED}错误: docker-compose未安装${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Docker已安装${NC}"

# 2. 创建存储目录
echo ""
echo -e "${YELLOW}创建存储目录结构...${NC}"
mkdir -p "$STORAGE_DIR/pools/pool-001/objects"
mkdir -p "$STORAGE_DIR/pools/pool-001/metadata"
mkdir -p "$STORAGE_DIR/pools/pool-002/objects"
mkdir -p "$STORAGE_DIR/pools/pool-002/metadata"
echo -e "${GREEN}✓ 存储目录已创建: $STORAGE_DIR${NC}"

# 3. 创建必要的本地目录
echo ""
echo -e "${YELLOW}创建本地数据和日志目录...${NC}"
mkdir -p ./data/config
mkdir -p ./logs
echo -e "${GREEN}✓ 本地目录已创建${NC}"

# 4. 检查配置文件
if [ ! -f "./data/config/objstor.json" ]; then
    echo ""
    echo -e "${YELLOW}创建默认配置文件...${NC}"
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

# 5. 构建Docker镜像
echo ""
echo -e "${YELLOW}构建Docker镜像...${NC}"
if [ -f "Dockerfile" ]; then
    docker build -t objstor:latest .
    echo -e "${GREEN}✓ Docker镜像构建完成${NC}"
else
    echo -e "${RED}错误: 未找到Dockerfile${NC}"
    exit 1
fi

# 6. 停止并删除旧容器
echo ""
echo -e "${YELLOW}停止旧容器...${NC}"
if docker ps -a --format '{{.Names}}' | grep -q "^objstor$"; then
    docker stop objstor 2>/dev/null || true
    docker rm objstor 2>/dev/null || true
    echo -e "${GREEN}✓ 旧容器已删除${NC}"
fi

# 7. 启动服务
echo ""
echo -e "${YELLOW}启动ObjStor服务...${NC}"
docker compose up -d
echo -e "${GREEN}✓ 服务已启动${NC}"

# 8. 等待服务就绪
echo ""
echo -e "${YELLOW}等待服务启动...${NC}"
sleep 3

# 9. 检查服务状态
if docker ps --format '{{.Names}}' | grep -q "^objstor$"; then
    echo -e "${GREEN}✓ 容器正在运行${NC}"

    # 尝试健康检查
    if command -v curl &> /dev/null; then
        echo ""
        echo -e "${YELLOW}执行健康检查...${NC}"
        if curl -s http://localhost:8080/health > /dev/null; then
            echo -e "${GREEN}✓ 健康检查通过${NC}"
        else
            echo -e "${YELLOW}⚠ 健康检查失败，但容器正在运行${NC}"
        fi
    fi
else
    echo -e "${RED}✗ 容器启动失败${NC}"
    echo ""
    echo "查看日志:"
    docker logs objstor
    exit 1
fi

# 10. 显示部署信息
echo ""
echo -e "${GREEN}=== 部署完成 ===${NC}"
echo ""
echo "服务信息:"
echo "  - S3 API:     http://localhost:8080"
echo "  - Web UI:     http://localhost:8080/web"
echo "  - Health:     http://localhost:8080/health"
echo "  - Metrics:    http://localhost:8080/api/v1/metrics"
echo ""
echo "存储映射:"
echo "  - 宿主机: $STORAGE_DIR"
echo "  - 容器内: /app/storage"
echo ""
echo "默认凭证:"
echo "  - Access Key: test-access-key"
echo "  - Secret Key: test-secret-key"
echo ""
echo "常用命令:"
echo "  - 查看日志: docker logs -f objstor"
echo "  - 停止服务: docker compose down"
echo "  - 重启服务: docker compose restart"
echo "  - 进入容器: docker exec -it objstor sh"
echo ""
echo "测试S3连接:"
echo "  aws s3 ls --endpoint-url http://localhost:8080"
echo ""
