# M11 · B2B 批发整块（wholesale 阶梯价子能力）

> 模块归属：执行章程 §二 M11（泳道 C · 交易与大块头线）
> 来源仓库：`NexTrade-All/NexTrade-Pc`（标准版 wholesale 源码完整；跨境版嵌入逻辑同构，`check.vue` 用 FormatPrice 多币种）
> 开关：`isB2B()`（身份/批量采购）+ `common.wholesale`（批发阶梯价，`@vort-if common.wholesale`）
> 创建：2026-07-04
> ⚠️ 合法组合：B2B 可叠加跨境（跨境批发）；o2o 与 B2B 互斥（需求方案 §1.2.1 组合矩阵）

## 起点核实（已用 Test-Path / Select-String 逐条确认，2026-07-04）

- ✅ 标准版 `NexTrade-Pc` **wholesale 专用源码完整存在**（5 组件 + 4 hooks + api/store/types）。
- ✅ B2B 页面群（`inquiry`/`termPay`/`userCertification`、`api/user/inquiry.ts`/`termPay.ts`、`B2bTab`/`B2bSku`/`ProductEnquiry`）**o2o 中已存在**，本模块**不重复迁**。
- ✅ **购物车侧批发在 o2o 已完成且自洽**（推翻此前"全无"判断）：`components/wholesale/WholesaleCartStepper.vue`、`hooks/useWholesaleCart.ts`、`hooks/useWholesaleQuantity.ts` 均在，`pages/cart/index.vue` 已带 `@vort-if common.wholesale` 接线（步进器/勾选拦截/`validateCheckout` 全在）。且 `useWholesaleCart` / `WholesaleCartStepper` **仅依赖 `useWholesaleQuantity` + utils/message**，不依赖任何缺失文件 → 购物车侧无需重迁，本轮仅回归验证。
- ❌ o2o 缺的是**"商品详情 + 下单侧"批发**（真实迁移对象，~15 文件）：
  - `components/wholesale/`：**WholesaleBuyBar / WholesaleCartTipBar / WholesalePriceTable / WholesaleSkuSelector**（4，CartStepper 已在）
  - `hooks/`：**useWholesaleIdentity / useWholesaleOrder**（2，Cart/Quantity 已在）
  - `api/wholesale/`（cart.ts/me.ts/product.ts）、`store/wholesale.ts`、`types/wholesale/`（5）全无
- ⚠️ o2o `ProductInfo.vue` 的 `b2b.batchBuy` 条件**缺 `isB2B()` guard**（既存缺陷），本模块顺带修正（需求方 2026-07-04 确认）。

## S1 范围

**IN**：
1. **迁入缺失的 wholesale 叶子文件（~15 个，从 `NexTrade-Pc` copy + 打 `@vort-if common.wholesale` / 头部 `// 源:` 注释）**——仅补详情/下单侧，购物车侧（CartStepper/useWholesaleCart/useWholesaleQuantity）已在，不重迁：
   - `components/wholesale/`：WholesaleBuyBar(77) / WholesaleCartTipBar(87) / WholesalePriceTable(341) / WholesaleSkuSelector(615)
   - `hooks/`：useWholesaleIdentity(61) / useWholesaleOrder(613)
   - `api/wholesale/`：cart.ts / me.ts / product.ts
   - `store/wholesale.ts`(40)
   - `types/wholesale/`：buyRule / cart / identity / price / sku.d.ts
2. **共享组件三方嫁接 `common.wholesale` 层**（详见 §S2 冲突表；不能 git-copy，需增量合并）：
   - `components/product/ProductInfo.vue`（高冲突：actions 区包 `!isWholesale` + 插 wholesale 分支 + script 宿主）
   - `pages/item/src/Product.vue`（中：新增 `isWholesaleMode`/`wholesalePriceList` + fetch 解析 + 两处传 props）
   - `components/product/FloatBuyBar.vue`（中：插 wholesale 按钮块 + props + handler）
   - `pages/order/check.vue`（补批发标签/券·积分排除提示；跨境用 FormatPrice 多币种）
3. **购物车侧批发（需求方 2026-07-04 确认纳入）**：o2o `pages/cart/index.vue` 已接线且自洽，**本轮不改代码，仅回归验证**（批发行步进/勾选拦截/结算校验在 `common.wholesale` 开时正常、裁剪后零售流不受影响）；若详情/下单侧迁入后需与购物车 `useWholesaleCart` 对齐，再做最小微调。
4. **修正既存缺陷**：`ProductInfo.vue`/相关 batchBuy 条件补 `isB2B()` guard，使 `isB2B() && bulkPurchase && !isWholesale` 判据自洽。
5. `.vort/feature-manifest.json` 补 `common.wholesale` includes（新增叶子文件 build 裁剪；核对购物车侧已有条目）。

