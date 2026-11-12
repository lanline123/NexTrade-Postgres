# M5 · 礼品卡增强（transfer 对照补差异）

> 模块归属：执行章程 §二 M5（泳道 B · 跨境旁路线）
> 来源仓库：`NexTrade-All/nextrade-overseas-pc`
> 创建：2026-07-03

## S2 盘点结论（2026-07-03）—— 与章程预期不同

**o2o 已拥有完整礼品卡体系**（`pages/giftCard/` 含 `transfer/[id].vue`、`src/giftCard.vue`、`giftCardActions.vue`、`productItem.vue`；`api/giftCard`、`types/giftCard` 与跨境版完全一致）。章程"对照 o2o 现有 giftCard 补差异"执行后，实际差异极小：

| 文件 | 差异 | 处置 |
|---|---|---|
| `pages/giftCard/src/productItem.vue` | o2o 同时渲染 `tig-image` + `el-image` 双图（跨境版已修复为单图并将样式边框图移出链接） | **补修复**（默认分支通用修复） |
| `pages/member/giftCard/exchange.vue` | 跨境版标签宽度按语言自适应（英文 90px 防换行）+ label nowrap | **isOverseas() 分支迁入** |
| `pages/giftCard/src/giftCardActions.vue` | 仅 padding 差异（跨境版式） | 不动（chrome） |
| `pages/giftCard/transfer/[id].vue` | 头部组件（PageHeaderNew）+ 变量重命名 | 不动（chrome / 禁止重命名） |
| `api/giftCard/giftCard.ts`、`types/giftCard/giftCard.d.ts` | 无差异 | 不动 |

- 后端依赖：无新增。
- 开关回归：`VITE_IS_OVERSEAS=0` 时 exchange 表单 label-width 维持 52px、无 nowrap class，与改动前一致；productItem 双图修复属通用 bug 修复（o2o 站视觉去重，行为无回退）。

## S3 Task 拆分

| Task | 内容 | 预算 |
|---|---|---|
| T1 | productItem 双图修复 + exchange 标签宽度 i18n 适配 | ~40 行 / 2 文件 |

## S5 验收

- [x] `npm run build` 通过
- [x] 版本相关代码 `isOverseas()` 包裹 + `@vort-if` 标记
- [x] 更新执行章程 §四进度表、§五基线清单
