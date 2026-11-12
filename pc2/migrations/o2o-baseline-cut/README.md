# o2o 基座切割 · o2o 独有功能清单与 vort-if 包裹任务书

> **性质**：大融合**前置**任务（先于 M1~M11 功能迁移）。目标 = ①整理 o2o 独有功能清单；②给 o2o 独有代码补 `isStore()` 包裹 + `@vort-if o2o` 标记，使「非 o2o 站点不出现门店」成立。
> **配套**：《[需求方案](../../docs/o2o大融合-需求方案.md)》§5.3（基座反向改造）、§3.3（@vort-if 规范）；《[执行章程](../../docs/o2o大融合-执行章程.md)》§三 DoD。
> **落地仓库**：`NexTrade-o2o-Pc`　**创建**：2026-07-02

---

## 一、背景与现状（为什么要做）

- o2o 门店/地图/自提代码目前**几乎不受 `isStore()` 控制**：全仓库业务代码只有 `app.vue`、`decorate/components/pc/PcHomeHero.vue` 两处调用 `isStore()`，门店功能是**无条件存在**的。
- 因此「非 o2o 站点（标准/B2B/跨境）不出现门店」**当前根本不成立**。本任务就是把 o2o 独有资产**标清楚 + 包起来**。
- `VITE_IS_STORE` 此前三个 `.env` 缺失，`isStore()` 恒为 false，**从未真实以 1 运行过**。本任务已补齐（见 §四）。

## 二、盘点方法（结论可复现）

**三方对照 diff**：以 o2o 为基准，逐一比对 `NexTrade-Pc`（标准，含 B2B）与 `nextrade-overseas-pc`（跨境）。

- 判定「o2o 独有」= 该目录/文件在 o2o **存在**，且标准版与跨境版**都不存在**。
- 对 `pages/order`、`pages/member` 等**共用目录**，下钻到**文件级**比对相对路径。

> ⚠️ **边界**：文件级 diff 只能发现「独有整文件」。**同名但实现不同**的共用文件（如 `check.vue`、`Price.vue`、`app.vue` 里的门店分支）里的 o2o 差异**片段**发现不了——那部分不在本清单拍死，标「待 S2 核实」，随各业务模块处理。

## 三、o2o 独有功能清单（已写入 `.vort/feature-manifest.json` 的 `o2o.includes`）

高置信度门店资产，`@vort-if o2o` / `isStore()` 保护：

| 功能 | 路径 | 类型 | 切割接入点（粗粒度包裹处） |
|---|---|---|---|
| 门店体系（列表/详情/门店购物车/拨号/门店菜单） | `pages/store/` | 整目录 | 导航/菜单入口、跳门店链接、路由守卫 |
| 门店接口 | `api/store/` | 整目录 | 引用它的组件（本身随文件裁剪） |
| 门店类型 | `types/store/` | 整目录 | — |
| 地图/定位（高德 amap） | `components/map/`（`MapContainer`/`SelectMapRegion`） | 整目录 | 使用处 **client-only**，SSR 守卫 |
| 地图接口/类型 | `api/map/`、`types/map/` | 整目录 | — |
| 城市选择 | `components/city/`（`SelectCity`） | 整目录 | 使用处（头部/定位） |
| 门店切换弹窗 | `components/product/StoreSwitchDialog.vue` | 文件（共用目录内） | `ProductInfo.vue`、`ProductJoining.vue` 引入处（细粒度） |
| 地址地图选点 | `components/member/address/AddressMap.vue` | 文件 | 使用处（当前疑似未被引用，仍随文件裁剪） |
| 门店定位/当前区域状态 | `store/region.ts` | 文件 | 初始化处（`app.vue` 已有一处 `isStore()`） |
| 自提 | `pages/order/src/SelfPickup.vue` | 文件（共用目录内） | `check.vue` 引入处 |
| 预约 | `pages/order/src/ReserveCard.vue` | 文件 | `check.vue` 引入处 |
| 自提时间 | `pages/order/src/PickupTimePop.vue` | 文件 | 同上 |
| 配送方式（结算） | `pages/order/src/DeliveryTypeCard.vue` | 文件 | `check.vue` 引入处 |
| 配送方式（购物车） | `pages/cart/src/DeliveryType.vue` | 文件 | `cart/index.vue` 引入处 |
| 联系人（下单） | `pages/order/src/Contact.vue`、`ContactInfo.vue` | 文件 | 同上 |
| 订单自提信息展示 | `pages/member/order/src/pickupInfo.vue` | 文件 | 订单详情引入处 |
| 订单预约信息展示 | `pages/member/order/src/bookInfo.vue` | 文件 | 同上 |
| 会员联系人（页面+接口+类型） | `pages/member/contact/List.vue`、`api/user/contact.ts`、`types/user/contact.d.ts` | 文件 | 会员中心菜单入口（✅ 批次1 已包）；接口 `o2o/userPickup/*` |
| 门店收藏 | `pages/member/collectShop/shopList.vue` | 文件 | 见 §四注：收藏入口本身通用 |