**OUT**（记录理由）：
- `inquiry` / `termPay` / `userCertification` 页面与 API：o2o 已存在，不重复迁（仅校验 Nav 链接可达）。
- `B2bTab` / `B2bSku` / `ProductAmount` / `ProductEnquiry`（`b2b.batchBuy`）：o2o 已有，不迁。
- 跨境 `pages/product/` 整页：属阶段二 R1，不引入；wholesale 落点用 o2o 现有 `pages/item/src/Product.vue`。
- 门店履约块（自提/预约/配送）：M11 不得触碰。

## S2 盘点结论（共享组件冲突已做只读 diff，2026-07-04）

| 文件 | o2o 现状（PD 改过） | 标准版（含 wholesale） | 冲突度 | 嫁接动作 |
|---|---|---|---|---|
| `ProductInfo.vue` | 0 处 wholesale；batchBuy 缺 `isB2B()`/`!isWholesale`；零售 actions 直连 | actions 包 `!isWholesale` + 前插 `isWholesale` 分支；价格区逐条件加 `!isWholesale`；script `useWholesaleOrder` 宿主 | **高** | actions 区重组 + 30+ 触点增量嫁接 |
| `pages/item/src/Product.vue` | 无 b2b/无 wholesale（仅 overseas/o2o 标记） | fetch 读 `result.wholesalePriceList` 驱动 `isWholesaleMode`；两处传 props | 中 | 纯新增，认准 PD 版 fetch/模板落点 |
| `FloatBuyBar.vue` | 有 `b2b.batchBuy`+`showEnquiryBtn`；无 wholesale | 加 `common.wholesale` 按钮块 + props + handler | 中 | 与既有 batchBuy 并列插入 |
| `check.vue` | 有账期支付；无 wholesale | 批发标签 + 券/积分排除提示 | 中 | 补 wholesale UI 块（`@vort-if` 精确定界） |
| `pages/cart/index.vue` | **已接线 wholesale（CartStepper/validateCheckout）且自洽** | 同构 | 无 | **不改，仅回归验证** |
| 详情/下单侧叶子文件（~15） | 全无 | 完整 | 低 | copy + `common.wholesale` |
| 购物车侧叶子（CartStepper/useWholesaleCart/useWholesaleQuantity） | **已在，仅依赖 Quantity+utils** | — | 无 | 不迁 |
| `.vort/feature-manifest.json` | 有 `b2b`、无 `common.wholesale` | — | 低 | 补 includes（核对购物车侧现有条目） |

- 跨境 vs 标准：嵌入逻辑同构；差异在 `check.vue` 跨境用 `FormatPrice`（叠加场景以此为准）。迁移基准 = `NexTrade-Pc`。
- 后端依赖：`api/wholesale/{cart,me,product}.ts` 对应后端接口 —— ⬜ **S2 待向 `NexTrade-Api-Java` 核实 ready**。
- env：`isB2B()` 已定义（`VITE_IS_B2B`），o2o 三份 env 均有；o2o dev 三开关同开与"o2o∩B2B 互斥"冲突，生产 profile 需明确（仅 dev 本地配置）。

## S3 Task 拆分（待 S2 后端核实后细化）

| Task | 内容 | 预算（估） |
|---|---|---|
| T1 | 迁 ~15 个详情/下单侧 wholesale 叶子文件 + manifest includes（compile-only 先通过构建） | ~15 文件 / ~1900 行 |
| T2 | `ProductInfo.vue` 嫁接 wholesale（高冲突，单独一 Task） | ~1 文件大改 |
| T3 | `Product.vue` + `FloatBuyBar.vue` 嫁接 + 修 batchBuy `isB2B()` guard | ~2 文件 |
| T4 | `check.vue` 批发提示 + 券/积分排除 | ~1 文件 |
| T5 | 购物车侧 wholesale 回归验证（不改代码，验开关开/关两态） | 验证项 |

## S5 验收（DoD）

- [ ] `npm run build` + `npm run vort:check` 通过（`@vort-if common.wholesale` 配对校验）
- [ ] wholesale 代码 `@vort-if common.wholesale` 标记；batchBuy 代码 `isB2B()` guard 齐全
- [ ] **开关回归**：`common.wholesale` 裁剪 / `VITE_IS_B2B=0` 时，o2o 站商品详情/购物车/结算行为与改动前完全一致（零售流零变化）
- [ ] **PD/CART 回归**：商品详情、购物车专项迁移的既有功能未被 wholesale 嫁接破坏
- [ ] 基线回归：跑 §五清单
- [ ] 更新执行章程 §四进度表、§五基线清单、§六（后端接口 + batchBuy guard 修复记录）
