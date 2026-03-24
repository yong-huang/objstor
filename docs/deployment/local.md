# ObjStor 本地部署指南

## 部署状态

✅ **部署成功！** ObjStor 已成功部署并运行。

### 服务信息

- **运行方式**: 本地运行（非Docker）
- **S3 API**: http://localhost:8080
- **Web UI**: http://localhost:8080/web
- **健康检查**: http://localhost:8080/health
- **存储目录**: `/Users/hyhit/Desktop/workspace/storage`

### 存储配置

存储池已映射到指定目录：
```
/Users/hyhit/Desktop/workspace/storage/
└── pools/
    ├── pool-001/
    │   ├── objects/     # 对象数据
    │   └── metadata/    # 池元数据
    └── pool-002/
        ├── objects/
        └── metadata/
```

### 默认凭证

```
Access Key ID: test-access-key
Secret Key: test-secret-key
```

## 使用方法

### 启动服务

```bash
# 前台运行
./target/release/objstor

# 后台运行
nohup ./target/release/objstor > logs/objstor.log 2>&1 &

# 使用部署脚本
./scripts/local-deploy.sh
```

### 停止服务

```bash
# 查找进程
ps aux | grep objstor

# 停止服务
pkill -f "objstor"
```

### 查看日志

```bash
# 实时日志
tail -f logs/objstor.log

# 最近100行
tail -100 logs/objstor.log
```

## S3 客户端使用

### 使用 AWS CLI

```bash
# 列出所有 buckets
aws s3 ls --endpoint-url http://localhost:8080

# 创建 bucket
aws s3 mb s3://my-bucket --endpoint-url http://localhost:8080

# 上传文件
aws s3 cp file.txt s3://my-bucket/ --endpoint-url http://localhost:8080

# 下载文件
aws s3 cp s3://my-bucket/file.txt ./downloaded.txt --endpoint-url http://localhost:8080

# 列出 bucket 内容
aws s3 ls s3://my-bucket/ --endpoint-url http://localhost:8080

# 删除文件
aws s3 rm s3://my-bucket/file.txt --endpoint-url http://localhost:8080

# 删除 bucket
aws s3 rb s3://my-bucket --endpoint-url http://localhost:8080
```

### 测试验证

```bash
# 健康检查
curl http://localhost:8080/health

# 列出 buckets (XML格式)
curl http://localhost:8080/

# 查看指标
curl http://localhost:8080/api/v1/metrics | jq
```

## 配置文件

配置文件位置: `data/config/objstor.json`

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

## 数据验证

验证文件是否存储在映射目录：

```bash
# 查找存储的文件
find /Users/hyhit/Desktop/workspace/storage/pools/ -type f -name "data"

# 查看文件内容
cat /Users/hyhit/Desktop/workspace/storage/pools/pool-001/objects/*/data
```

## 故障排查

### 问题1: 端口已被占用

```bash
# 查看端口占用
lsof -i :8080

# 停止占用端口的进程
kill -9 <PID>
```

### 问题2: 服务无法启动

```bash
# 查看详细日志
cat logs/objstor.log

# 检查配置文件
cat data/config/objstor.json | jq
```

### 问题3: 无法连接到服务

```bash
# 检查服务是否运行
ps aux | grep objstor

# 测试端口
telnet localhost 8080
```

## 性能监控

```bash
# 查看进程资源使用
top -p $(pgrep objstor)

# 查看存储使用情况
du -sh /Users/hyhit/Desktop/workspace/storage/

# 查看对象数量
find /Users/hyhit/Desktop/workspace/storage/pools/ -type f -name "data" | wc -l
```

## 下一步

1. **配置认证**: 创建生产环境的 access keys
2. **设置监控**: 配置 Prometheus + Grafana
3. **备份策略**: 设置定期备份
4. **性能调优**: 根据负载调整配置

## 相关文档

- [Docker 部署指南](DOCKER_DEPLOY.md)
- [配置说明](docs/CONFIGURATION.md)
- [API 文档](docs/API.md)
