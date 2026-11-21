-- PC 导航「扫码入口」（论坛 B 类缓做项 #25 底部 UI 企业微信/公众号二维码入口，我方新增）：
-- pc_navigation 增加入口类型三列，供侧边导航（type=4）条目配置为企微/公众号二维码入口，
-- PC 前台悬浮工具栏点击弹出二维码大图。移动端底部导航的同名字段走装修 JSON，不涉及本表。
-- 幂等：ADD COLUMN IF NOT EXISTS，可重复执行。生产 auto_migrate=false，部署时先执行本 SQL 再发后端。
-- 编号说明：本线开工时仓库最大编号为 015（015_site_clone_tables）；暂存前复查主仓，
-- 并行线已占用 016（配送员已取消订单）/ 017（分销素材媒体类型）/ 018（商品销售渠道），
-- 本迁移顺延 019。若主聊合并时再重排编号，改文件名即可（迁移执行器只认文件名，代码无引用）。
BEGIN;

ALTER TABLE pc_navigation
    ADD COLUMN IF NOT EXISTS entry_type varchar(16) NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS qr_type    varchar(16) NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS qr_image   varchar(500) NOT NULL DEFAULT '';

COMMENT ON COLUMN pc_navigation.entry_type IS '入口类型（我方新增）：''''=普通链接 / qrcode=扫码入口';
COMMENT ON COLUMN pc_navigation.qr_type IS '二维码类型（我方新增）：wecom=企业微信 / official=公众号，entry_type=qrcode 时有效';
COMMENT ON COLUMN pc_navigation.qr_image IS '二维码图片 URL（我方新增），entry_type=qrcode 时有效';

COMMIT;