## 四、待 S2 核实清单（**未**进 `o2o.includes`，避免误裁）

这些是「o2o 结构上独有，但不能直接判为 o2o 功能」的项，需在相关模块 S2 逐一核实归属，**当前一律不标 o2o**：

| 路径 | 结构现状 | 为什么不直接标 o2o |
|---|---|---|
| `api/decorate/`、`store/decorateRuntime.ts`、`types/decorate/`、`types/decoration/` | 仅 o2o 有目录 | **装修是通用功能**（§4.2），需前后台配合、留到最后统一，不属门店 |
| `components/icons/`（Arrow/Cart/Cat/Language/Mine/Search） | 仅 o2o 有 | 通用 UI 图标，疑似非功能性差异，可能只是结构不同 |
| `components/common/components/AllCategoryNew.vue`、`DefSearchNew.vue`、`Mine.vue` | 仅 o2o 有 | 通用头部/搜索/我的组件，o2o 结构独有但功能通用，归属需核实 |
| `components/product/SoldOut.vue` | 仅 o2o 有（当前未见引用） | 售罄提示，功能通用，疑似孤儿文件，归属需核实 |
| `composables/useBuyBtnStyle.ts`、`usePromotionActivity.ts` | 仅 o2o 有 | 购买按钮样式/促销活动，疑似通用能力 |
| `types/region.d.ts` | 仅 o2o 有（但 `api/region.ts` 三方共用） | `ByLocationData` 可能被共用 `api/region` 引用，归属需核实 |
| `pages/member/joining/List.vue` | 仅 o2o 有（但 `pages/joining` 三方共用） | 加盟顶层是共用功能，此会员页归属需核实 |

> **盘点范围升级说明**：§三/§四基于**全仓库文件级** o2o-only diff（`pages`/`api`/`components`/`store`/`types`/`composables` 全量），已覆盖此前只做 `pages/order`、`pages/member` 时漏掉的 `StoreSwitchDialog`、`AddressMap`、`cart/DeliveryType`、`api/user/contact` 等。

## 五、切割包裹执行指引（分两粒度）

遵循《需求方案》§5.3 与 §7 负面清单：**只加守卫、不改逻辑、不改 props、不重命名、不格式化**。

### 5.1 粗粒度（本前置任务集中做，低风险）

对 §三「整目录/整文件」的 o2o 独有资产，只在**引入/使用处**加保护：

- **路由入口**：Nuxt 文件路由无法条件生成，用**路由守卫**或导航层 `isStore()` 隐藏入口即可（页面文件本体靠 `@vort-if o2o` 归属，最终裁剪时整文件删除）。
- **组件引入处**：
  ```vue
  <!-- @vort-if o2o -->
  <SelfPickup v-if="isStore()" ... />
  <!-- @vort-endif -->
  ```
  ```ts
  // @vort-if o2o
  import SelfPickup from "~/pages/order/src/SelfPickup.vue";
  // @vort-endif
  ```
