# M2 · 会员拼团 + 服务协议（标准版来源，默认分支）

> 模块归属：执行章程 §二 M2（泳道 A · o2o 站体验线）
> 来源仓库：`NexTrade-All/NexTrade-Pc`（标准版）
> 开关：**默认分支**（通用功能补齐，不用 `isXxx()` 包裹、无需 `@vort-if` 标记）
> 创建：2026-07-03

## S1 范围

**IN**：
1. `pages/member/account/src/serviceAgreement.vue` —— o2o 完全缺失，迁标准版文件（新增文件，非顶替，无 `.v1` 存档）。
2. `pages/member/group/List.vue` —— o2o 缺该路由（标准版会员导航/外链使用 `/member/group/list`）；o2o 已有等价页 `pages/member/joining/List.vue`，本文件仅作路由兼容补齐。
3. `pages/member/group/Info.vue` —— 补齐标准版侧 bug 修复：剩余人数按 `joinUsers.length` 计算、团长邀请图标可点击、`joinUsers` 防御性拷贝、表头 `white-space: nowrap`。
4. `types/joining/joining.d.ts` —— 增量补 `JoiningItem.groupNum?: string` 字段（o2o 组件已在使用该字段但类型缺失）。

**OUT**：
- `components/member/joining/*`：o2o 与标准版仅 2 行模板样式差异（o2o 侧为改进版），按「o2o 现有文件为权威源」原则不动。
- `api/joining/joining.ts`：接口完全一致（仅注释措辞差异），不动。
- 跨境版 `PageHeaderNew`/`TopJumpBar` 头部：属跨境 UI chrome，随跨境相关模块迁移，本模块用 o2o 现有 `CommonPageHeader`（即标准版写法）。
- o2o 导航 `components/member/Nav.vue` 已指向 `/member/joining/list`，不改。

## S2 盘点结论（三方对照 2026-07-03）

| 文件 | 标准版 | 跨境版 | o2o | 结论 |
|---|---|---|---|---|
| `pages/member/group/List.vue` | ✓ | ✓（PageHeaderNew 版） | ✗ | 迁标准版 |
| `pages/member/joining/List.vue` | ✗ | ✗ | ✓ | o2o 独有等价页，保留 |
| `pages/member/group/Info.vue` | ✓（含修复） | — | ✓（落后） | 就地补修复 |
| `pages/member/account/src/serviceAgreement.vue` | ✓ | ✓（仅头部组件不同） | ✗ | 迁标准版 |
| `components/member/joining/JoiningOrderList.vue` | ✓ | ✓（差异大，跨境样式） | ✓（≈标准版） | 不动 |
| `api/joining/joining.ts` | ✓ | ✓ | ✓（等价） | 不动 |
| `types/joining/joining.d.ts` | ✓（多 groupNum） | — | ✓ | 增量补字段 |

- 文档 §5.2(a) 提到的 `types/groupon/groupon.ts` 在三个仓库均不存在，属文档笔误，实际对应 `types/joining/joining.d.ts`。
- 后端接口：拼团 `promotion/groupon/*`、文章 `article/article/issueInfo` 均为 o2o 现有接口，无新增后端依赖。

## S3 Task 拆分

| Task | 内容 | 预算 |
|---|---|---|
| T1 | 迁 `serviceAgreement.vue` + 补 `group/List.vue` + types 增量字段 | ~210 行 / 3 文件 |
| T2 | `group/Info.vue` 补齐标准版修复 | ~30 行 / 1 文件 |

## S5 验收

- [x] `npm run build` 通过（含 `@vort-if` 配对校验）
- [x] 默认分支通用补齐，无版本开关代码，无需开关回归
- [x] 迁移文件头部有 `// 源：` 注释
- [x] 更新执行章程 §四进度表、§五基线清单
