# 部署说明（Rust 版）

## 1. Docker Compose（PostgreSQL 口径，2026-11-10 重写）

```bash
cd rust
export TIG_JWT_SECRET="$(openssl rand -base64 48)"   # 必填，见下
docker compose up -d --build
```

- 数据库是 **`postgres:16-alpine`**（原 `mysql:8.0` 已废弃）。数据落在 `pg-data` volume。
- `TIG_JWT_SECRET` 用 `${VAR:?}` 声明为**必填**：未设置时 `docker compose up` 会直接报错退出，
  而不是悄悄用弱密钥跑起来。release 构建里另有兜底——密钥若仍是内置默认值，进程拒绝启动
  （`crates/common/src/config.rs` 的 `assert_secret_is_overridden`）。
- `TIG_DATASOURCE__AUTO_MIGRATE` 在 compose 里显式写成 `false`：`migrations/` 是 MySQL 时代的
  DDL（`AUTO_INCREMENT`、反引号、`ENGINE=InnoDB`），对 PG 会直接语法报错。**不要打开。**
- 如需重新初始化数据库，删掉 `pg-data` volume 后重启。

### 全新安装：库结构从哪来

仓库里**没有** PG 的基线 DDL（`deploy/mysql-init/*.sql` 是 MySQL 基线，`migrations/` 也是 MySQL 口径）。
`deploy/pg/*.sql` 全是 `ALTER` 增量脚本，必须有基线结构才能执行，所以 initdb 阶段不挂载它们
（只以 `/deploy/pg` 只读挂进去，供手工执行）。

推荐顺序：

```bash
# 1) 从现存生产库导结构（不含数据）
pg_dump --schema-only --no-owner --no-privileges \
  "postgresql://<user>:<pwd>@<prod-host>:5432/nextrade" > schema.sql

# 2) 首启前把 schema.sql 放进 pg-data 的初始化路径，或起库后手工导入
docker compose up -d postgres
docker compose exec -T postgres psql -U nextrade -d nextrade < schema.sql

# 3) 再起应用
docker compose up -d app
```

若是从更早的基线（未包含 `deploy/pg` 里那些修复）重建，按文件名日期顺序执行：

```bash
for f in $(ls /deploy/pg/*.sql | sort); do
  docker compose exec -T postgres psql -U nextrade -d nextrade -f "$f" || break
done
```

### 关于 `deploy/mysql-init/`

MySQL 时代的建库脚本（`00-pure.sql` / `01-demo-signed.sql`），**compose 已不再挂载**。
保留只为追溯历史基线（审计文档里引用过它的 `AUTO_INCREMENT` 列）。新部署不要用。

> 注意：本仓库开发机没有 Docker，compose 文件已改为 PG 口径但尚未实际拉起验证；
> 首次部署请保留 `docker compose logs -f postgres app` 观察初始化。

## 2. 前端 / Nginx

- `deploy/nginx.conf` 是参考反代配置：`/adminapi`、`/im`、`/static`、
  `/adminws` → 8080；`/api`、`/ws` → 8181；`/metrics` → 8080。
- **后端 8181 只提供 `/api` JSON，不返回任何 HTML**。PC 前端是独立 SPA，必须由 nginx 静态托管。
  （历史上的 Leptos SSR fallback 已于 2026-10-20 下线，见 `rust/crates/bin/src/main.rs` 注释。）
- **Rust 打包不包含前端**：`cargo build --release` / `Dockerfile` 只产出
  `nextrade` 后端二进制与 `config.yaml`、`migrations`、`template`；前端 `dist`
  必须单独构建并由 nginx（或对象存储/CDN）托管。

### 前端构建与部署（2026-11 更新）

当前前端只有两套 React 工程，均为独立 npm 安装（`admin-semi` 也有 `package-lock.json`）：

```bash
# 管理端：Vite + React + Semi
cd rust/web/admin-semi
npm install
npx tsc -b && npx vite build --base=/admin-semi/

# PC 商城：Next.js 14
cd rust/web/pc-semi
npm install
npm run build
```

| 端 | 源码 | 产物 | 部署到 |
|---|---|---|---|
| 管理端 | `rust/web/admin-semi` | `dist/` | 静态托管 `/admin-semi/`（宝塔：`/www/wwwroot/<站点>/public/admin-semi/`） |
| PC 商城 | `rust/web/pc-semi` | `.next` + `public` | Node 服务 `node server.js`（端口 3004），nginx 反向代理 `/` |

- `rust/web` 仍是 pnpm workspace 根，但 `packages` 只保留 `pc-semi`。
- 2026-11 已移除旧的 Vue3 重写原型 `rust/web/{admin,pc,shared}`（依据
  `rust/docs/FRONTEND-1TO1-REVIEW-2026-10-25.md` 的 R3 决策），不要再按旧文档构建。
- 原版前端 `java/view/{admin,NexTrade-Pc}` 仅作对照基准，不是部署来源。

- 本地开发也可用 `nginx -c /path/to/deploy/nginx.conf` 验证路由。
- 开发期各有 dev server：admin-semi `:3002`（代理 `/adminapi`、`/static`→8080）、
  pc-semi `:3004`（`PC_API_BASE` 指定后端）。

## 3. 环境变量（关键）

| 变量 | 说明 |
|---|---|
| `TIG_DATASOURCE__URL` | **PostgreSQL** URL（`postgres://user:pwd@host:5432/db`，compose 已注入） |
| `TIG_DATASOURCE__AUTO_MIGRATE` | 必须 `false`：`migrations/` 是 MySQL 口径 |
| `TIG_REDIS__HOST` / `TIG_RABBITMQ__HOST` | Redis / RabbitMQ |
| `TIG_JWT__SECRET` | **必填**。留空或用内置默认值时，release 构建拒绝启动 |
| `TIG_SERVER__ADMIN_PORT` / `TIG_SERVER__API_PORT` | 8080 / 8181 |
| `TIG_SCHEDULER__ENABLED` | 是否启用轮询任务 |
| `OPEN_TEMPLATE_BASE_URL` | 装修模板 OpenTemplate API（缺省 `https://api.nextrade.com/index`） |

## 4. 验收清单

1. `curl http://127.0.0.1:8080/adminapi/health` → `{"data":"UP",...}`
2. `curl http://127.0.0.1:8080/metrics` → Prometheus 文本
3. 管理端登录（`admin/admin123`）→ token/adminType 正常
4. 前端经 nginx 登录、商品列表、下单、支付回调、IM WS 正常
5. `docker compose ps` 中 postgres/app 为 healthy/running
