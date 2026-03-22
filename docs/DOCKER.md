# ObjStor Docker 部署指南

## 概述

ObjStor支持通过Docker容器化部署，提供简单、可靠、可移植的部署方式。

## 前置要求

- Docker 20.10+
- Docker Compose 2.0+ (可选)

## 快速开始

### 1. 构建镜像

```bash
# 克隆仓库
git clone https://github.com/yong-huang/objstor.git
cd objstor

# 构建Docker镜像
docker build -t objstor:latest .

# 或使用构建脚本
./scripts/docker-build.sh
```

### 2. 运行容器

```bash
# 基础运行
docker run -d \
  --name objstor \
  -p 8080:8080 \
  -v objstor_data:/app/data \
  -e RUST_LOG=info \
  objstor:latest

# 使用docker-compose
docker-compose up -d
```

### 3. 验证部署

```bash
# 检查容器状态
docker ps | grep objstor

# 查看日志
docker logs -f objstor

# 健康检查
curl http://localhost:8080/health

# 访问Web UI
open http://localhost:8080
```

## Docker Compose 部署

### 基础部署

使用 `docker-compose.yml`:

```bash
# 启动服务
docker-compose up -d

# 查看日志
docker-compose logs -f

# 停止服务
docker-compose down

# 停止并删除数据
docker-compose down -v
```

### 开发环境

使用 `docker-compose.dev.yml`:

```bash
# 启动开发环境
docker-compose -f docker-compose.dev.yml up -d

# 重新构建并启动
docker-compose -f docker-compose.dev.yml up -d --build
```

### 高可用部署（多存储卷）

使用 `docker-compose.storage.yml`:

```bash
# 创建存储目录
sudo mkdir -p /mnt/disk{1,2,3}/objstor
sudo chown -R 1000:1000 /mnt/disk*/objstor

# 启动服务
docker-compose -f docker-compose.storage.yml up -d
```

## 配置

### 环境变量

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `RUST_LOG` | info | 日志级别 (debug/info/warn/error) |
| `OBJSTOR_DATA_DIR` | /app/data | 数据目录 |
| `OBJSTOR_HOST` | 0.0.0.0 | 监听地址 |
| `OBJSTOR_PORT` | 8080 | 监听端口 |

### 卷挂载

| 卷路径 | 说明 |
|--------|------|
| `/app/data` | 数据目录（包含pools和config） |
| `/app/logs` | 日志目录 |
| `/app/data/config/objstor.json` | 配置文件（可选） |

### 自定义配置

创建自定义配置文件并挂载：

```bash
# 1. 创建配置文件
cat > objstor.json <<EOF
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080
  },
  "storage": {
    "data_dir": "/app/data",
    "pools": [
      {
        "id": "pool-001",
        "path": "/app/data/pools/pool-001",
        "capacity": 107374182400,
        "max_objects": 1000000
      }
    ]
  }
}
EOF

# 2. 运行容器并挂载配置
docker run -d \
  --name objstor \
  -p 8080:8080 \
  -v $(pwd)/objstor.json:/app/data/config/objstor.json:ro \
  -v objstor_data:/app/data \
  objstor:latest
```

## 存储配置

### 单卷部署

```yaml
volumes:
  objstor_data:
    driver: local
```

### 多卷部署（推荐生产环境）

```yaml
volumes:
  pool1_data:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: /mnt/disk1/objstor
  pool2_data:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: /mnt/disk2/objstor
```

### NFS存储

```yaml
volumes:
  nfs_data:
    driver: local
    driver_opts:
      type: nfs
      o: addr=nfs-server-ip,rw
      device: ":/path/to/nfs/share"
```

## 网络

### 默认网络配置

```yaml
networks:
  objstor_network:
    driver: bridge
    ipam:
      config:
        - subnet: 172.20.0.0/16
```

### 自定义网络

```bash
# 创建自定义网络
docker network create --driver bridge objstor-net

# 使用自定义网络
docker run -d \
  --name objstor \
  --network objstor-net \
  -p 8080:8080 \
  objstor:latest
```

### 外部网络访问

```yaml
services:
  objstor:
    networks:
      - objstor_network
      - external_network

networks:
  objstor_network:
    internal: true
  external_network:
    external: true
```

## 生产环境部署

### 资源限制

```yaml
services:
  objstor:
    image: objstor:latest
    deploy:
      resources:
        limits:
          cpus: '4'
          memory: 8G
        reservations:
          cpus: '2'
          memory: 4G
```

