-- 2026-11-15: 询价功能（第 1 期）+ 认证看价（第 0 期）schema 变更。
-- 商品/SKU 两级询价标志：product.is_inquiry（商品级总开关）、
-- product_sku.is_inquiry（型号级开关，覆盖商品级，支撑同商品部分
-- 型号有价、部分型号询价）。价格列均为 NOT NULL，「无价」不能用
-- NULL/0 表达，故用显式标志位。
-- price_inquiry 留资扩列：公司名称、职务、SKU 维度（回拨时要知道
-- 客户问的是哪个型号，sku_value 为提交时的规格文本快照）。
-- 幂等：全部 ADD COLUMN IF NOT EXISTS，可重复执行。
BEGIN;

ALTER TABLE product     ADD COLUMN IF NOT EXISTS is_inquiry integer NOT NULL DEFAULT 0;
ALTER TABLE product_sku ADD COLUMN IF NOT EXISTS is_inquiry integer NOT NULL DEFAULT 0;

ALTER TABLE price_inquiry ADD COLUMN IF NOT EXISTS company_name varchar(128);
ALTER TABLE price_inquiry ADD COLUMN IF NOT EXISTS job_title    varchar(64);
ALTER TABLE price_inquiry ADD COLUMN IF NOT EXISTS sku_id       integer;
ALTER TABLE price_inquiry ADD COLUMN IF NOT EXISTS sku_value    varchar(255);

COMMIT;
