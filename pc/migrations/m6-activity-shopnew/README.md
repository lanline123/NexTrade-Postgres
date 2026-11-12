# M6 · 营销活动 activity + 新版店铺页 shopNew

> 模块归属：执行章程 §二 M6（泳道 B · 跨境旁路线；前置 M1 已完成）
> 来源仓库：`NexTrade-All/nextrade-overseas-pc`
> 创建：2026-07-03
> **结论：S2 三方盘点后判定无功能缺口，本模块无代码迁移，分析结案。**

## S2 盘点结论（2026-07-03）

### activity 活动专题页（16 个文件全部两仓库同构）

| 文件 | 差异定性 | 处置 |
|---|---|---|
| `index.vue` | 跨境用 `CommonPageHeaderNew` 顶部（R1 已决策不迁该 chrome）、删 ICP 页脚、`decorateType: 3` 请求参数、`isMobileBrowser()` 重构 | **不迁**：chrome 属跨境改版；`decorateType` 参数是否为 o2o 后端所支持未核实，贸然加参有风险 |
| `pcGroupon/index.vue` | 「查看更多」改为倒计时组件（跨境 UX 改版）、标题字号 16→24、`getMaxGroupNum` 等价实现、样式微调 | **不迁**：纯 UI 改版，activity 页为 o2o 默认分支，整页换观感违反"开关置 0 行为不变"红线 |
| `pcSeckill`/`pcProductGrouping` 等其余 14 件 | 注释措辞、换行格式、`borderImage \|\| undefined` 防御性微调 | **不迁**：无功能差异 |

### shopNew 新版店铺街

- 跨境 `pages/shopNew/`（List + src/ShopInfo + src/ShopList）在标准版中已演进合并进 `pages/shop/List.vue` 改版（ShopInfo/ShopList 行卡片 + keyword 搜索），**该改版已随 M1 迁入 o2o**。
- o2o 的 `DefSearchNew.vue` 店铺搜索本就跳 `/shop/list?keyword=`（非 `/shopNew/list`），M1 后该链路完整可用。
- **结论：shopNew 不迁，已被 M1 标准版店铺街取代**；跨境 `DefSearchNew` 对 `/shopnew/list` 的路径判断在 o2o 不存在该路由，无影响。

## S5 验收

- [x] 无代码改动，无需构建验证（沿用 M1 的 build ✅）
- [x] 更新执行章程 §四进度表（M6 分析结案）
