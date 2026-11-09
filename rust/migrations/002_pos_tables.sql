-- ============================================================
-- 002_pos_tables.sql
-- POS 收银模块补齐表（建表幂等；两条 ALTER 仅可执行一次，MySQL 8 不支持 IF NOT EXISTS）
-- ============================================================

-- 收银桌台区
CREATE TABLE IF NOT EXISTS `pos_table_area` (
  `area_id` int NOT NULL AUTO_INCREMENT COMMENT '区域ID',
  `shop_id` int NOT NULL DEFAULT 0 COMMENT '店铺ID',
  `area_name` varchar(64) NOT NULL DEFAULT '' COMMENT '区域名称',
  `sort` int NOT NULL DEFAULT 0 COMMENT '排序',
  `add_time` int DEFAULT NULL,
  PRIMARY KEY (`area_id`),
  KEY `idx_shop` (`shop_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci COMMENT='POS桌台区域';

-- 收银桌台
CREATE TABLE IF NOT EXISTS `pos_table` (
  `table_id` int NOT NULL AUTO_INCREMENT COMMENT '桌台ID',
  `shop_id` int NOT NULL DEFAULT 0 COMMENT '店铺ID',
  `area_id` int NOT NULL DEFAULT 0 COMMENT '区域ID',
  `table_name` varchar(64) NOT NULL DEFAULT '' COMMENT '桌台名称',
  `capacity` int NOT NULL DEFAULT 4 COMMENT '容纳人数',
  `status` tinyint NOT NULL DEFAULT 0 COMMENT '0空闲 1占用',
  `order_id` int NOT NULL DEFAULT 0 COMMENT '当前订单ID',
  `user_id` int NOT NULL DEFAULT 0 COMMENT '当前绑定会员',
  `remark` varchar(255) DEFAULT '' COMMENT '备注',
  `add_time` int DEFAULT NULL,
  `update_time` int DEFAULT NULL,
  PRIMARY KEY (`table_id`),
  KEY `idx_shop_area` (`shop_id`, `area_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci COMMENT='POS桌台';

-- 挂单类别：0=工作中购物车草稿 1=已保存挂单（pos_pending_order 复用列）
ALTER TABLE `pos_pending_order` ADD COLUMN `kind` tinyint NOT NULL DEFAULT 0 COMMENT '0工作草稿 1已保存挂单';
ALTER TABLE `pos_pending_order` ADD COLUMN `table_id` int NOT NULL DEFAULT 0 COMMENT '关联桌台';

-- 挂单/购物车明细
CREATE TABLE IF NOT EXISTS `pos_pending_order_item` (
  `item_id` bigint NOT NULL AUTO_INCREMENT,
  `pending_id` int NOT NULL COMMENT '挂单ID',
  `product_id` bigint NOT NULL DEFAULT 0,
  `sku_id` bigint NOT NULL DEFAULT 0,
  `product_name` varchar(768) DEFAULT '',
  `pic_thumb` varchar(500) DEFAULT '',
  `price` decimal(10,2) NOT NULL DEFAULT 0.00 COMMENT '单价（含改价）',
  `origin_price` decimal(10,2) NOT NULL DEFAULT 0.00 COMMENT '原价',
  `quantity` int NOT NULL DEFAULT 1,
  `is_checked` tinyint NOT NULL DEFAULT 1,
  `sku_data` text COMMENT '规格JSON',
  `add_time` int DEFAULT NULL,
  PRIMARY KEY (`item_id`),
  KEY `idx_pending` (`pending_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci COMMENT='POS挂单明细';
