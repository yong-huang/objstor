# ObjStor 项目结构

## 目录结构

```
objstor/
├── src/                      # 源代码
│   ├── main.rs              # 程序入口
│   ├── lib.rs               # 库导出
│   ├── api/                 # API 层
│   │   ├── s3/             # S3 协议实现
│   │   ├── admin.rs        # 管理 API
│   │   └── middleware.rs   # 中间件
│   ├── storage/             # 存储引擎
│   │   ├── pool.rs         # 存储池
│   │   ├── pool_manager.rs # 池管理器
│   │   └── multipart.rs    # 分片上传
│   ├── scheduler/           # 调度系统
│   │   ├── load_balancer.rs # 负载均衡
│   │   └── placement.rs     # 数据放置
│   ├── metadata/            # 元数据存储
│   ├── auth/               # 认证授权
│   ├── logging/            # 日志系统
│   ├── web/                # Web 界面
│   └── config/             # 配置管理
│
├── tests/                   # 测试
│   ├── integration_test.rs # 集成测试
│   ├── api_test.rs        # API 测试
│   └── cli_test.sh        # CLI 测试脚本
│
├── scripts/                 # 工具脚本
│   ├── benchmark.sh       # 性能测试
│   ├── configure.sh       # 快速配置
│   ├── docker-build.sh    # Docker 构建
│   ├── docker-deploy.sh   # Docker 部署
│   ├── docker-push.sh     # Docker 推送
│   ├── init_config.sh     # 配置向导
│   └── local-deploy.sh    # 本地部署
│
├── examples/               # 示例配置
│   ├── storage.example.json       # 存储配置示例
│   ├── prometheus.yml             # Prometheus 配置
│   ├── docker-compose-metrics.yml # 监控堆栈
│   ├── grafana-datasources.yml    # Grafana 数据源
│   └── alertmanager.yml           # 告警配置
│
├── docs/                   # 文档
│   ├── CONFIGURATION.md   # 配置说明
│   └── POOL_CONFIG_GUIDE.md # Pool 配置指南
│
├── data/                   # 运行时数据（gitignore）
│   ├── config/           # 配置文件
│   └── metadata.db       # 元数据数据库
│
├── logs/                   # 日志文件（gitignore）
│
├── Dockerfile             # Docker 构建文件
├── Dockerfile.local       # 本地构建 Dockerfile
├── docker-compose.yml     # Docker Compose 配置
├── docker-compose.dev.yml # 开发环境配置
│
├── README.md              # 项目主文档
├── BUILD.md               # 构建说明
├── CLAUDE.md              # Claude 项目指南
├── DOCKER_DEPLOY.md       # Docker 部署指南
├── LOCAL_DEPLOYMENT.md    # 本地部署指南
├── PROJECT_STRUCTURE.md   # 本文件
│
└── Cargo.toml             # Rust 项目配置
```

## 核心模块说明

### API 层 (src/api/)
- **S3 协议**: 完整实现 S3 API
- **认证**: AWS4-HMAC-SHA256 签名验证
- **管理 API**: 健康检查、指标、bucket 管理

### 存储层 (src/storage/)
- **存储池**: 多池管理，独立容量配置
- **对象存储**: 内容寻址存储（SHA256）
- **分片上传**: 支持大文件分片上传

### 调度系统 (src/scheduler/)
- **负载均衡**: LeastLoaded、WeightedRoundRobin、Adaptive
- **数据放置**: 智能选择存储池
- **指标收集**: 实时性能监控

### 元数据 (src/metadata/)
- **SQLite 存储**: Bucket、Object、用户信息
- **索引优化**: 快速查询
- **事务支持**: 数据一致性

### Web 界面 (src/web/)
- **Dashboard**: 实时监控和统计
- **Bucket 管理**: 创建、删除、浏览
- **WebSocket**: 实时日志和指标推送

## 部署方式

### 1. 本地部署
```bash
./scripts/local-deploy.sh
```
- 直接运行编译的二进制
- 无需 Docker
- 存储映射到指定目录

### 2. Docker 部署
```bash
./scripts/docker-deploy.sh
```
- 容器化部署
- 需要配置 Docker registry mirrors
- 支持数据卷持久化

## 配置文件

