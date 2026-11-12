# M8 · 客服/IM：SOCKET_URL + WhatsApp 客服

> 模块归属：执行章程 §二 M8（泳道 B · 跨境旁路线）
> 来源仓库：`NexTrade-All/nextrade-overseas-pc`
> 创建：2026-07-03

## S1 范围

**IN**：
1. `.env.*` —— 补 `VITE_SOCKET_URL` 声明（空占位；`pages/chat/index.vue` 的 `getSocketUrl()` 已内置该变量优先、回退当前域名协议推导的逻辑，代码零改动）。
2. `pages/chat/src/ChatRight.vue` —— 咨询商品价格改为 `isOverseas()` 分支：跨境用 `FormatPrice`（多币种），非跨境保持原 `¥{{ price }}` 硬编码。

**OUT**（记录理由）：
- **WhatsApp 客服**：跨境仓库内无现成组件/页面（全仓库搜 whatsapp 仅商品详情页社交分享链接一处，属 R1 商品详情范围、另一会话在迁）。《跨境版-WhatsApp客服-开发文档》在主工作区，需向负责人索取 → **阻塞记录于章程 §六**，文档到位后另开 Task。
- `pages/chat/index.vue` 其余 diff：仅注释措辞与 optional-chaining 差异，且 o2o 版（`currentShopInfo?.shopId`）更安全，o2o 权威保留。
- `pages/chat/src/emojiElements.ts`：o2o 版实现更优（预编译 regex、alt 转义、hasEmoji 精确匹配表情名），已含跨境版的 XSS 转义能力，不迁。
- `text-transform: none` 修复（ChatInput/ChatList/ChatMessageList 3 处）：该修复只为对冲跨境版 `assets/css/global.css` 的全局 `text-transform: capitalize`；o2o 无该全局样式，无病灶不吃药。⚠️ **风险联动**：若后续模块（M9 或全局样式迁移）把 overseas 全局 capitalize 带入 o2o，必须同步补回这 3 处 `text-transform: none`。

## S2 盘点结论（2026-07-03）

| 项 | 结论 |
|---|---|
| `getSocketUrl()` | o2o 与跨境版逐行一致，`VITE_SOCKET_URL` 优先 → 回退 `wss://当前域名/ws`，无代码缺口 |
| `FormatPrice` | = 自动导入的 `components/format/Price.vue`，o2o 已存在（注意：该组件是 M9 热点文件，本模块只引用不修改） |
| WhatsApp | 跨境仓库无客服组件，仅 `pages/product/src/Product.vue` 分享链接（R1 范围） |
| chat 目录其余 6 文件 | 差异为注释块 + o2o 侧更优实现，无功能缺口 |

## S5 验收

- [x] `npm run build` 通过（含 @vort-if 校验）
- [x] 跨境开关关时 ChatRight 价格渲染与改动前一致（保留 ¥ 硬编码分支）
- [x] 更新执行章程 §四进度表、§五基线清单、§六阻塞记录（WhatsApp 文档索取）
