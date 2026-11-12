# M3 · 跨境合规：Cookie/GDPR 同意条（useConsent）

> 模块归属：执行章程 §二 M3（泳道 B · 跨境旁路线）
> 来源仓库：`NexTrade-All/nextrade-overseas-pc`
> 开关：`isOverseas()`（叠加 `isDemo()`，与跨境版现状一致）
> 创建：2026-07-03

## S1 范围

**IN**：
1. 新文件（跨境独有，整文件迁移）：
   - `composables/useConsent.ts` —— 同意状态读写（cookie 持久化、政策版本升级重弹窗）
   - `components/common/PrivacyConsent.vue` —— 首次弹窗 banner + 自定义抽屉 UI
   - `plugins/consent-scripts.client.ts` —— 三方脚本受同意约束的懒加载骨架（内部已自带 `isOverseas() && isDemo()` 守卫）
2. 接入点（共用文件，`isXxx()` 分支 + `@vort-if` 标记）：
   - `app.vue` —— 挂载 `<PrivacyConsent>`（`isOverseas() && isDemo()` 条件）
   - `components/common/PageFooter.vue` —— 页脚「Cookie 设置」入口链接
   - `components/common/StatisticLog.vue` —— 埋点上报前检查 analytics 同意（非跨境/非演示恒通过，行为不变）

**OUT**：
- 三方 SDK 实际接入（plugin 为骨架，跨境版自身也未接入）。
- `.vort/feature-manifest.json` overseas.includes 补录：**编辑但暂不提交**——该文件正被商品详情迁移会话并发修改，避免夹带（见章程 §六记录）。

## S2 盘点结论（2026-07-03）

| 文件 | 标准版 | 跨境版 | o2o | 结论 |
|---|---|---|---|---|
| `composables/useConsent.ts` | ✗ | ✓ | ✗ | 整文件迁移 |
| `components/common/PrivacyConsent.vue` | ✗ | ✓ | ✗ | 整文件迁移 |
| `plugins/consent-scripts.client.ts` | ✗ | ✓ | ✗ | 整文件迁移 |
| `app.vue` | — | 挂载点 | 无 consent | 加 `isOverseas()` 分支 |
| `components/common/PageFooter.vue` | — | 有 Cookie 设置入口 | 无 | 加 `isOverseas()` 分支 |
| `components/common/StatisticLog.vue` | — | 有 consent 门 | 无 | 加 consent 门（非跨境恒通过） |

- o2o 无 `isOverseas()` 残留 consent 分支（三方对照无冲突）。
- 后端依赖：无（纯前端 cookie 方案）。
- i18n：文案 key 为中文原文，i18n 采用"缺 key 显示原文 + 自动上报缺失 key"机制，无需前置补语言包。
- 开关回归：`VITE_IS_OVERSEAS=0` 时 `PrivacyConsent` 不渲染、页脚无入口、埋点门恒通过，o2o 站行为与改动前一致。

## S3 Task 拆分

| Task | 内容 | 预算 |
|---|---|---|
| T1 | 迁 3 个新文件（useConsent / PrivacyConsent / consent-scripts） | ~620 行 / 3 文件 |
| T2 | 3 个接入点加 `isOverseas()` 分支 + `@vort-if` 标记 | ~30 行 / 3 文件 |

## S5 验收

- [x] `npm run build` 通过（含 `@vort-if` 配对校验）
- [x] 版本代码 `isOverseas()` 包裹 + `@vort-if overseas` 标记
- [x] 迁移文件头部 `// 源：` 注释
- [x] 开关置 0 时 o2o 站行为不变（组件不挂载、门恒通过）
- [x] 更新执行章程 §四进度表、§五基线清单