### 主配置文件
**位置**: `data/config/objstor.json`

**结构**:
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
        "path": "/path/to/storage/pool-001",
        "capacity": 107374182400,
        "max_objects": 1000000
      }
    ],
    "scheduler": {
      "strategy": "least_loaded",
      "rebalance_threshold": 0.2
    }
  }
}
```

### 存储配置示例
**位置**: `examples/storage.example.json`

包含多种存储配置场景：
- 单存储池
- 多存储池（SSD+HDD 混合）
- NFS 网络存储
- 多磁盘独立挂载

## 开发指南

### 构建项目
```bash
# 开发版本
cargo build

# 发布版本
cargo build --release

# 运行测试
cargo test

# 代码检查
cargo clippy
cargo fmt
```

### 添加新功能
1. **S3 API**: 在 `src/api/s3/` 添加处理器
2. **调度策略**: 在 `src/scheduler/load_balancer.rs` 添加算法
3. **存储池**: 更新 `src/storage/pool.rs`
4. **Web 页面**: 修改 `src/web/static/`

### 测试
```bash
# 单元测试
cargo test

# 集成测试
cargo test --test integration_test

# API 测试（需要运行服务）
cargo test --test api_test -- --ignored

# CLI 测试
./tests/cli_test.sh
```

## 文档说明

### 用户文档
- **README.md**: 项目概述、快速开始
- **BUILD.md**: 详细构建说明
- **DOCKER_DEPLOY.md**: Docker 部署完整指南
- **LOCAL_DEPLOYMENT.md**: 本地部署指南

### 开发文档
- **CLAUDE.md**: Claude AI 项目指南
- **docs/CONFIGURATION.md**: 配置参数详解
- **docs/POOL_CONFIG_GUIDE.md**: 存储池配置指南

### 示例配置
- **examples/**: 各种场景的配置示例
- **scripts/**: 自动化部署脚本

## 运行时目录

### 数据目录 (data/)
```
data/
├── config/
│   └── objstor.json        # 主配置文件
└── metadata.db              # 元数据数据库
```

### 日志目录 (logs/)
```
logs/
├── objstor.log              # 主日志文件
└── objstor.log.1            # 日志轮转文件
```

### 存储目录（外部）
```
/Users/hyhit/Desktop/workspace/storage/
└── pools/
    ├── pool-001/
    │   ├── objects/         # 对象数据
    │   │   └── [hash]/      # SHA256 哈希分片
    │   │       ├── data      # 实际数据
    │   │       └── meta.json # 元数据
    │   └── metadata/
    │       └── pool.json    # 池元数据
    └── pool-002/
        └── ...
```

## 性能优化

### 编译优化
```bash
# 使用发布配置
cargo build --release

# 启用 LTO（链接时优化）
RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo build --release
```

### 运行时优化
- 调整 `max_objects` 参数
- 选择合适的调度策略
- 使用多存储池分散负载
- 启用日志轮转避免日志文件过大

### 存储优化
- 使用 SSD 存储 pool-001（热数据）
- 使用 HDD 存储 pool-002（冷数据）
- 定期清理未使用的对象
- 监控存储使用率

## 监控和告警

### Prometheus 指标
- 请求速率
- 存储使用率
- 对象数量
- 错误率

### Grafana 仪表板
使用 `examples/grafana-dashboards.yml` 配置

### 告警规则
使用 `examples/alerts/objstor.yml` 配置

## 故障排查

### 常见问题
1. **端口占用**: 检查 8080 端口是否被占用
2. **权限问题**: 确保存储目录有写权限
3. **配置错误**: 验证 JSON 格式是否正确
4. **数据库锁定**: SQLite 使用 WAL 模式

### 日志级别
```bash
# 调试模式
RUST_LOG=debug ./target/release/objstor

# 详细日志
RUST_LOG=trace ./target/release/objstor
```

## 贡献指南

1. Fork 项目
2. 创建特性分支
3. 提交变更
4. 推送到分支
5. 创建 Pull Request

### 代码规范
- 使用 `cargo fmt` 格式化代码
- 通过 `cargo clippy` 检查
- 编写单元测试
- 更新相关文档

## 许可证

MIT License

## 联系方式

- GitHub Issues: 报告问题
- Discussions: 功能讨论
