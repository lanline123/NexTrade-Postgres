# M1b · 店铺街列表样式统一（行卡片布局）

> 模块归属：M1 店铺页的样式口径修订（承 M1 §六决策记录）
> 来源仓库：`NexTrade-All/nextrade-overseas-pc`（跨境版为准，与标准版 `NexTrade-Pc` 一致）
> 创建：2026-07-13

## 背景

M1 整合重做时，店铺街 `/shop/list` **保留了 o2o 原 2 列网格布局**，仅并入 综合/评分 Tab + keyword 搜索（见 `migrations/m1-shop-pages/README.md` 与执行章程 §六 2026-07-04「M1 整合重做」行）。

需求方于 2026-07-13 变更口径：**店铺街样式统一为标准版/跨境版的行卡片布局，旧 o2o 网格弃用，以跨境版/标准版为准。**

## S1 范围

**IN**：
1. 新增行卡片卡片组件（源：跨境版 `pages/shop/src/`，跨境版为标准版超集，含 seoLink / i18n / 商品样式边框）：
   - `pages/shop/src/ShopInfo.vue` —— 店铺信息卡（Logo + 店名 + 评分 el-rate + 粉丝数 + 进入店铺按钮）
   - `pages/shop/src/ShopList.vue` —— 店铺热销商品横向滚动（左右箭头 + 商品样式边框 useProductStyle）
2. 重写 `pages/shop/List.vue`：以跨境版/标准版行卡片布局为基础（`.shop-row` = `<ShopInfo>` + `<ShopList>`），**保留 o2o 已有能力**：
   - 综合/评分 Tab（`sortField`）、keyword 搜索（watch `route.query.keyword`）
   - o2o 原 SEO 块（title/description/keywords/canonical/robots，M1 判定 o2o SEO 权威，不回退）
   - o2o 站头/脚 chrome：`CommonPageHeader` / `CommonHeader` / `CommonPageFooter`（不换 overseas 的 PageHeaderNew）
   - 新增 `productNumber: 12` 入参（与标准版/跨境版一致，供横向滚动展示足量商品）

**样式统一属全版本生效（o2o/标准版/跨境版），不做 `@vort-if` 版本裁剪**——本页 `List.vue` 本就是三版共用、无版本守卫。

**OUT / 决策**：
- 旧 o2o 2 列网格结构（`.shop-list` grid + 内联卡片）——删除。
- `pages/shop/components/ShopInfo.vue` / `ShopList.vue`（o2o 自有 432px 重设计行卡片）——**不动**，仍由 `pages/search/components/SearchShopList.vue`（`@vort-if merchant`）使用；本次仅改店铺街，不并搜索页。
- 进入店铺图标：跨境版源用 `assets/images/shop/shop_icon.png`，该 png 在 o2o 仓为 `@vort-if merchant` 死引用、o2o 构建不含此二进制 → 用 o2o 现有 `assets/icons/shop.svg`（`<tig-icon name="shop">`）替代，视觉等价、避免缺失资源导致 o2o 构建失败。

## S2 盘点结论（2026-07-13）

| 项 | 结论 |
|---|---|
| 类型字段 | `shopListItem` 已含 `shopLogo` / `shopStar?` / `collectCount` / `seoLink?` / `listingProduct.picUrl/picThumb/seoLink/productSn`，无需改 `types/shop/shop.d.ts` |
| API | `getShopList` 已有，`productNumber` 作为透传 params 由后端消费，无需改 API |
| 依赖 | `@element-plus/icons-vue`（ArrowLeft/Right）、`el-rate`、`useProductStyle`（`~/hooks`）、`ratingFormat`（`@/utils/format`）、`FormatPrice`（`@/components/format/Price.vue`）o2o 全部已有 |
| 资源 | `shop_icon.png` o2o 无 → 用 `assets/icons/shop.svg`（`tig-icon`）；商品边框由 `useProductStyle` 生成，无静态资源依赖 |
| 版本守卫 | `List.vue` 三版共用无 `@vort-if`；样式统一为全版本生效，符合"以跨境版/标准版为准" |

## S5 验收

- [ ] `npm run vort:check` 通过
- [ ] `npm run build` 通过
- [ ] 人工回归：
  - 店铺街 `/shop/list` 呈行卡片布局（左店铺信息卡 + 右商品横向滚动），综合/评分 Tab 与分页正常
  - keyword 搜索（`/shop/list?keyword=xxx`）正常、特殊字符不丢参、SEO title 随 keyword 变化
  - 评分星级、粉丝数、进入店铺按钮显示正常；商品样式边框（styleMap）正常叠加
