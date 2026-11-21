-- 016 配送员已取消订单展示与移除（论坛 B 类第 18 项，帖 417866，我方新增）：
-- 配送员可把指派给自己的已取消派送单从本人列表中移除（隐藏）。移除只置
-- 配送员侧标记，不删数据：后台派送单列表/详情、买家侧进度均不受影响，
-- 后台仍可按状态=已取消查到该单（含取消方/原因/时间）。
-- staff_hidden：0-未移除（默认）/ 1-已从配送员侧移除；
-- staff_hide_time：配送员移除时间（秒级 Unix 时间戳，未移除为 NULL）。
-- 幂等：ADD COLUMN IF NOT EXISTS，可重复执行。生产 auto_migrate=false，
-- 部署时先执行本 SQL 再发后端二进制（实体已含新列）。

ALTER TABLE delivery_order ADD COLUMN IF NOT EXISTS staff_hidden integer NOT NULL DEFAULT 0;
ALTER TABLE delivery_order ADD COLUMN IF NOT EXISTS staff_hide_time integer;
