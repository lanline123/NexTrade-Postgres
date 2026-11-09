-- ============================================================================
-- NexTrade Java 数据库 → Rust 后端兼容修正
-- ============================================================================
--
-- 适用场景：
--   把 Java 原版 `pure.sql` 导入 MySQL/MariaDB 后，改用 Rust 后端。
--   Rust/sqlx 的实体类型比 Java/MyBatis 严格，以下三类列会导致运行时
--   解码失败或查询 400：
--
--   1. INT/TINYINT/SMALLINT/MEDIUMINT/BIGINT UNSIGNED
--      → Rust 使用 i32/i64，sqlx 报:
--        mismatched types ... SQL type `INT UNSIGNED`
--
--   2. JSON
--      → Rust 实体使用 String/Option<String>，sqlx 报:
--        SQL type `JSON` is not compatible with Rust type `String`
--
--   3. DATE
--      → Rust 实体使用 String/Option<String>，sqlx 报:
--        SQL type `DATE` is not compatible with Rust type `String`
--
--   4. DATETIME
--      → Rust 实体使用 String/Option<String>，sqlx 报:
--        SQL type `DATETIME` is not compatible with rust type `String`
--        （2026-10-21 线上实例：`wechat_live.create_time`
--           → /adminapi/content/wechatLive/list 500）
--
-- 执行方式（推荐 mysql 命令行，支持 DELIMITER）：
--
--   mysql -h 127.0.0.1 -u root -p 你的数据库名 < rust/数据库兼容修正.sql
--
-- 也可以用支持 DELIMITER 的客户端（Navicat / DBeaver / phpMyAdmin）导入。
--
-- 重要：
--   * 执行前务必备份数据库。
--   * 本脚本可重复执行；已转换的列不会被再次处理。
--   * 整数 UNSIGNED 转有符号时，超过有符号上限的异常值会先钳制到上限。
--   * 只处理整数 UNSIGNED；DECIMAL UNSIGNED 不影响 sqlx 的 i32/i64 解码，
--     脚本不会改动。
--   * 脚本末尾会输出剩余未转换列数量，四个结果都应为 0。
--
-- ============================================================================

SET NAMES utf8mb4;
SET SESSION group_concat_max_len = 1024 * 1024;
SET FOREIGN_KEY_CHECKS = 0;


-- ============================================================================
-- 第一步：整数 UNSIGNED → 有符号整数
-- ============================================================================
-- Java dump 中大量主键、状态、店铺 ID 是 INT UNSIGNED，转成有符号后
-- Rust 的 i32/i64 才能正常解码。保留 NULL/NOT NULL、默认值、AUTO_INCREMENT、
-- COMMENT 和显示宽度。

DROP PROCEDURE IF EXISTS nextrade_fix_unsigned_columns;

DELIMITER $$

