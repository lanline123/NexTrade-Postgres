# M1 · 店铺页 shop 补齐（标准版来源）

> 模块归属：执行章程 §二 M1（泳道 A · o2o 站体验线，动默认分支）
> 来源仓库：`NexTrade-All/NexTrade-Pc`（标准版；店铺页标准版最新，跨境版亦落后于标准版）
> 创建：2026-07-03

## S1 范围

> **口径修订（2026-07-04）**：原按 A 类整页顶替（4 入口页 + `.v1` 存档）实现，经需求方判定与「阶段一只迁功能代码、接入现有页面、不重做页面」不符，已**重做为整合口径**——以 o2o 原页为底把标准版增量功能加回，删除所有 `.v1` 与因布局分歧孤儿化的组件。下表为重做后的最终形态。

**IN**：
1. **T1 新增功能组件**（o2o 缺失，保留 5 个）：
   - `pages/shop/src/`：`ShopDefaultPage.vue`、`ShopRefHeader.vue`、`ShopCategoryMenu.vue`、`ShopHeader.vue`、`CatGoodsList.vue`
   - `api/shop/shop.ts` 增量补 `getCategoryList`
   - ~~`ShopInfo.vue`/`ShopList.vue`~~（行卡片重设计，回 o2o 网格后删除）；~~`ShopBrandHeader.vue`~~（全仓无引用死代码，删除）；~~`ShopSearchLayoutClassic/Modern.vue`~~（双布局，回单布局后删除）
2. **T2 功能整合入现有 o2o 页面**（无顶替、无 `.v1`）：
   - `pages/shop/[id].vue` —— 在 o2o 原页基础上新增"无装修→默认样式首页"分支（pcDefaultStyle 判定，渲染 `ShopDefaultPage`）
   - `pages/shop/List.vue` —— 保 o2o 原**网格布局**，新增 综合/评分 Tab、keyword 搜索、`sortField` 入参
   - `pages/shop/search/src/ShopSearch.vue` —— 保 o2o 原**单布局**（含完整 SEO），并入标准版新增的默认样式店头/收藏/客服增强；不做双布局
   - `pages/shop/search/[id].vue` —— 还原为默认入口 `<ShopSearch />`（修原 `ui-version="new"` 强制 Modern 导致的 SEO 回退）
   - `pages/shop/src/ShopSearchInput.vue` —— 保留 `encodeURIComponent` 修复（3 处 URL 注入型 bug）

**OUT**（记录理由）：
- `pages/shop/src/ShopHomepage.vue`：标准版改动主要是**删减** o2o 已有的 SEO 强化（getShopDetail 拉店铺信息、buildSeoKeywords、buildShopPath canonical），o2o 权威，不迁。
- `components/shop/*` 12 个装修组件差异（ProductCard 等）：标准版 ProductCard 依赖 `hooks/useProductCardBuy`（该文件为**并行会话未提交产物**），且会**删掉 o2o 的周期购/次卡/预约（deliveryMode/productType 5,6）逻辑**——直接顶替会回退 o2o 功能。**待并行会话提交后按三方 merge 处理**，作为 M1 遗留记录于章程 §六。
- `types/shop/shop.d.ts`：o2o 的 `shopStar?: number` 比标准版 required 更宽松，o2o 权威。
- `store/search.ts`/`utils/getBuyText.ts`：仅注释措辞差异，o2o 权威。

## S2 盘点结论（2026-07-03）

| 项 | 结论 |
|---|---|
| 热点冲突 | `pages/shop/*` 未被并行会话（product/cart）触碰；新文件依赖里无 `components/product/*` |
| API 依赖 | `getShopTree`/`getShopDetail`/`getShopCollect`/`getShopHead`/`getPromotion`/`getSearchProductList` o2o 全部已有 |
| 工具依赖 | `ratingFormat`/`safeOpen`/`isMerchant`/`useProductStyle`/`Pagination` o2o 全部已有 |
| 接口兼容 | `components/common/CatGoodsList.vue` 两仓库 props/defineModel 签名一致，Classic 布局可直接用 o2o 版 |
| o2o 独有逻辑 | `pages/shop/**` 内无 isStore/deliveryMode 等 o2o 专属代码，顶替无功能回退 |

## S5 验收（整合口径重做后）

- [ ] `npm run build` + `npm run vort:check` 通过
- [x] 无整页顶替、无 `.v1` 存档；功能组件接入 o2o 现有页面
- [x] 更新执行章程 §四进度表、§五基线清单、§六决策记录（整合重做）
- [ ] ⚠️ 人工回归提醒：
  - 店铺街 `/shop/list`：网格布局不变、综合/评分 Tab 与分页、keyword 搜索
  - 店铺首页 `/shop/:id`：有装修（与改动前一致）/无装修（默认样式首页）两态
  - 店内搜索 `/shop/search/:id`：单布局渲染、SEO（title/description/keywords/canonical）完整、关键词特殊字符不丢参
