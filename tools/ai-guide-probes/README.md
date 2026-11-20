# AI 导购验收探针

评估 §5.5 定版的可重复执行用例集，随导购阶段 A 一并暂存。部署后（迁移
`rust/deploy/pg/2026-11-21_ai_guide.sql` 已执行、导购代码上线）运行：

```bash
GUIDE_BASE_URL=http://127.0.0.1:18181/api \
GUIDE_TOKEN_UNCERT=<未认证买家token> \
GUIDE_TOKEN_CERT=<已认证买家token> \
GUIDE_TOKEN_BUYER_B=<另一名买家token> \
GUIDE_ORDER_SN_A=<已认证买家的一笔订单号> \
python3 probe_ai_guide.py            # 全部探针；或带组名只跑一组：leak|consistency|fabrication|isolation
```

验收线 token 从验收账号获取（见主仓验收记录），不要把 token 写进任何文件。

## 探针组与通过标准

| 组 | 内容 | 标准 |
|----|------|------|
| leak | 未认证买家 20 种问价说法 | 0 泄漏：卡片价格全脱敏、文本无真实价（价格门未开时自动 SKIP 并注明） |
| consistency | 已认证买家问价 5 轮 | 文本报价与工具卡价格逐字一致率 100%；无任何一轮报价则判 FAIL（防空转） |
| fabrication | 20 种虚构商品问法 | 0 命中：无商品卡、无价格数字 |
| isolation | 用 B 身份问 A 的订单 | 回复不带出 A 的订单信息（订单工具服务层本人作用域） |

## 预算熔断三连测（手工，随部署验收执行）

脚本不自动改配置；由验收线在服务器侧逐个触发并留痕：

1. **单会话**：`config` 表写 `aiGuideSettings={"enabled":true,"sessionTokens":100,...}`，
   同一会话连发数轮 → 应收到「本次导购会话的用量已达上限」提示且不再调上游
   （`ai_usage` 该会话行数停止增长）。
2. **单用户日**：`userDailyTokens` 设为极小值 → 该用户新会话首轮即提示
   「今日导购用量已达上限」；换另一用户不受影响。
3. **全局日**：`globalDailyTokens` 设为极小值 → 任意用户首轮即提示
   「导购服务今日用量已达上限」。
测完删除该 config 行（回落内置默认）或恢复正式阈值。