CREATE PROCEDURE nextrade_fix_unsigned_columns()
BEGIN
  DECLARE done INT DEFAULT 0;
  DECLARE v_table VARCHAR(64);
  DECLARE v_column VARCHAR(64);
  DECLARE v_data_type VARCHAR(64);
  DECLARE v_column_type VARCHAR(255);
  DECLARE v_nullable VARCHAR(3);
  DECLARE v_default VARCHAR(255);
  DECLARE v_extra VARCHAR(255);
  DECLARE v_comment VARCHAR(2048);
  DECLARE v_max BIGINT;
  DECLARE v_new_type VARCHAR(255);

  DECLARE cur CURSOR FOR
    SELECT TABLE_NAME,
           COLUMN_NAME,
           DATA_TYPE,
           COLUMN_TYPE,
           IS_NULLABLE,
           COLUMN_DEFAULT,
           EXTRA,
           COLUMN_COMMENT
      FROM information_schema.COLUMNS
     WHERE TABLE_SCHEMA = DATABASE()
       AND DATA_TYPE IN ('tinyint', 'smallint', 'mediumint', 'int', 'bigint')
       AND COLUMN_TYPE LIKE '%unsigned%'
     ORDER BY TABLE_NAME, ORDINAL_POSITION;

  DECLARE CONTINUE HANDLER FOR NOT FOUND SET done = 1;

  OPEN cur;

  read_loop: LOOP
    FETCH cur INTO v_table, v_column, v_data_type, v_column_type,
                  v_nullable, v_default, v_extra, v_comment;

    IF done THEN
      LEAVE read_loop;
    END IF;

    -- 先把超出有符号上限的异常值钳制到上限，避免 ALTER 报 Out of range
    SET v_max = CASE v_data_type
      WHEN 'tinyint'   THEN 127
      WHEN 'smallint'  THEN 32767
      WHEN 'mediumint' THEN 8388607
      WHEN 'int'       THEN 2147483647
      WHEN 'bigint'    THEN 9223372036854775807
      ELSE NULL
    END;

    IF v_max IS NOT NULL THEN
      SET @clamp_sql = CONCAT(
        'UPDATE `', v_table, '` SET `', v_column, '` = ', v_max,
        ' WHERE `', v_column, '` > ', v_max
      );
      PREPARE clamp_stmt FROM @clamp_sql;
      EXECUTE clamp_stmt;
      DEALLOCATE PREPARE clamp_stmt;
    END IF;

    -- 去掉 unsigned / zerofill，保留类型与显示宽度
    SET v_new_type = REPLACE(REPLACE(v_column_type, ' unsigned', ''), ' zerofill', '');

    SET @alter_sql = CONCAT(
      'ALTER TABLE `', v_table, '` MODIFY COLUMN `', v_column, '` ', v_new_type,
      IF(v_nullable = 'NO', ' NOT NULL', ' NULL'),
      IF(v_default IS NULL,
         IF(v_nullable = 'YES', ' DEFAULT NULL', ''),
         CONCAT(' DEFAULT ', QUOTE(v_default))),
      IF(LOCATE('auto_increment', v_extra) > 0, ' AUTO_INCREMENT', ''),
      IF(v_comment = '', '', CONCAT(' COMMENT ', QUOTE(v_comment)))
    );

    PREPARE alter_stmt FROM @alter_sql;
    EXECUTE alter_stmt;
    DEALLOCATE PREPARE alter_stmt;
  END LOOP;

  CLOSE cur;
END$$

DELIMITER ;

CALL nextrade_fix_unsigned_columns();
DROP PROCEDURE IF EXISTS nextrade_fix_unsigned_columns;


-- ============================================================================
-- 第二步：JSON → TEXT
-- ============================================================================
-- Java 实体中的 JSON 字段在 Rust 里按 String/Option<String> 读取。
-- JSON 转 TEXT 不丢内容；应用层仍按 JSON 字符串处理。

DROP PROCEDURE IF EXISTS nextrade_fix_json_columns;

DELIMITER $$

CREATE PROCEDURE nextrade_fix_json_columns()
BEGIN
  DECLARE done INT DEFAULT 0;
  DECLARE v_table VARCHAR(64);
  DECLARE v_column VARCHAR(64);
  DECLARE v_nullable VARCHAR(3);
  DECLARE v_comment VARCHAR(2048);

  DECLARE cur CURSOR FOR
    SELECT TABLE_NAME, COLUMN_NAME, IS_NULLABLE, COLUMN_COMMENT
      FROM information_schema.COLUMNS
     WHERE TABLE_SCHEMA = DATABASE()
       AND DATA_TYPE = 'json'
     ORDER BY TABLE_NAME, ORDINAL_POSITION;

  DECLARE CONTINUE HANDLER FOR NOT FOUND SET done = 1;

  OPEN cur;

  read_loop: LOOP
    FETCH cur INTO v_table, v_column, v_nullable, v_comment;

    IF done THEN
      LEAVE read_loop;
    END IF;

    SET @json_alter_sql = CONCAT(
      'ALTER TABLE `', v_table, '` MODIFY COLUMN `', v_column, '` TEXT',
      IF(v_nullable = 'NO', ' NOT NULL', ' NULL'),
      IF(v_comment = '', '', CONCAT(' COMMENT ', QUOTE(v_comment)))
    );

    PREPARE json_alter_stmt FROM @json_alter_sql;
    EXECUTE json_alter_stmt;
    DEALLOCATE PREPARE json_alter_stmt;
  END LOOP;

  CLOSE cur;