- **菜单/导航入口**：会员中心/主导航里指向 o2o 独有页的入口，用 `isStore()` 控制 `show` 并 `@vort-if o2o`/`@vort-endif` 成对包裹。
- **整文件/整目录归属**：由 `.vort/feature-manifest.json` 的 `o2o.includes` 表达（裁剪的唯一权威源），**不在文件头写不成对的 `@vort-if o2o`**——那会被 `vort:check` 判为缺 `@vort-endif`。`@vort-if`/`@vort-endif` 只用于「共用文件内的片段」。

> ⚠️ **入口归属必先三方核实**：菜单项/链接是否 o2o 独有，要对照 `NexTrade-Pc` / `nextrade-overseas-pc` 的同名文件确认。例：会员中心「收藏店铺 `/member/collectShop/list`」三方都有（通用，标准版叫"收藏的店铺"），**不可**加 `isStore()`；而「联系人 `/member/contact/list`」仅 o2o 有，才包裹。

> 每批一个 commit，`≤800 行 / ≤15 文件`；每批 `npm run build`（会先跑 `vort:check`）+ 跑 §六回归。

### 5.2 细粒度（**不在**本任务，随各业务模块 S2 顺带做）

共用文件里的 o2o 差异**片段**（如 `check.vue`/`pay.vue`/`app.vue`/`Price.vue` 内的门店分支）——逐行核实归属风险高，集中做易标错，**留给对应模块**用 `if(isStore())` + `@vort-if o2o`/`@vort-endif` 处理。

## 六、DoD 与校验闭环

- [ ] `npm run vort:check` 通过（`@vort-if`/`@vort-endif` 配对、特性名合法、**`includes` 路径均存在**）。
- [ ] `npm run build` 通过。
- [ ] **开关回归（切割成功的硬证据）**：`VITE_IS_STORE=0` 构建 → 门店/地图/自提**入口全部消失且不报错**；置 1 → 门店主链路正常。
- [ ] `.vort/feature-manifest.json` 的 `o2o.includes` 与代码里 `@vort-if o2o` 标记交叉核对无遗漏。
- [ ] 每个 Task 一个 commit。

## 七、o2o 站现状回归基线（切割前录一次，切割后重跑）

- [ ] 首页正常渲染（装修驱动）
- [ ] 商品搜索 → 商品详情（`item/`）→ 加购物车
- [ ] 门店列表/详情、地图定位正常
- [ ] 下单主链路：购物车 → 结算 `check`（自提/配送切换）→ 支付页可达
- [ ] 会员中心：登录、订单列表、订单详情（自提/预约信息展示）、联系人、门店收藏
- [ ] 多语言/币种基础展示无异常

## 八、粗粒度切割批次与进度

| 批次 | 状态 | 内容 | 接入点 |
|---|---|---|---|
| 批次1 | ✅ 完成（build 通过） | 会员中心菜单入口 | `components/member/Nav.vue`：联系人加 `isStore()`+标记；收藏店铺经核实为通用，未包 |
| 批次2 | 🟡 收敛为「入口盘点」 | 门店主入口 | **无独立主导航门店入口**；`DefNav` 仅 `isDemo` 商家入驻；门店访问=共用文件内上下文链接（`/store/info`）+ 装修 `format.ts` `store`/`storeInfo` 链接类型 → 属细粒度/装修，随模块做 |
| 批次3 | 🟡 收敛为「入口盘点」 | 地图/城市/定位 | `SelectCity`/`MapContainer`/`SelectMapRegion`/`AddressMap` **全仓无引用**（孤儿，随文件裁剪）；`store/region.ts` 仅在 `app.vue`/`item/Product.vue`/`ProductInfo.vue` 内使用=细粒度，随模块做 |
| 批次4 | ↪ 归入业务模块 S2 | 结算/购物车履约 | `check.vue`/`cart/index.vue`/`ProductInfo`/`ProductJoining` 里门店逻辑交织=细粒度，且 `check.vue` 是支付热点（§8.3），随结算/商品模块做 |
| 批次5 | ↪ 归入业务模块 S2（**已做组件拆分预处理**，见 §九） | 会员订单履约信息 | 订单详情内 `pickupInfo`/`bookInfo` 使用处交织=细粒度，随订单模块做；已先把订单详情/列表拆成子组件、门店 UI 收拢为独立连续区域，为后续 `@vort-if` 留干净落点 |
| 批次6 | ✅ 完成 | 收尾核查 | `vort:check` 已扩展为「配对 + 特性名 + **includes 路径存在**」；manifest 42 项 includes 均存在；代码内 `@vort-if o2o` 仅 `Nav.vue:138`（批次1），与 manifest 一致 |

