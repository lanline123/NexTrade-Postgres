# 多商户电商平台后端

这是一个面向 B2B2C 场景的多商户电商平台后端服务，提供商城前台 API 与管理后台 API 两套接口，覆盖商品、订单、用户、支付、营销、内容、商户、门店配送等核心业务。

服务使用 Rust 编写，基于 Axum + SQLx + MySQL + Redis，支持 JWT 认证、异步任务、文件存储与 Elasticsearch 搜索扩展。

## 功能概览

- **商城前台**
  - 首页装修、分类导航、商品列表与详情
  - 搜索、购物车、下单、支付、订单查询
  - 会员中心、优惠券、收藏、浏览记录、售后
  - 文章、公告、帮助中心、活动专题
- **管理后台**
  - 商品、分类、品牌、规格、库存
  - 订单、发货、退款、售后
  - 用户、会员等级、权限、角色
  - 营销活动、优惠券、秒杀、拼团、积分
  - 内容、装修、消息通知、系统设置
  - 商户、门店、同城配送、批发
- **基础能力**
  - JWT 登录与刷新
  - 统一响应结构与业务错误码
  - Redis 缓存
  - MySQL 事务与原子库存扣减
  - 本地文件上传，可扩展对象存储
  - Elasticsearch 商品搜索扩展

## 技术栈

| 组件 | 技术 |
|------|------|
| Web 框架 | Axum 0.8 + Tokio |
| 数据库 | MySQL + SQLx 0.8 |
| 缓存 | Redis |
| 消息队列 | RabbitMQ |
| 搜索 | Elasticsearch（可选） |
| 认证 | JWT（HS256） |
| 序列化 | serde / serde_json |
| 金额计算 | rust_decimal |
| 日志 | tracing |

## 目录结构

```text
.
├── Cargo.toml                  # workspace 与依赖版本
├── config.yaml                 # 运行配置
├── Dockerfile
├── docker-compose.yml
├── migrations/                 # 数据库初始化/升级脚本
└── crates/
    ├── core/                   # 领域模型、实体、错误、分页
    ├── common/                 # 配置、日志、通用工具
    ├── infrastructure/         # 数据库、缓存、队列、存储
    ├── security/               # JWT、密码、认证中间件
    ├── services/               # 业务服务层
    ├── api/
    │   ├── admin/              # 管理后台 API（/adminapi）
    │   ├── public/             # 商城前台 API（/api）
    │   ├── openapi/            # 接口契约与覆盖率端点
    │   └── security/           # 认证与安全相关路由
    └── bin/                    # 可执行入口
```

## 端口

默认启动两个 HTTP 服务：

```text
管理后台 API: http://127.0.0.1:8080/adminapi
商城前台 API: http://127.0.0.1:8181/api
```

健康检查：

```text
GET /adminapi/health
GET /api/health
```

## 配置

配置来自 `config.yaml`，也可用环境变量覆盖，格式为：

```text
TIG_<配置路径，层级用 __ 分隔>
```

例如：

```text
TIG_DATASOURCE__URL
TIG_REDIS__HOST
TIG_JWT__SECRET
TIG_SERVER__ADMIN_PORT
TIG_SERVER__API_PORT
```

主要配置项：

```yaml
server:
  admin_port: 8080
  api_port: 8181
  host: 0.0.0.0

datasource:
  url: "mysql://user:password@127.0.0.1:3306/database"
  max_connections: 20

redis:
  host: 127.0.0.1
  port: 6379
  password: ""

jwt:
  secret: "please-change-this-secret"
  access_expire_secs: 7200
  refresh_expire_secs: 604800

rabbitmq:
  host: 127.0.0.1
  port: 5672
  username: admin
  password: ""

elasticsearch:
  url: "http://127.0.0.1:9200"
  product_index: products

file:
  upload_dir: /opt/uploads

storage:
  kind: local
```

## 本地运行

```bash
# 1. 准备 MySQL、Redis（可选 RabbitMQ / Elasticsearch）
# 2. 修改 config.yaml 中的连接信息
# 3. 启动
cargo run --release
```

指定配置文件目录时，请从包含 `config.yaml` 的目录启动，或通过环境变量覆盖全部连接参数。

## 构建

```bash
cargo build --release
```

产物：

```text
target/release/ 下的可执行文件
```

## Docker

```bash
docker compose up -d --build
```

默认包含 MySQL、Redis、RabbitMQ 与应用服务，具体端口与账号见 `docker-compose.yml`。

## 接口约定

所有接口统一返回：

```json
{
  "code": 0,
  "msg": "success",
  "message": "success",
  "data": {},
  "timestamp": 1718000000000
}
```

- `code = 0` 表示成功
- 业务失败仍返回 HTTP 200，通过 `code` / `msg` 区分
- 需要登录的接口通过请求头携带：

```text
Authorization: Bearer <token>
```

## 部署

推荐使用 systemd 托管：

```ini
[Unit]
Description=E-commerce backend
After=network.target mysql.service redis.service

[Service]
Type=simple
WorkingDirectory=/opt/ecommerce
ExecStart=/opt/ecommerce/server
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
```

Nginx 反向代理示例：

```nginx
location /api/ {
    proxy_pass http://127.0.0.1:8181;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}

location /adminapi/ {
    proxy_pass http://127.0.0.1:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

## 测试

```bash
cargo test --workspace
cargo clippy --workspace
```