### 健康检查

```yaml
healthcheck:
  test: ["CMD", "wget", "--spider", "-q", "http://localhost:8080/health"]
  interval: 30s
  timeout: 10s
  retries: 3
  start_period: 40s
```

### 日志管理

```yaml
logging:
  driver: "json-file"
  options:
    max-size: "10m"
    max-file: "3"
```

### 重启策略

```yaml
restart: unless-stopped
# 或
restart: always
# 或
restart: on-failure:5
```

## 高可用配置

### 负载均衡（多实例）

```yaml
services:
  objstor-1:
    image: objstor:latest
    container_name: objstor-1
    environment:
      - INSTANCE_ID=1
    volumes:
      - objstor_data_1:/app/data

  objstor-2:
    image: objstor:latest
    container_name: objstor-2
    environment:
      - INSTANCE_ID=2
    volumes:
      - objstor_data_2:/app/data

  nginx:
    image: nginx:alpine
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
    ports:
      - "8080:8080"
    depends_on:
      - objstor-1
      - objstor-2
```

### 共享存储后端

使用NFS或分布式文件系统：

```yaml
volumes:
  shared_storage:
    driver: local
    driver_opts:
      type: nfs
      o: addr=nfs-server,rw
      device: ":/objstor/data"
```

## 监控和日志

### 查看容器日志

```bash
# 实时日志
docker logs -f objstor

# 最近100行
docker logs --tail 100 objstor

# 带时间戳
docker logs -t objstor

# 指定时间范围
docker logs --since 2024-01-01T00:00:00 objstor
```

### 集成Prometheus监控

```yaml
services:
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus_data:/prometheus
    ports:
      - "9090:9090"
```

prometheus.yml:
```yaml
scrape_configs:
  - job_name: 'objstor'
    static_configs:
      - targets: ['objstor:8080']
    metrics_path: '/api/v1/metrics'
```

## 备份和恢复

### 备份数据

```bash
# 备份整个数据目录
docker run --rm \
  -v objstor_data:/data \
  -v $(pwd)/backup:/backup \
  alpine tar czf /backup/objstor-backup-$(date +%Y%m%d).tar.gz /data

# 仅备份配置
docker cp objstor:/app/data/config/objstor.json ./objstor-config-backup.json
```

### 恢复数据

```bash
# 恢复数据目录
docker run --rm \
  -v objstor_data:/data \
  -v $(pwd)/backup:/backup \
  alpine tar xzf /backup/objstor-backup-20240101.tar.gz -C /

# 恢复配置
docker cp ./objstor-config-backup.json objstor:/app/data/config/objstor.json
```

## 故障排查

### 容器无法启动

```bash
# 查看详细日志
docker logs objstor

# 检查容器状态
docker inspect objstor

# 进入容器调试
docker exec -it objstor sh
```

### 权限问题

```bash
# 检查卷权限
docker run --rm \
  -v objstor_data:/data \
  alpine ls -la /data

# 修复权限
docker run --rm \
  -v objstor_data:/data \
  alpine chown -R 1000:1000 /data
```

### 网络问题

```bash
# 测试容器网络
docker exec objstor wget -O- http://localhost:8080/health

# 检查端口映射
docker port objstor

# 测试外部访问
curl http://$(docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' objstor):8080/health
```

### 性能调优

```bash
# 查看资源使用
docker stats objstor

# 限制资源使用
docker update \
  --memory="4g" \
  --cpus="2" \
  objstor
```

## 安全建议

1. **使用非root用户运行** - Dockerfile已配置
2. **限制容器权限** - 添加 `--cap-drop=ALL`
3. **只读根文件系统** - 添加 `--read-only`
4. **使用TLS加密** - 配置HTTPS
5. **定期更新镜像** - 及时获取安全补丁
6. **扫描镜像漏洞** - 使用 `docker scan`
7. **网络隔离** - 使用独立网络
8. **日志审计** - 启用访问日志

## 示例配置

### 最小化配置

```yaml
version: '3.8'
services:
  objstor:
    image: objstor:latest
    ports:
      - "8080:8080"
    volumes:
      - data:/app/data
```

### 完整生产配置

参见 `docker-compose.yml` 和 `docker-compose.storage.yml`

## 相关文档

- [配置指南](CONFIGURATION.md)
- [BUILD.md](../BUILD.md)
- [README.md](../README.md)
