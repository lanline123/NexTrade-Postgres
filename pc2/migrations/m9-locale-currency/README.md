# M9 · 语言/币种切换增强（Price.vue、切换器）

> 模块归属：执行章程 §二 M9（泳道 B · 跨境旁路线；红线：全局热点组件，与 M10 时序错开——M10 未启动，已确认无冲突；`components/format/Price.vue` 当前无并行会话改动）
> 来源仓库：`NexTrade-All/nextrade-overseas-pc`
> 创建：2026-07-03

## S1 范围

**IN**：
1. `components/format/Price.vue` —— 增量补 `priceStyle` prop（默认 `{}`，不传时渲染与改动前逐字节一致；为后续迁移页面传自定义价格样式做准备，跨境版 6 个会员页在用）。
2. `store/i18n.ts` —— **默认分支 bug 修复**（SSR/多标签页语言切换失效）：cacheLang 解析 try/catch、Cookie 与本地缓存语言不一致时废弃旧缓存、缓存写入与 reacquireLanguage 清理仅限客户端、init/fetch 链路 async 化、新增 `clearCache()`。
3. `utils/versionManage.ts` —— 新增（@vort-if overseas）：后台 version/translationVersion 变更时自动清理语言缓存并重拉翻译。
4. `store/common.ts` —— 增量补 `version`/`translationVersion` 字段 + `loadBaseConfig` 客户端侧 `isOverseas()` 守卫调用 versionManage。

**OUT**（记录理由）：
- `components/locate/LanguageChangeNew.vue`：o2o 版为**更新的重构版**（CommonPopover、SSR 友好内联 SVG、防抖 storage 刷新、triggerMode/langLabelMode 扩展 props），优于跨境版（a-popover + 硬编码 top 偏移）。o2o 权威，不迁。跨境版"显示语言短码 EN"的 UX 差异属外观口味，不构成功能缺口。
- `components/locate/LanguageChange.vue`：仅 padding-left 5px→10px 外观差异，o2o 权威。
- Price.vue 的 `price-row` DOM 重构与 `completeDecimal` prop 删除：全局热点组件改 DOM 结构会波及全站深度选择器样式，且 o2o 的 completeDecimal 是更强的能力（跨境版反而删了），均不迁。
- `store/i18n.ts` 中 `currentLanguage = localeCode.toUpperCase()`（跨境版把语言显示名改为大写短码）：外观差异，保留 o2o 的 `result.language`。

## S2 盘点结论（2026-07-03）

| 项 | 结论 |
|---|---|
| `store/currency.ts` | 两仓库逐字节一致，无需动 |
| `versionManage` 依赖 | 仅依赖 common/i18n store；`version`/`translationVersion` 为空时自动 return，o2o 后端未返回也安全 |
| 热点检查 | `git status` 确认 Price.vue/locate/ 无并行会话未提交改动 |
| i18n 修复定性 | 属"语言切换无效"类通用 bug 修复（先例：M2/M5 默认分支修复），非跨境专属 |

## S3 Task 拆分

| Task | 内容 |
|---|---|
| T1 | Price.vue 增量 priceStyle prop |
| T2 | i18n store SSR 修复 + clearCache |
| T3 | versionManage + common store 字段与守卫调用 |

## S5 验收

- [x] `npm run build` 通过（含 @vort-if 校验）
- [x] Price.vue 不传 priceStyle 时输出与改动前一致
- [x] 跨境开关关时 versionManage 不执行（isOverseas 守卫）
- [x] 更新执行章程 §四进度表、§五基线清单
- [ ] ⚠️ 人工回归提醒：语言切换（含 SSR 站点 `.env.test`）、币种切换、多标签页同步刷新