END$$

DELIMITER ;

CALL nextrade_fix_json_columns();
DROP PROCEDURE IF EXISTS nextrade_fix_json_columns;


-- ============================================================================
-- 第三步：DATE → VARCHAR(10)
-- ============================================================================
-- Rust 实体把这些日期字段声明为 String/Option<String>。
-- DATE 的文本形式就是 YYYY-MM-DD，转 VARCHAR(10) 不丢数据，
-- 原有按字典序比较/排序仍然成立。

DROP PROCEDURE IF EXISTS nextrade_fix_date_columns;

DELIMITER $$

CREATE PROCEDURE nextrade_fix_date_columns()
BEGIN
  DECLARE done INT DEFAULT 0;
  DECLARE v_table VARCHAR(64);
  DECLARE v_column VARCHAR(64);
  DECLARE v_nullable VARCHAR(3);
  DECLARE v_comment VARCHAR(2048);

  DECLARE cur CURSOR FOR
    SELECT TABLE_NAME, COLUMN_NAME, IS_NULLABLE, COLUMN_COMMENT
      FROM information_schema.COLUMNS
     WHERE TABLE_SCHEMA = DATABASE()
       AND DATA_TYPE = 'date'
     ORDER BY TABLE_NAME, ORDINAL_POSITION;

  DECLARE CONTINUE HANDLER FOR NOT FOUND SET done = 1;

  OPEN cur;

  read_loop: LOOP
    FETCH cur INTO v_table, v_column, v_nullable, v_comment;

    IF done THEN
      LEAVE read_loop;
    END IF;

    SET @date_alter_sql = CONCAT(
      'ALTER TABLE `', v_table, '` MODIFY COLUMN `', v_column, '` VARCHAR(10)',
      IF(v_nullable = 'NO', ' NOT NULL', ' NULL'),
      IF(v_comment = '', '', CONCAT(' COMMENT ', QUOTE(v_comment)))
    );

    PREPARE date_alter_stmt FROM @date_alter_sql;
    EXECUTE date_alter_stmt;
    DEALLOCATE PREPARE date_alter_stmt;
  END LOOP;

  CLOSE cur;
END$$

DELIMITER ;

CALL nextrade_fix_date_columns();
DROP PROCEDURE IF EXISTS nextrade_fix_date_columns;


-- ============================================================================
-- 第四步：DATETIME → VARCHAR(19)
-- ============================================================================
-- Rust 实体把这些时间字段声明为 String/Option<String>
-- （见 `crates/core/src/entities/*.rs`，例如 `wechat_live.create_time`）。
-- DATETIME 的文本形式 `YYYY-MM-DD HH:MM:SS` 正好 19 字符；
-- 转 VARCHAR(19) 不丢数据，字典序比较/排序与原语义一致。
--
-- ⚠️ 转为 VARCHAR 后 `DEFAULT CURRENT_TIMESTAMP` / `ON UPDATE CURRENT_TIMESTAMP`
--    不再可用，故：
--    * 默认值改为 NULL（写入端由 Rust 显式赋值）；
--    * 原 NOT NULL 列保留 NOT NULL（Rust 侧总会写值）；
--    * 依赖 DB 自动更新的列（ON UPDATE），由 Rust 代码显式写时间。
--
-- 只转换**实体里确实声明为 String 的时间列**所在表现存的全部 datetime 列；
-- 若某表的 datetime 列在实体里是 i32/i64（UNIX 时间戳），**不能**转 —— 见下方
-- 排除清单。

