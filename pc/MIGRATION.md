# PC 前端技术栈迁移交接

> 更新时间：2026-11-13　｜　代码位置：`pc/`　｜　当前基线：`faffbbb`
>
> 本文只讲 **PC 前端从 Vue 3 / Nuxt 3 迁到 React 18 + Next 14 + Semi Design** 这件事。
> PostgreSQL / 后端 / 运维的交接见仓库根 `HANDOVER.md`。

---

## 0. 一页速览

| 项 | 现状 |
|---|---|
| 迁移方式 | **在 `pc/` 原地重写**：Vue 源码保留作 1:1 参照，React 代码写在 `src/`，两者互不参与对方构建 |
| 技术栈 | React 18.3 + Next 14.2（App Router，真 SSR）+ Semi UI 2.104 + react-i18next |
| 已完成 | **72 个页面路由 + 24 条旧 URL 重定向**（Vue 111 条路由 100% 有落点）+ 全局 Header/Footer + 装修首页 + 地区级联 + 滑动验证码 + 完整发票 + 以图搜图 |
| 验证 | `npm run build` 通过，全部 `ƒ Dynamic`；`next start` 抽查 `/`、`/search/image`、`/member/address/list`、`/member/address/editregion`、`/order/check`、`/migration` 全 200 |
| 尚未完成 | 功能深度：秒杀/拼团完整模块、IM 客服、第三方登录、账户充值/银行卡/账期还款、结算页账期与预售；首屏数据仍在浏览器端取（见 §7 第 4 项） |
| 本地启动 | `cd pc && npm run dev` → http://127.0.0.1:3006 |

**注意别混淆两套 React 前端**：

| 目录 | 说明 |
|---|---|
| `pc-semi/` | 早先重构的 React + Semi 站，**线上 `battery.recy.cc` 正在跑它**，继续维护但不再是本次目标 |
| `pc/` | 本次新重构的站，独立工程、独立端口 3006，暂不接管线上流量 |

---

## 1. 为什么换栈

- 原版 `pc/` 是 TigShop 开源 PC 前端：Nuxt 3 + Vue 3 + Element Plus + Ant Design Vue，`VITE_SSR=0`，本质是 **SPA 静态生成**（`nuxt generate` 出 `.output`）。
- 新目标：React 18 + Next 14 App Router 的**真 SSR**（用户明确要求「需要 ssr」）+ Semi Design，同时完成品牌去 TigShop、数据库口径切 PostgreSQL。
- 结果是同目录双栈并存：Vue 只读作参照，React 才是交付物。

---

## 2. 目录约定

```text
pc/
├── src/                        # ← React 实现区（唯一参与构建的代码）
│   ├── app/                    # Next App Router
│   │   ├── layout.tsx          # 根布局：读 pc_locale cookie + 挂 I18nProvider
│   │   ├── page.tsx            # 迁移看板（force-dynamic，展示 SSR 时间戳）
│   │   ├── globals.css         # 全局样式（pc-* 前缀）
│   │   ├── semi.css            # Semi 主题 CSS 本地副本（见 §4.3）
│   │   ├── cart/ product/ list/ search/ seckill/ coupon/ article/   # 购物车与商品浏览
│   │   ├── order/              # 订单：check 结算（核心版）+ pay 收银台（只读）
│   │   └── member/             # 会员中心：layout + 23 个路由 + [...slug] 兜底
│   ├── components/
│   │   ├── layout/             # SiteHeader（导航/搜索/购物车角标/登录态）+ SiteFooter
│   │   ├── HomeClient.tsx      # 看板（'use client'）
│   │   ├── MemberShell.tsx     # 会员中心外壳：Semi Layout + 侧栏 + 语言切换
│   │   ├── LocaleSwitch.tsx
│   │   ├── member/             # 24 个会员业务组件（见 §6）
│   │   ├── catalog/ cart/ product/ order/ shop/ join/   # 商品与交易
│   │   └── decorate/           # 装修模块渲染器（活动页 / 店铺装修共用）
│   ├── i18n/                   # react-i18next：index / I18nProvider / server / locales
│   └── lib/                    # client-api / format / api / routes / member-menu
├── pages/  components/  decorate/  composables/  store/  api/  ...
│                               # ← Vue 原版参照区，只读，不参与构建
├── next.config.mjs             # React 侧构建配置
├── tsconfig.react.json         # React 侧 TS 配置（根 tsconfig 仍属 Nuxt）
├── scripts/patch-next-trace.mjs
└── public/assets/              # 本地化的 OSS 资源（141 表情 + 默认头像 + 提示音）
```

