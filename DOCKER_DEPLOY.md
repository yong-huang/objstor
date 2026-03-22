# ObjStor Docker 部署指南

## 快速部署

### 1. 使用自动化脚本（推荐）

```bash
# 运行部署脚本
./scripts/docker-deploy.sh
```

脚本会自动完成：
- ✓ 创建存储目录结构
- ✓ 构建Docker镜像
- ✓ 启动服务
- ✓ 健康检查

### 2. 手动部署

#### 步骤 1: 创建存储目录

```bash
# 在宿主机创建存储目录
mkdir -p /Users/hyhit/Desktop/workspace/storage/pools/pool-001/{objects,metadata}
mkdir -p /Users/hyhit/Desktop/workspace/storage/pools/pool-002/{objects,metadata}
```

#### 步骤 2: 构建镜像

```bash
docker build -t objstor:latest .
```

#### 步骤 3: 启动服务

```bash
docker compose up -d
```

## 目录映射

### 宿主机 → 容器内映射

| 宿主机路径 | 容器内路径 | 说明 |
|-----------|-----------|------|
| `/Users/hyhit/Desktop/workspace/storage` | `/app/storage` | 存储池数据 |
| `./data` | `/app/data` | 元数据和配置 |
| `./logs` | `/app/logs` | 日志文件 |

### 配置文件中的路径

配置文件 `data/config/objstor.json` 中的路径使用**容器内路径**：

```json
{
  "storage": {
    "pools": [
      {
        "id": "pool-001",
        "path": "./storage/pools/pool-001",  // 容器内路径
        "capacity": 107374182400,
        "max_objects": 1000000
      }
    ]
  }
}
```

## 服务访问

### Web界面和API

- **S3 API**: http://localhost:8080
- **Web UI**: http://localhost:8080/web
- **健康检查**: http://localhost:8080/health
- **Metrics**: http://localhost:8080/api/v1/metrics

### 默认凭证

```
Access Key ID: test-access-key
Secret Key: test-secret-key
```

## 常用命令

### 容器管理

```bash
# 查看日志
docker logs -f objstor

# 停止服务
docker compose down

# 重启服务
docker compose restart

# 进入容器
docker exec -it objstor sh

# 查看容器状态
docker ps
```

### 数据管理

```bash
# 查看存储目录内容
ls -la /Users/hyhit/Desktop/workspace/storage/

# 查看容器内存储映射
docker exec objstor ls -la /app/storage/

# 备份数据
tar -czf objstor-backup-$(date +%Y%m%d).tar.gz \
    /Users/hyhit/Desktop/workspace/storage/ \
    ./data/ \
    ./logs/
```

## S3客户端测试

### 使用AWS CLI

```bash
# 配置别名
alias s3='aws s3 --endpoint-url http://localhost:8080'

# 列出buckets
s3 ls

# 创建bucket
s3 mb s3://test-bucket

# 上传文件
echo "Hello ObjStor" > test.txt
s3 cp test.txt s3://test-bucket/

# 下载文件
s3 cp s3://test-bucket/test.txt downloaded.txt

# 删除文件
s3 rm s3://test-bucket/test.txt

# 删除bucket
s3 rb s3://test-bucket
```

### 使用rclone

```bash
# 配置rclone
rclone config create objstor s3 \
    --provider "Other" \
    --access-key-id "test-access-key" \
    --secret-access-key "test-secret-key" \
    --endpoint "http://localhost:8080" \
    --location "us-east-1"

# 列出buckets
rclone lsd objstor:

# 上传文件
rclone copy /path/to/files objstor:test-bucket/

# 下载文件
rclone copy objstor:test-bucket/ /path/to/download/
```

## 监控和调试

### 查看实时日志

```bash
# 查看所有日志
docker logs -f objstor

# 只看错误日志
docker logs -f objstor 2>&1 | grep ERROR

# 看最近100行
docker logs --tail 100 objstor
```

