-- ============================================================
-- 001_core_tables.sql
-- 核心表结构初始化（幂等：IF NOT EXISTS）
--
-- 说明：Rust 版本直接复用原 Java 系统的 MySQL 数据库结构。
-- 若是全新安装，先导入原项目 database/ 下的完整建库脚本，
-- 本文件仅用于补齐 Rust 版所需的辅助表。
-- ============================================================

-- 迁移记录表（见 db::run_migrations）
-- CREATE TABLE IF NOT EXISTS _tig_migrations (...);

-- ------------------------------------------------------------
-- 后台登录日志（原 Java AdminLog 的轻量替代，可选）
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS admin_login_log (
    log_id      BIGINT       NOT NULL AUTO_INCREMENT,
    admin_id    INT          NOT NULL,
    username    VARCHAR(64)  NOT NULL DEFAULT '',
    login_ip    VARCHAR(64)  NOT NULL DEFAULT '',
    login_time  BIGINT       NOT NULL DEFAULT 0,
    success     TINYINT      NOT NULL DEFAULT 1,
    fail_reason VARCHAR(255) NOT NULL DEFAULT '',
    PRIMARY KEY (log_id),
    KEY idx_admin_id (admin_id),
    KEY idx_login_time (login_time)
) ENGINE = InnoDB
  DEFAULT CHARSET = utf8mb4
  COLLATE = utf8mb4_general_ci COMMENT ='后台登录日志';

-- ------------------------------------------------------------
-- 支付流水（对齐原 finance.Paylog，供支付回调幂等使用）
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS paylog_rust (
    paylog_id     BIGINT        NOT NULL AUTO_INCREMENT,
    order_id      INT           NOT NULL DEFAULT 0,
    order_sn      VARCHAR(32)   NOT NULL DEFAULT '',
    user_id       INT           NOT NULL DEFAULT 0,
    pay_type_id   INT           NOT NULL DEFAULT 0,
    trade_no      VARCHAR(64)   NOT NULL DEFAULT '',
    out_trade_no  VARCHAR(64)   NOT NULL DEFAULT '',
    paid_amount   DECIMAL(20, 4) NOT NULL DEFAULT 0,
    status        TINYINT       NOT NULL DEFAULT 1 COMMENT '1-成功 2-失败 3-退款',
    notify_raw    TEXT COMMENT '渠道回调原始报文',
    create_time   BIGINT        NOT NULL DEFAULT 0,
    PRIMARY KEY (paylog_id),
    UNIQUE KEY uk_out_trade_no (out_trade_no),
    KEY idx_order_sn (order_sn),
    KEY idx_create_time (create_time)
) ENGINE = InnoDB
  DEFAULT CHARSET = utf8mb4
  COLLATE = utf8mb4_general_ci COMMENT ='支付流水(Rust)';
