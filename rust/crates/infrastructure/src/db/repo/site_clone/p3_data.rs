//! 站点搬家 P3 仓储（site_clone::p3_data）：营销模板 / 文章友链 /
//! 邮件消息模板 / 店铺壳四域的导出读取与导入落库、撤销删除。
//!
//! 口径（设计 `~/workspace/site-clone-design/DOMAINS.md` P3 行 + P3
//! 报告字段级口径表）：
//! - 导出只读「模板/规则/定义」行：营销域排除全部已发放/参与实例表
//!   （user_coupon / gift_cards / groupon_promotion_record /
//!   lottery_record 等根本不出现在本文件的 SQL 里）；
//! - 导入落库在本层统一执行防呆改写（券 is_show=0、促销/镜像
//!   is_available=0、礼品卡 status=0、积分兑换 is_enabled=0、充值
//!   is_show=0、文章 is_show=0、店铺壳 status=4），服务层不重复处理；
//! - 运营计数一律归 0（coupon.send_num、seckill_item.seckill_sales、
//!   lottery_prize.prize_stock_used、article.click_count）。

use sqlx::PgPool;

use nextrade_core::models::site_clone::clone::{
    CloneArticle, CloneArticleCategory, CloneArticleProduct, CloneCoupon, CloneFriendLink,
    CloneGiftCardGroup, CloneGiftCardProduct, CloneGiftScopeRef, CloneGroupon, CloneGrouponDetail,
    CloneGrouponItem, CloneGrouponShops, CloneLotteryActivity, CloneLotteryPrize,
    CloneMailTemplate, CloneMessageType, ClonePointsExchange, CloneProductGift,
    CloneProductPromotion, ClonePromotion, CloneRechargeSetting, CloneSalesmanConfig,
    CloneSeckill, CloneSeckillItem, CloneShop, CloneSignInSetting, CloneSkuRef, CloneTimeDiscount,
    CloneTimeDiscountItem,
};
use nextrade_core::{Error, Result};

/// 站点搬家 P3 仓储。
#[derive(Clone)]
pub struct SiteCloneP3Repository {
    pool: PgPool,
}