> **前置任务收官结论（2026-07-02）**：粗粒度前置切割的"干净独立面"仅批次1（会员联系人入口）。其余 o2o 独有资产分两类——①**整目录/整文件**：归属由 `o2o.includes` 表达，最终裁剪整体删除，无需现在改代码；②**共用热点文件内的门店分支**（`check.vue`/`cart`/`item·Product`/`ProductInfo`/`app.vue` 等）：属细粒度，按 §5.3/§8.3 **随各业务模块 S2 顺带做**，不在前置阶段脱离上下文集中改造。
>
> **开关回归（`VITE_IS_STORE=0` 构建后门店入口全消失）** 这个硬验收，须等细粒度切割在各模块完成后才有完整意义；届时在最终裁剪前做一次全局扫尾（§6 DoD）。

## 九、订单详情/列表 · 组件拆分（融合前预处理，2026-07-03）

> **目的**：①提升可维护性（把巨石页面按区块拆成子组件）；②**为融合后细粒度切割做准备**——把门店（o2o）UI 收拢成**独立连续区域**，用 `<!-- 门店 start/end -->` 注释标出边界，将来 `@vort-if o2o` 有干净落点。
> **本次不注入版本开关、不改判断逻辑**：`isStore()` 之类留到真正切割时再加；当前门店块仍按原有 `formState` 业务字段（`isPickup`/`orderType`/`shop`/`deliveryOrderInfo` 等）数据驱动条件渲染，行为零变更。

### 9.1 参考 vortmall-pc 的结论

核对 `vortmall-pc` 的 `member/order`：其订单详情 `Info.vue`（949 行）与列表 `MemberOrderList.vue`（623 行）**本身并未做区块级拆分**，专门拆出的子组件只有 `pickupInfo.vue`/`bookInfo.vue`（o2o 本就已有）。故本次拆分**按 o2o 自身代码逻辑设计**，vortmall-pc 仅提供目录/命名参考，未强行对齐。

### 9.2 拆分清单

**订单列表**（`components/member/order/`，`MemberOrderList.vue` 593 行 → 改壳）：

| 子组件 | 覆盖区块 | 门店内容 |
|---|---|---|
| `OrderListFilter.vue` | 顶部 tab + 搜索框 | — |
| `OrderListCard.vue` | 单条订单表格（商品/金额/状态/操作列） | 表头下单门店 `/store/info` 链接（门店框住） |

- 父级 `MemberOrderList.vue` 保留全部 API/`filterParams`/动作函数；列头（含时间筛选下拉，与数据加载耦合）留父级；`v-for` 渲染 `OrderListCard`，操作经 `emit` 回父级。
- `onReBuy` 由多参数改为传整个 `item`，父级内部再取值，逻辑不变。物流弹窗由 `item.showPopover` 改为卡片本地 `ref`，不再 mutate prop。

**订单详情**（`pages/member/order/src/`，`Info.vue` 869 行 → 约 190 行壳）：

| 子组件 | 覆盖区块 | 门店内容 |
|---|---|---|
| `OrderInfoHeader.vue` | 订单号/下单门店/订单状态 + 操作按钮 | 下单门店链接、自提/预约状态描述（门店框住） |
| `OrderStatusSteps.vue` | 状态步骤/预售步骤 + 物流跟踪 | — |
| `OrderDeliveryInfo.vue` | 订单信息（收货/配送/支付） | `PickupInfo`/`BookInfo`、配送员信息（门店框住） |
| `OrderGiftCard.vue` | 礼品卡卡片信息 | — |
| `OrderEcard.vue` | 卡券信息 | — |
| `OrderProductTable.vue` | 商品表 + 金额合计 + 礼品卡兑换区 | — |

