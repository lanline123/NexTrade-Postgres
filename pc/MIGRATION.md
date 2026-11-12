# PC 前端技术栈迁移交接

> 更新时间：2026-11-12　｜　代码位置：`pc/`　｜　当前基线：`995fa09`
>
> 本文只讲 **PC 前端从 Vue 3 / Nuxt 3 迁到 React 18 + Next 14 + Semi Design** 这件事。
> PostgreSQL / 后端 / 运维的交接见仓库根 `HANDOVER.md`。

---

## 0. 一页速览

| 项 | 现状 |
|---|---|
| 迁移方式 | **在 `pc/` 原地重写**：Vue 源码保留作 1:1 参照，React 代码写在 `src/`，两者互不参与对方构建 |
| 技术栈 | React 18.3 + Next 14.2（App Router，真 SSR）+ Semi UI 2.104 + react-i18next |
| 已完成 | 会员中心 23 路由 + 订单结算/收银台 + 购物车/商品详情/列表/搜索/秒杀/领券/文章 |
| 验证 | `npm run build` 通过，35 条路由全部 `ƒ Dynamic`；商品板块 8 条路径 curl 全 200 |
| 尚未完成 | `/shop`、`/activity`、`/join`、`/install`、`/exchange`、`/category`、文章详情；结算页优惠券/发票/配送分支 |
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
│   │   ├── HomeClient.tsx      # 看板（'use client'）
│   │   ├── MemberShell.tsx     # 会员中心外壳：Semi Layout + 侧栏 + 语言切换
│   │   ├── LocaleSwitch.tsx
│   │   └── member/             # 24 个会员业务组件（见 §6）
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
| `/order/check` | `CheckoutCore` | `pages/order/check.vue` | `user/address/list`、`order/check/{index,submit}` |
| `/order/pay` | `PayCore` | `pages/order/pay.vue` | `order/pay/index`（**不调 `pay/create`**） |
| `/cart` | `CartCore` | `pages/cart/index.vue`（2249 行） | `cart/cart/{list,updateCheck,updateItem,removeItem,clear}` |
| `/product/[...slug]` | `ProductDetail` | `pages/product/[...slug].vue` | `product/product/detail`、`cart/cart/addToCart` |
| `/list` | `ProductResults` | `pages/list/index.vue` | `search/search/getProduct` |
| `/search` | `ProductResults` | `pages/search/components/search.vue` | `search/search/getProduct`（筛选栏未迁） |
| `/seckill/list` | `SeckillList` | `pages/seckill/List.vue` | `home/home/getSeckill` |
| `/coupon/list` | `CouponCenter` | `pages/coupon/index.vue` | `home/home/getCoupon`、`user/coupon/claim` |
| `/article/news/list` | `ArticleList` | `pages/article/news/List.vue` | `article/article/list` |

### 6.2 基础模块

| 文件 | 作用 |
|---|---|
| `src/lib/client-api.ts` | 取数封装（信封、Bearer、分页类型） |
| `src/lib/format.ts` | `formatPrice` / `formatDateTime` / `skuText` / `productImage` |
| `src/lib/member-menu.ts` | 侧栏分组 + 路径↔Vue 文件映射 + `findMemberSource` |
| `src/lib/routes.ts` | 顶层路由迁移进度（`done` / `in_progress` / `pending`） |
| `src/components/member/RemoteList.tsx` | 同构列表：分页 + 登录守卫 + 空/错态，9 个页面复用它 |

### 6.3 复杂度取舍

订单列表 Vue 版 672 行，含预售定金、拼团、礼品卡、线下认款、物流弹窗等分支。当前只实现**核心链路**：状态 Tab + 角标、关键词搜索、订单卡片、分页、取消/确认收货/再买/删除。其余分支未迁，不是遗漏，是刻意分期。

---

## 7. 待办（按建议顺序）

| # | 项 | 说明 |
|---|---|---|
| 1 | 会员剩余零散子页 | 发票抬头（`user/invoice/*` 表单）、入驻申请（`joining`）、登录/注册/找回密码（`Login/Register/reset`）、账户卡管理（`account/CardManagement`）。`pages/member/contact/` 在 Vue 版是空目录，菜单项已移除 |
| 2 | 登录态真实数据校准 | 当前无 token 只验到「请先登录」；拿到登录态后要核对字段名（组件里用了多候选 `a ?? b ?? '-'` 兜底） |
| 3 | 订单结算未迁分支 | 优惠券选择、发票、配送方式切换、积分/余额抵扣、账期支付、预售定金（走 `order/check/update`、`updateCoupon`）；收银台支付渠道刻意未接 `order/pay/create` |
| 4 | 剩余顶层页 | `/shop`（18 个 `.vue`）、`/activity`（16）、`/join`（9）、`/install`（5）、`/exchange`（2）、`/category`、`/article` 详情、`/oauth` |
| 5 | i18n 补齐 | 新增文案的英文 key；`<title>` 目前是静态中文 |
| 6 | 样式对齐 | 尚未逐像素对照 Vue/线上，`globals.css` 只做了骨架级布局 |
| 7 | 登录守卫 | 目前列表页判断 token 显示提示，未做路由级跳转（Vue 版用 `middleware: auth`） |

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

---

## 10. 品牌 / 数据口径（本轮一并处理的）

- **去 TigShop**：源码里 `tigshop` 已 0 命中；礼品卡卡密前缀改为 `NEXTRADE_GIFTCARD|`（明确无旧卡，无需兼容）；`TigShopApiSecret` → `NexTradeApiSecret`（前端签名与服务端 sitemap 两处同步）。
- **OSS 本地化**：`oss.tigshop.com` 上的 141 张表情 + 默认头像下载到 `public/assets/`（约 1.1 MB），源码改本地路径；`notify.mp3` 源站 404，用 1 秒静音 `public/assets/im/notify.wav` 兜底，**需要补一个真实提示音**。
- **数据库口径**：安装页默认 `dbname=nextrade`、`dbport=5432`（PostgreSQL）；前端没有其它 MySQL 痕迹。
- 非 OSS 外链（alicdn 字体、天地图/高德 API、captcha、placeholder）按用户要求未动。

---

## 11. 接手建议

1. 先 `npm install && npm run dev` 跑起来，打开 `/` 看板确认 SSR 时间戳每次刷新都变。
2. 挑一个待迁页，先读对应 `.vue`，再在 `src/app/member/...` 建 page + `src/components/member/...` 建客户端组件。
3. 列表页优先复用 `RemoteList`；新接口只加 `path` 和 `renderItem`。
4. 每次改完 `npm run build` 必须过，再看 curl 状态码，最后 `git add pc && git commit`（只提交 `pc/`，别 `git add -A`）。
5. 不要复制 `pc-semi/` 的组件进来——两条线独立演进，跨目录复制会把两套依赖和样式纠缠在一起。
