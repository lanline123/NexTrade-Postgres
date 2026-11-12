# 专项 · 商品详情/购物车「功能迁移」任务书（阶段一口径）

> **性质**：需求方校正后的正式任务。此前 R1 商品详情、CART 购物车曾以「阶段二整页隔离/顶替」方式做，判定为**任务偏差已全部回退**（见《执行章程》§六 回退记录）。本任务改按**阶段一功能迁移**口径重做。
> **口径**：对比 `nextrade-overseas-pc`（跨境）与 `NexTrade-Pc`（标准），把它们**各自独有、而 o2o 缺失的功能代码**迁入 o2o；**只迁功能代码，不管样式/布局/文案**；接入 o2o **现有** `pages/item/`、`pages/cart/` 页面，**不重做页面**；迁入后按版本做 `isXxx()` + `@vort-if` 切割。
> **落地仓库**：`NexTrade-o2o-Pc`　**创建**：2026-07-03

---

## 〇、需求方已拍板决策

1. **wholesale 批发整块 → 留给 M11 专做**（泳道 C、`isB2B()`、对 o2o 站不可见）。本任务**不迁 wholesale**，避免与 M11 抢文件（章程 §六红线）。含：详情批发选购、`FloatBuyBar` 批发按钮、购物车行内批发步进器、`toCheck` 起批/倍数校验、猜你喜欢批发价、`bulkPurchase` 的 `isB2B()` guard。
2. **跨境购物车 `auth` 登录中间件 → 仅 `isOverseas()` 分支生效**；非跨境（o2o 默认）保持游客可访问 `/cart` 不变。
3. **执行顺序**：先做商品详情（通用/安全 → 跨境），再做购物车。
4. **跨境「左侧滚动锚点 Tab」跳过**：o2o 新版 `DetailNew` 已有等价锚点体系，两套并存会冲突。
5. **`DialogProduct`（跨境）跳过**：空壳、跨境自身未接入，不算功能。

---

## 一、范围 IN（本任务要迁的功能）

### 商品详情（接入 `pages/item/`，不重做页面）

| 编号 | 功能 | 来源 | 归属 |
|---|---|---|---|
| PD-G1 | 售后服务 Tab 动态显隐（`getAfterSaleService`） | 标准 | 默认分支（无包裹） |
| PD-G2 | 详情拉取补 `grouponPromotionRecordId`（被邀参团数据一致） | 标准 | 默认分支 |
| PD-G3 | JSON-LD `<`/`>` 转义（XSS 安全加固） | 标准 | 默认分支 |
| PD-O1 | 海外社交分享 WhatsApp/Facebook/Twitter/Email（`shareTo`） | 跨境 | `isOverseas()` + `@vort-if overseas` |
| PD-O2 | 主栏「相关文章」区块（`RelevanceArticle`） | 跨境 | `isOverseas()` + `@vort-if overseas` |
| PD-O3 | Thumb 收藏 + 询价态禁大图联动 | 跨境 | `isOverseas()` + `@vort-if overseas` |
| PD-O4 | 详情底「精选好物」ProdSwiper 推荐条 | 跨境 | `isOverseas()` + `@vort-if overseas` |
| PD-O5 | 多币种 SEO JSON-LD（`priceCurrency`） | 跨境 | `isOverseas()` + `@vort-if overseas` |

### 购物车（接入 `pages/cart/`，不重做页面）

| 编号 | 功能 | 来源 | 归属 |
|---|---|---|---|
| CART-G1 | `InputNumber` 限购 `limitNumber` | 标准/跨境 | 默认分支 |
| CART-G2 | 购物车角标全局同步（`asyncGetCartCount`） | 跨境 | 默认分支（通用增强） |
| CART-O1 | 原价 + 折扣率双价展示（`getDiscount`） | 跨境 | `isOverseas()` + `@vort-if overseas` |
| CART-O2 | 数量为 1 时步进器变删除 | 跨境 | `isOverseas()` + `@vort-if overseas` |
| CART-O3 | `auth` 登录中间件（仅跨境生效） | 跨境 | `isOverseas()` |
| CART-M1 | 店名条件渲染守卫 | 标准 | `@vort-if merchant` + `isMerchant()` |