impl SiteCloneP3Repository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ———————————————————— 计数（manifest / 预览） ————————————————————

    /// 营销活动总数（券+秒杀+拼团+限时折扣+促销+礼品卡组+抽奖+积分兑换）。
    pub async fn count_marketing_activities(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM coupon WHERE COALESCE(is_delete, 0) = 0)
                  + (SELECT COUNT(*) FROM seckill)
                  + (SELECT COUNT(*) FROM groupon_promotion)
                  + (SELECT COUNT(*) FROM time_discount)
                  + (SELECT COUNT(*) FROM product_promotion)
                  + (SELECT COUNT(*) FROM gift_card_groups)
                  + (SELECT COUNT(*) FROM lottery_activity WHERE is_delete = 0)
                  + (SELECT COUNT(*) FROM points_exchange)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    pub async fn count_articles(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM article")
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    pub async fn count_friend_links(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM friend_links")
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    /// 邮件模板 + 消息类型合计。
    pub async fn count_mail_templates(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM mail_templates) + (SELECT COUNT(*) FROM message_type)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    pub async fn count_shops(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM shop WHERE COALESCE(is_delete, 0) = 0",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    // ———————————————————— 营销域导出 ————————————————————

    pub async fn list_coupons(&self) -> Result<Vec<CloneCoupon>> {
        sqlx::query_as::<_, CloneCoupon>(
            "SELECT coupon_id, coupon_name, coupon_money, coupon_discount, coupon_desc,
                    coupon_type, send_range, send_range_data, min_order_amount,
                    send_start_date, send_end_date, send_type, use_day, use_start_date,
                    use_end_date, is_show, is_global, is_new_user, enabled_click_get,
                    limit_user_rank, shop_id, limit_num, delay_day, max_order_amount,
                    coupon_unit, reduce_type
             FROM coupon WHERE COALESCE(is_delete, 0) = 0 ORDER BY coupon_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_seckills(&self) -> Result<Vec<CloneSeckill>> {
        sqlx::query_as::<_, CloneSeckill>(
            "SELECT seckill_id, seckill_name, seckill_start_time, seckill_end_time,
                    seckill_limit_num, product_id, shop_id
             FROM seckill ORDER BY seckill_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_seckill_items(&self) -> Result<Vec<CloneSeckillItem>> {
        sqlx::query_as::<_, CloneSeckillItem>(
            "SELECT rec_id, seckill_id, product_id, sku_id, seckill_price, seckill_stock,
                    seckill_start_time, seckill_end_time
             FROM seckill_item ORDER BY rec_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_groupons(&self) -> Result<Vec<CloneGroupon>> {
        sqlx::query_as::<_, CloneGroupon>(
            "SELECT groupon_promotion_id, name, start_time, end_time, group_type,
                    expiration_time, groupon_image, pc_groupon_image, share_image,
                    groupon_explain, shop_id
             FROM groupon_promotion ORDER BY groupon_promotion_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_groupon_items(&self) -> Result<Vec<CloneGrouponItem>> {
        sqlx::query_as::<_, CloneGrouponItem>(
            "SELECT groupon_promotion_item_id, groupon_promotion_id, product_id, group_type,
                    group_num, order_limit_num, is_auto_group, min_group_num, sort,
                    min_group_price
             FROM groupon_promotion_item ORDER BY groupon_promotion_item_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_groupon_details(&self) -> Result<Vec<CloneGrouponDetail>> {
        sqlx::query_as::<_, CloneGrouponDetail>(
            "SELECT groupon_detail_id, groupon_promotion_id, groupon_promotion_item_id,
                    product_id, sku_id, group_price, limit_num, is_join
             FROM groupon_promotion_detail ORDER BY groupon_detail_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_groupon_shops(&self) -> Result<Vec<CloneGrouponShops>> {
        sqlx::query_as::<_, CloneGrouponShops>(
            "SELECT groupon_shop_id, groupon_promotion_id, shop_ids
             FROM groupon_promotion_shops ORDER BY groupon_shop_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_time_discounts(&self) -> Result<Vec<CloneTimeDiscount>> {
        sqlx::query_as::<_, CloneTimeDiscount>(
            "SELECT discount_id, promotion_name, start_time, end_time, shop_id
             FROM time_discount ORDER BY discount_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_time_discount_items(&self) -> Result<Vec<CloneTimeDiscountItem>> {
        sqlx::query_as::<_, CloneTimeDiscountItem>(
            "SELECT item_id, discount_id, start_time, end_time, product_id, sku_ids, value,
                    discount_type
             FROM time_discount_item ORDER BY item_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_product_promotions(&self) -> Result<Vec<CloneProductPromotion>> {
        sqlx::query_as::<_, CloneProductPromotion>(
            "SELECT promotion_id, promotion_name, start_time, end_time, limit_user_rank,
                    \"range\", range_data, min_order_amount, max_order_amount, promotion_type,
                    promotion_type_data, is_available, sort_order, shop_id, rules_type, unit
             FROM product_promotion ORDER BY promotion_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_promotions(&self) -> Result<Vec<ClonePromotion>> {
        sqlx::query_as::<_, ClonePromotion>(
            "SELECT promotion_id, promotion_name, start_time, end_time, \"type\", shop_id,
                    relation_id, \"range\", range_data, sku_ids, member_range, member_marks,
                    member_scope
             FROM promotion WHERE COALESCE(is_delete, 0) = 0 ORDER BY promotion_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_points_exchanges(&self) -> Result<Vec<ClonePointsExchange>> {
        sqlx::query_as::<_, ClonePointsExchange>(
            "SELECT id, product_id, exchange_integral, points_deducted_amount, is_hot,
                    is_enabled, sku_id, exchange_type, coupon_id
             FROM points_exchange ORDER BY id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_product_gifts(&self) -> Result<Vec<CloneProductGift>> {
        sqlx::query_as::<_, CloneProductGift>(
            "SELECT gift_id, gift_name, gift_stock, product_id, sku_id, shop_id
             FROM product_gift ORDER BY gift_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_recharge_settings(&self) -> Result<Vec<CloneRechargeSetting>> {
        sqlx::query_as::<_, CloneRechargeSetting>(
            "SELECT recharge_id, money, discount_money, sort_order, is_show, gift_id,
                    gift_quantity
             FROM recharge_setting ORDER BY recharge_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_sign_in_settings(&self) -> Result<Vec<CloneSignInSetting>> {
        sqlx::query_as::<_, CloneSignInSetting>(
            "SELECT id, name, points, day_num, sort_order FROM sign_in_setting ORDER BY id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_gift_card_groups(&self) -> Result<Vec<CloneGiftCardGroup>> {
        sqlx::query_as::<_, CloneGiftCardGroup>(
            "SELECT card_groups_id, group_type, card_type, card_group_name, image_url,
                    validity_type, validity_days, expire_time, face_amount, status, remark,
                    shop_id, availability_stock, scope_type
             FROM gift_card_groups ORDER BY card_groups_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_gift_card_products(&self) -> Result<Vec<CloneGiftCardProduct>> {
        sqlx::query_as::<_, CloneGiftCardProduct>(
            "SELECT gift_product_id, card_groups_id, product_id, sku_id
             FROM gift_card_products ORDER BY gift_product_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_gift_scope_products(&self) -> Result<Vec<CloneGiftScopeRef>> {
        sqlx::query_as::<_, CloneGiftScopeRef>(
            "SELECT card_groups_id, product_id AS ref_id
             FROM gift_card_group_scope_products ORDER BY card_groups_id ASC, product_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_gift_scope_categories(&self) -> Result<Vec<CloneGiftScopeRef>> {
        sqlx::query_as::<_, CloneGiftScopeRef>(
            "SELECT card_groups_id, category_id AS ref_id
             FROM gift_card_group_scope_categories ORDER BY card_groups_id ASC, category_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_lotteries(&self) -> Result<Vec<CloneLotteryActivity>> {
        sqlx::query_as::<_, CloneLotteryActivity>(
            "SELECT activity_id, activity_name, start_time, end_time, cost_points,
                    daily_limit, total_limit, win_limit, decorate_config, grid_config,
                    no_win_name, no_win_image, no_win_tips, shop_id, cost_type,
                    share_reward_enabled, share_reward_count, share_reward_daily_limit,
                    share_reward_total_limit
             FROM lottery_activity WHERE is_delete = 0 ORDER BY activity_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_lottery_prizes(&self) -> Result<Vec<CloneLotteryPrize>> {
        sqlx::query_as::<_, CloneLotteryPrize>(
            "SELECT prize_id, activity_id, prize_name, prize_type, prize_value, prize_image,
                    prize_stock
             FROM lottery_prize ORDER BY prize_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 源 SKU 目录：给定商品集合的全部 SKU（营销明细引用对齐用）。
    pub async fn list_sku_refs(&self, product_ids: &[i32]) -> Result<Vec<CloneSkuRef>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        sqlx::query_as::<_, CloneSkuRef>(
            "SELECT sku_id, product_id, COALESCE(sku_sn, '') AS sku_sn
             FROM product_sku WHERE product_id = ANY($1) ORDER BY sku_id ASC",
        )
        .bind(product_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 本站某商品的 SKU（id + 货号），导入端按货号对齐源 SKU 引用。
    pub async fn load_target_skus(&self, product_id: i32) -> Result<Vec<(i32, String)>> {
        let rows: Vec<(i32, Option<String>)> = sqlx::query_as(
            "SELECT sku_id, sku_sn FROM product_sku WHERE product_id = $1 ORDER BY sku_id ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id, sn)| (id, sn.unwrap_or_default())).collect())
    }

    // ———————————————————— 内容域导出 ————————————————————

    pub async fn list_article_categories(&self) -> Result<Vec<CloneArticleCategory>> {
        sqlx::query_as::<_, CloneArticleCategory>(
            "SELECT article_category_id, article_category_name, category_sn, category_type,
                    keywords, description, sort_order, parent_id
             FROM article_category ORDER BY article_category_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_articles(&self) -> Result<Vec<CloneArticle>> {
        sqlx::query_as::<_, CloneArticle>(
            "SELECT article_id, article_title, article_category_id, article_sn, article_thumb,
                    article_tag, article_author, article_type, content, description, keywords,
                    is_hot, is_top, link
             FROM article ORDER BY article_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_article_products(&self) -> Result<Vec<CloneArticleProduct>> {
        sqlx::query_as::<_, CloneArticleProduct>(
            "SELECT article_id, goods_id FROM product_article ORDER BY article_id ASC, goods_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_friend_links(&self) -> Result<Vec<CloneFriendLink>> {
        sqlx::query_as::<_, CloneFriendLink>(
            "SELECT link_id, link_title, link_logo, link_url, sort_order
             FROM friend_links ORDER BY link_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ———————————————————— 邮件域导出 ————————————————————

    pub async fn list_mail_templates(&self) -> Result<Vec<CloneMailTemplate>> {
        sqlx::query_as::<_, CloneMailTemplate>(
            "SELECT template_id, template_code, is_html, template_subject, template_content,
                    \"type\"
             FROM mail_templates ORDER BY template_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_message_types(&self) -> Result<Vec<CloneMessageType>> {
        sqlx::query_as::<_, CloneMessageType>(
            "SELECT message_id, name, describe, send_type, is_wechat, is_mini_program,
                    is_message, is_msg, is_app, is_ding, is_wx_work, is_mail
             FROM message_type ORDER BY message_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ———————————————————— 店铺壳域导出 ————————————————————

    /// 全部未删除店铺的壳字段（豁免字段根本不查，见模型层说明）。
    pub async fn list_shops(&self) -> Result<Vec<CloneShop>> {
        sqlx::query_as::<_, CloneShop>(
            "SELECT shop_id, shop_title, shop_logo, description, contact_mobile, kefu_phone,
                    kefu_weixin, kefu_link, is_contact_kefu, kefu_inlet, ship_info, receive_info,
                    shop_type, store_parent_id, shop_cover_picture, shop_show_picture,
                    shop_contact_json, shop_open_close_json, shop_region_id,
                    shop_detailed_address, shop_longitude, shop_latitude, use_shop_category,
                    shop_show_category, auto_erase_zero, auto_deduction_points,
                    default_collection_method, auto_add_cart_after_scan,
                    auto_print_online_order, auto_print_offline_order, shop_common_remarks,
                    pos_product_source
             FROM shop WHERE COALESCE(is_delete, 0) = 0 ORDER BY shop_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_salesman_configs(&self) -> Result<Vec<CloneSalesmanConfig>> {
        sqlx::query_as::<_, CloneSalesmanConfig>(
            "SELECT id, shop_id, code, data FROM salesman_config ORDER BY id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ———————————————————— 营销域导入落库 ————————————————————

    /// 优惠券落库（is_show=0 / send_num=0 / is_delete=0 / add_time=now）。
    pub async fn insert_coupon(
        &self,
        row: &CloneCoupon,
        shop_id: Option<i32>,
        send_range_data: &str,
        now: i32,
    ) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO coupon (coupon_name, coupon_money, coupon_discount, coupon_desc,
                    coupon_type, send_range, send_range_data, min_order_amount, send_start_date,
                    send_end_date, send_type, use_day, use_start_date, use_end_date, is_show,
                    is_global, is_new_user, enabled_click_get, limit_user_rank, shop_id,
                    is_delete, limit_num, delay_day, send_num, max_order_amount, coupon_unit,
                    reduce_type, add_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 0, $15, $16,
                     $17, $18, $19, 0, $20, $21, 0, $22, $23, $24, $25) RETURNING coupon_id",
        )
        .bind(&row.coupon_name)
        .bind(row.coupon_money)
        .bind(row.coupon_discount)
        .bind(&row.coupon_desc)
        .bind(row.coupon_type)
        .bind(row.send_range)
        .bind(send_range_data)
        .bind(row.min_order_amount)
        .bind(row.send_start_date)
        .bind(row.send_end_date)
        .bind(row.send_type)
        .bind(row.use_day)
        .bind(row.use_start_date)
        .bind(row.use_end_date)
        .bind(row.is_global)
        .bind(row.is_new_user)
        .bind(row.enabled_click_get)
        .bind(&row.limit_user_rank)
        .bind(shop_id)
        .bind(row.limit_num)
        .bind(row.delay_day)
        .bind(row.max_order_amount)
        .bind(row.coupon_unit)
        .bind(row.reduce_type)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    pub async fn insert_seckill(&self, row: &CloneSeckill, shop_id: i32, product_id: Option<i32>) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO seckill (seckill_name, seckill_start_time, seckill_end_time,
                    seckill_limit_num, product_id, shop_id)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING seckill_id",
        )
        .bind(&row.seckill_name)
        .bind(row.seckill_start_time)
        .bind(row.seckill_end_time)
        .bind(row.seckill_limit_num)
        .bind(product_id)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    /// 秒杀明细落库（seckill_sales 归 0）。
    pub async fn insert_seckill_item(
        &self,
        row: &CloneSeckillItem,
        seckill_id: i32,
        product_id: i32,
        sku_id: i32,
    ) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO seckill_item (seckill_id, product_id, sku_id, seckill_price,
                    seckill_stock, seckill_sales, seckill_start_time, seckill_end_time)
             VALUES ($1, $2, $3, $4, $5, 0, $6, $7) RETURNING rec_id",
        )
        .bind(seckill_id)
        .bind(product_id)
        .bind(sku_id)
        .bind(row.seckill_price)
        .bind(row.seckill_stock)
        .bind(row.seckill_start_time)
        .bind(row.seckill_end_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    pub async fn insert_groupon(&self, row: &CloneGroupon, shop_id: Option<i32>, now: i64) -> Result<i64> {
        let id: (i64,) = sqlx::query_as(
            "INSERT INTO groupon_promotion (name, start_time, end_time, group_type,
                    expiration_time, groupon_image, pc_groupon_image, share_image,
                    groupon_explain, shop_id, add_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             RETURNING groupon_promotion_id",
        )
        .bind(&row.name)
        .bind(row.start_time)
        .bind(row.end_time)
        .bind(row.group_type)
        .bind(row.expiration_time)
        .bind(&row.groupon_image)
        .bind(&row.pc_groupon_image)
        .bind(&row.share_image)
        .bind(&row.groupon_explain)
        .bind(shop_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    pub async fn insert_groupon_item(
        &self,
        row: &CloneGrouponItem,
        promotion_id: i64,
        product_id: i64,
        now: i64,
    ) -> Result<i64> {
        let id: (i64,) = sqlx::query_as(
            "INSERT INTO groupon_promotion_item (groupon_promotion_id, product_id, group_type,
                    group_num, order_limit_num, is_auto_group, min_group_num, sort, add_time,
                    min_group_price)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             RETURNING groupon_promotion_item_id",
        )
        .bind(promotion_id)
        .bind(product_id)
        .bind(row.group_type)
        .bind(&row.group_num)
        .bind(row.order_limit_num)
        .bind(row.is_auto_group)
        .bind(row.min_group_num)
        .bind(row.sort)
        .bind(now)
        .bind(row.min_group_price)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    pub async fn insert_groupon_detail(
        &self,
        row: &CloneGrouponDetail,
        promotion_id: i64,
        item_id: i64,
        product_id: i64,
        sku_id: i64,
        now: i64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO groupon_promotion_detail (groupon_promotion_id,
                    groupon_promotion_item_id, product_id, sku_id, group_price, limit_num,
                    is_join, add_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(promotion_id)
        .bind(item_id)
        .bind(product_id)
        .bind(sku_id)
        .bind(&row.group_price)
        .bind(row.limit_num)
        .bind(row.is_join)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 拼团适用店铺行（该表 promotion 列为 i32，与主表 i64 口径不一致，
    /// 按表结构落 i32）。
    pub async fn insert_groupon_shops(&self, promotion_id: i64, shop_ids: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO groupon_promotion_shops (groupon_promotion_id, shop_ids) VALUES ($1, $2)",
        )
        .bind(i32::try_from(promotion_id).unwrap_or(0))
        .bind(shop_ids)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn insert_time_discount(&self, row: &CloneTimeDiscount, shop_id: i32, now: i32) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO time_discount (promotion_name, start_time, end_time, shop_id, add_time)
             VALUES ($1, $2, $3, $4, $5) RETURNING discount_id",
        )
        .bind(&row.promotion_name)
        .bind(row.start_time)
        .bind(row.end_time)
        .bind(shop_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    pub async fn insert_time_discount_item(
        &self,
        row: &CloneTimeDiscountItem,
        discount_id: i32,
        product_id: i32,
        sku_ids: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO time_discount_item (discount_id, start_time, end_time, product_id,
                    sku_ids, value, discount_type)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(discount_id)
        .bind(row.start_time)
        .bind(row.end_time)
        .bind(product_id)
        .bind(sku_ids)
        .bind(row.value)
        .bind(row.discount_type)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 满减/折扣/满赠落库（is_available 强制 0）。
    pub async fn insert_product_promotion(
        &self,
        row: &CloneProductPromotion,
        shop_id: i32,
        range_data: &str,
        type_data: &str,
    ) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO product_promotion (promotion_name, start_time, end_time,
                    limit_user_rank, \"range\", range_data, min_order_amount, max_order_amount,
                    promotion_type, promotion_type_data, is_available, sort_order, shop_id,
                    rules_type, unit)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 0, $11, $12, $13, $14)
             RETURNING promotion_id",
        )
        .bind(&row.promotion_name)
        .bind(row.start_time)
        .bind(row.end_time)
        .bind(&row.limit_user_rank)
        .bind(row.range)
        .bind(range_data)
        .bind(row.min_order_amount)
        .bind(row.max_order_amount)
        .bind(row.promotion_type)
        .bind(type_data)
        .bind(row.sort_order)
        .bind(shop_id)
        .bind(row.rules_type)
        .bind(row.unit)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    /// promotion 镜像行落库（is_available=0 / is_delete=0）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_promotion(
        &self,
        row: &ClonePromotion,
        shop_id: i32,
        relation_id: i32,
        range_data: Option<&str>,
        sku_ids: Option<&str>,
    ) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO promotion (promotion_name, start_time, end_time, \"type\", shop_id,
                    relation_id, \"range\", range_data, sku_ids, is_available, is_delete,
                    member_range, member_marks, member_scope)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0, 0, $10, $11, $12)
             RETURNING promotion_id",
        )
        .bind(&row.promotion_name)
        .bind(row.start_time)
        .bind(row.end_time)
        .bind(row.r#type)
        .bind(shop_id)
        .bind(relation_id)
        .bind(row.range)
        .bind(range_data)
        .bind(sku_ids)
        .bind(row.member_range)
        .bind(&row.member_marks)
        .bind(&row.member_scope)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    /// 积分兑换落库（is_enabled 强制 0）。
    pub async fn insert_points_exchange(
        &self,
        row: &ClonePointsExchange,
        product_id: i32,
        sku_id: Option<i32>,
        coupon_id: Option<i32>,
    ) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO points_exchange (product_id, exchange_integral,
                    points_deducted_amount, is_hot, is_enabled, sku_id, exchange_type, coupon_id)
             VALUES ($1, $2, $3, $4, 0, $5, $6, $7) RETURNING id",
        )
        .bind(product_id)
        .bind(row.exchange_integral)
        .bind(row.points_deducted_amount)
        .bind(row.is_hot)
        .bind(sku_id)
        .bind(row.exchange_type)
        .bind(coupon_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    pub async fn insert_product_gift(
        &self,
        row: &CloneProductGift,
        shop_id: i32,
        product_id: i32,
        sku_id: i32,
    ) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO product_gift (gift_name, gift_stock, product_id, sku_id, shop_id)
             VALUES ($1, $2, $3, $4, $5) RETURNING gift_id",
        )
        .bind(&row.gift_name)
        .bind(row.gift_stock)
        .bind(product_id)
        .bind(sku_id)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    /// 充值档位落库（is_show 强制 0）。
    pub async fn insert_recharge_setting(&self, row: &CloneRechargeSetting, gift_id: i32) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO recharge_setting (money, discount_money, sort_order, is_show, gift_id,
                    gift_quantity)
             VALUES ($1, $2, $3, 0, $4, $5) RETURNING recharge_id",
        )
        .bind(row.money)
        .bind(row.discount_money)
        .bind(row.sort_order)
        .bind(gift_id)
        .bind(row.gift_quantity)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    pub async fn find_sign_in_by_day(&self, day_num: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT id FROM sign_in_setting WHERE day_num = $1 ORDER BY id ASC LIMIT 1",
        )
        .bind(day_num)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn insert_sign_in_setting(&self, row: &CloneSignInSetting, now: i32) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO sign_in_setting (name, points, day_num, sort_order, create_time)
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(&row.name)
        .bind(row.points)
        .bind(row.day_num)
        .bind(row.sort_order)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    /// 礼品卡组落库（status 强制 0 = 关闭；add/update_time = now）。
    pub async fn insert_gift_card_group(
        &self,
        row: &CloneGiftCardGroup,
        shop_id: Option<i32>,
        now: i32,
    ) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO gift_card_groups (group_type, card_type, card_group_name, image_url,
                    validity_type, validity_days, expire_time, face_amount, status, add_time,
                    update_time, remark, shop_id, availability_stock, scope_type)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, $9, $9, $10, $11, $12, $13)
             RETURNING card_groups_id",
        )
        .bind(row.group_type)
        .bind(row.card_type)
        .bind(&row.card_group_name)
        .bind(&row.image_url)
        .bind(row.validity_type)
        .bind(row.validity_days)
        .bind(row.expire_time)
        .bind(row.face_amount)
        .bind(now)
        .bind(&row.remark)
        .bind(shop_id)
        .bind(row.availability_stock)
        .bind(row.scope_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    pub async fn insert_gift_card_product(&self, group_id: i32, product_id: i32, sku_id: i32) -> Result<()> {
        sqlx::query(
            "INSERT INTO gift_card_products (card_groups_id, product_id, sku_id) VALUES ($1, $2, $3)",
        )
        .bind(group_id)
        .bind(product_id)
        .bind(sku_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn insert_gift_scope_product(&self, group_id: i32, product_id: i32) -> Result<()> {
        sqlx::query(
            "INSERT INTO gift_card_group_scope_products (card_groups_id, product_id)
             VALUES ($1, $2) ON CONFLICT (card_groups_id, product_id) DO NOTHING",
        )
        .bind(group_id)
        .bind(product_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn insert_gift_scope_category(&self, group_id: i32, category_id: i32) -> Result<()> {
        sqlx::query(
            "INSERT INTO gift_card_group_scope_categories (card_groups_id, category_id)
             VALUES ($1, $2) ON CONFLICT (card_groups_id, category_id) DO NOTHING",
        )
        .bind(group_id)
        .bind(category_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 抽奖活动落库（is_delete=0，create/update_time=now）。
    pub async fn insert_lottery(
        &self,
        row: &CloneLotteryActivity,
        shop_id: i32,
        decorate_config: Option<&str>,
        now: i32,
    ) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO lottery_activity (activity_name, start_time, end_time, cost_points,
                    daily_limit, total_limit, win_limit, decorate_config, grid_config,
                    no_win_name, no_win_image, no_win_tips, shop_id, is_delete, create_time,
                    update_time, cost_type, share_reward_enabled, share_reward_count,
                    share_reward_daily_limit, share_reward_total_limit)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 0, $14, $14, $15,
                     $16, $17, $18, $19) RETURNING activity_id",
        )
        .bind(&row.activity_name)
        .bind(row.start_time)
        .bind(row.end_time)
        .bind(row.cost_points)
        .bind(row.daily_limit)
        .bind(row.total_limit)
        .bind(row.win_limit)
        .bind(decorate_config)
        .bind(&row.grid_config)
        .bind(&row.no_win_name)
        .bind(&row.no_win_image)
        .bind(&row.no_win_tips)
        .bind(shop_id)
        .bind(now)
        .bind(row.cost_type)
        .bind(row.share_reward_enabled)
        .bind(row.share_reward_count)
        .bind(row.share_reward_daily_limit)
        .bind(row.share_reward_total_limit)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    /// 抽奖奖品落库（prize_stock_used 归 0）。
    pub async fn insert_lottery_prize(
        &self,
        row: &CloneLotteryPrize,
        activity_id: i32,
        prize_value: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO lottery_prize (activity_id, prize_name, prize_type, prize_value,
                    prize_image, prize_stock, prize_stock_used, create_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $7)",
        )
        .bind(activity_id)
        .bind(&row.prize_name)
        .bind(row.prize_type)
        .bind(prize_value)
        .bind(&row.prize_image)
        .bind(row.prize_stock)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ———————————————————— 内容域导入落库 ————————————————————

    pub async fn find_article_category_by_name_parent(
        &self,
        name: &str,
        parent_id: i32,
    ) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT article_category_id FROM article_category
             WHERE article_category_name = $1 AND parent_id = $2
             ORDER BY article_category_id ASC LIMIT 1",
        )
        .bind(name)
        .bind(parent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn insert_article_category(&self, row: &CloneArticleCategory, parent_id: i32) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO article_category (article_category_name, category_sn, category_type,
                    keywords, description, sort_order, parent_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING article_category_id",
        )
        .bind(&row.article_category_name)
        .bind(&row.category_sn)
        .bind(row.category_type)
        .bind(&row.keywords)
        .bind(&row.description)
        .bind(row.sort_order)
        .bind(parent_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    /// 文章落库（is_show=0 / click_count=0 / add_time=now）。
    pub async fn insert_article(
        &self,
        row: &CloneArticle,
        category_id: i32,
        content: &str,
        thumb: &str,
        now: i32,
    ) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO article (article_title, article_category_id, article_sn, article_thumb,
                    article_tag, article_author, article_type, content, description, keywords,
                    is_show, add_time, is_hot, is_top, click_count, link)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 0, $11, $12, $13, 0, $14)
             RETURNING article_id",
        )
        .bind(&row.article_title)
        .bind(category_id)
        .bind(&row.article_sn)
        .bind(thumb)
        .bind(&row.article_tag)
        .bind(&row.article_author)
        .bind(row.article_type)
        .bind(content)
        .bind(&row.description)
        .bind(&row.keywords)
        .bind(now)
        .bind(row.is_hot)
        .bind(row.is_top)
        .bind(&row.link)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    pub async fn insert_article_product(&self, article_id: i32, goods_id: i32) -> Result<()> {
        sqlx::query("INSERT INTO product_article (goods_id, article_id) VALUES ($1, $2)")
            .bind(goods_id)
            .bind(article_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn find_friend_link(&self, title: &str, url: &str) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT link_id FROM friend_links WHERE link_title = $1 AND link_url = $2
             ORDER BY link_id ASC LIMIT 1",
        )
        .bind(title)
        .bind(url)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn insert_friend_link(&self, row: &CloneFriendLink) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO friend_links (link_title, link_logo, link_url, sort_order)
             VALUES ($1, $2, $3, $4) RETURNING link_id",
        )
        .bind(&row.link_title)
        .bind(&row.link_logo)
        .bind(&row.link_url)
        .bind(row.sort_order)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    // ———————————————————— 邮件域导入（按 code / id 对齐覆盖） ————————————————————

    pub async fn find_mail_template(&self, code: &str) -> Result<Option<CloneMailTemplate>> {
        sqlx::query_as::<_, CloneMailTemplate>(
            "SELECT template_id, template_code, is_html, template_subject, template_content,
                    \"type\"
             FROM mail_templates WHERE template_code = $1 LIMIT 1",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 覆盖或新建邮件模板（不动 last_send；last_modify=now）。返回是否新建。
    pub async fn upsert_mail_template(&self, row: &CloneMailTemplate, now: i32) -> Result<bool> {
        let updated = sqlx::query(
            "UPDATE mail_templates SET is_html = $2, template_subject = $3,
                    template_content = $4, \"type\" = $5, last_modify = $6
             WHERE template_code = $1",
        )
        .bind(&row.template_code)
        .bind(row.is_html)
        .bind(&row.template_subject)
        .bind(&row.template_content)
        .bind(&row.r#type)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        if updated.rows_affected() > 0 {
            return Ok(false);
        }
        sqlx::query(
            "INSERT INTO mail_templates (template_code, is_html, template_subject,
                    template_content, last_modify, last_send, \"type\")
             VALUES ($1, $2, $3, $4, $5, 0, $6)",
        )
        .bind(&row.template_code)
        .bind(row.is_html)
        .bind(&row.template_subject)
        .bind(&row.template_content)
        .bind(now)
        .bind(&row.r#type)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(true)
    }

    /// 快照回滚：按 code 还原模板字段（原行不存在的删除动作走 delete）。
    pub async fn restore_mail_template(
        &self,
        code: &str,
        is_html: i32,
        subject: &str,
        content: &str,
        ty: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE mail_templates SET is_html = $2, template_subject = $3,
                    template_content = $4, \"type\" = $5, last_modify = $6
             WHERE template_code = $1",
        )
        .bind(code)
        .bind(is_html)
        .bind(subject)
        .bind(content)
        .bind(ty)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn delete_mail_template(&self, code: &str) -> Result<()> {
        sqlx::query("DELETE FROM mail_templates WHERE template_code = $1")
            .bind(code)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn find_message_type(&self, message_id: i32) -> Result<Option<CloneMessageType>> {
        sqlx::query_as::<_, CloneMessageType>(
            "SELECT message_id, name, describe, send_type, is_wechat, is_mini_program,
                    is_message, is_msg, is_app, is_ding, is_wx_work, is_mail
             FROM message_type WHERE message_id = $1 LIMIT 1",
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 覆盖或新建消息类型（add_time 仅新建时写 now）。返回是否新建。
    pub async fn upsert_message_type(&self, row: &CloneMessageType, now: i32) -> Result<bool> {
        let updated = sqlx::query(
            "UPDATE message_type SET name = $2, describe = $3, send_type = $4, is_wechat = $5,
                    is_mini_program = $6, is_message = $7, is_msg = $8, is_app = $9,
                    is_ding = $10, is_wx_work = $11, is_mail = $12
             WHERE message_id = $1",
        )
        .bind(row.message_id)
        .bind(&row.name)
        .bind(&row.describe)
        .bind(row.send_type)
        .bind(row.is_wechat)
        .bind(row.is_mini_program)
        .bind(row.is_message)
        .bind(row.is_msg)
        .bind(row.is_app)
        .bind(row.is_ding)
        .bind(row.is_wx_work)
        .bind(row.is_mail)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        if updated.rows_affected() > 0 {
            return Ok(false);
        }
        sqlx::query(
            "INSERT INTO message_type (message_id, name, describe, send_type, is_wechat,
                    is_mini_program, is_message, is_msg, is_app, is_ding, add_time, is_wx_work,
                    is_mail)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(row.message_id)
        .bind(&row.name)
        .bind(&row.describe)
        .bind(row.send_type)
        .bind(row.is_wechat)
        .bind(row.is_mini_program)
        .bind(row.is_message)
        .bind(row.is_msg)
        .bind(row.is_app)
        .bind(row.is_ding)
        .bind(now)
        .bind(row.is_wx_work)
        .bind(row.is_mail)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(true)
    }

    pub async fn delete_message_type(&self, message_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM message_type WHERE message_id = $1")
            .bind(message_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    // ———————————————————— 店铺壳导入 ————————————————————

    /// 新建店铺壳：status=4（暂停运营）、merchant_id=0、资金归 0、
    /// 运营计数归 0、费率/定价条款不落（NULL）。父级先落 0，二遍回填。
    pub async fn insert_shop_shell(&self, row: &CloneShop, now: i32) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO shop (shop_title, add_time, shop_logo, click_count, status,
                    merchant_id, shop_money, frozen_money, contact_mobile, description,
                    kefu_phone, kefu_weixin, kefu_link, is_contact_kefu, kefu_inlet, ship_info,
                    receive_info, last_login_time, shop_type, store_parent_id,
                    shop_cover_picture, shop_show_picture, shop_contact_json,
                    shop_open_close_json, shop_region_id, shop_detailed_address,
                    shop_longitude, shop_latitude, shop_star, shop_total_star, shop_star_num,
                    shop_sales, use_shop_category, shop_show_category, auto_erase_zero,
                    auto_deduction_points, default_collection_method, auto_add_cart_after_scan,
                    auto_print_online_order, auto_print_offline_order, shop_common_remarks,
                    pos_product_source, is_delete)
             VALUES ($1, $2, $3, 0, 4, 0, 0, 0, $4, $5, $6, $7, $8, $9, $10, $11, $12, 0,
                     $13, 0, $14, $15, $16, $17, $18, $19, $20, $21, 0, 0, 0, 0, $22, $23,
                     $24, $25, $26, $27, $28, $29, $30, $31, 0) RETURNING shop_id",
        )
        .bind(&row.shop_title)
        .bind(now)
        .bind(&row.shop_logo)
        .bind(&row.contact_mobile)
        .bind(&row.description)
        .bind(&row.kefu_phone)
        .bind(&row.kefu_weixin)
        .bind(&row.kefu_link)
        .bind(row.is_contact_kefu)
        .bind(&row.kefu_inlet)
        .bind(&row.ship_info)
        .bind(&row.receive_info)
        .bind(row.shop_type)
        .bind(&row.shop_cover_picture)
        .bind(&row.shop_show_picture)
        .bind(&row.shop_contact_json)
        .bind(&row.shop_open_close_json)
        .bind(&row.shop_region_id)
        .bind(&row.shop_detailed_address)
        .bind(row.shop_longitude)
        .bind(row.shop_latitude)
        .bind(row.use_shop_category)
        .bind(&row.shop_show_category)
        .bind(row.auto_erase_zero)
        .bind(row.auto_deduction_points)
        .bind(row.default_collection_method)
        .bind(row.auto_add_cart_after_scan)
        .bind(row.auto_print_online_order)
        .bind(row.auto_print_offline_order)
        .bind(&row.shop_common_remarks)
        .bind(&row.pos_product_source)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(id.0)
    }

    /// 店铺壳父级回填（二遍：全部壳建完后按映射更新 store_parent_id）。
    pub async fn update_shop_parent(&self, shop_id: i32, parent_id: i32) -> Result<()> {
        sqlx::query("UPDATE shop SET store_parent_id = $2 WHERE shop_id = $1")
            .bind(shop_id)
            .bind(parent_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 分销规则现值（快照/覆盖判定）：返回 (行 id, data)。
    pub async fn find_salesman_config(
        &self,
        shop_id: i32,
        code: &str,
    ) -> Result<Option<(i32, Option<String>)>> {
        let row: Option<(i32, Option<String>)> = sqlx::query_as(
            "SELECT id, data FROM salesman_config WHERE shop_id = $1 AND code = $2
             ORDER BY id ASC LIMIT 1",
        )
        .bind(shop_id)
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    pub async fn upsert_salesman_config(&self, shop_id: i32, code: &str, data: Option<&str>) -> Result<()> {
        let updated = sqlx::query(
            "UPDATE salesman_config SET data = $3 WHERE shop_id = $1 AND code = $2",
        )
        .bind(shop_id)
        .bind(code)
        .bind(data)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        if updated.rows_affected() == 0 {
            sqlx::query("INSERT INTO salesman_config (shop_id, code, data) VALUES ($1, $2, $3)")
                .bind(shop_id)
                .bind(code)
                .bind(data)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        }
        Ok(())
    }

    pub async fn delete_salesman_config(&self, shop_id: i32, code: &str) -> Result<()> {
        sqlx::query("DELETE FROM salesman_config WHERE shop_id = $1 AND code = $2")
            .bind(shop_id)
            .bind(code)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 撤销店铺壳时连带清掉这些壳店铺的分销规则行。
    pub async fn delete_salesman_configs_for_shops(&self, shop_ids: &[i32]) -> Result<()> {
        if shop_ids.is_empty() {
            return Ok(());
        }
        sqlx::query("DELETE FROM salesman_config WHERE shop_id = ANY($1)")
            .bind(shop_ids)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    // ———————————————————— 撤销导入删除（仅本任务新建行） ————————————————————

    pub async fn soft_delete_coupons(&self, ids: &[i32]) -> Result<u64> {
        delete_count(&self.pool, "UPDATE coupon SET is_delete = 1 WHERE coupon_id = ANY($1)", ids).await
    }

    pub async fn soft_delete_promotions(&self, ids: &[i32]) -> Result<u64> {
        delete_count(&self.pool, "UPDATE promotion SET is_delete = 1 WHERE promotion_id = ANY($1)", ids).await
    }

    pub async fn soft_delete_lotteries(&self, ids: &[i32]) -> Result<u64> {
        // 奖品行随活动软删一并硬删（奖品无独立软删位，且只属本活动）。
        if !ids.is_empty() {
            sqlx::query("DELETE FROM lottery_prize WHERE activity_id = ANY($1)")
                .bind(ids)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        }
        delete_count(&self.pool, "UPDATE lottery_activity SET is_delete = 1 WHERE activity_id = ANY($1)", ids).await
    }

    pub async fn delete_seckills(&self, ids: &[i32]) -> Result<u64> {
        if !ids.is_empty() {
            sqlx::query("DELETE FROM seckill_item WHERE seckill_id = ANY($1)")
                .bind(ids)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        }
        delete_count(&self.pool, "DELETE FROM seckill WHERE seckill_id = ANY($1)", ids).await
    }

    pub async fn delete_groupons(&self, ids: &[i64]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        for sql in [
            "DELETE FROM groupon_promotion_detail WHERE groupon_promotion_id = ANY($1)",
            "DELETE FROM groupon_promotion_item WHERE groupon_promotion_id = ANY($1)",
        ] {
            sqlx::query(sql).bind(ids).execute(&self.pool).await.map_err(map_err)?;
        }
        let ids32: Vec<i32> = ids.iter().map(|id| i32::try_from(*id).unwrap_or(0)).collect();
        sqlx::query("DELETE FROM groupon_promotion_shops WHERE groupon_promotion_id = ANY($1)")
            .bind(&ids32)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        let res = sqlx::query("DELETE FROM groupon_promotion WHERE groupon_promotion_id = ANY($1)")
            .bind(ids)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    pub async fn delete_time_discounts(&self, ids: &[i32]) -> Result<u64> {
        if !ids.is_empty() {
            sqlx::query("DELETE FROM time_discount_item WHERE discount_id = ANY($1)")
                .bind(ids)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        }
        delete_count(&self.pool, "DELETE FROM time_discount WHERE discount_id = ANY($1)", ids).await
    }

    pub async fn delete_product_promotions(&self, ids: &[i32]) -> Result<u64> {
        delete_count(&self.pool, "DELETE FROM product_promotion WHERE promotion_id = ANY($1)", ids).await
    }

    pub async fn delete_points_exchanges(&self, ids: &[i32]) -> Result<u64> {
        delete_count(&self.pool, "DELETE FROM points_exchange WHERE id = ANY($1)", ids).await
    }

    pub async fn delete_product_gifts(&self, ids: &[i32]) -> Result<u64> {
        delete_count(&self.pool, "DELETE FROM product_gift WHERE gift_id = ANY($1)", ids).await
    }

    pub async fn delete_recharge_settings(&self, ids: &[i32]) -> Result<u64> {
        delete_count(&self.pool, "DELETE FROM recharge_setting WHERE recharge_id = ANY($1)", ids).await
    }

    pub async fn delete_sign_in_settings(&self, ids: &[i32]) -> Result<u64> {
        delete_count(&self.pool, "DELETE FROM sign_in_setting WHERE id = ANY($1)", ids).await
    }

    /// 礼品卡组撤销：绑定/范围行全删；卡组行仅当未发行任何卡时才删
    /// （已发行卡引用卡组，删组会破坏持卡人资产——保留并由调用方在
    /// 撤销结果中注明）。返回实际删除的卡组数。
    pub async fn delete_gift_card_groups(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        for sql in [
            "DELETE FROM gift_card_products WHERE card_groups_id = ANY($1)",
            "DELETE FROM gift_card_group_scope_products WHERE card_groups_id = ANY($1)",
            "DELETE FROM gift_card_group_scope_categories WHERE card_groups_id = ANY($1)",
        ] {
            sqlx::query(sql).bind(ids).execute(&self.pool).await.map_err(map_err)?;
        }
        let res = sqlx::query(
            "DELETE FROM gift_card_groups WHERE card_groups_id = ANY($1)
             AND NOT EXISTS (SELECT 1 FROM gift_cards WHERE gift_cards.card_groups_id = gift_card_groups.card_groups_id)",
        )
        .bind(ids)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    pub async fn delete_articles(&self, ids: &[i32]) -> Result<u64> {
        if !ids.is_empty() {
            sqlx::query("DELETE FROM product_article WHERE article_id = ANY($1)")
                .bind(ids)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        }
        delete_count(&self.pool, "DELETE FROM article WHERE article_id = ANY($1)", ids).await
    }

    pub async fn delete_friend_links(&self, ids: &[i32]) -> Result<u64> {
        delete_count(&self.pool, "DELETE FROM friend_links WHERE link_id = ANY($1)", ids).await
    }

    /// 店铺壳撤销：软删（is_delete=1 + status=10 关店），不物理删行
    /// （壳可能已被本站商品/订单引用行指向，物理删风险高）。
    pub async fn soft_delete_shops(&self, ids: &[i32]) -> Result<u64> {
        delete_count(
            &self.pool,
            "UPDATE shop SET is_delete = 1, status = 10 WHERE shop_id = ANY($1)",
            ids,
        )
        .await
    }
}

/// 单参数（id 数组）删除/软删通用执行，返回影响行数。
async fn delete_count(pool: &PgPool, sql: &str, ids: &[i32]) -> Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let res = sqlx::query(sql).bind(ids).execute(pool).await.map_err(map_err)?;
    Ok(res.rows_affected())
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
