# ObjStor 配置指南

## 概述

ObjStor使用JSON配置文件来管理服务器和存储池设置。配置文件位于 `data/config/objstor.json`。

## 配置文件结构

```json
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
      }
    ],
    "scheduler": {
      "strategy": "least_loaded",
      "rebalance_threshold": 0.2
    }
  }
}
```

## 配置项说明

### Server 配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `host` | string | "0.0.0.0" | 服务器监听地址 |
| `port` | integer | 8080 | HTTP服务端口 |
| `s3_port` | integer | 8080 | S3 API端口 |
| `enable_tls` | boolean | false | 是否启用TLS |
| `tls_cert` | string | "" | TLS证书路径 |
| `tls_key` | string | "" | TLS私钥路径 |
| `max_request_size` | integer | 5368709120 | 最大请求大小（字节） |
| `log_level` | string | "info" | 日志级别：debug/info/warn/error |
| `log_dir` | string | "./logs" | 日志目录 |

### Storage 配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `data_dir` | string | "./data" | 数据根目录 |
| `pools` | array | [] | 存储池配置数组 |
| `scheduler.strategy` | string | "least_loaded" | 调度策略 |
| `scheduler.rebalance_threshold` | float | 0.2 | 重平衡阈值 |

### Pool 配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `id` | string | 必填 | 存储池唯一标识 |
| `path` | string | 必填 | 存储池路径（绝对或相对） |
| `capacity` | integer | 107374182400 | 容量（字节） |
| `max_objects` | integer | 1000000 | 最大对象数量 |
| `quota_enabled` | boolean | false | 是否启用配额限制 |

## 容量计算

```
1 KB  = 1024 bytes
1 MB  = 1024 KB  = 1,048,576 bytes
1 GB  = 1024 MB  = 1,073,741,824 bytes
1 TB  = 1024 GB  = 1,099,511,627,776 bytes
```

示例：
- 100 GB = 107,374,182,400 bytes
- 500 GB = 536,870,912,000 bytes
- 1 TB  = 1,099,511,627,776 bytes
- 2 TB  = 2,199,023,255,552 bytes

## 使用方法

### 1. 使用默认配置

首次运行时，如果配置文件不存在，会自动创建默认配置：

```bash
cargo run
```

### 2. 使用交互式配置向导

运行配置初始化脚本：

```bash
./scripts/init_config.sh
```

### 3. 手动编辑配置文件

编辑 `data/config/objstor.json`：

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 9000
  },
  "storage": {
    "pools": [
      {
        "id": "ssd-pool",
        "path": "/fast/ssd/objstor",
        "capacity": 536870912000,
        "max_objects": 5000000
      }
    ]
  }
}
```

## 配置示例

### 混合存储配置（SSD + HDD）

```json
{
  "storage": {
    "data_dir": "./data",
    "pools": [
      {
        "id": "hot-ssd",
        "path": "/fast/ssd/objstor",
        "capacity": 536870912000,
        "max_objects": 5000000,
        "quota_enabled": false
      },
      {
        "id": "cold-hdd",
        "path": "/slow/hdd/objstor",
        "capacity": 2199023255552,
        "max_objects": 50000000,
        "quota_enabled": true
      }
    ],
    "scheduler": {
      "strategy": "least_loaded",
      "rebalance_threshold": 0.2
    }
  }
}
```

### 多磁盘分布配置

```json
{
  "storage": {
    "data_dir": "./data",
    "pools": [
      {
        "id": "disk1",
        "path": "/mnt/disk1/objstor",
        "capacity": 1099511627776,
        "max_objects": 10000000,
        "quota_enabled": false
      },
      {
        "id": "disk2",
        "path": "/mnt/disk2/objstor",
        "capacity": 1099511627776,
        "max_objects": 10000000,
        "quota_enabled": false
      },
      {
        "id": "disk3",
        "path": "/mnt/disk3/objstor",
        "capacity": 1099511627776,
        "max_objects": 10000000,
        "quota_enabled": false
      }
    ],
    "scheduler": {
      "strategy": "least_loaded",
      "rebalance_threshold": 0.2
    }
  }
}
```

### 网络存储配置（NFS）

```json
{
  "storage": {
    "data_dir": "./data",
    "pools": [
      {
        "id": "local-ssd",
        "path": "./data/pools/pool-001",
        "capacity": 536870912000,
        "max_objects": 1000000,
        "quota_enabled": false
      },
      {
        "id": "nfs-backup",
        "path": "/mnt/nfs/objstor-backup",
        "capacity": 10995116277760,
        "max_objects": 100000000,
        "quota_enabled": true
      }
    ],
    "scheduler": {
      "strategy": "least_loaded",
      "rebalance_threshold": 0.2
    }
  }
}
```

## 调度策略

### least_loaded（默认）

选择使用率最低的健康存储池。

**优点**：
- 负载均衡效果好
- 自动适应不同容量的pool

**适用场景**：
- Pool容量相近
- 读写负载均衡

### weighted_round_robin

按可用空间比例加权轮询。

**优点**：
- 考虑pool容量差异
- 分配更均匀

**适用场景**：
- Pool容量差异大
- 需要精确控制分配比例

### adaptive

自适应调度，综合考虑负载、IOPS、网络带宽。

**优点**：
- 智能调度
- 性能最优

**适用场景**：
- 复杂存储环境
- 对性能要求高

## 故障排查

### 配置文件未找到

```
Error: Failed to load config: No such file or directory
```

**解决方案**：
1. 检查 `data/config/objstor.json` 是否存在
2. 运行 `./scripts/init_config.sh` 创建配置

### Pool路径权限错误

```
Error: Failed to initialize directories: Permission denied
```

**解决方案**：
1. 检查pool路径的读写权限
2. 确保运行用户有权限创建目录

### Pool容量不足

```
Error: Storage pool 'pool-001' is full
```

**解决方案**：
1. 增加pool的capacity配置
2. 添加新的pool
3. 清理旧对象

## 最佳实践

1. **规划容量**：预留20-30%的冗余空间
2. **使用独立磁盘**：每个pool使用独立的物理磁盘
3. **监控使用率**：定期检查pool使用情况
4. **定期备份**：重要数据配置多个pool
5. **测试配置**：在生产环境前先测试配置

## 配置验证

启动ObjStor时，会自动验证配置并显示：

```
INFO Loaded configuration from data/config/objstor.json
INFO Loaded 2 storage pools
INFO   Pool: pool-001 - Path: "./data/pools/pool-001", Capacity: 100 GB, Max Objects: 1000000
INFO   Pool: pool-002 - Path: "./data/pools/pool-002", Capacity: 100 GB, Max Objects: 1000000
```

## 相关文档

- [BUILD.md](../BUILD.md) - 构建和安装指南
- [README.md](../README.md) - 项目概述