- 子组件统一接 `:formState`；各区块 `computed`（运费行/礼品卡/状态文案等）就近下沉；跨组件状态（`copiedCardNumber/Password`、`showGiftCard/showEcard` 渲染开关）留父级下发；操作经 `emit` 回父级。
- 复用既有：`pickupInfo.vue`、`bookInfo.vue`、`LogisticsTracking.vue`、`presaleSteps.vue`、`giftCard*`。

### 9.3 踩坑记录（scoped 样式作用域）

`.tig-steps`/`.steps-card`/`.font-color`/`.page-info-body` 等在全局 `member.less`，但项目约定是在**每个组件 `<style scoped>` 里 `@import member`**——`@import` 进 scoped 块会被打上该组件 scope，**只作用本组件渲染的元素**。父组件的 `@import` 够不到子组件内部嵌套元素（仅子组件根元素继承父级 scope）。

- 现象：拆分后订单状态步骤条竖排、丢样式（`.tig-steps` 未生效）。
- 修复：**给每个子组件的 scoped 样式都补 `@import "/assets/css/member/member";`**（与原 `Info.vue`、列表子组件一致）。member 里没有的类（`.sub-title`/`.lr`/`.order-left/right`/`.order-ul`/`.order-table-info` 等，原本是 `Info.vue` 自己 scoped 的）则手写下沉到对应子组件。

### 9.4 校验

- `npm run vort:check` 通过；`npm run build` 完整生产构建 **通过（exit 0）**。
- 本次为纯组件拆分（零行为变更、无开关注入），未新增 `@vort-if` 标记，`o2o.includes` 无变更。
- 待办：dev（`VITE_IS_STORE=1`）目视回归订单列表/详情各区块（含自提/预约/礼品卡/卡券/金额合计）。

> ⚠️ **工作树现状说明（2026-07-03 复核）**：§九 描述的订单详情/列表子组件（`OrderListFilter`/`OrderListCard`/`pages/member/order/src/*`）在**当前工作树中不存在**（`components/member/order/` 下仅 `MemberOrderList.vue`、`LogisticsTracking.vue`；`pages/member/order/` 下仅 `Info.vue`、`List.vue`，均无 `<!-- 门店 start/end -->` 标记）。该拆分应在别的分支/工作副本，未落到本副本。故本轮的「订单模块 门店 start/end → @vort-if o2o」统一替换 **无对象可改，跳过**。

## 十、结算/购物车/商品详情 · 组件拆分 + `@vort-if o2o` 标记（融合前预处理，2026-07-03）

> **目标校准**：经与需求方确认，本轮拆分的**首要目的是「方便 O2O 独有代码切割」**，而非最大化拆分。三版共用的展示/交互块（价格、SKU、发票、优惠）拆分对 O2O 切割无帮助且会引入回归风险，故**只拆 O2O 切割单元 + 明确边界，共用块不强拆**。
> **口径**：O2O 独有块统一用 `<!-- @vort-if o2o -->` / `<!-- @vort-endif -->`（模板）与 `// @vort-if o2o` / `// @vort-endif`（脚本）成对标记；共用块不动。**零行为变更**。

### 10.1 购物车（`pages/cart/index.vue` 1356 行 → 改壳）

| 子组件（`pages/cart/src/`） | 覆盖区块 | O2O |
|---|---|---|
| `CartEmptyState.vue` | 空购物车态 | — |
| `CartShopGroup.vue` | 单店铺分组：商品行 + 赠品 + 促销/券标签 | — |
| `CartCheckoutBar.vue` | 底部结算栏：全选/合计/优惠明细/去结算（吸底态经 `defineExpose` 暴露根节点给父级测量） | — |

