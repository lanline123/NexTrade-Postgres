# pos-semi · POS 收银台

原生重写的收银台前端，与 `admin-semi/`、`pc-semi/` 并列，独立构建、独立部署。

> 现状：**立项骨架**。已打通登录态、分类与商品查询；购物车、结算、桌台等在后续里程碑实现。
> 当前线上 `/pos` 仍是 5.8.60 无源码产物，本项目建设完成后替换它。

## 技术栈

- React 18 + TypeScript + Vite 6
- Semi UI（与 admin-semi 一致，复用同一套视觉与组件习惯）
- react-router-dom 6（history 模式，`base` 由部署路径决定）

## 目录

```text
pos-semi/
├─ src/
│  ├─ api.ts              # /adminapi 请求封装（token、X-ADMIN-TYPE: pos）
│  ├─ session.ts          # 读取同源 localStorage 登录态
│  ├─ App.tsx             # 布局、导航、路由、未登录门禁
│  ├─ pages/
│  │  ├─ Cashier.tsx      # 收银（分类 + 商品查询，已接通）
│  │  └─ Placeholder.tsx  # 桌台/订单/预约/核销/售后/会员占位
│  └─ styles.css
├─ vite.config.ts         # base 可配置，dev 端口 3003
└─ README.md
```

## 认证与接口

与 admin-semi 同源（`admin.recy.cc/pos`），直接复用后台登录态：

- `localStorage.accessToken` → `Authorization: Bearer <token>`
- `localStorage.shopId` → `X-SHOP-ID`
- 固定请求头：`X-ADMIN-TYPE: pos`、`X-CLIENT-TYPE: admin`
- 统一前缀 `/adminapi`，响应 `{ code, data, message }`，`code !== 0` 抛 `ApiError`

已对齐的后端路由（Rust，`crates/api/admin/src/routes/pos/`）：

| 域 | 路由 |
|---|---|
| 商品 | `/pos/product/index`、`getCategoryInfo`、`getProduct`、`productDetail`、`getProductAvailability` |
| 购物车 | `/pos/cart/list`、`addToCart`、`updateItem`、`removeItem`、`clearCart`、`changePrice`、`pendingOrder`、`takePendingOrder`、`cancelPendingOrder` |
| 桌台 | `/pos/table/list`、`create`、`update`、`del`、`area/*`、`operate/open|board|checkout|...` |
| 结账 | `/pos/order/check/index|update|submit`、`/pos/order/check/getOrderRemark` |
| 交班 | `/pos/posSwitch/work`、`workOff`、`posStatistics` |
| 会员 | `/pos/posManager/createMember|createTourists`、`/pos/user/*` |
| 设备 | `/pos/device/*` |

## 开发

```bash
npm install
npm run dev        # http://localhost:3003/pos/
```

dev 代理 `/adminapi`、`/static` 到 `https://battery.recy.cc`。未登录时显示门禁页，可直接在后台登录后刷新。

## 构建与独立部署

本项目与 admin-semi **完全解耦**：单独 `npm run build` 产出 `dist/`，不依赖后台构建产物。

```bash
npm run build                        # 默认 base=/pos/
```

换部署路径时复制 `.env.example` 为 `.env.local`，设置 `VITE_BASE_PATH`（如 `/` 或 `/cashier/`）后重新构建。
Windows Git Bash 下不要把 `/` 直接写在命令行环境变量里（MSYS 会转成 `Program Files` 路径），用 `.env.local` 最稳。

部署到同源路径（推荐，登录态共享）：

1. 上传 `dist/` 内容到站点根目录下的 `pos/`。
2. nginx 深链兜底（必须 `^~`，避免被面板默认正则抢走）：

   ```nginx
   location ^~ /pos {
       try_files $uri $uri/ /pos/index.html;
   }
   ```

独立子域部署：`VITE_BASE_PATH=/`，并设置 `VITE_ADMIN_URL=https://admin.recy.cc/`；
注意子域与后台不同源，`localStorage` 登录态不共享，需要另行接 SSO 或跳转登录。

## 里程碑

| 阶段 | 内容 | 状态 |
|---|---|---|
| M0 | 项目骨架、登录态、商品/分类查询 | ✅ 本次 |
| M1 | 购物车、改价、整单备注、结算下单 | 待排期 |
| M2 | 挂单/取单、桌台开台/并台/结账 | 待排期 |
| M3 | 交接班、收银员、设备绑定、小票打印 | 待排期 |
| M4 | 会员/优惠券/充值、售后、核销 | 待排期 |

## 与 admin-semi 的边界

- `admin-semi` 保留 POS **配置与管理**页：收银工作台、收银设置、设备管理、小票打印、收银员列表。
- 收银操作界面由本项目承载；后台「进入收银台」继续打开 `/pos`，替换时无需改后台代码。