DROP PROCEDURE IF EXISTS nextrade_fix_datetime_columns;

DELIMITER $$

CREATE PROCEDURE nextrade_fix_datetime_columns()
BEGIN
  DECLARE done INT DEFAULT 0;
  DECLARE v_table VARCHAR(64);
  DECLARE v_column VARCHAR(64);
  DECLARE v_nullable VARCHAR(3);
  DECLARE v_comment VARCHAR(2048);

  DECLARE cur CURSOR FOR
    SELECT TABLE_NAME, COLUMN_NAME, IS_NULLABLE, COLUMN_COMMENT
      FROM information_schema.COLUMNS
     WHERE TABLE_SCHEMA = DATABASE()
       AND DATA_TYPE IN ('datetime', 'timestamp')
     ORDER BY TABLE_NAME, ORDINAL_POSITION;

  DECLARE CONTINUE HANDLER FOR NOT FOUND SET done = 1;

  OPEN cur;

  read_loop: LOOP
    FETCH cur INTO v_table, v_column, v_nullable, v_comment;

    IF done THEN
      LEAVE read_loop;
    END IF;

    SET @dt_alter_sql = CONCAT(
      'ALTER TABLE `', v_table, '` MODIFY COLUMN `', v_column, '` VARCHAR(19)',
      IF(v_nullable = 'NO', ' NOT NULL', ' NULL'),
      IF(v_comment = '', '', CONCAT(' COMMENT ', QUOTE(v_comment)))
    );

    PREPARE dt_alter_stmt FROM @dt_alter_sql;
    EXECUTE dt_alter_stmt;
    DEALLOCATE PREPARE dt_alter_stmt;
  END LOOP;

  CLOSE cur;
END$$

DELIMITER ;

CALL nextrade_fix_datetime_columns();
DROP PROCEDURE IF EXISTS nextrade_fix_datetime_columns;


-- ============================================================================
-- 第五步：校验结果
-- ============================================================================
-- 四个 remaining_* 都应为 0。
-- decimal_unsigned 仅作信息展示：DECIMAL UNSIGNED 不影响 Rust i32/i64。

SET FOREIGN_KEY_CHECKS = 1;

SELECT 'remaining_unsigned_integer_columns' AS check_item,
       COUNT(*) AS remaining_count
  FROM information_schema.COLUMNS
 WHERE TABLE_SCHEMA = DATABASE()
   AND DATA_TYPE IN ('tinyint', 'smallint', 'mediumint', 'int', 'bigint')
   AND COLUMN_TYPE LIKE '%unsigned%'

UNION ALL

SELECT 'remaining_json_columns' AS check_item,
       COUNT(*) AS remaining_count
  FROM information_schema.COLUMNS
 WHERE TABLE_SCHEMA = DATABASE()
   AND DATA_TYPE = 'json'

UNION ALL

SELECT 'remaining_date_columns' AS check_item,
       COUNT(*) AS remaining_count
  FROM information_schema.COLUMNS
 WHERE TABLE_SCHEMA = DATABASE()
   AND DATA_TYPE = 'date'

UNION ALL

SELECT 'remaining_datetime_columns' AS check_item,
       COUNT(*) AS remaining_count
  FROM information_schema.COLUMNS
 WHERE TABLE_SCHEMA = DATABASE()
   AND DATA_TYPE IN ('datetime', 'timestamp')

UNION ALL

SELECT 'decimal_unsigned_columns' AS check_item,
       COUNT(*) AS remaining_count
  FROM information_schema.COLUMNS
 WHERE TABLE_SCHEMA = DATABASE()
   AND DATA_TYPE = 'decimal'
   AND COLUMN_TYPE LIKE '%unsigned%';

-- ============================================================================
-- 完成
-- ============================================================================
