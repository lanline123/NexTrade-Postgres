# M10 · 跨境海外支付渠道 + 结算功能块（接入现有 check.vue / pay.vue）

> 模块归属：执行章程 §二 M10（泳道 C · 交易与大块头线）
> 来源仓库：`NexTrade-All/nextrade-overseas-pc`
> 开关：`isOverseas()`
> 创建：2026-07-04
> ⚠️ 支付链路高风险：默认 o2o 行为零变化，新分支仅 `isOverseas()` 开启才生效（需求方案 §6.3）

## 前置阻塞（S2 必须先闭环，未闭环不进 S3）

- 🔴 **后端 stripe 接口 ready 未确认**（执行章程 §六挂账）：前端 `api/order/pay.ts` 封装已齐（`create`/`createDepositPayment`/`checkStatus`），但需向 `NexTrade-Api-Java` 核实：
  1. `order/pay/index` 的 `paymentList` 是否返回 `stripe`（及是否含 `antom`/`xendit`）；
  2. `order/pay/create?type=stripe` 是否返回可用的 `payInfo.url` 或 `payInfo.html`；
  3. 预售定金 `createDepositPayment` 是否支持 stripe。
- 未 ready → 本模块阻塞并回写执行章程 §六，不强行迁前端。

## S1 范围

**IN**：
1. `pages/order/pay.vue` —— `isOverseas()` 分支接入 **Stripe** 在线支付（跨境版为内联实现，非独立页）：
   - 支付方式 radio 增加 stripe 选项（`@vort-if overseas`）；
   - 引入 `creatPay` / `createDepositPayment`、`handleStripePay`、`paying` 状态；
   - `payLink` / 跳转逻辑按 `payInfo.url|html` 分流。
2. `pages/order/check.vue` —— 发票弹窗接线：把发票引用从旧 `pages/member/orderInvoice/Info.vue` 切到 M4 已迁的 `components/invoice/InvoiceDialog.vue`（仅 `isOverseas()` 下），**不得触碰 o2o 自提/预约/配送/联系人履约块**。
3. 支付图标资源：补 `assets/extra/payment_method_*`（stripe 等）缺失静态资源，防运行时 404。

**OUT**（记录理由）：
- ~~`pages/order/stripe.vue`~~：需求方案路径过时，跨境仓库**无此文件**，Stripe 内联在 `pay.vue`，不新建独立页。
- `antom` / `xendit`：跨境版**自身也是半成品**（有 UI 选项、无 `payLink` 实现）。**需求方确认不纳入本轮（2026-07-04）**，待跨境版补全或需求明确后另说。
- `pages/order/src/CouponInfo.vue` 跨境新版 UI（可用/不可用 Tab）：**需求方确认不接入（2026-07-04）**——功能等价、差异属交互样式，归阶段二页面改造；M10 只做支付功能，保持 o2o 老版券区不变。
- `pages/order/src/PayType.vue`：三方功能等价，仅老 Tab vs el-radio 样式差异，属阶段二样式，不动。
- `IntegralAndBalance.vue` / `InvoiceInfo.vue` / `RuleInput.vue`：三方均为占位骨架，真实逻辑内联在 `check.vue`，无功能缺口，不动。
- `paypal.vue` 及各渠道页、`api/order/pay.ts`：o2o 与跨境已对齐，不动。

## S2 盘点结论（待补：后端接口核实 + 逐文件 diff）

| 文件 | 标准版 | 跨境版 | o2o | 结论 |
|---|---|---|---|---|
| `pages/order/pay.vue` | 无 stripe | stripe/antom/xendit 内联 | 无 stripe | 加 `isOverseas()` stripe 分支 |
| `pages/order/paypal.vue` | ✓ | ✓ | ✓（已对齐） | 不动 |
| `pages/order/check.vue` 发票引用 | Info.vue | InvoiceDialog | 仍用旧 Info.vue | 切 InvoiceDialog（isOverseas） |
| `pages/order/src/CouponInfo.vue` | 老版 | 新版 Tab | 老版 | 不动（保持 o2o 老版，需求方 2026-07-04 定） |
| `antom` / `xendit` 渠道 | — | 半成品 UI | — | 不纳入（需求方 2026-07-04 定） |
| `PayType.vue` | ≈o2o | el-radio 样式 | 老 Tab | 不动 |
| `api/order/pay.ts` | 一致 | 一致 | 一致 | 不动 |
| 后端 stripe 接口 | — | — | — | ⬜ **S2 待核实** |

## S3 Task 拆分（待 S2 后细化）

| Task | 内容 | 预算（估） |
|---|---|---|
| T1 | 补支付图标资源 + `check.vue` 发票接线切 InvoiceDialog（isOverseas） | ~1 文件改 + 资源 |
| T2 | `pay.vue` stripe 分支（isOverseas）+ handleStripePay + createDepositPayment | ~1 文件改 |

## S5 验收（DoD）

- [ ] `npm run build` + `npm run vort:check` 通过（`@vort-if` 配对校验）
- [ ] 版本代码 `isOverseas()` 包裹 + `@vort-if overseas` 标记
- [ ] 迁移/改动文件头部 `// 源：` 注释
- [ ] **开关回归**：`VITE_IS_OVERSEAS=0` 时 o2o 收银台/结算行为与改动前完全一致（默认支付链路零变化）
- [ ] 基线回归：下单主链路（购物车→check→pay 可达）无破坏
- [ ] 更新执行章程 §四进度表、§五基线清单、§六（后端接口结论）
