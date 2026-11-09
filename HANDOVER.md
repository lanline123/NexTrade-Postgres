# NexTrade-Postgres 交接文档

> 更新时间：2026-11-09
> 范围：PostgreSQL 迁移版仓库的**环境接入**——服务器、宝塔面板、GitHub、生产 PG。
> 迁移进度与缺陷台账见 `MIGRATION-AUDIT-2026-11-09.md`（被 `.gitignore` 排除，仅存本地）。
> 上游项目的功能/业务交接文档在 `D:\tigshop\NexTrade\docs\HANDOVER.md`，本文只补「怎么连上去、用什么工具」。

---

## 0. 速查

| 目标 | 方式 | 入口 | 本机是否可用 |
|---|---|---|---|
| 宝塔面板 | 面板 HTTP API（HTTPS + 签名） | `C:\Users\Public\bt2api.ps1` | ✅ 已实测 |
| GitHub | git over HTTPS + Windows 凭据管理器 | `credential.helper=manager` | ✅ 已推送 `afbf86e` |
| 服务器 shell | 未开放 | 无 SSH 私钥；面板 API 不含命令执行 | ❌ |
| 生产 PostgreSQL | 需在服务器上执行 `psql` | `/www/server/pgsql/bin/psql` | ❌ 本地不可直连 |

**首要原则：本仓库是公开仓库**（`https://github.com/lanline123/NexTrade-Postgres`，匿名 API 返回 200）。
因此**面板 IP、API 密钥、数据库口令一律不写进本文件，也不提交**。本文只写机制与文件位置。

---

## 1. 宝塔面板（HTTP API）

### 1.1 接入原理

服务器未开放 SSH，但宝塔面板开放了带签名的 HTTP API。所有调用都是 POST 表单：

```text
POST {panel_base}/{module}?action={Action}
form: request_time, request_token, ...业务参数
```

签名算法：

```text
api_md5       = md5_hex(api_key)
request_time  = 当前 Unix 秒（字符串）
request_token = md5_hex(request_time + api_md5)
```

`request_time` 与服务端时间偏差过大会被拒，注意本机时钟是否准确。

### 1.2 凭证位置（仅本机，不入库）

`C:\Users\Public\bt2api.ps1`（843 字节）提供四个符号：

| 符号 | 含义 |
|---|---|
| `$script:base` | 面板地址，形如 `https://<ip>:8888` |
| `$script:api` | 32 位 API 密钥 |
| `New-BTAuth` | 返回 `@{ request_time; request_token }` |
| `Invoke-BTPost <route> <hashtable>` | 带签名 POST，返回响应文本 |

调用方：

- `D:\tigshop\NexTrade\tools\upload-to-bt.ps1` —— 分片上传（`/files?action=UploadFile`）
- `D:\tigshop\NexTrade\tools\deploy-rust-bt.ps1` —— 后端发布：上传 → 合并 → 停服 → 备份 → 切换 → 探活 → 失败回滚