### 健康检查

```bash
# 使用curl
curl http://localhost:8080/health

# 使用wget
wget -qO- http://localhost:8080/health

# 查看metrics
curl http://localhost:8080/api/v1/metrics | jq
```

### 性能监控

使用带监控的docker-compose配置：

```bash
# 使用监控配置启动
docker compose -f docker-compose.dev.yml up -d

# 访问Prometheus
open http://localhost:9090

# 访问Grafana (admin/admin)
open http://localhost:3000
```

## 故障排查

### 问题1: 容器启动失败

```bash
# 查看详细日志
docker logs objstor

# 检查目录权限
ls -la /Users/hyhit/Desktop/workspace/storage/
ls -la ./data/

# 修复权限
chmod -R 755 /Users/hyhit/Desktop/workspace/storage/
chmod -R 755 ./data/
```

### 问题2: 无法访问服务

```bash
# 检查端口是否被占用
lsof -i :8080

# 检查容器状态
docker ps -a

# 检查网络
docker network ls
docker network inspect objstor_objstor_network
```

### 问题3: 数据无法写入

```bash
# 检查存储目录权限
docker exec objstor ls -la /app/storage/

# 检查磁盘空间
df -h /Users/hyhit/Desktop/workspace/storage/

# 重新创建目录结构
rm -rf /Users/hyhit/Desktop/workspace/storage/*
mkdir -p /Users/hyhit/Desktop/workspace/storage/pools/pool-001/{objects,metadata}
```

### 问题4: 配置文件错误

```bash
# 验证JSON格式
cat data/config/objstor.json | jq

# 重新生成默认配置
rm data/config/objstor.json
docker compose up -d
```

## 高级配置

### 修改存储路径

编辑 `docker-compose.yml`:

```yaml
volumes:
  # 修改为你的路径
  - /your/custom/path:/app/storage
```

编辑 `data/config/objstor.json`:

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

### 添加更多存储池

```bash
# 创建新池目录
mkdir -p /Users/hyhit/Desktop/workspace/storage/pools/pool-003/{objects,metadata}

# 更新配置文件
vim data/config/objstor.json

# 重启服务
docker compose restart
```

### 修改端口

编辑 `docker-compose.yml`:

```yaml
ports:
  - "9000:8080"  # 宿主机端口:容器端口
```

### 启用调试日志

编辑 `docker-compose.yml`:

```yaml
environment:
  - RUST_LOG=debug  # 改为debug级别
```

## 备份和恢复

### 备份

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="./backups"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p $BACKUP_DIR

# 停止服务（可选，确保数据一致性）
docker compose stop

# 创建备份
tar -czf $BACKUP_DIR/objstor-$DATE.tar.gz \
    /Users/hyhit/Desktop/workspace/storage/ \
    ./data/ \
    ./logs/

# 重启服务
docker compose start

echo "备份完成: $BACKUP_DIR/objstor-$DATE.tar.gz"
```

### 恢复

```bash
#!/bin/bash
# restore.sh

BACKUP_FILE=$1

if [ -z "$BACKUP_FILE" ]; then
    echo "用法: ./restore.sh <备份文件>"
    exit 1
fi

# 停止服务
docker compose down

# 解压备份
tar -xzf $BACKUP_FILE -C /

# 启动服务
docker compose up -d

echo "恢复完成"
```

## 升级

```bash
# 1. 备份数据
./backup.sh

# 2. 拉取新代码
git pull

# 3. 重新构建镜像
docker build -t objstor:latest .

# 4. 重启服务
docker compose up -d

# 5. 验证服务
curl http://localhost:8080/health
```

## 卸载

```bash
# 停止并删除容器
docker compose down

# 删除镜像
docker rmi objstor:latest

# 删除数据（谨慎！）
# rm -rf /Users/hyhit/Desktop/workspace/storage/
# rm -rf ./data/
# rm -rf ./logs/
```
