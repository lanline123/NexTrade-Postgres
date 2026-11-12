# M4 · 跨境发票/税务（components/invoice + member/userInvoice）

> 模块归属：执行章程 §二 M4（泳道 B · 跨境旁路线）
> 来源仓库：`NexTrade-All/nextrade-overseas-pc`
> 开关：`isOverseas()`
> 创建：2026-07-03

## S1 范围

**IN**：
1. `components/invoice/InvoiceDialog.vue` —— 跨境独有共享发票弹窗（整文件迁移，@vort-if overseas）。跨境版中被 `pages/order/check.vue`（M10 范围）与 `pages/member/orderInvoice/List.vue` 复用；本模块先迁文件，M10 结算功能块接入时直接使用。
2. `pages/member/userInvoice/Info.vue` —— 增票资质表单跨境校验增强（`isOverseas()` 分支）：
   - 纳税人识别码/注册电话/银行账户"不能包含中文"校验器
   - 提交时 `isAdd` 标记（新增 vs 更新）

**OUT**（记录理由）：
- `pages/member/userInvoice/confirmInfo.vue`：跨境差异仅为头部组件（PageHeaderNew）与容器 margin，属跨境 UI chrome，不在本模块。
- `pages/member/orderInvoice/List.vue`：o2o 版本有 `commonStore.canInvoice` 门（o2o 权威保留）；跨境版差异为 chrome + seoLink 跳转（属商品详情模块）+ 弹窗组件路径切换。o2o 继续使用自己的 `orderInvoice/Info.vue`（其比跨境版多发票图片附件展示功能）。
- `api/user/userInvoice.ts`、`types/user/userInvoice.d.ts`：三方 diff 完全一致，不动。

## S2 盘点结论（2026-07-03）

| 文件 | 标准版 | 跨境版 | o2o | 结论 |
|---|---|---|---|---|
| `components/invoice/InvoiceDialog.vue` | ✗ | ✓ | ✗ | 整文件迁移 |
| `pages/member/userInvoice/Info.vue` | ✓（≈o2o） | ✓（校验增强） | ✓ | 加 isOverseas 分支 |
| `pages/member/userInvoice/confirmInfo.vue` | ✓（≈o2o） | ✓（仅 chrome） | ✓ | 不动 |
| `pages/member/orderInvoice/Info.vue` | — | 已移位为 InvoiceDialog | ✓（多附件展示） | o2o 保留 |
| api/types | 一致 | 一致 | 一致 | 不动 |

- 后端依赖：发票接口 `user/orderInvoice/*`、`user/userInvoice/*` 三仓库一致，无新增后端依赖。`isAdd` 为跨境后端已有入参，仅在 `isOverseas()` 下发送。
- 开关回归：`VITE_IS_OVERSEAS=0` 时表单校验规则与提交参数和改动前完全一致。

## S3 Task 拆分

| Task | 内容 | 预算 |
|---|---|---|
| T1 | 迁 `InvoiceDialog.vue` + userInvoice/Info.vue 校验增强 | ~560 行 / 2 文件 |

## S5 验收

- [x] `npm run build` 通过（含 `@vort-if` 配对校验）
- [x] 版本代码 `isOverseas()` 包裹 + `@vort-if overseas` 标记
- [x] 迁移文件头部 `// 源：` 注释
- [x] 更新执行章程 §四进度表、§五基线清单