> ⚠️ 该文件位于 `C:\Users\Public\`，本机所有用户可读。建议移到 `%USERPROFILE%\.bt\bt2api.ps1` 并收紧 ACL，同时同步修改上述两个脚本里 `. 'C:\Users\Public\bt2api.ps1'` 的路径。
> 上游 `tools/README-upload-to-bt.md` 第 108 行也写明「不要把宝塔 API token 写进脚本或提交到仓库」。

### 1.3 快速验证（只读）

```powershell
. 'C:\Users\Public\bt2api.ps1'
Invoke-BTPost 'system?action=GetSystemTotal' @{}
```

2026-11-09 实测返回：面板 `13.2.0`、`OpenCloudOS 9.4 x86_64`、4 核、3.6 GB 内存、已运行 18 天。

### 1.4 可用接口

| 用途 | 调用 | 读/写 | 验证状态 |
|---|---|---|---|
| 系统概况 | `system?action=GetSystemTotal` | 读 | ✅ 已实测 |
| 列目录 | `files?action=GetDir`（`path=`） | 读 | ✅ 已实测 |
| 读文件 | `files?action=GetFileBody`（`path=`） | 读 | 待验证 |
| 写文件 | `files?action=SaveFileBody`（`path=`、`data=`） | 写 | 待验证 |
| 分片上传 | `files?action=UploadFile`（`zunfile`、`path`、`fname`、`f_size`、`f_start`） | 写 | 见 §1.2 脚本 |
| 建目录 | `files?action=CreateDir` | 写 | 待验证 |
| 站点列表 | `site?action=GetSiteList` | 读 | 待验证 |
| 计划任务 | `crontab?action=GetCrontab` / `AddCrontab` | 读/写 | 待验证 |

「待验证」表示来自宝塔通用 API，首次使用前先用只读调用确认一次。

**已知行为**：`SaveFileBody` 按 `utf-8` 处理，且会被 WAF 剥离请求体里的双引号；需要写含引号的 JSON 时改走 multipart `UploadFile`（详见上游 `docs/HANDOVER.md` §5.5 第 2 条）。

### 1.5 没有 shell 怎么执行命令

面板 API 不含命令执行。要跑服务器脚本（例如仓库根的 `probe-pg-pks.sh`），两种做法：

1. **计划任务**：`UploadFile` 上传脚本 → `AddCrontab` 建一条每分钟任务 → `GetDir` / `GetFileBody` 取输出 → 删除任务。
2. **本地执行、只传产物**（部署场景推荐）。

> 任何写操作（改文件、加计划任务、重启服务）都会影响线上，执行前必须先确认。
> 上游有 `/www/backup/pc-*.sh` 每分钟把部署回滚成旧包的历史坑——**动手前先 `crontab?action=GetCrontab` 查一遍**。

---

## 2. GitHub

### 2.1 仓库与认证

| 项 | 值 |
|---|---|
| 仓库 | `https://github.com/lanline123/NexTrade-Postgres.git` |
| 可见性 | **public** |
| 分支 | `main` |
| 认证 | `credential.helper=manager`（Windows 凭据管理器，已存 github.com 凭据） |
| `gh` CLI | 未安装 |
| SSH 密钥 | 无（`~/.ssh` 只有 `known_hosts`） |
| git | 2.55.0.windows.3 |

`git push` 已验证可非交互执行（2026-11-09 推送 `afbf86e`）。

### 2.2 常用命令

```bash
git fetch origin main
git status
git diff --stat origin/main
git add <具体文件>        # 不要 git add -A：会带上 scratch/、日志和别人的半成品
git commit -m "..."
git push origin main
```

### 2.3 CI

| workflow | 触发 | 内容 |
|---|---|---|
| `check-rust.yml` | push / PR 到 `main`，命中 `rust/crates/**` 或 `.github/workflows/**` | `cargo check --workspace`（`rust/crates` 目录下执行） |
| `build-rust.yml` | `workflow_dispatch` 手动 | Release `build-<n>`，资产 `nextrade` / `nextrade.sha256` |
| `build-staging.yml` | 手动 | thin LTO，只出 artifact |
| `build-pc.yml` | 手动 | Release `pc-build-<n>`，资产 `nextrade-pc.tgz` |

**约定**：全量编译只在 GitHub 跑，本地不执行 `cargo check`（用户 2026-11-09 14:07 指令）。
本地可用 `rustfmt --edition 2021 --emit stdout <file>` 做纯语法解析（不编译、不拉依赖）。

读取运行结果：

- 网页：`https://github.com/lanline123/NexTrade-Postgres/actions`
- 匿名 API **读日志会 403**（`Must have admin rights`），需要仓库管理员的 token 才能下载日志。

---

## 3. 服务器结构

以下为上游 `docs/SERVER-MIGRATION-2026-11-06.md` §11 与 `docs/HANDOVER.md` §10 的记录，
末尾「实测差异」是 2026-11-09 通过面板 `GetDir` 核对的结果。

| 组件 | 路径 / 单元 | 端口 |
|---|---|---|
| PC 商城 `battery.recy.cc` | `/www/wwwroot/battery.recy.cc`（Next.js SSR，`nextrade-pc`） | 3005 |
| 后台 `admin.recy.cc` | `/www/wwwroot/admin.recy.cc`（admin-semi 静态） | — |
| Rust 后端 | `/www/wwwroot/nextrade/nextrade`（systemd `nextrade`） | 8080 `/adminapi`、8181 `/api` `/static` |
| nginx 主配置 | `/www/server/panel/vhost/nginx/html_<域名>.conf`（面板生成） | 80/443 |
| nginx 业务配置 | `/www/server/panel/vhost/nginx/extension/<域名>/custom.conf`（手工维护） | — |
| 证书 | `/www/server/panel/vhost/cert/<域名>/` | — |
| 备份 | `/www/backup/` | — |

必须保留的 nginx 片段（否则页面白屏）：面板默认配置里的 `location ~ .*\.(js|css)?$` 会把 `/_next/`
当成静态文件返回 404：

