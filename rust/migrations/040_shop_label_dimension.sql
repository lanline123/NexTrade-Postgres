-- 店铺标签三期：类目榜与地区榜（论坛帖 207878955346177229，我方新增）。
-- 设计：~/workspace/shop-label-phase3/REPORT.md 第二、三节。
-- 幂等：ADD COLUMN IF NOT EXISTS，可重复执行。
-- 生产 auto_migrate=false，部署时先执行本 SQL 再发后端（新二进制会读写
-- shop_label_rule 的维度列与 shop_label_rel.scope_text；旧二进制不读
-- 新列，前后兼容）。
-- 编号说明：编号迁移主干已占用至 038（037 采购明细部分唯一 /
-- 038 采购结算应付），本迁移顺延 039。若主聊合并时重排编号，改文件名
-- 即可（迁移执行器只认文件名，代码无引用）。
BEGIN;

-- 1) 规则表扩维度（二期只有全平台榜，存量行 dimension='platform' 即
--    原口径，行为逐字不变）：
--    dimension：platform 全平台 / category 类目榜 / region 地区榜；
--    dimension_id：类目榜 = 一级类目 category_id，地区榜 = region_id
--      （省 level=1 或市 level=2），全平台恒 0；
--    dimension_name：维度名快照（类目名 / 地区全名），保存时落、
--      重算时刷新，供规则列表展示与追溯。
ALTER TABLE shop_label_rule ADD COLUMN IF NOT EXISTS dimension      varchar(20)  NOT NULL DEFAULT 'platform';
ALTER TABLE shop_label_rule ADD COLUMN IF NOT EXISTS dimension_id   integer      NOT NULL DEFAULT 0;
ALTER TABLE shop_label_rule ADD COLUMN IF NOT EXISTS dimension_name varchar(100) NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_shop_label_rule_dimension ON shop_label_rule (dimension, dimension_id);

-- 2) 分配关系加徽章维度前缀快照：维度榜授标时由规则维度生成
--    （类目榜「{类目}类目」/ 省榜「{省}」/ 市榜「{省}{市}」），买家侧
--    回填组装为「{scope_text}·{标签名}」，保证类目/地区榜徽章自带
--    维度语义、不被误读为全平台榜。手动行与全平台榜行恒为空串。
ALTER TABLE shop_label_rel ADD COLUMN IF NOT EXISTS scope_text varchar(100) NOT NULL DEFAULT '';

COMMIT;
