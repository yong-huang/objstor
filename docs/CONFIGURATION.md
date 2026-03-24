# ObjStor Configuration Guide

## Overview

ObjStor uses JSON configuration files to manage server and storage pool settings. The configuration file is located at `data/config/objstor.json`.

## Configuration File Structure

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

## Configuration Parameters

### Server Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | string | "0.0.0.0" | Server listening address |
| `port` | integer | 8080 | HTTP service port |
| `s3_port` | integer | 8080 | S3 API port |
| `enable_tls` | boolean | false | Enable TLS |
| `tls_cert` | string | "" | TLS certificate path |
| `tls_key` | string | "" | TLS private key path |
| `max_request_size` | integer | 5368709120 | Maximum request size (bytes) |
| `log_level` | string | "info" | Log level: debug/info/warn/error |
| `log_dir` | string | "./logs" | Log directory |

### Storage Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data_dir` | string | "./data" | Data root directory |
| `pools` | array | [] | Storage pool configuration array |
| `scheduler.strategy` | string | "least_loaded" | Scheduler strategy |
| `scheduler.rebalance_threshold` | float | 0.2 | Rebalance threshold |

### Pool Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | string | Required | Unique pool identifier |
| `path` | string | Required | Pool path (absolute or relative) |
| `capacity` | integer | 107374182400 | Capacity (bytes) |
| `max_objects` | integer | 1000000 | Maximum object count |
| `quota_enabled` | boolean | false | Enable quota limit |

## Capacity Calculation

```
1 KB  = 1024 bytes
1 MB  = 1024 KB  = 1,048,576 bytes
1 GB  = 1024 MB  = 1,073,741,824 bytes
1 TB  = 1024 GB  = 1,099,511,627,776 bytes
```

Examples:
- 100 GB = 107,374,182,400 bytes
- 500 GB = 536,870,912,000 bytes
- 1 TB  = 1,099,511,627,776 bytes
- 2 TB  = 2,199,023,255,552 bytes

## Usage

### 1. Use Default Configuration

On first run, if the configuration file doesn't exist, a default configuration will be automatically created:

```bash
cargo run
```

### 2. Use Interactive Configuration Wizard

Run the configuration initialization script:

```bash
./scripts/init_config.sh
```

### 3. Manually Edit Configuration File

Edit `data/config/objstor.json`:

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

## Configuration Examples

### Hybrid Storage (SSD + HDD)

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

### Multi-Disk Distribution

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

### Network Storage (NFS)

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

## Scheduler Strategies

### least_loaded (Default)

Selects the healthy storage pool with the lowest usage rate.

**Advantages**:
- Good load balancing effect
- Automatically adapts to pools of different capacities

**Use Cases**:
- Similar pool capacities
- Balanced read/write load

### weighted_round_robin

Weighted round-robin based on available space ratio.

**Advantages**:
- Considers pool capacity differences
- More uniform distribution

**Use Cases**:
- Large differences in pool capacities
- Need precise control over distribution ratio

### adaptive

Adaptive scheduling considering load, IOPS, and network bandwidth.

**Advantages**:
- Intelligent scheduling
- Optimal performance

**Use Cases**:
- Complex storage environments
- High performance requirements

## Troubleshooting

### Configuration File Not Found

```
Error: Failed to load config: No such file or directory
```

**Solutions**:
1. Check if `data/config/objstor.json` exists
2. Run `./scripts/init_config.sh` to create configuration

### Pool Path Permission Error

```
Error: Failed to initialize directories: Permission denied
```

**Solutions**:
1. Check read/write permissions for pool paths
2. Ensure the running user has permission to create directories

### Pool Capacity Insufficient

```
Error: Storage pool 'pool-001' is full
```

**Solutions**:
1. Increase pool capacity configuration
2. Add new pools
3. Clean up old objects

## Best Practices

1. **Capacity Planning**: Reserve 20-30% redundant space
2. **Use Independent Disks**: Each pool should use an independent physical disk
3. **Monitor Usage**: Regularly check pool usage
4. **Regular Backups**: Configure multiple pools for important data
5. **Test Configuration**: Test configuration before production deployment

## Configuration Validation

When ObjStor starts, it automatically validates the configuration and displays:

```
INFO Loaded configuration from data/config/objstor.json
INFO Loaded 2 storage pools
INFO   Pool: pool-001 - Path: "./data/pools/pool-001", Capacity: 100 GB, Max Objects: 1000000
INFO   Pool: pool-002 - Path: "./data/pools/pool-002", Capacity: 100 GB, Max Objects: 1000000
```

## Related Documentation

- [BUILD.md](deployment/build.md) - Build and installation guide
- [README.md](../README.md) - Project overview