- 父级 `index.vue` 保留全部 API/选中同步/金额逻辑；列头留父级；`v-for` 渲染 `CartShopGroup`，动作经 `emit` 回父级。
- **O2O 标记**：配送方式选择弹窗 `DeliveryType`（模板）、其 `import`、`getDeliveryOptions` 分流全套（`deliveryProducts/pickupProducts/...`、`getDeliveryOptionsData`、`changeCart`、`handleOrderCheckType`、`deliveryTypeDialogVisible`）、`toCheck` 内的 `getDeliveryOptionsData()` 调用，均 `@vort-if o2o`。

### 10.2 商品详情（`pages/item/src/Product.vue`、`components/product/ProductInfo.vue`）

| 子组件 | 覆盖区块 | O2O |
|---|---|---|
| `pages/item/src/ProductDetailTabs.vue` | 老版 Tab 区（产品详情/评价/咨询/售后），Tab 切换与滚动内聚 | — |
| `components/product/ProductStoreCard.vue` | **O2O 当前门店卡片 + 切换门店弹窗**（整体 `@vort-if o2o`，O2O 切割单元） | ✅ |

- **PriceSection/Actions 经确认不拆**：价格/活动/预售/购买区样式深度嵌套在 `.product_info.new-ui` 祖先选择器下、且价格与购买区通过 `loadPrice/promotionManage` 共享大量响应式状态，拆分属高回归风险的重构；且这两块是**三版共用**、与 O2O 切割无关。`ProductInfo` 维持 controller 形态。
- **门店脚本线 `@vort-if o2o` 标记**（闭合 O2O 门店切割边界）：
  - `Product.vue`：`getStoreDetail` import、`useRegionStore` import + `regionStore`、经纬度定位块、`nearestShopId` 参数、初始 `getStoreDetailData()` 调用、`getStoreDetailData()` 定义、`addShopCollect()`。
  - `ProductInfo.vue`：`handleSelectStore()`（切门店改路由）。
  - **未标共用项**：`storeInfo`/`getShopDetail`/`ProductShopInfo`（多店铺共用），保持数据驱动降级。

### 10.3 结算页（`pages/order/check.vue` 2038 行）

- **共用块不拆**（发票 `Info`、优惠券/积分/余额/账期、金额汇总、submit 均保留原位）——与 §10.2 同口径，拆它们无助 O2O 切割。
- **O2O 履约层 `@vort-if o2o` 标记**（本页的 O2O 切割 delta）：
  - 模板：自提联系人 `Contact`（与共用 `Address` 的 if/else，含 else 包裹标记）、`DeliveryTypeCard`、`ReserveCard`、自提信息内联块（`deliveryType===2`）、`PickupTimePop` + `SelfPickup` 弹窗。
  - 脚本：O2O 组件 `import`（`Contact`/`DeliveryTypeCard`/`PickupTimePop`/`SelfPickup`/`ReserveCard`）、自提弹窗纯 UI 状态与 `handleShowPickupTimePop`/`changePickup`、`pickupDateTypeMap`。
- **脚本刻意保守**：`deliveryType`/`shippingType`/预约/联系人状态**交织进共用结算数据管线**（`getOrderCheckData`/`updateOrderCheckData`/`formatFormState`/`onSubmit`/`initPageData`），逐符号标记会产生大量悬挂引用、反而误导——这部分留到融合期整体处理，本轮只标「仅被已标记模板/handler 引用」的自包含叶子块。

### 10.4 校验

- 三页均 `npm run vort:check` 通过；`npm run build` 完整生产构建**逐页通过（exit 0）**。
- 无 lint 错误；`o2o.includes` 无变更（新增均为共用目录内的 `@vort-if o2o` 片段标记，符合 §5.1「片段用成对标记、整文件靠 manifest」）。
- 待办：dev（`VITE_IS_STORE=1`）目视回归——购物车（配送分流去结算）、商品详情（门店卡片/切换门店/新旧版）、结算页（自提/配送/预约/自提时间与自提点）。
