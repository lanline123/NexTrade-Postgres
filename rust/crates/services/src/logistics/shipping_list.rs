//! 物流服务：运费模板/配送方式/物流公司。
//! 对应原 Java `ShippingTplServiceImpl` / `LogisticsCompanyServiceImpl` 等。
//!
//! TODO(域实现者)：对照 Java 源补全运费计算（按重量/件数、区域首续费用）。

use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_core::entities::{LogisticsCompany, ShippingTpl, ShippingType};
use nextrade_core::models::ShippingTplListQuery;
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

pub struct LogisticsService {
    repos: Arc<Repositories>,
}

impl LogisticsService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 运费模板分页（按登录态店铺过滤：`None` = 平台不过滤，`Some(shop)` = 本店）。
    pub async fn tpl_list(
        &self,
        q: &ShippingTplListQuery,
        shop_id: Option<i32>,
    ) -> Result<PageResult<ShippingTpl>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        // 复用 shipping_tpl 仓储的带店铺过滤分页（shop_id: Option<i32>）。
        // 两处 `ShippingTplListQuery` 分属不同 slice（shipping_list / shipping_tpl），
        // 这里按字段映射后再调用，避免把页面级 query 直接传给另一 slice 的仓储。
        let scoped_q = nextrade_core::models::logistics::shipping_tpl::ShippingTplListQuery {
            keyword: q.keyword.clone(),
            page: q.page,
            size: q.page_size,
            ..Default::default()
        };
        let (total, list) = self
            .repos
            .logistics
            .shipping_tpl
            .paginate_tpl(&scoped_q, shop_id)
            .await?;
        Ok(PageResult::new(total, list, &page))
    }

    /// 运费模板详情。
    pub async fn tpl_detail(&self, tpl_id: i32) -> Result<ShippingTpl> {
        self.repos.logistics.shipping_list
            .find_tpl(tpl_id)
            .await?
            .ok_or_else(|| Error::NotFound("运费模板不存在".into()))
    }

    /// 启用的物流公司。
    pub async fn companies(&self) -> Result<Vec<LogisticsCompany>> {
        self.repos.logistics.shipping_list.find_enabled_companies().await
    }

    /// 配送方式。
    pub async fn shipping_types(&self) -> Result<Vec<ShippingType>> {
        self.repos.logistics.shipping_list.find_all_types().await
    }

    /// 计算运费（简化版：首件/首重 + 续件/续重）。
    pub fn calc_freight(
        first_price: Decimal,
        next_price: Decimal,
        first_qty: i32,
        next_qty: i32,
        quantity: i32,
    ) -> Decimal {
        if quantity <= first_qty {
            return first_price;
        }
        let extra = quantity - first_qty;
        let extra_units = extra.div_euclid(next_qty.max(1))
            + i32::from(extra.rem_euclid(next_qty.max(1)) != 0);
        first_price + next_price * Decimal::from(extra_units)
    }

    /// 运费模板精确匹配与计费（支持区域精确匹配、默认行回退、包邮与满额包邮判定、件数/重量阶梯计费）。
    pub async fn match_and_calc_freight(
        &self,
        tpl_id: i32,
        shop_amount: Decimal,
        quantity: Decimal,
        weight: Decimal,
        volume: Decimal,
        region_ids: &[i32],
    ) -> Result<Decimal> {
        let Some(tpl) = self.repos.logistics.shipping_tpl.find_tpl_by_id(tpl_id).await? else {
            return Ok(Decimal::ZERO);
        };
        if tpl.is_free == 1 {
            return Ok(Decimal::ZERO);
        }
        let infos = self.repos.logistics.shipping_tpl.find_infos_by_tpl(tpl_id).await?;
        if infos.is_empty() {
            return Ok(Decimal::ZERO);
        }

        // 优先匹配收货地址所在区域，无匹配则回退默认行或首条
        let mut matched = None;
        if !region_ids.is_empty() {
            for info in &infos {
                if crate::trade::check::region_match(info.region_data.as_deref(), region_ids) {
                    matched = Some(info);
                    break;
                }
            }
        }
        let info = matched
            .or_else(|| infos.iter().find(|i| i.is_default == 1))
            .or_else(|| infos.first())
            .unwrap();

        let rule = nextrade_core::models::trade::check::ShippingRule {
            pricing_type: tpl.pricing_type,
            is_free: info.is_free == 1 || tpl.is_free == 1,
            start_number: info.start_number,
            start_price: info.start_price,
            add_number: info.add_number,
            add_price: info.add_price,
            free_price: info.free_price,
        };

        Ok(nextrade_core::models::trade::check::calc_shipping_fee(
            &rule,
            shop_amount,
            quantity,
            weight,
            volume,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// 金额字面量解析（`from_scientific` 只接受科学计数法，此处用常规解析）。
    fn money(s: &str) -> Decimal {
        Decimal::from_str(s).expect("测试金额字面量应合法")
    }

    #[test]
    fn freight_calc_first_tier_only() {
        // 首 1 件 10 元，续每 2 件 5 元；共 1 件 => 仅首件费
        assert_eq!(
            LogisticsService::calc_freight(money("10"), money("5"), 1, 2, 1),
            Decimal::from(10)
        );
    }

    #[test]
    fn freight_calc_rounds_extra_units_up() {
        // 共 5 件 => 10 + ceil((5-1)/2)*5 = 10 + 2*5 = 20
        assert_eq!(
            LogisticsService::calc_freight(money("10"), money("5"), 1, 2, 5),
            Decimal::from(20)
        );
        // 共 2 件 => 10 + ceil(1/2)*5 = 15（不足一档也按一档计）
        assert_eq!(
            LogisticsService::calc_freight(money("10"), money("5"), 1, 2, 2),
            Decimal::from(15)
        );
    }

    #[test]
    fn freight_calc_exact_multiple_no_extra_tier() {
        // 共 3 件 => 10 + ceil(2/2)*5 = 15（正好整除，不多计一档）
        assert_eq!(
            LogisticsService::calc_freight(money("10"), money("5"), 1, 2, 3),
            Decimal::from(15)
        );
    }

    #[test]
    fn freight_calc_zero_quantity_returns_first_price() {
        assert_eq!(
            LogisticsService::calc_freight(money("8.50"), money("3"), 1, 1, 0),
            money("8.50")
        );
    }

    #[test]
    fn freight_calc_guards_zero_next_qty() {
        // next_qty = 0 时不应除零 panic（内部以 max(1) 兜底）
        let fee = LogisticsService::calc_freight(money("10"), money("5"), 1, 0, 4);
        assert!(fee > money("10"));
    }

    #[test]
    fn test_region_match_matching_cases() {
        use crate::trade::check::region_match;

        let region_data_json = r#"{
            "areaRegions": [
                [110000, 110100, 110101],
                [440000, 440100, 440106]
            ]
        }"#;

        // 匹配天河区 (440106)
        assert!(region_match(Some(region_data_json), &[440000, 440100, 440106]));
        // 匹配东城区 (110101)
        assert!(region_match(Some(region_data_json), &[110101]));
        // 未匹配的区域
        assert!(!region_match(Some(region_data_json), &[310000, 310100]));
        // 全国通配规则 (last == 1)
        let national_json = r#"{"areaRegions": [[1]]}"#;
        assert!(region_match(Some(national_json), &[310100]));
        // 空列表或非法数据
        assert!(!region_match(Some(region_data_json), &[]));
        assert!(!region_match(None, &[110101]));
    }
}