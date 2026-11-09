-- 紧急兼容：为审计字段补默认值，修复 Rust INSERT 未显式写审计列时的 1364
-- 适用：Java 原始 schema 中 create_by_id/create_by_name/update_by_id/update_by_name 等
--       NOT NULL 且无 DEFAULT，Rust 某些插入路径没有写这些列。
SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

DROP PROCEDURE IF EXISTS nextrade_fix_audit_defaults;

DELIMITER $$

CREATE PROCEDURE nextrade_fix_audit_defaults()
BEGIN
  DECLARE done INT DEFAULT 0;
  DECLARE v_table VARCHAR(64);
  DECLARE v_column VARCHAR(64);
  DECLARE v_data_type VARCHAR(64);
  DECLARE v_default VARCHAR(64);

  DECLARE cur CURSOR FOR
    SELECT TABLE_NAME, COLUMN_NAME, DATA_TYPE
      FROM information_schema.COLUMNS
     WHERE TABLE_SCHEMA = DATABASE()
       AND COLUMN_NAME IN ('create_by_id', 'create_by_name', 'update_by_id', 'update_by_name',
                           'create_time', 'update_time')
       AND IS_NULLABLE = 'NO'
       AND COLUMN_DEFAULT IS NULL
     ORDER BY TABLE_NAME, ORDINAL_POSITION;

  DECLARE CONTINUE HANDLER FOR NOT FOUND SET done = 1;

  OPEN cur;

  read_loop: LOOP
    FETCH cur INTO v_table, v_column, v_data_type;
    IF done THEN
      LEAVE read_loop;
    END IF;

    SET v_default = CASE
      WHEN v_data_type IN ('char', 'varchar', 'text', 'tinytext', 'mediumtext', 'longtext') THEN ''''''
      ELSE '0'
    END;

    SET @sql = CONCAT('ALTER TABLE `', v_table, '` ALTER COLUMN `', v_column,
                      '` SET DEFAULT ', v_default);
    PREPARE stmt FROM @sql;
    EXECUTE stmt;
    DEALLOCATE PREPARE stmt;
  END LOOP;

  CLOSE cur;
END$$

DELIMITER ;

CALL nextrade_fix_audit_defaults();
DROP PROCEDURE IF EXISTS nextrade_fix_audit_defaults;

SET FOREIGN_KEY_CHECKS = 1;
