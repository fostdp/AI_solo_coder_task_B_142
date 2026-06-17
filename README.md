# 古代司南磁石指向精度仿真与地磁场重建系统

> 基于 Actor 架构的全栈系统，支持汉代司南磁石指向精度仿真、地磁场重建、实时告警推送

---

## 目录

1. [系统架构](#系统架构)
2. [技术栈](#技术栈)
3. [核心功能](#核心功能)
4. [目录结构](#目录结构)
5. [快速开始 - Docker Compose](#快速开始---docker-compose)
6. [传感器模拟器用法](#传感器模拟器用法)
7. [API 接口](#api-接口)
8. [Prometheus 指标](#prometheus-指标)
9. [ClickHouse 降采样与保留策略](#clickhouse-降采样与保留策略)
10. [配置说明](#配置说明)

---

## 系统架构

### Actor 架构图

```
                                    ┌───────────────────────────────────────────────────────────────────┐
                                    │                      Prometheus 监控面板                          │
                                    │  (Grafana + Prometheus + Alertmanager)                           │
                                    └───────────────────────────────────────────────────────────────────┘
                                                                 │
                                                                 ▼
┌────────────────────┐      ┌───────────────────────────────────────────────────────────────────────────┐
│  传感器模拟器       │      │                         Rust 后端服务 (Actor 架构)                         │
│  (Python)          │      │                                                                           │
│                    │      │  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐            │
│  • 磁石材料可选    │──────▶│  │ dtu_receiver │────▶│ magnetic_sim │────▶│ alarm_mqtt   │─────▶ MQTT  │
│  • 地磁场时期可选  │  HTTP│  │  (数据采集)   │ mpsc│  (微磁仿真)   │ mpsc│  (告警推送)   │       Broker │
│  • 故障模式模拟    │  SSE │  └──────────────┘     └──────────────┘     └──────────────┘            │
│  • 多设备并发      │      │         │                   │                   │                          │
└────────────────────┘      │         ▼                   ▼                   ▼                          │
                            │  ┌──────────────────────────────────────────────────────────────┐        │
                            │  │ geomagnetic_reconstructor                                  │        │
                            │  │ (地磁场重建 / 时空插值 / 矢量场生成)                        │        │
                            │  └──────────────────────────────────────────────────────────────┘        │
                            │                                                                           │
                            └───────────────────────────────────────────────────────────────────────────┘
                                                         │
                                                         ▼
                                    ┌───────────────────────────────────────────────────────────┐
                                    │                   ClickHouse 时序数据库                      │
                                    │                                                                 │
                                    │  [原始数据] → [1分钟降采样] → [1小时降采样] → [1天降采样]    │
                                    │  TTL: 3月       7天          30天          10年               │
                                    └───────────────────────────────────────────────────────────┘
                                                         │
                                                         ▼
                                    ┌───────────────────────────────────────────────────────────┐
                                    │                    前端可视化 (Nginx + Gzip)                  │
                                    │                                                                 │
                                    │  • sinan_3d.js     - 3D 司南模型渲染                          │
                                    │  • magnetic_panel.js - 地磁场面板/矢量场/仿真                │
                                    └───────────────────────────────────────────────────────────┘
```

### 模块间通信

```
         Tokio mpsc Channel (buffer=256)
┌─────────────┐      ┌──────────────────┐      ┌──────────────┐
│ dtu_receiver │─────▶│ magnetic_simulator │─────▶│ alarm_mqtt   │
└─────────────┘      └──────────────────┘      └──────────────┘
       │                       │                          │
       └───────────┬───────────┘                          │
                   ▼                                      ▼
         ┌─────────────────────┐                  ┌──────────────┐
         │ geomagnetic_recon-  │                  │  MQTT Broker │
         │ structor            │                  │  (Mosquitto)  │
         └─────────────────────┘                  └──────────────┘
                   ▲
                   │  Tokio broadcast Channel (SSE)
                   ▼
         ┌─────────────────────┐
         │    Frontend SSE     │
         └─────────────────────┘
```

---

## 技术栈

### 后端
| 组件 | 选型 | 说明 |
|------|------|------|
| **语言** | Rust 1.80+ | 内存安全、高性能 |
| **Web 框架** | axum 0.7 | 异步、模块化 |
| **异步运行时** | tokio 1.38 | 多线程、高性能 |
| **数据库** | ClickHouse 24.3 | 时序数据、列式存储 |
| **MQTT 客户端** | rumqttc 0.24 | 异步 MQTT |
| **指标采集** | metrics + Prometheus | 标准指标接口 |
| **日志** | tracing | 结构化日志 |
| **序列化** | serde + serde_json | 高效序列化 |
| **线性代数** | nalgebra 0.32 | 数值计算 |

### 前端
| 组件 | 选型 | 说明 |
|------|------|------|
| **3D 渲染** | Three.js r160 | WebGL 3D 引擎 |
| **图表** | Chart.js 4.x | 数据可视化 |
| **Web 服务器** | Nginx 1.27 | 静态资源 + Gzip 压缩 |
| **样式** | 原生 CSS3 | 无框架依赖 |

### 基础设施
| 组件 | 选型 | 说明 |
|------|------|------|
| **容器编排** | Docker Compose v3.8 | 一键部署 |
| **MQTT Broker** | Eclipse Mosquitto 2.0 | 开源 MQTT 代理 |
| **监控** | Prometheus + Grafana | 指标采集与可视化 |
| **容器监控** | cAdvisor + Node Exporter | 主机/容器监控 |

---

## 核心功能

### 1. 四 Actor 模块

#### dtu_receiver (数据采集器)
- 接收传感器数据（HTTP API）
- 数据校验与清洗
- 缓存最新数据
- 广播到 SSE 流
- 转发到仿真模块

#### magnetic_simulator (微磁学仿真器)
- 实时读取地磁场矢量
- 朗之万函数平衡磁化计算
- 斯托纳-沃尔法斯单畴模型
- 退磁效应修正
- 指向偏差计算
- 转发到告警模块

#### geomagnetic_reconstructor (地磁场重建器)
- CALS10K 球谐展开（10阶）
- 东亚考古地磁克里金插值
- 时空插值（年份/经纬度）
- 矢量场生成
- 长期变计算

#### alarm_mqtt (告警评估推送)
- 偏差阈值评估（5°/10°）
- 告警生成与存储
- MQTT 异步推送
- 告警确认管理
- 60秒定时推送检查

### 2. 前端模块

#### sinan_3d.js - 3D 司南模型
- Three.js 真实感渲染
- 实时方位角平滑动画
- 磁场线可视化
- 偏差告警动效
- 交互式视角控制

#### magnetic_panel.js - 地磁场面板
- 单点地磁场计算
- 矢量场可视化（RK1 流线 + Douglas-Peucker 简化）
- 指向精度仿真
- 设备列表管理
- 告警列表展示

---

## 目录结构

```
AI_solo_coder_task_A_142/
├── backend/                          # Rust 后端
│   ├── src/
│   │   ├── main.rs                  # 主程序入口 (Actor 组装)
│   │   ├── channels.rs              # 消息通道定义 (mpsc/broadcast/oneshot)
│   │   ├── metrics.rs               # Prometheus 指标
│   │   ├── config.rs                # 配置加载
│   │   ├── errors.rs                # 错误处理
│   │   ├── models.rs                # 数据模型
│   │   ├── database.rs              # 数据库操作
│   │   ├── handlers.rs              # API 处理器
│   │   ├── dtu_receiver/            # DTU 数据采集 Actor
│   │   │   ├── mod.rs
│   │   │   └── validation.rs        # 数据校验
│   │   ├── magnetic_simulator/      # 微磁学仿真 Actor
│   │   │   └── mod.rs
│   │   ├── geomagnetic_reconstructor/  # 地磁场重建 Actor
│   │   │   └── mod.rs
│   │   ├── alarm_mqtt/              # 告警 MQTT Actor
│   │   │   └── mod.rs
│   │   ├── micromagnetic_simulation.rs  # 微磁学核心算法
│   │   ├── cals10k_model.rs         # CALS10K 地磁场模型
│   │   ├── alert_service.rs         # 告警服务
│   │   └── mqtt_service.rs          # MQTT 服务
│   ├── config/
│   │   ├── magnetic_params.json     # 磁学参数外置配置
│   │   └── geomagnetic_data.json    # 地磁数据外置配置
│   ├── Dockerfile                   # 多阶段静态编译
│   ├── Cargo.toml
│   └── .env.example
├── frontend/                         # 前端应用
│   ├── index.html
│   ├── css/
│   │   └── style.css
│   ├── js/
│   │   ├── sinan_3d.js             # 3D 司南模型（拆分后）
│   │   ├── magnetic_panel.js       # 地磁场面板（拆分后）
│   │   ├── config.js
│   │   ├── data.js
│   │   ├── vectorfield.js
│   │   ├── charts.js
│   │   └── main.js
│   ├── Dockerfile                   # Nginx + Gzip
│   ├── nginx.conf                   # Gzip 压缩配置
│   └── conf.d/
│       └── default.conf             # 站点配置
├── scripts/                          # 工具脚本
│   ├── sensor_simulator_enhanced.py # 增强版传感器模拟器
│   ├── clickhouse_init.sql          # 数据库初始化 (含降采样)
│   ├── Dockerfile                   # 模拟器 Dockerfile
│   └── requirements.txt
├── deploy/                           # 部署配置
│   ├── clickhouse/
│   │   └── config.xml               # ClickHouse 冷热存储配置
│   ├── mosquitto/
│   │   └── mosquitto.conf           # MQTT Broker 配置
│   ├── prometheus/
│   │   ├── prometheus.yml           # Prometheus 抓取配置
│   │   └── rules/
│   │       └── sinan_alerts.yml     # 告警规则
│   └── grafana/
│       ├── provisioning/            # 数据源自动配置
│       └── dashboards/              # 预置仪表盘
├── docker-compose.yml                # 一键编排
├── .env                              # 环境变量
└── README.md                         # 本文档
```

---

## 快速开始 - Docker Compose

### 前置要求
- Docker ≥ 24.0
- Docker Compose ≥ 2.20
- 至少 4GB 可用内存
- 至少 10GB 可用磁盘空间

### 1. 最小化部署（核心服务）

```bash
# 1. 克隆项目
git clone <repository-url>
cd AI_solo_coder_task_A_142

# 2. 复制环境变量文件（可选修改）
cp .env.example .env

# 3. 启动核心服务
docker compose up -d

# 4. 查看服务状态
docker compose ps

# 5. 查看日志
docker compose logs -f backend
docker compose logs -f clickhouse
```

**访问地址**:
- 前端界面: http://localhost
- 后端 API: http://localhost:8080
- API 文档: http://localhost:8080/health

### 2. 完整部署（含监控）

```bash
# 启动所有服务（含监控面板）
docker compose --profile monitoring up -d

# 访问监控
# Prometheus: http://localhost:9090
# Grafana:    http://localhost:3000 (admin/admin123)
# cAdvisor:   http://localhost:8081
```

### 3. 启动传感器模拟器

```bash
# 启动单个模拟器（汉代司南）
docker compose --profile simulator up -d sensor-simulator

# 查看模拟器日志
docker compose logs -f sensor-simulator
```

### 4. 常用操作

```bash
# 停止所有服务
docker compose down

# 停止含监控的服务
docker compose --profile monitoring --profile simulator down

# 重启后端服务
docker compose restart backend

# 清理数据（谨慎！）
docker compose down -v

# 查看资源使用
docker stats
```

### 5. 健康检查

```bash
# 检查后端
curl http://localhost:8080/health

# 检查 ClickHouse
curl http://localhost:8123/ping

# 检查 Prometheus 指标
curl http://localhost:8080/metrics

# 检查前端
curl -I http://localhost
```

---

## 传感器模拟器用法

### 增强版模拟器特性

```
📦 6种磁石材料可选
   ├─ magnetite_natural  天然磁铁矿 (汉代标准)
   ├─ magnetite_pure     高纯磁铁矿
   ├─ alnico             铝镍钴合金
   ├─ neodymium          钕铁硼 (现代强磁)
   ├─ ferrite            铁氧体
   └─ smco               钐钴合金 (高温稳定)

🌍 7种地磁场时期
   ├─ modern             现代 (2000年)
   ├─ han_dynasty        汉代 (-100年)
   ├─ tang_dynasty       唐代 (700年)
   ├─ song_dynasty       宋代 (1100年)
   ├─ laschamp_event     Laschamp地磁漂移 (4万年前)
   ├─ low_intensity      低磁场 (30%现代)
   └─ high_intensity     强磁场 (200%现代)

⚠️  7种故障模式
   ├─ none               正常运行
   ├─ noise_high         高噪声
   ├─ demagnetization    磁石退磁
   ├─ sensor_drift       传感器漂移
   ├─ interference_magnetic  强磁干扰
   ├─ temperature_extreme    极端温度
   └─ spikes             数据尖峰
```

### 命令行用法

```bash
cd scripts
pip install -r requirements.txt

# 查看帮助
python sensor_simulator_enhanced.py --help

# 列出所有可用配置
python sensor_simulator_enhanced.py --list-materials
python sensor_simulator_enhanced.py --list-periods
python sensor_simulator_enhanced.py --list-failures

# ========== 基础用法 ==========

# 汉代司南（天然磁铁矿 + 汉代地磁场）
python sensor_simulator_enhanced.py \
  --device-id HAN-001 \
  --device-name "汉代司南·长安" \
  --material magnetite_natural \
  --period han_dynasty \
  --lat 34.265 --lon 108.955 \
  --interval 1000

# 现代钕铁硼对比测试
python sensor_simulator_enhanced.py \
  --device-id MODERN-001 \
  --device-name "现代司南·钕铁硼" \
  --material neodymium \
  --period modern \
  --custom-moment 0.5 \
  --custom-remanence 1.2 \
  --interval 500

# ========== 故障模拟 ==========

# 退磁效应模拟
python sensor_simulator_enhanced.py \
  --device-id DEMAG-001 \
  --material magnetite_pure \
  --failure demagnetization \
  --interval 500 \
  --duration 3600  # 运行1小时

# 低磁场环境测试
python sensor_simulator_enhanced.py \
  --device-id LOWFIELD-001 \
  --material ferrite \
  --period low_intensity \
  --alert-threshold 8.0

# 多故障模式叠加
python sensor_simulator_enhanced.py \
  --device-id STRESS-001 \
  --material alnico \
  --failure noise_high \
  --period low_intensity \
  --noise 0.5

# ========== 多设备对比 ==========

# 多设备模式（4台设备同时上报，材料对比测试）
python sensor_simulator_enhanced.py --multi

# 自定义多设备配置
python sensor_simulator_enhanced.py --config-file my_devices.json

# ========== 上报方式 ==========

# 仅 API 上报
python sensor_simulator_enhanced.py --use-api true

# 仅 MQTT 上报
python sensor_simulator_enhanced.py \
  --use-mqtt \
  --mqtt-broker localhost \
  --mqtt-topic sinan/sensor

# 同时 API + MQTT 上报
python sensor_simulator_enhanced.py \
  --use-api true \
  --use-mqtt \
  --mqtt-broker localhost
```

### 配置文件示例 (my_devices.json)

```json
{
  "common": {
    "api_base_url": "http://localhost:8080",
    "mqtt_broker": "localhost",
    "mqtt_port": 1883,
    "use_mqtt": false,
    "alert_threshold": 5.0,
    "duration": null
  },
  "devices": [
    {
      "device_id": "COMPARE-001",
      "device_name": "天然磁铁矿",
      "material": "magnetite_natural",
      "geomagnetic_period": "han_dynasty",
      "lat": 34.265,
      "lon": 108.955,
      "interval_ms": 1000
    },
    {
      "device_id": "COMPARE-002",
      "device_name": "高纯磁铁矿",
      "material": "magnetite_pure",
      "geomagnetic_period": "han_dynasty",
      "lat": 34.265,
      "lon": 108.955,
      "interval_ms": 1000
    },
    {
      "device_id": "COMPARE-003",
      "device_name": "钕铁硼",
      "material": "neodymium",
      "geomagnetic_period": "modern",
      "lat": 34.265,
      "lon": 108.955,
      "interval_ms": 1000,
      "custom_moment": 0.3,
      "custom_remanence": 1.0
    }
  ]
}
```

---

## API 接口

### 传感器数据
| 方法 | 路径 | 说明 |
|------|------|------|
| `POST` | `/api/v1/sensor` | 上报传感器数据 |
| `GET` | `/api/v1/sensor/data` | 查询传感器数据 |
| `GET` | `/api/v1/sensor/latest` | 获取最新数据 |
| `GET` | `/api/v1/sensor/stream` | SSE 实时数据流 |

### 地磁场
| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/api/v1/geomagnetic/field` | 计算单点地磁场 |
| `POST` | `/api/v1/geomagnetic/vectorfield` | 生成矢量场 |
| `GET` | `/api/v1/geomagnetic/secular` | 获取长期变数据 |

### 仿真
| 方法 | 路径 | 说明 |
|------|------|------|
| `POST` | `/api/v1/simulation/pointing` | 运行指向仿真 |
| `GET` | `/api/v1/simulation/results` | 查询仿真结果 |

### 告警
| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/api/v1/alerts/active` | 获取活动告警 |
| `POST` | `/api/v1/alerts/acknowledge` | 确认告警 |

### 系统
| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/api/v1/devices` | 获取设备列表 |
| `GET` | `/api/v1/devices/status` | 获取设备状态 |
| `GET` | `/api/v1/statistics` | 获取统计数据 |
| `GET` | `/health` | 健康检查 |
| `GET` | `/metrics` | Prometheus 指标 |

### 示例请求

```bash
# 上报传感器数据
curl -X POST http://localhost:8080/api/v1/sensor \
  -H "Content-Type: application/json" \
  -d '{
    "device_id": "TEST-001",
    "magnetic_moment_x": 0.1,
    "magnetic_moment_y": 0.05,
    "magnetic_moment_z": 0.02,
    "magnetic_moment_magnitude": 0.113,
    "remanence": 0.05,
    "pointing_deviation": 3.5,
    "environment_temp": 25.0,
    "location_lat": 34.0,
    "location_lon": 108.9,
    "is_alert": false
  }'

# 计算地磁场
curl "http://localhost:8080/api/v1/geomagnetic/field?lat=34.0&lon=108.9&year=-100.0"

# 获取 Prometheus 指标
curl http://localhost:8080/metrics
```

---

## Prometheus 指标

### 核心指标

| 指标名称 | 类型 | 说明 |
|----------|------|------|
| `dtu_sensor_data_received_total` | Counter | 接收传感器数据包总数 |
| `dtu_sensor_data_valid_total` | Counter | 有效传感器数据包数 |
| `dtu_sensor_data_invalid_total` | Counter | 无效传感器数据包数 |
| `simulation_run_total` | Counter | 指向仿真运行次数 |
| `geomagnetic_calculation_total` | Counter | 地磁场计算次数 |
| `alerts_generated_total` | Counter | 生成告警总数 |
| `alerts_mqtt_published_total` | Counter | MQTT 推送成功数 |
| `alerts_mqtt_failed_total` | Counter | MQTT 推送失败数 |
| `clickhouse_insert_total` | Counter | 数据库插入成功数 |
| `clickhouse_insert_failed_total` | Counter | 数据库插入失败数 |
| `http_requests_total` | Counter | HTTP 请求总数 (labels: method, endpoint, status) |
| `dtu_connected_devices` | Gauge | 在线设备数 |
| `alerts_active` | Gauge | 活跃告警数 |
| `simulation_pointing_accuracy_degrees` | Gauge | 当前指向精度 (度) |
| `dtu_processing_latency_seconds` | Histogram | DTU 处理延迟 |
| `simulation_latency_seconds` | Histogram | 仿真计算延迟 |
| `geomagnetic_calculation_latency_seconds` | Histogram | 地磁场计算延迟 |
| `http_request_latency_seconds` | Histogram | HTTP 请求延迟 (labels: method, endpoint) |

### 告警规则

| 告警名称 | 触发条件 | 级别 |
|----------|----------|------|
| SinanBackendDown | 后端服务宕机 > 1m | Critical |
| HighHttpErrorRate | HTTP 5xx 错误率 > 5% | Warning |
| HighHttpRequestLatency | P95 延迟 > 2s | Warning |
| HighPointingDeviation | 指向偏差 > 10° > 5m | Critical |
| LowMagneticRemanence | 10分钟无数据 | Warning |
| HighSensorDataInvalidRate | 无效率 > 10% | Warning |
| ClickHouseDown | 数据库宕机 > 2m | Critical |
| TooManyActiveAlerts | 活跃告警 > 10 个 | Warning |

---

## ClickHouse 降采样与保留策略

### 数据分级存储

| 数据粒度 | 表名 | 保留时间 | 聚合指标 |
|----------|------|----------|----------|
| 原始数据 | `sinan_sensor_data` | 7天热 + 1个月冷 + 3个月删除 | 所有字段 |
| 1分钟 | `sinan_sensor_data_1min` | 7天 | avg/min/max/p95 |
| 1小时 | `sinan_sensor_data_1hour` | 30天 | avg/min/max/p95 |
| 1天 | `sinan_sensor_data_1day` | 10年 | avg/min/max/p95 + 温度范围 |

### 降采样链路

```
原始数据 (每1s)
    │
    ▼ 物化视图 mv_sensor_1min
1分钟汇总 (count, avg, min, max, p95)
    │
    ▼ 物化视图 mv_sensor_1hour
1小时汇总 (sum count, avg avg, min min, max max)
    │
    ▼ 物化视图 mv_sensor_1day
1天汇总 (sum count, avg avg, min min, max max)
    │
    ▼ 保留 10 年
```

### TTL 策略

```sql
-- 原始数据 TTL
TTL
    timestamp + INTERVAL 7 DAY TO VOLUME 'hot',
    timestamp + INTERVAL 1 MONTH TO VOLUME 'cold',
    timestamp + INTERVAL 3 MONTH DELETE

-- 其他表 TTL
sinan_sensor_data_1min:   7 天
sinan_sensor_data_1hour:  30 天
sinan_sensor_data_1day:   10 年
alert_events:             2 年
pointing_simulation_results: 5 年
geomagnetic_field_data:   5 年
```

### 冷热存储配置

```xml
<storage_configuration>
    <disks>
        <hot><path>/var/lib/clickhouse/hot/</path></hot>
        <cold><path>/var/lib/clickhouse/cold/</path></cold>
    </disks>
    <policies>
        <default>
            <volumes>
                <hot><disk>hot</disk></hot>
                <cold><disk>cold</disk></cold>
            </volumes>
        </default>
    </policies>
</storage_configuration>
```

---

## 配置说明

### 后端环境变量 (.env)

```env
# 服务配置
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# ClickHouse
CLICKHOUSE_HOST=clickhouse
CLICKHOUSE_PORT=8123
CLICKHOUSE_USER=default
CLICKHOUSE_PASSWORD=
CLICKHOUSE_DATABASE=sinan_db

# MQTT
MQTT_HOST=mosquitto
MQTT_PORT=1883
MQTT_CLIENT_ID=sinan-backend
MQTT_TOPIC=sinan/alerts
MQTT_USERNAME=
MQTT_PASSWORD=

# 告警阈值
POINTING_DEVIATION_THRESHOLD=5.0
CRITICAL_DEVIATION_THRESHOLD=10.0

# 监控
PROMETHEUS_PORT=9090
GRAFANA_PORT=3000
GRAFANA_ADMIN_PASSWORD=admin123

# 模拟器
SIMULATOR_BACKEND_URL=http://backend:8080
SIMULATOR_MQTT_URL=mqtt://mosquitto:1883
SIMULATOR_DEVICE_ID=sim-sinan-001
SIMULATOR_SEND_INTERVAL_MS=1000
```

### 外置 JSON 配置

#### 磁学参数 (backend/config/magnetic_params.json)

```json
{
  "spoon_dimensions": {
    "length_cm": 17.8,
    "bowl_diameter_cm": 5.0,
    "handle_diameter_cm": 1.0
  },
  "material_properties": {
    "magnetite": {
      "remanence": 0.06,
      "coercivity_kA_m": 15.0,
      "density_g_cm3": 5.18
    }
  },
  "simulation_params": {
    "friction_coefficient": 0.05,
    "temperature_celsius": 25.0
  }
}
```

#### 地磁数据 (backend/config/geomagnetic_data.json)

```json
{
  "archaeomagnetic_sites": [
    {
      "site_name": "汉长安城遗址",
      "lat": 34.265,
      "lon": 108.955,
      "sample_age": -100.0,
      "declination": -2.5,
      "inclination": 56.2,
      "intensity_nT": 55000.0
    }
  ],
  "kriging_params": {
    "nugget": 0.05,
    "sill": 1.0,
    "range_km": 800
  }
}
```

### Nginx Gzip 配置

```nginx
gzip on;
gzip_comp_level 6;
gzip_min_length 1024;
gzip_types
    text/plain
    text/css
    text/javascript
    application/javascript
    application/json
    application/xml
    image/svg+xml
    application/wasm;
```

---

## 部署架构

```
                                         ┌─────────────────────────────────────────┐
                                         │               Docker Host               │
                                         │                                         │
┌─────────┐  :80   ┌──────────┐ :8080 ┌──────────┐  mpsc  ┌──────────┐ :9000 ┌──────────┐
│ Browser │──────▶│  Nginx   │──────▶│  Rust    │────────▶│ ClickHouse│       │ Mosquitto│
└─────────┘       │ (Frontend)│       │ (Backend)│        └──────────┘       └──────────┘
                  └──────────┘       └──────────┘              ▲                   ▲
                       │                │  :8080/metrics        │                   │
                       │                ▼                        │                   │
                       │         ┌──────────┐  :9090    ┌──────┴──────┐   :9234  ┌──────┴──────┐
                       │         │ Prometheus│◀─────────│ cAdvisor    │         │  Mosquitto  │
                       │         └──────────┘           └─────────────┘         │  Exporter   │
                       │                │ :3000                               └─────────────┘
                       │                ▼
                       │         ┌──────────┐
                       └────────▶│  Grafana  │
                                 └──────────┘

                                       ┌──────────────┐
                                       │  Simulator   │─────▶  HTTP API
                                       │ (Python)     │─────▶  MQTT
                                       └──────────────┘
```

---

## 常见问题

### Q: 如何修改告警阈值？
A: 修改 `.env` 文件中的 `POINTING_DEVIATION_THRESHOLD` 和 `CRITICAL_DEVIATION_THRESHOLD`，然后重启服务。

### Q: 如何添加新的磁石材料？
A: 编辑 `scripts/sensor_simulator_enhanced.py` 中的 `MAGNET_MATERIALS` 字典。

### Q: ClickHouse 数据满了怎么办？
A: 数据会自动按 TTL 策略清理。可通过修改 `clickhouse_init.sql` 中的 TTL 配置调整保留时间。

### Q: 如何接入真实的司南设备？
A: 设备通过 HTTP POST 到 `/api/v1/sensor` 即可，格式参考 API 文档。

### Q: Prometheus 告警如何通知？
A: 配置 Alertmanager 的 Webhook 或邮件通知，参考 Prometheus 文档。

---

## 参考文献

1. Constable, C. G., & Johnson, C. L. (2005). CALS7K.2: A continuous geomagnetic field model for the past 7000 years.
2. Stoner, E. C., & Wohlfarth, E. P. (1948). A mechanism of magnetic hysteresis in heterogeneous alloys.
3. 中国古代磁石指向仪器研究 - 自然科学史研究所

---

## License

MIT License