规则：

1. **只往 `src/` 写新代码**。Vue 目录一个文件都不要动，它的作用是「打开对照」。
2. 页面级只放路由与数据入口；交互与 Semi 组件抽成 `'use client'` 组件。
3. 公共能力进 `src/lib`，不要在各页里重复实现。

---

## 3. 技术栈与依赖

```json
{
  "dependencies": {
    "@douyinfe/semi-icons": "^2.104.0",
    "@douyinfe/semi-ui": "^2.104.0",
    "i18next": "^26.7.1",
    "next": "^14.2.15",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-i18next": "^17.1.12"
  },
  "devDependencies": {
    "@types/node": "^22.9.0",
    "@types/react": "^18.3.12",
    "@types/react-dom": "^18.3.1",
    "cross-env": "^10.1.0",
    "prettier": "3.5.3",
    "typescript": "5.9.3"
  }
}
```

Vue 运行时依赖（nuxt / vue / element-plus / pinia / ant-design-vue / @nuxtjs/*）**已全部移除**，`*:vue` 脚本也已删除；`.vue` 源码只是文件，跑不起来也不需要跑。

**为什么不是 Vite SPA**：旧站 `VITE_SSR=0` 只说明旧实现是 SPA，不代表新实现不要 SSR。用户明确要求 SSR，因此选 Next 14 App Router，且与 `pc-semi` 的部署形态（`.next` 包 + 3005 端口）一致，将来切流量不用重做运维。

---

## 4. 关键配置（改之前先读）

### 4.1 `next.config.mjs`

```js
distDir: process.env.PC_NEXT_DIST_DIR || '.next-react',  // 与 .nuxt/.next 隔离，支持并行构建
typescript: { tsconfigPath: 'tsconfig.react.json' },      // 根 tsconfig.json 归 Nuxt，不能被 Next 改写
pageExtensions: ['tsx', 'jsx'],                           // 隔离 Vue 的 pages/，见 §9 坑 1
env: { PC_API_BASE, PC_API_PREFIX },
rewrites: ['/api', '/adminapi', '/static'] → PC_API_BASE（默认 https://battery.recy.cc）
```

### 4.2 双 tsconfig

- 根 `tsconfig.json`：Nuxt 的，`extends ./.nuxt/tsconfig.json`，**不要动**。
- `tsconfig.react.json`：Next 用，`include` 只含 `src/**`，`exclude` 掉 Vue 目录。`npm run typecheck` 跑它。

### 4.3 Semi CSS 必须本地副本

`@douyinfe/semi-ui` 的 `dist/css` **没在 package.json `exports` 里导出**，直接 `import '@douyinfe/semi-ui/dist/css/semi.min.css'` 会编译失败。
做法：把 `node_modules/@douyinfe/semi-ui/dist/css/semi.min.css` 拷成 `src/app/semi.css`，`layout.tsx` 里 `import './semi.css'`。主题升级时重新拷一次。

### 4.4 Windows 构建 trace EPERM

`next build` 末尾会同时清理 `distDir` 与异步写 `distDir/trace`，Windows 上偶发
`EPERM: operation not permitted, open '.next-react/trace'`。Next 没有官方开关，因此：

- `scripts/patch-next-trace.mjs` 给 `node_modules/next/dist/trace/report/to-json.js` 的 `reportToLocalHost` 加一行
  `if (process.env.PC_SKIP_BUILD_TRACE) return;`（幂等）；
- `prebuild` / `predev` 自动执行该补丁，`build` / `dev` 带 `cross-env PC_SKIP_BUILD_TRACE=1`。

重装依赖后补丁会丢，但 `prebuild` 会自动补回来，不用手工处理。

---

## 5. 迁移约定（接手必读）

### 5.1 路由映射

- Vue 文件路由 → Next 目录路由，**路径一律小写**（App Router 大小写敏感）：
  `pages/member/coupon/List.vue` → `/member/coupon/list`；`pages/member/order/Info.vue` → `/member/order/info?id=<orderId>`。
- 全量映射清单在 `src/lib/member-menu.ts`（含 `path → 原 Vue 文件` 对照），顶层路由进度在 `src/lib/routes.ts`。
- 未迁的子路由由 `src/app/member/[...slug]/page.tsx` 兜底承接，**不会 404**，页面上会显示对应 Vue 原件路径。

### 5.2 页面分层

```text
src/app/<route>/page.tsx      服务端组件：export const dynamic = 'force-dynamic'，只做壳与元数据
src/components/**.tsx         'use client'：Semi 组件、hooks、事件、取数
```

服务端组件里**不要直接用 Semi 组件**（它们没有 `'use client'`，SSR 阶段会踩 DOM API）；需要 Semi 就抽客户端组件。

### 5.3 取数口径

`src/lib/client-api.ts` 是浏览器侧唯一入口：

- `GET /api/<path>`、`POST /api/<path>`；
- `Authorization: Bearer <localStorage.accessToken || token>`，`credentials: 'include'`；
- 响应信封 `{ code, data, msg }`，`code !== 0` 即抛错；
- 列表统一是 `{ records, total }`（`Paged<T>`）。

**SSR 不直接打业务接口**，页面壳服务端渲染，数据由客户端按需拉取（面板按需取数，登录态留在浏览器）。

### 5.4 多语言（统一 react-i18next）

- 实例在 `src/i18n/index.ts`，资源 `src/i18n/locales/{zh_cn,en_us}.json`（key 沿用中文原文）。
- 语言 cookie `pc_locale`；根 `layout.tsx` 用 `cookies()` 读取并传 `I18nProvider`。
- `I18nProvider` 在**组件体里同步** `changeLanguage(locale)`，不是放 `useEffect` —— 放 effect 会让英文首屏先闪中文。
- 页面文案一律 `const { t } = useTranslation(); t('我的订单')`，**禁止**再用 Vue 的 `$t`。
- 新增 key 要同时补 `zh_cn.json` 与 `en_us.json`（缺失时 i18next 回落到中文 key 本身）。

### 5.5 样式

- 全局样式集中在 `src/app/globals.css`，类名统一 `pc-*` 前缀（`pc-shell` / `pc-member-body` / `pc-order-card` / `pc-remote-list` …）。
- 布局与视觉主要由 Semi 组件承担，自定义 CSS 只补布局与间距，不覆写 Semi 内部结构。
- 页面暂未做像素级对齐，Vue 的 `less` 样式**没有搬**，视觉打磨单独一轮。

---

## 6. 已完成清单

### 6.1 路由（15 个会员子路由 + 看板）

| 路由 | 组件 | Vue 基准 | 接口 |
|---|---|---|---|
| `/` | `HomeClient` | `pages/index.vue`（迁移看板，非首页还原） | — |
| `/member` | `MemberHome` | `pages/member/index.vue` | `user/user/memberCenter`、`user/order/orderNum` |
| `/member/order/list` | `OrderList` | `pages/member/order/List.vue` + `MemberOrderList.vue` | `user/order/list`、`orderNum`、`cancelOrder`、`delOrder`、`confirmReceipt`、`buyAgain` |
| `/member/order/info` | `OrderDetail` | `pages/member/order/Info.vue` | `user/order/detail` |
| `/member/address/list` | `AddressList` | `pages/member/address/List.vue` | `user/address/{list,del,add}` |
| `/member/coupon/list` | `CouponList` | `pages/member/coupon/List.vue` | `user/coupon/list` |
| `/member/point/list` | `PointsList` | `pages/member/point/List.vue` | `user/pointsLog/list` |
| `/member/return/list` | `AfterSalesList` | `pages/member/return/List.vue` | `user/aftersales/list` |
| `/member/comment/list` | `CommentList` | `pages/member/comment/List.vue` | `user/comment/list` |
| `/member/orderInvoice/list` | `InvoiceList` | `pages/member/userInvoice/List.vue` | `user/orderInvoice/list` |
| `/member/collectProduct/list` | `CollectProductList` | `pages/member/collectProduct/List.vue` | `user/collectProduct/list` |
| `/member/collectShop/list` | `CollectShopList` | `pages/member/collectShop/List.vue` | `user/user/collectionShop` |
| `/member/browsingHistory/list` | `HistoryList` | `pages/member/browsingHistory/List.vue` | `user/user/historyProduct` |
| `/member/userMessage/list` | `MessageList` | `pages/member/userMessage/List.vue` | `user/message/list` |
| `/member/feedback/list` | `FeedbackList` | `pages/member/feedback/List.vue` | `user/feedback/list` |
| `/member/inquiry/list` | `InquiryList` | `pages/member/inquiry/List.vue` | `user/priceInquiry/list` |
| `/member/account/detail` | `AccountDetail` | `pages/member/account/Detail.vue` | `user/user/detail`、`user/account/list` |
| `/member/account/log` | `AccountLog` | `pages/member/account/Log.vue` | `user/rechargeOrder/list` |
| `/member/giftCard/exchange` | `GiftCardExchange` | `pages/member/giftCard/exchange.vue` | `promotion/giftCardGroups/exchange` |
| `/member/profile/info` | `ProfileForm` | `pages/member/profile/Info.vue` | `user/user/detail`、`user/user/updateInformation` |
| `/member/security/info` | `SecurityPanel` | `security/Info.vue` + `VerificationSecurity.vue` | `user/user/modifyMobile`、`modifyEmail`、`checkModifyPasswordMobileCode`、`sendMobileCodeBy*`、`sendEmailCodeBy*` |
| `/member/userCertification/info` | `CertificationPanel` | `userCertification/Info.vue` | `user/company/{myApply,detail,apply}` |
| `/member/pin/list` | `PinList` | `pages/member/pin/List.vue` | `promotion/groupon/record/list` |
| `/member/termPay/list` | `CreditList` | `pages/member/termPay/list.vue` | `user/user/creditDetail` |
| `/order/check` | `CheckoutCore` | `pages/order/check.vue` | `order/check/{index,update,updateCoupon,submit,getStoreShippingType}`、`user/address/list` |
| `/order/pay` | `PayCore` | `pages/order/pay.vue` | `order/pay/index`（**不调 `pay/create`**） |
| `/cart` | `CartCore` | `pages/cart/index.vue`（2249 行） | `cart/cart/{list,updateCheck,updateItem,removeItem,clear}` |
| `/product/[...slug]` | `ProductDetail` | `pages/product/[...slug].vue` | `product/product/detail`、`cart/cart/addToCart` |
| `/list` | `ProductResults` | `pages/list/index.vue` | `search/search/getProduct` |
| `/search` | `ProductResults` | `pages/search/components/search.vue` | `search/search/getProduct`（筛选栏未迁） |
| `/seckill/list` | `SeckillList` | `pages/seckill/List.vue` | `home/home/getSeckill` |
| `/coupon/list` | `CouponCenter` | `pages/coupon/index.vue` | `home/home/getCoupon`、`user/coupon/claim` |
| `/article/news/list` | `ArticleList` | `pages/article/news/List.vue` | `article/article/list` |
| `/shop/list` | `ShopList` | `pages/shop/List.vue` | `shop/shop/list` |
| `/shop/[id]` | `ShopHome` | `pages/shop/[id].vue` | `shop/shop/decorate` + `search/search/getProduct` |
| `/joining/list` | `JoiningList` | `pages/joining/List.vue` | `merchant/merchant/myApply` |
| `/join` | `JoinIntro` | `pages/join/index.vue` | 静态流程说明（申请表单待接 `merchant/merchant/apply`） |
| `/activity` | `DecoratePage` | `pages/activity/index.vue` | `home/home/pcIndex`（`?id` / `?previewId`） |

### 6.2 基础模块

| 文件 | 作用 |
|---|---|
| `src/lib/client-api.ts` | 取数封装（信封、Bearer、分页类型） |
| `src/lib/format.ts` | `formatPrice` / `formatDateTime` / `skuText` / `productImage` |
| `src/lib/member-menu.ts` | 侧栏分组 + 路径↔Vue 文件映射 + `findMemberSource` |
| `src/lib/routes.ts` | 顶层路由迁移进度（`done` / `in_progress` / `pending`） |
| `src/components/member/RemoteList.tsx` | 同构列表：分页 + 登录守卫 + 空/错态，9 个页面复用它 |
| `src/components/layout/SiteHeader.tsx` | 全局头：`common/pc/getNav`（topBarNav）、`common/pc/getCatFloor`（分类下拉）、`cart/cart/getCount`（角标）、`user/user/memberCenter`（昵称）；`mainNav` 为空时用 5 项兜底导航 |
| `src/components/layout/SiteFooter.tsx` | 全局脚：四列链接全部指向已迁路由，无死链 |
| `src/components/decorate/DecorateModules.tsx` | 装修模块渲染器：`type → 渲染函数` 注册表（**27 个 type，kebab-case 归一化**），活动页与店铺装修、首页共用 |
| `src/components/home/HomePage.tsx` | 商城首页：`home/home/pcIndex` → `DecorateModules`（`/` 用它，迁移看板已移到 `/migration`） |
| `src/lib/region.ts` | 地区三级级联数据层（`getProvinceList` / `getRegionListByRegionId` / `getRegion`），根节点 `regionId=1`（中国） |
| `src/components/region/RegionPicker.tsx` | 地区级联选择器（分层 Tab + 末层自动提交 + 已有值按 `getRegion` 一次性还原） |
| `src/components/member/AddressForm.tsx` | 地址表单（新增/编辑共用），`user/address/{create,update,detail}` |
| `src/components/captcha/SlideCaptcha.tsx` | 滑动拼图验证码（`common/verification/{captcha,check}`），登录遇 `code=1002` 时弹出 |
| `src/lib/aes-ecb.ts` | 纯 JS AES-128-ECB/PKCS7 + Base64（仅加密），用于验证码点位与 `verifyToken`，免掉 crypto-js 依赖 |
| `src/components/invoice/InvoiceDialog.tsx` | 完整发票弹窗：不开发票/普通/增值税专用 × 个人/企业，`check` 模式只回传、`apply` 模式落库 |
| `src/components/catalog/ImageSearchPage.tsx` | 以图搜图：能力探测 + multipart 上传拿 `searchToken`，复用 `ProductResults` |
| `src/lib/catalog.ts` | 检索统一口径：`getProduct` / `getFilter` / `getAttributeSuggestions` 的字段映射（**分类是 `cat`**） |
| `src/components/catalog/FilterBar.tsx` | 筛选栏：排序（综合/销量/价格升降/新品）+ 分类单选 + 品牌多选 + 价格区间 + 属性多选 + 已选 chips |
| `src/components/catalog/CatalogResults.tsx` | 检索外壳：筛选状态 + 筛选项联动重拉 + 结果网格，`/search` `/list` `/category` `/shopcategory` `/shop/search` 共用 |

### 6.3 路由覆盖率（2026-11-13 盘点，覆盖率 100%）

Vue 侧 `pages/` 共 **111 条路由**（已排除 `src/`、`components/` 子目录），当前处理方式：

| 方式 | 说明 |
|---|---|
| React 页面（72） | 真实实现，见 §6.1。会员详情页用 `src/lib/member-details.ts` 配置表（接口 + 字段 + 多候选键）+ `components/member/EntityDetail.tsx` 通用壳批量承接：12 个详情页、2 个复用列表、4 个说明页。另有 `/migration`（迁移看板，非 Vue 路由），故 `src/app/**/page.tsx` 共 73 个 |
| 旧 URL 重定向（24 条规则） | `next.config.mjs` 的 `redirects`，覆盖约 38 个旧路径：`*.v1` 旧版页、`/item/:id`→`/product/:id`、`/exchange/:id`→`/product/:id`、9 条旧支付渠道→`/order/pay`、14 条 `/activity/pcXxx`→`/activity`、`/join/*`→`/join`、`/member/reset`→`/member/login`、`/member/bindMobilePhone`→`/member/security/info`。**`/search/image` 已从重定向升级为真实页面** |
| 兜底 404 | `app/not-found.tsx` + `member/[...slug]`，覆盖原 `pages/[...slug].vue` 与 `pages/member/[...].vue` |

盘点脚本思路：遍历 `pages/**/*.vue` 生成路由集（`index.vue` 折成父路径、`src`/`components` 目录排除），与 `src/app/**/page.tsx` 求差集，再按 redirects 规则过滤。
**注意**：Vue 侧文件名是 PascalCase（`member/address/List.vue` → `/member/address/list`），React 侧是小写目录，直接字符串差集会误报一堆“未迁”，比对前必须统一小写。

### 6.4 复杂度取舍

订单列表 Vue 版 672 行，含预售定金、拼团、礼品卡、线下认款、物流弹窗等分支。当前只实现**核心链路**：状态 Tab + 角标、关键词搜索、订单卡片、分页、取消/确认收货/再买/删除。其余分支未迁，不是遗漏，是刻意分期。

### 6.5 真实接口校准（2026-11-12，联调后修正）

用测试账号登录后端（`user/login/signin` 拿 token），逐个接口比对真实返回，纠正了 6 处凭 Vue 类型推断出的偏差：

| 接口 | 原假设 | 实际返回 | 影响与处理 |
|---|---|---|---|
| `user/user/detail` | `{ item: {...} }` | **直接返回用户对象**，无 `item` | 账户余额、个人资料、账号安全三页会全空，已改为直接读返回体 |
| `user/order/list` 的 `availableActions` | `cancel` / `confirmReceipt` / `del` | `cancelOrder` / `confirmReceipt` / `delOrder` / `rebuy` / `toPay` | 订单操作按钮不显示，键名已对齐 |
| `user/pointsLog/list` | 时间字段 `addTime` | `changeTime` | 积分列表时间列空白，已加候选 |
| `article/article/list` | 摘要 `summary` | `description` | 文章摘要不显示，已加候选 |
| `user/aftersales/list` | 售后单列表 | **可申请售后的订单**（`orderSn` / `shippingTime` / `items`） | 售后页字段全错，已按订单维度重写，并注明售后单进度从订单详情进入 |
| `user/user/memberCenter` | 有 `couponCount` / `rankName` | 无（只有 `balance` / `points` / `rankId`） | 会员首页两处留空，用兜底显示 |

请求参数侧也已验证被后端接受（`order/status0` 过滤生效、`account/list` 10 条、`aftersales/list` 8 条、`message/list` 1 条、`pointsLog` 1 条，均 `code: 0`）。

**校准方法**（下次接口有变时照做）：登录拿 token → `curl` 打接口 → 打印 `data` 的 keys 与首个数组项的 keys → 与组件里的字段/候选比对。联调账号由负责人提供，**不写入仓库**。

### 6.6 第二批真实接口校准（2026-11-13：地区 / 地址 / 验证码 / 发票 / 图搜）

| 接口 | 真实形状 / 行为 | 落到的实现 |
|---|---|---|
| `sys/region/getProvinceList` | 省级节点**扁平数组**（34 条，`level=2`，`parentId=1`），不是树 | `lib/region.ts` |
| `sys/region/getRegionListByRegionId?regionId=` | 下一级节点数组；上海市的市层只有 1 条（`310100`），区层 16 条 | `RegionPicker` 逐层加载 |
| `sys/region/getRegion?regionIds=1,310000,310100,310120` | **按层分组的二维数组**（`[国家], [省], [市], [区], []`），末层为空数组 | `RegionPicker` 一次还原已有值 |
| 地址 `regionIds` | 首项恒为 `1`（中国根节点），如 `[1,310000,310100,310120]`；列表额外带 `regionNames[]` | `AddressForm` 提交时补 `1` |
| `user/address/add` | **404 不存在**。新增是 `user/address/create`（Vue 里 `action=add → operation=create`），返回新 `addressId` | `AddressForm` |
| `user/address/update` | 需带 `addressId`，返回 `data: null` | 同上 |
| `user/address/del` | 同时传 `{ id, addressId }` | `AddressList` |
| `user/address/setSelected` | `{ id }` 即设为默认 | `AddressList`「设为默认」 |
| `verification/tpCaptcha` | **404 不存在**（Vue 里 `components/captcha/Captcha.vue` 是死代码） | 未迁，改用滑动验证码 |
| `common/verification/captcha`（POST `{captchaType:'blockPuzzle'}`） | `{ originalImageBase64(310×155), jigsawImageBase64(47×155), token, secretKey }` | `SlideCaptcha` |
| `common/verification/check` | 失败 `code=1001`；成功 `code=0`。`pointJson` 必须用 `secretKey` 做 **AES/ECB/PKCS7** 加密，明文 `{"x":…,"y":5.0}` | `lib/aes-ecb.ts`（实现与 `openssl enc -aes-128-ecb` 对拍一致） |
| 登录 `verifyToken` | 不是服务端 `token`，而是 `AES(token + "---" + JSON.stringify({x,y}), secretKey)` | `AuthPanel` 原样回传 |
| 登录 `code=1002` | 表示本次登录要求图形验证码 | `client-api.ts` 的 `ApiError.code` + `errorCode()` |
| `common/config/initConfigSettings` | `canInvoice=1`、`invoiceAdded=1` → 允许开增值税专用发票 | 结算页发票入口 |
| `user/orderInvoice/create` | 发票落库（`update` 同形，带 `id`） | `InvoiceDialog` 的 `apply` 模式 |
| `search/image/capabilities` | `{ enabled, pcEnabled, h5Enabled, appEnabled, shopEnabled, formats[] }`，当前全开 | `ImageSearchPage` 入口守卫 |
| `search/image/search`（multipart） | `file` + `client=pc` + `size` → `{ searchToken }`；图片预处理在云端，超时给到 90s | `apiUpload` |
| `search/search/getProduct` | 带 `searchToken` 时结果限定在图片召回集合内 | `ProductResults` 新增 `searchToken` |

**教训**：Vue 里凡是「`action`/`operation` 二次映射」的地方（如 `add → create`），一定要用真实接口打一遍确认，`api/*.ts` 里的 url 拼接常常把动词藏起来。

### 6.7 第三批真实接口校准（2026-11-13：商品检索 / 筛选）

| 接口 | 真实行为 | 处理 |
|---|---|---|
| `search/search/getProduct` 的分类字段 | **是 `cat`，不是 `categoryId`**。传 `categoryId` 后端直接忽略，返回全站商品 | `lib/catalog.ts` 统一用 `cat`；顺带修掉了 `/list?categoryId=x` 一直显示全站商品的旧 bug |
| 同接口可用字段 | `page` `size` `keyword` `cat` `pId` `brand`(逗号分隔) `sort`(sale\|price\|time) `order`(asc\|desc) `min` `max` `attrs[{attrName,attrValue[]}]` `searchToken` `shopId` `shopCategoryId` | 全部实测生效（`sort=price&order=asc` 返回价升序；`attrs` 传不存在的值 total=0） |
| `search/search/getFilter` | `{ filter: { category[], brand[], shopCategory, maxPrice }, filterSelected }`；分类返回的是**当前条件下的同层分类**（无 keyword 时给顶级，带 keyword 时给命中的叶子） | `fetchFilterOptions` |
| 同上 + `shopId` | **HTTP 500**，该接口不认店铺维度 | 不给 getFilter 传 shopId；店铺内检索（`/shop/search/[id]`、`/shopcategory/[id]`）直接 `showFilter={false}`，店铺筛选属于另一套 ShopFilter |
| `search/search/getAttributeSuggestions` | GET，参数 `keyword` / `cat` → `{ attributes: [{ name, values: [{count, value}] }] }` | `fetchAttrSuggestions` |

---

## 7. 待办（按建议顺序）

| # | 项 | 说明 |
|---|---|---|
| 1 | 会员剩余零散子页 | 发票抬头（`user/invoice/*` 表单）、入驻申请（`joining`）、找回密码（`reset`）、账户卡管理（`account/CardManagement`）。登录/注册已完成；`pages/member/contact/` 在 Vue 版是空目录，菜单项已移除 |
| 2 | 订单结算剩余 | 已补：优惠券、**完整发票（普通/专用 × 个人/企业）**、配送方式、积分/余额抵扣；仍缺：账期支付、预售定金、支付方式选择（提交时 `payTypeId=0`）；收银台支付渠道刻意未接 `order/pay/create` |
| 3 | 功能深度 | 秒杀/拼团内部逻辑（时间轴、库存进度、成团状态机）、IM 客服（`/chat` 仍是占位）、第三方登录（`/oauth/callback` 占位）、账户充值/银行卡/账期还款 |
| 4 | 数据获取时机 | 当前全部页面都是 **SSR 骨架 + 浏览器端取数**（`apiGet/apiPost` 依赖 `localStorage`，Node 侧拿不到 token）。如需真正的「首屏带数据」，要把商品详情、首页装修这类公开接口搬到 `generateMetadata`/服务端 fetch，并统一 `/api` 转发 |
| 5 | 筛选栏 URL 同步 | 筛选栏已迁（分类/品牌/价格/属性/排序），但状态在组件内、未写回 URL query（Vue 会同步路由）。要支持「分享带筛选的链接」需父级接管 |
| 6 | i18n 补齐 | 新增文案的英文 key；`<title>` 目前是静态中文 |
| 7 | 样式对齐 | 尚未逐像素对照 Vue/线上，`globals.css` 只做了骨架级布局 |
| 8 | 登录守卫 | 目前列表页判断 token 显示提示，未做路由级跳转（Vue 版用 `middleware: auth`）；`/order/check` 已在组件内做 token 判定 |

---

## 8. 常用命令与环境变量

```bash
cd pc
npm install                                   # 本机需带 CODEBUDDY_SAFE_DELETE_ENABLED=0，否则被安全删除钩子拦停
npm run dev                                   # http://127.0.0.1:3006
npm run build                                 # 产物 .next-react，构建前自动打 trace 补丁
npm run start                                 # 生产预览，3006
npm run typecheck                             # tsc -p tsconfig.react.json
```

| 变量 | 默认 | 作用 |
|---|---|---|
| `PC_API_BASE` | `https://battery.recy.cc` | 后端地址，供 `rewrites` 与 `env` 用 |
| `PC_API_PREFIX` | `/api` | 接口前缀 |
| `PC_NEXT_DIST_DIR` | `.next-react` | 构建输出目录，多会话并行时隔离 |
| `PC_SKIP_BUILD_TRACE` | 构建脚本自动置 1 | 跳过 Windows 上有问题的 trace 写入 |

---

## 9. 踩坑速查（按症状找）

| 症状 | 根因 | 解法 |
|---|---|---|
| `pageExtensions.map is not a function` | `pageExtensions` 只给了一个元素，Next 14 内部按非数组处理 | 至少两个元素：`['tsx','jsx']` |
| 编译报 `~/utils/request` 找不到 | Next 把根 `pages/`（Vue 的）当 Pages Router 扫了 | 同上，`pageExtensions` 不含 `ts`，Vue 的 `.ts` 不再被当页面 |
| `Package path ./dist/css/semi.min.css is not exported` | Semi 的 `exports` 没导出该路径 | 拷成 `src/app/semi.css` 本地引入 |
| `EPERM ... .next-react/trace` | Windows 上清理目录与写 trace 竞态 | `scripts/patch-next-trace.mjs` + `PC_SKIP_BUILD_TRACE=1` |
| `npm install` 报 `SAFE_DELETE_BULK_CONFIRM_REQUIRED` | 本机安全删除钩子单轮删除超 50 个文件 | 命令前加 `CODEBUDDY_SAFE_DELETE_ENABLED=0` |
| Git 大量 `LF will be replaced by CRLF` | Windows 换行符转换提示 | 仅警告，不影响；提交时保持现状即可 |
| 英文页面首屏先闪中文 | 语言只在 `useEffect` 里切换 | `I18nProvider` 组件体同步 `changeLanguage` |
| 根 `tsconfig.json` 被 Next 改写 | Next 默认用根 tsconfig | `typescript.tsconfigPath: 'tsconfig.react.json'` |
| 装修模块整片空白 | 渲染表按 camelCase（`pcImageAd`）写，真实数据是 kebab-case（`pc-image-ad`），全部匹配不上 | 注册表前先 `normalizeType()` 归一化 |
| 地址新增报 404 | 调了 `user/address/add`；真实动词是 `create` | 见 §6.6 |
| 验证码点位后端不收 | 浏览器 WebCrypto **不支持 AES-ECB**，直接 `crypto.subtle` 找不到算法 | 用 `lib/aes-ecb.ts`（S-box 由 GF(2^8) 逆元现算，已与 `openssl enc -aes-128-ecb` 对拍） |
| 路由盘点出现大量“未迁” | Vue 文件名 PascalCase（`List.vue`），React 目录小写 | 比对前统一 `.toLowerCase()` |

---

## 10. 品牌 / 数据口径（本轮一并处理的）

- **去 TigShop**：源码里 `tigshop` 已 0 命中；礼品卡卡密前缀改为 `NEXTRADE_GIFTCARD|`（明确无旧卡，无需兼容）；`TigShopApiSecret` → `NexTradeApiSecret`（前端签名与服务端 sitemap 两处同步）。
- **OSS 本地化**：`oss.tigshop.com` 上的 141 张表情 + 默认头像下载到 `public/assets/`（约 1.1 MB），源码改本地路径；`notify.mp3` 源站 404，用 1 秒静音 `public/assets/im/notify.wav` 兜底，**需要补一个真实提示音**。
- **数据库口径**：安装页默认 `dbname=nextrade`、`dbport=5432`（PostgreSQL）；前端没有其它 MySQL 痕迹。
- 非 OSS 外链（alicdn 字体、天地图/高德 API、captcha、placeholder）按用户要求未动。

---

## 11. 接手建议

1. 先 `npm install && npm run dev` 跑起来，打开 `/` 看首页装修是否渲染；SSR 自检看 `curl -s localhost:3006/ | head`。
2. 挑一个待迁页，先读对应 `.vue`，再在 `src/app/member/...` 建 page + `src/components/member/...` 建客户端组件。
3. 列表页优先复用 `RemoteList`；新接口只加 `path` 和 `renderItem`。
4. 每次改完 `npx tsc -p tsconfig.react.json --noEmit`（秒级）→ `npm run build` → curl 状态码 → `git add pc && git commit`（只提交 `pc/`，别 `git add -A`）。
5. 不要复制 `pc-semi/` 的组件进来——两条线独立演进，跨目录复制会把两套依赖和样式纠缠在一起。
6. **任何新接口先用测试账号打一遍真实返回**，再写组件。Vue 的 `api/*.ts` 与 `.d.ts` 类型都靠不住（§6.5、§6.6 的 20 多处偏差全是这么发现的）。