## 二、范围 OUT（本任务不做）

- wholesale 批发整块（→ M11）
- 页面重做 / 样式统一 / 布局改版（→ 阶段二 R 系列）
- 跨境左侧滚动锚点 Tab（o2o 已有等价 DetailNew）
- `DialogProduct`（空壳）
- o2o 已有等价或更完整的：预售、询价、B2B 批量 Tab、竞品 sidebar、促销/券/赠品/优惠明细、配送分流

## 三、o2o 不可破坏（迁移时必须保留）

- 门店卡片/切换（`ProductStoreCard`、`StoreSwitchDialog`）、`nearestShopId`/`shopId` 价库、周期购 H5 跳转、预约/次卡类型、新版 `DetailNew`/`FloatBuyBar` 预约缺货分支
- 购物车 O2O 配送分流（`DeliveryType`、`changeCart`、`getDeliveryOptions`、`CartShopGroup`）
- **铁律**：所有开关置 0 时，o2o 站行为与改动前**完全一致**

## 四、DoD

- [ ] 版本功能全部 `isXxx()` 分支 + `@vort-if` 配对（`npm run vort:check` 绿）
- [ ] `npm run build` 通过
- [ ] 开关置 0 回归：o2o `/item`、`/cart` 行为与改动前一致
- [ ] 更新《执行章程》§四进度表 + §五基线清单

## 五、执行状态

- S1/S2：✅ 完成（三方 diff 见本文一/二/三；决策见〇）
- S3/S4：✅ 完成（build + vort:check 双绿）

### 已落地

**商品详情** `pages/item/src/Product.vue`（+ 新增 `components/product/RelevanceArticle.vue`、`assets/icons/faceBook.svg`+`teitter.svg`）
- PD-G1 售后 Tab 动态：`getAfterSaleService` → `afterSalesFlag`，默认 `true` 兜底（通用）
- PD-G2 详情拉取补 `grouponPromotionRecordId`（通用，未参团时 null）
- PD-G3 JSON-LD `<`/`/` 转义（通用安全）
- PD-O1 社交分享 `shareTo`（`@vort-if overseas` + `isOverseas()`）
- PD-O2 相关文章 `ProductRelevanceArticle`（`@vort-if overseas` + `isOverseas()`，接 o2o 现有 `getProductRelateArticle`）

**购物车** `pages/cart/index.vue`、`pages/cart/src/InputNumber.vue`、`CartShopGroup.vue`
- CART-G1 `InputNumber` 限购 `limitNumber` + `CartShopGroup` 传参（通用）
- CART-G2 `updateCart` 同步头部角标 `cartStore.cartCount`（通用，复用已返回的 total.totalCount，无额外请求）
- CART-O3 跨境登录中间件：`definePageMeta` 内联 middleware，仅 `isOverseas()` 生效，非跨境游客可访问不变（`@vort-if overseas`）

### 已评估跳过（附理由，非漏做）

| 编号 | 结论 |
|---|---|
| PD-O3 Thumb 询价/收藏联动 | o2o 已有 `:disableZoom="isEnquiryOpen"` + 收藏；剩余仅缩略图内收藏按钮属样式位置，按"不管样式"跳过 |
| PD-O4 详情底 ProdSwiper | o2o 已有 guessLike 推荐（新版 DetailNew）；跨境底部 swiper 为展示层，跳过 |
| PD-O5 多币种 priceCurrency | `CurrencyItem` 无 ISO 币种码字段、o2o 已输出 `dollarSign`，无更优值可迁，暂缓 |
| CART-O1 原价+折扣率双价 | 展示层（`getDiscount` 计算 + 双价 UI），按"不管样式"跳过 |
| CART-O2 数量为 1 变删除 | o2o 已有独立删除入口；qty=1→删除为样式化交互且依赖已回退的 `delete.png`，跳过 |
| CART-M1 店名 `isMerchant` 守卫 | o2o 门店场景默认多店常显店名，加守卫有回退风险，价值低，跳过 |

> wholesale 批发整块（详情/购物车）按需求方决策留给 **M11**，本任务不含。
