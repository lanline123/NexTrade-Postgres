# tools/hot-update — admin 自助热更新工具链（我方新增）

设计：`~/workspace/hot-update-design/REPORT.md`；实现与部署步骤：
`~/workspace/hot-update-design/IMPL-REPORT.md`。

## 契约速览

- **更新源**：GitHub Releases。每产品版本一个 tag `v<x.y.z>` 的统一
  Release，资产 = 四面产物（沿用各构建线既有包）+ `release-manifest.json`。
- **清单结构**：与后端 `core::models::system::system_update::ReleaseManifest`
  逐字段对应（camelCase）。`migrations` 为 `rust/migrations/` 编号迁移的
  **全序**（待执行 = 清单 − `sys_schema_migration` 登记表）。
- **版本戳**：后端编译期嵌入（`common::version`，build.rs 采集 git sha）；
  静态面包内 `version.json`（`write_version_json.mjs` 生成，`stamp` 字段
  为判版依据），缺失时后端降级读站点指纹并如实标注口径。
- **执行**：后端只发起 job（spool 文件）→ 服务器独立 `updater` 执行
  （下载→验 sha→备份→换面→健康检查→失败自动回滚），P2 仅静态三面。

## 文件

| 文件 | 用途 |
|---|---|
| `make_manifest.py` | 发版收尾生成 `release-manifest.json`（含迁移分类与 sha、CHANGELOG 更新说明、与后端内嵌版本交叉校验） |
| `classify_sql.py` | 迁移 SQL 语句级分类器（增/删/改字段标注 + 破坏性判定），`--selftest` 自检 |
| `baseline.py` + `baseline_applied.txt` | P1 基线回填：生成探针 SQL 与探针守卫的登记 SQL（只生成不连库） |
| `write_version_json.mjs` | CI 打包前给静态面包写 `version.json` |
| `updater/nextrade-updater.sh` | 服务器执行器（root 运行，状态机全量实现） |
| `updater/nextrade-updater.{service,path}` | systemd 单元（path 监视 spool/jobs 触发） |
| `updater/updater.conf.example` | 执行器配置模板（与后端 `update` 配置段对应） |

## 纪律

- 迁移只进 `rust/migrations/` 编号序列，一文件一号、永不改已发布文件
  （改动会被登记表 sha 比对发现并在更新页告警）。
- updater 本身永不经热更新通道升级，只能人工部署。
- 令牌（细粒度 PAT，单仓库 Contents 只读）只存服务器 root-only 文件，
  路径经配置引用；不进 DB、不经 API 回显、不进日志。