```nginx
location ^~ /_next/ {
    proxy_pass http://127.0.0.1:3005;
}
```

`/adminapi` 必须关闭缓冲，否则 SSE 消息不推送：

```nginx
proxy_buffering off;
proxy_cache off;
proxy_read_timeout 3600s;
```

**实测差异（与上游文档不一致，待确认）**：

- `/www/wwwroot/battery.recy.cc` 属主是 `lighthouse`（文档写 `www`）
- `/www/wwwroot/nextrade` 属主是 `root`（文档写 `www`）
- 多出 `battery.recy.cc.bak.20261109005101`（11-09 00:51 的备份，上游文档未记录）

---

## 4. 生产 PostgreSQL

| 项 | 值 |
|---|---|
| 客户端 | `/www/server/pgsql/bin/psql` |
| 连接 | `-h 127.0.0.1 -U NexTrade -d nextrade`（本机 trust/peer，脚本中无明文口令） |
| 主键导出脚本 | 仓库根 `probe-pg-pks.sh`，输出 `/www/backup/nextrade-pks.tsv` |
| 已有导出副本 | `D:\tigshop\NexTrade\scratch\nextrade-pk-map.tsv`（342 张表，含列类型） |

5432 **未对公网开放**，本地无法直连。需要查询时按 §1.5 上传脚本 + 计划任务执行，或用 `psql` 输出
到 `/www/backup/` 再用 `GetFileBody` 取回。

> 实测补充：公网 :5432 是「TCP 能握手、协议无响应」的黑洞；`postgresql.conf` 里
> `listen_addresses` 与 `port` 都是注释状态（默认只监听 127.0.0.1）。`pg_hba.conf` 里虽然有一条
> `host nextrade NexTrade 0.0.0.0/0 md5`，但没有监听就无从连起。不要试图直连。

### 4.1 已验证的服务器侧执行流程

```text
1. multipart 上传 <tag>.sql 与 <tag>.sh 到 /www/backup/
2. crontab?action=AddCrontab 建一条 minute-n 任务（sBody=/bin/bash /www/backup/<tag>.sh）
3. crontab?action=StartTask 立即执行（不必等 cron 触发）
4. files?action=GetFileBody 轮询 /www/backup/<tag>.out，直到出现 ##FINISHED
5. crontab?action=DelCrontab 删除任务
6. files?action=DeleteFile 清理临时文件（path=/www/backup, filename=<name>）
```

脚本模板见 `scratch/run_pg_sql.py`（本机，未入库）。执行 SQL 时统一加
`-v ON_ERROR_STOP=1`，写操作包在 `BEGIN; SET LOCAL lock_timeout='10s'; ... COMMIT;` 里，
失败整体回滚。

**注意**：脚本末尾会自动 `crontab -l | grep -v <tag> | crontab -` 自删系统 cron，
但面板数据库里的任务记录仍需 `DelCrontab` 清理。

### 4.2 Schema 对齐记录

`rust/deploy/pg/2026-11-09_schema_align.sql`：生产库与 MySQL 基线（`database/pure.sql`）的
一次性对齐脚本，198 条语句，已于 2026-11-09 执行完毕。内容包括：

- 8 列 `smallint → integer`（`role_id`/`parent_id`/`article_category_id` 等）
- 54 列 `numeric` 精度恢复（经纬度 `numeric(10,6)`、金额 `numeric(12..20,2)`、重量体积 `numeric(10,3)/(12,5)`）
- 108 列 `varchar(255) → varchar(n)` 恢复原长度
- 3 列 `DROP NOT NULL`、3 列 `SET NOT NULL`（`vendor*` 的 3 个 MySQL `GENERATED STORED` 列保持可空）
- 16 个缺失索引、1 个冗余索引删除

**未改动、属有意适配**：39 列 `datetime/date → varchar(255)`、10 列 `json → text`。
原因是 Rust 实体里这些字段是 `String`，改成 `timestamp` / `jsonb` 需要同步改代码与序列化，
属于独立改造项，不是 schema bug。**这两类已单独准备，见 §4.3。**

### 4.3 时间 / JSON 列切换（**已于 2026-11-09 执行**）

49 列（39 时间 + 10 JSON）要从 `varchar/text` 改为 `timestamp` / `date` / `jsonb`。

