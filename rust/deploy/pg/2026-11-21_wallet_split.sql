-- 2026-11-21 钱包拆分补齐（D1107，论坛帖 657752790030844828）
--
-- 背景：账户模型本身（user.commission_balance / frozen_commission_balance、
-- user_balance_log.wallet_type、user_withdraw_apply.wallet_type）在分销移植时
-- 已齐备，本文件**无结构变更**，只做一处历史数据订正：
--
-- 买家端佣金提现链（with_draw_apply_client）写流水时曾把 wallet_type
-- 硬编码为 0，导致「佣金提现冻结」「佣金提现扣除」两类流水被误标为
-- 余额账户——买家余额明细里混入佣金数字、佣金明细里缺提现记录（串账）。
-- 这两条文案全仓仅由佣金提现链写入（已 grep 实证），按文案精确回标为 1；
-- 金额与 before/after 本就是佣金账户口径，不动。本语句幂等（只命中
-- wallet_type = 0 的行）。
--
-- 执行前先跑计数（应与执行后回标行数一致）：
--   SELECT change_desc, COUNT(*) FROM user_balance_log
--   WHERE wallet_type = 0 AND change_desc IN ('佣金提现冻结', '佣金提现扣除')
--   GROUP BY change_desc;

UPDATE user_balance_log
SET wallet_type = 1
WHERE wallet_type = 0
  AND change_desc IN ('佣金提现冻结', '佣金提现扣除');

-- =====================================================================
-- 部署门禁（只读盘点，不要在本文件内自动改数）
--
-- 旧口径的买家端余额提现申请只冻结、未扣可用余额。换版后，后台审核
-- 「拒绝」会按新口径返还可用余额 —— 对旧申请执行拒绝将多返一笔（造钱）。
-- 换版前必须盘点未完结的余额提现申请并逐笔处置：
--
--   SELECT id, user_id, amount, add_time FROM user_withdraw_apply
--   WHERE wallet_type = 0 AND status = 0 ORDER BY id;
--
-- 建议处置（逐笔人工核定后执行，不在本迁移内）：
--   对余额充足的待审申请，按新口径补扣可用余额（balance -= amount，
--   并补一笔 change_desc='提现扣除余额'、wallet_type=0 的流水，
--   before/after 与冻结维度按申请时点口径填写），使其与新口径申请等价；
--   余额不足或有争议的逐笔人工处理后再换版。
-- =====================================================================
