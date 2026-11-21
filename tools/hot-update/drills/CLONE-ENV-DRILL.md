# 热更新 P3 克隆环境灾难演练规程（真实 PG / 真实目录）

> 定位：`run-drills.sh` 是桩件演练（验控制流与闸门逻辑，本线已跑 27/27
> PASS）；**本规程是上生产前的必备验收**——在克隆环境里用真实
> PostgreSQL、真实二进制、真实目录布局把灾难路径再走一遍，验的是桩件
> 验不了的东西：真事务回滚、真 pg_dump 产物可恢复、`--single-transaction`
> 与真实迁移 SQL 的兼容性、systemd 真重启与版本端点真响应。
>
> 红线：在克隆环境通过之前，P3 不得在生产开放（含 backend 的 job 一律
> 不许在生产发起）。演练全程不碰生产库、不碰生产目录。

## 0. 克隆环境准备

1. **克隆库**：从生产最近一份 pg_dump 恢复到独立实例/独立库
   （如 `nextrade_clone`），连接串只经 `PG*` 环境变量注入演练 shell，
   不写进任何脚本与报告。恢复后先跑 P1 的基线回填，使
   `sys_schema_migration` 与生产登记进度一致，并核对
   `SELECT count(*) FROM sys_schema_migration` 与基线清单一致。
2. **克隆目录**：复制生产布局到沙箱根（如 `/www/clone-p3/`）：
   `rust/nextrade`（现行生产二进制）、`admin/`、`pc/`、`mobile/`、
   `updater/`（放 `tools/hot-update/` 全部脚本）、`spool/{jobs,status,logs}`、
   `backups/`。systemd 用同名单元的克隆版（改 WorkingDirectory/端口，
   如后端 19080/19181、PC 3905），单元名经 `HU_BACKEND_UNIT` 覆盖。
3. **演练 Release**：用 make_manifest.py 造一个演练产品版本（版本号如
   `0.0.3-drill`），资产 = 现行生产二进制**重新打包**（基线对照）+
   一个**故意改坏的后端二进制**（见 D-2）+ 三个演练迁移（见下）。
   清单 migrations 按序包含演练迁移，sha 与文件一致。
4. **演练迁移三件套**（放克隆仓库外目录，仅演练用，不入正式迁移序）：
   - `901_drill_ok.sql`：`CREATE TABLE drill_p3 (id int); INSERT ...` 正常件；
   - `902_drill_fail.sql`：前半 `CREATE TABLE drill_p3_fail (id int);`、
     后半一条必错语句（如引用不存在的表）——验单文件事务整体回滚；
   - `903_drill_after.sql`：正常件，排在失败件之后——验「失败后不再往下跑」。

## D-1 迁移中途失败（真实事务回滚）

步骤：以含 backend 的 job 发起演练更新（清单 migrations =
[901, 902, 903]，updater 指向克隆 spool/目录/库）。

断言（全部满足才算过）：
- [ ] job 终态 `migration_failed`，stage 指向 `902_drill_fail.sql`；
- [ ] 克隆库中 `drill_p3` 表**存在**且已登记（901 成功保持——只进不退）；
- [ ] `drill_p3_fail` 表**不存在**（902 的前半句随事务回滚）；
- [ ] 903 未执行、未登记；`sys_schema_migration` 中无 902/903 行；
- [ ] 克隆后端二进制 sha 与演练前一致（未换任何面）、服务仍是旧版本；
- [ ] `backups/` 下有本 job 的 `pre-migrate-*.dump`，且
      `pg_restore -l` 可读；用它恢复到另一个临时库，表结构与迁移前一致。

## D-2 后端起不来自动回滚

准备：演练 Release 的 backend 资产换成「坏件」——两种坏法各演一遍：
(a) 二进制内容损坏（截断/随机字节，仍过 sha——即清单 sha 就是坏件的 sha，
模拟「构建产物本身坏」）；(b) 能启动但 `/api/version` 返回的 gitSha 与
清单 stamp 不符（用旧二进制冒充新件打包）。

断言：
- [ ] (a) 健康检查超时后自动回滚：克隆二进制 sha 回到演练前值、
      systemd 单元 active、版本端点返回旧 gitSha；
- [ ] (b) 同上（版本不符按健康失败处理，不因「进程活着」放行）；
- [ ] 两遍 job 终态均为 `rolled_back`，status.json 的 message 写明原因；
- [ ] D-1 已落的迁移登记保持不动（回滚只退程序面，schema 不退）；
- [ ] 回滚件 `nextrade_pre_hotupdate_<job 前 8 位>` 存在且 sha = 旧件 sha。

## D-3 破坏性迁移闸门（真实链路）

准备：清单 migrations 含一个带 `DROP TABLE drill_p3;` 的迁移且清单
`destructive=true`。

断言：
- [ ] admin 发起时未勾破坏性确认 → 后端直接拒绝（不生成 job）；
- [ ] 勾选后 job 生成，spool 文件含 `destructiveAck: true`；执行后
      `drill_p3` 被删、登记落行；
- [ ] 把清单 `destructive` 改成 false 重演 → migrate-runner 复扫拦截，
      job 落 `migration_failed`（stage=gate），表未被删。

## D-4 备份门与磁盘

断言：
- [ ] 把克隆 backups 目录所在文件系统可用空间压到小于
      「库体积 + HU_DISK_MARGIN_MB」（或用小 tmpfs 挂载冒充）→ job 落
      `migration_failed`（stage=backup），迁移一个未跑、二进制未换；
- [ ] 正常空间下含迁移 job 的 pg_dump 产物大小与库体积量级相符
      （非空壳），且演练中实测一次「从该 dump 恢复到临时库成功」。

## D-5 正常全链路（对照组）

清单 migrations 只含 901（正常件），backend 为真新二进制（版本端点
返回新 gitSha），外加一个静态面（admin）同 job。

断言：
- [ ] 状态流完整走过 migrating → swapping（后端先）→ health_checking
      → done；`sys_schema_migration` 落 901 行（applied_by='hot-update'、
      job_id 正确、sha 与清单一致）；
- [ ] 后端版本端点 = 新版本；admin 入口指纹 = 新版；顺序上迁移完成
      时间早于二进制替换时间（查 job 日志时间戳）；
- [ ] 再发一个同版本 job → 迁移阶段「无待执行」直接通过，不重复执行。

## 演练记录

每次演练把以下留档到 `~/workspace/hot-update-design/drill-logs/<日期>/`：
job 文件、status.json 全程快照、migrate-result.json、updater 日志、
断言逐项勾选结果、克隆库演练前后 `sys_schema_migration` 导出。
任一断言不过：停演、修脚本/代码、从 D-1 重演全程，不许只补跑失败项。
