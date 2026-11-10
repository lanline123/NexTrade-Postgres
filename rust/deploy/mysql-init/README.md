# MySQL 时代建库脚本（已停用）

这里的两份文件是迁移到 PostgreSQL **之前**的建库产物：

| 文件 | 原用途 |
|---|---|
| `00-pure.sql` | 由 `python tools/compat/prepare_docker_mysql.py` 从 `../java/database/pure.sql` 生成，去掉了 `INT UNSIGNED`（sqlx 按 `i32` 解码会失败） |
| `01-demo-signed.sql` | 演示数据，整型同样改成有符号 |

**现在的状态：**

- `rust/docker-compose.yml` 已改为 `postgres:16`，**不再挂载本目录**。
- 线上生产库是 PostgreSQL，结构以 `deploy/pg/*.sql` 的增量历史为准。
- 生成器 `tools/compat/prepare_docker_mysql.py` 不在本仓库内。

**保留原因**：审计文档（`MIGRATION-AUDIT-2026-11-09.md` 第十七章）引用过这里的
`AUTO_INCREMENT` 列作为主键权威来源，删掉会让那段结论无法复核。

**新部署不要使用本目录**，走 `deploy/README.md` 的 PostgreSQL 流程。
