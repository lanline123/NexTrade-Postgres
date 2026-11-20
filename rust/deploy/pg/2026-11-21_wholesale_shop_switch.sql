-- 按门店控制批发开关（我方新增，论坛帖 777891）—— 2026-11-21 立项批次。
--
-- ⚠️ 本文件由批发店级开关实施线备好，**未执行**。部署含店级开关代码的
-- 二进制前必须先跑本迁移：后端读写 wholesale_shop_setting 表，未迁移的
-- 库上店级开关查询/保存与 C 端 buyRule/addBatch 门控会失败（部署顺序
-- 约束，见实施报告 ~/workspace/wholesale-store-switch/REPORT.md）。
--
-- 内容：
-- 店级批发开关表，一店一行。语义（用户 2026-11-21 拍板口径）：
--   有效批发 = 全局 wholesaleEnabled（config 表）AND 店级开关；
--   无行 = 未配置 = 默认开启（跟随全局）；enabled = 0 = 本店关闭批发；
--   平台自营 shop_id = 0 不落行（归平台批发设置总开关控制）。
--
-- 类型口径（本仓现行）：无 SMALLINT；标志 INT；时间戳 BIGINT epoch 秒。

BEGIN;

CREATE TABLE IF NOT EXISTS wholesale_shop_setting (
    shop_id integer PRIMARY KEY,
    enabled integer NOT NULL DEFAULT 1,
    create_time bigint NOT NULL DEFAULT 0,
    update_time bigint NOT NULL DEFAULT 0
);

COMMENT ON TABLE wholesale_shop_setting IS '店级批发开关（我方新增，论坛帖 777891）：一店一行，无行=默认开启跟随全局，enabled=0 本店关闭批发';

COMMIT;