| 项 | 状态 |
|---|---|
| 代码 | 已提交 `256b3eb`：读 `col::text AS col`、写 `NULLIF($n,'')::timestamp|date|jsonb`；`row_to_json` 增加 TIMESTAMP/DATE 分支 |
| 迁移脚本 | `rust/deploy/pg/2026-11-09_time_json_align.sql`（49 条 ALTER） |
| 生产库 | **已转换**（49/49，`information_schema` 核验 `still_text=0`） |
| 为何不能单独转 | 线上二进制按 `String` 解码这些列，单独转列会让 `cart` / `order` / `promotion` / `user` 立即报错 |

切换步骤（已在 2026-11-09 执行；保留供环境重建时参考）：

```bash
systemctl stop nextrade
# 二进制必须是包含 256b3eb 的构建
/www/server/pgsql/bin/psql -h 127.0.0.1 -U NexTrade -d nextrade -X -v ON_ERROR_STOP=1 \
  -f /www/backup/2026-11-09_time_json_align.sql
systemctl start nextrade
curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:8080/adminapi/health
curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:8181/api/health
```

回滚语句在脚本末尾（`timestamp -> varchar(255)` 用 `to_char(...)` 转换）。
切换前预检已通过：空串 0、非法格式 0；当前只有 3 列有数据
（`wechat_live.create_time`、`statistics_base.date`、`user.birthday`）。

---

## 5. 本机工具矩阵

| 工具 | 版本 / 状态 | 用途 |
|---|---|---|
| git | 2.55.0.windows.3 | 提交、推送 |
| curl | 8.21.0（自带 `-k`） | 面板 API、GitHub API |
| pwsh | PowerShell 7.6.9 | 跑 `tools/*.ps1` |
| python | 3.8.6 | 校验脚本、签名计算 |
| rustfmt | 1.10.0-stable | 单文件语法解析（不编译） |
| cargo | 已装（**本地不跑 check**） | 仅 CI |
| `gh` | ❌ 未安装 | 用 API + curl 替代 |
| ssh / scp / rsync | ❌ 未安装（无私钥） | 用面板 API 替代 |

---

## 6. 安全与已知不一致

1. **`rust/config.yaml` 已移出仓库**（2026-11-09 处理）：
   - 新增 `rust/config.example.yaml`（全占位值）；`rust/config.yaml`、`rust/.env*` 已加入 `.gitignore`
     并从索引移除（`git rm --cached`），**本地文件保留**，服务照常运行。
   - 配置加载是 `config::File::with_name("config").required(false)`（`crates/common/src/config.rs:355`），
     文件可选，CI 不依赖它。
   - **仍需人工完成**：
     - 部署机用环境变量覆盖敏感值：`TIG_DATASOURCE__URL`、`TIG_JWT__SECRET`、`TIG_REDIS__PASSWORD`、
       `TIG_RABBITMQ__PASSWORD`；或将它们写进 systemd `EnvironmentFile`。
     - 历史提交里仍有明文（`git log -p -- rust/config.yaml`）。原值为「用户名/口令相同」，
       JWT 为占位串。**凡是在任何环境用过的，都需要轮换**；JWT secret 一换，现有登录态全部失效，
       建议低峰期执行。
     - `crates/common/src/config.rs:159` 还有一个内置默认 JWT secret（源码公开）。生产若未覆盖
       `TIG_JWT__SECRET`，等于签名密钥公开、任何人可伪造 token。建议后续改成「非 debug 构建且使用
       默认值时拒绝启动」。
2. `rust/docker-compose.yml` 仍是 `mysql:8.0` + `mysql://` DSN，与 PG 迁移不符。
3. `rust/deploy/README.md` 与 `rust/deploy/mysql-init/` 仍是 MySQL 口径。
4. 若面板地址或 API 密钥发生轮换，只需改 `C:\Users\Public\bt2api.ps1`，本文无需变更（本文不含具体值）。

---

## 7. 变更记录

| 日期 | 内容 |
|---|---|
| 2026-11-09 | 首次建立；记录面板 API 签名方式、凭证位置、已验证接口、GitHub 认证与 CI、生产 PG 入口、工具矩阵与安全项 |
| 2026-11-09 | `rust/config.yaml` 移出仓库：新增 `config.example.yaml`，`.gitignore` 增加 `rust/config.yaml` / `rust/.env*`，同步修正 `rust/README.md` 的配置说明与 PG DSN 示例 |
| 2026-11-09 | 生产 PG 与 MySQL 基线（`database/pure.sql`）全量比对：335 表 / 3927 列，0 表 0 列缺失；修复 198 条类型/索引/可空性差异，脚本见 `rust/deploy/pg/2026-11-09_schema_align.sql`；新增 §4.1 服务器侧执行流程、§4.2 对齐记录 |
