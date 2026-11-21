# auth-semi —— NexTrade 授权中心（auth.recy.cc）

与 `pc-semi`、`admin-semi` 并列的独立应用：零依赖 Node ≥18 服务，独立进程、独立站点、
独立数据，互不引用。职责是给所有 NexTrade 部署实例做授权判定与版本（档位）管理：

- **商业授权**：只可绑定 **1 个固定域名**；
- **企业授权**：可绑定 **1 个泛域名**，在该泛域名下可部署 **5 个子域名**（默认额度，
  后台可手动增减，续费/加购时调整）；
- 实例访问 admin 时主动上报，中心记录其访问地址并判定授权状态，实例侧据判定
  显示「已授权 · 商业版/企业版」或未授权提示，并以菜单显隐实现版本控制（客户端线实现）。

## 运行

```bash
node src/server.js        # 或 npm start
```

配置走环境变量或项目根 `.env`（环境变量优先）：

| 变量 | 说明 |
|---|---|
| `PORT` | 监听端口，默认 `3011`（只绑 127.0.0.1，由 nginx 反代对外） |
| `HOST` | 监听地址，默认 `127.0.0.1` |
| `AUTH_ADMIN_PASSWORD` | 管理端单密码（未设置则管理端禁用，checkin 照常） |
| `AUTH_SESSION_SECRET` | 会话签名密钥（未设置自动生成并存 `data/.session-secret`，0600） |
| `AUTH_DATA_FILE` | 数据文件路径，默认 `<项目>/data/db.json` |

## 数据存储说明

全部状态就是 `data/db.json` 一个 JSON 文件（授权、实例聚合、上报流水、序号），
单进程同步读写、临时文件 + rename 原子落盘；备份 = 拷贝此文件。
`data/` 目录不入仓库（运行时生成），部署时由服务器侧创建并归属运行用户。

## checkin 契约（与 NexTrade 客户端线共享，逐字约定）

### 请求

`POST /api/instances/checkin`，JSON 体：

```json
{ "instanceKey": "<实例唯一键>", "adminHost": "admin.example.com",
  "siteDomains": ["shop.example.com"], "appVersion": "1.2.3" }
```

- `instanceKey`：实例唯一标识（必填，≤128 字符），同时是响应签名的派生源；
- `adminHost`：实例 admin 访问地址（主机名，可带协议/端口，服务端归一化）；
- `siteDomains`：实例的站点域名数组；
- 候选域名 = `adminHost` + `siteDomains`（归一化：小写、去协议/路径/端口/尾点）。

### 判定

1. 任一候选域名与某**商业授权**的固定域名全等 → `authorized / commercial`；
2. 否则任一候选域名为某**企业授权**泛域名的真子域名：
   - 已登记 → `authorized / enterprise`；
   - 未登记且已用额度 < 上限 → **首次命中自动登记**占用额度 → `authorized / enterprise`；
   - 额度已满 → `unauthorized`（`tier` 回显 `enterprise`，管理端实例列表标红「额度已满」）；
3. 命中的授权已到期（`expiresAt > 0 且 <= now`）→ `unauthorized`，`tier` 保留回显，
   响应带 `"expired": true`；
4. 停用授权不参与判定；都不命中 → `unauthorized`，`tier: null`。

### 响应（扁平 JSON）

```json
{ "status": "authorized" | "unauthorized",
  "tier": "commercial" | "enterprise" | null,
  "expiresAt": 1735689600000 | null,
  "subdomainLimit": 5 | null,
  "subdomainsUsed": 2 | null,
  "checkedAt": 1735680000000,
  "expired": true,            // 仅到期未授权时出现
  "signature": "<hex>" }
```

- `expiresAt`：ms 时间戳，永不过期或未命中时为 `null`；
- `subdomainLimit` / `subdomainsUsed`：仅企业档（含额度已满、到期回显）有值，其余为 `null`。

### 响应验签（防伪造响应）

```
signKey   = SHA256("auth-semi-sign-v1:" + instanceKey)            // 原始 32 字节
payload   = [status, tier, expiresAt, subdomainLimit, subdomainsUsed, checkedAt].join("|")
            // null 一律以空串参与拼接，其余转十进制字符串
signature = HMAC-SHA256(signKey, payload) 的 hex 小写
```

每次 checkin 都会落一条上报记录，并按 `instanceKey` 更新实例聚合
（首次/最近上报时间、admin 地址、站点域名、程序版本、最近判定）。

## 管理端

`/admin.html` 单密码登录（签名 Cookie 会话，7 天）。功能：授权新建/编辑/停用/删除、
企业版子域名额度 ±调整与已登记子域名释放、实例上报聚合列表（额度已满标红）、
最近上报记录、清除实例记录。

管理 API（`{code,msg,data}` 信封）：`GET/POST /api/admin/licenses`、
`PUT/DELETE /api/admin/licenses/:id`、`POST /api/admin/licenses/:id/adjust-limit {delta}`、
`DELETE /api/admin/licenses/:id/subdomains/:sub`、`GET /api/admin/instances`、
`DELETE /api/admin/instances/:key`、`GET /api/admin/checkins?limit=`。

## 部署

独立 systemd 服务 `nextrade-auth`（www 用户、Node 24、端口 3011）+ 宝塔站点
`auth.recy.cc` nginx 全量反代 127.0.0.1:3011；管理员口令部署时在服务器侧随机生成，
只存服务器 `.env`（600）。部署细节与验收见交付报告（平台线维护）。
