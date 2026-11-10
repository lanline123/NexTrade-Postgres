//! 小票渲染（对应 Java `print.renderer.impl.FeieyunReceiptRenderer` /
//! `SunmiReceiptRenderer`）。
//!
//! 飞鹅云输出富文本标签（`<C>` / `<CB>` / `<BOLD>` / `<BR>` / `<CUT>`），
//! 商米云输出 UTF-8 文本 + ESC/POS 控制字节的 hex 串。
//! 两个渲染器共用同一份 `print_config.template`，因此版块/选项判断逐条对齐 Java。
//!
//! 金额一律使用 `Decimal::to_string()`（等价 Java `BigDecimal#toPlainString`）。

use nextrade_core::entities::{Order, OrderItem, OrderReservations, Paylog};
use nextrade_core::models::logistics::print::mask_middle_half;
use nextrade_core::models::logistics::print_config::{
    PrintTemplateItem, PrintTemplateOption, PAPER_SIZE_58, PAPER_SIZE_80,
};
use nextrade_core::models::payment::pay_method_name;
use rust_decimal::Decimal;

/// 渲染上下文（订单详情、模板、纸张与关联数据）。
pub struct ReceiptContext<'a> {
    pub order: &'a Order,
    pub items: &'a [OrderItem],
    pub template: &'a [PrintTemplateItem],
    pub paper_size: i32,
    pub currency: &'a str,
    /// 系统设置 `shopName`（订单店铺查不到时的兜底）。
    pub default_shop_name: &'a str,
    pub shop_title: Option<&'a str>,
    pub shop_address: Option<&'a str>,
    pub user_name: Option<&'a str>,
    pub user_mobile: Option<&'a str>,
    /// 最新一笔已支付的商品订单支付日志（Java 按 `paylog_id` 倒序取 1 条）。
    pub paylog: Option<&'a Paylog>,
    pub reservation: Option<&'a OrderReservations>,
}

// ————————————————————— 模板选项判断 —————————————————————

fn is_choose(options: Option<&Vec<PrintTemplateOption>>, title: &str) -> bool {
    options.is_some_and(|opts| {
        opts.iter().any(|o| {
            o.choose == Some(true) && o.choose_title.as_deref() == Some(title)
        })
    })
}

fn is_choose_any(options: Option<&Vec<PrintTemplateOption>>, titles: &[&str]) -> bool {
    options.is_some_and(|opts| {
        opts.iter().any(|o| {
            o.choose == Some(true)
                && o.choose_title
                    .as_deref()
                    .is_some_and(|t| titles.contains(&t))
        })
    })
}

fn first_option_value(options: Option<&Vec<PrintTemplateOption>>) -> String {
    options
        .and_then(|opts| opts.first())
        .and_then(|o| o.value.clone())
        .unwrap_or_default()
}

/// Java `orderVO.getOrderSource()` 判定。
fn is_pos_order(order: &Order) -> bool {
    order.order_source == "pos"
}

/// Java `OrderType`：9 预约到店 / 10 预约上门。
fn is_reservation_order(order: &Order) -> bool {
    matches!(order.order_type, Some(9 | 10))
}

fn is_reservation_at_store(order: &Order) -> bool {
    order.order_type == Some(9)
}

/// Java `orderVO.getUserAddress()`：`String.join(" ", regionNames) + " " + address`。
fn user_address(order: &Order) -> String {
    let regions: Vec<String> = serde_json::from_str(&order.region_names).unwrap_or_default();
    format!("{} {}", regions.join(" "), order.address)
}

fn masked(value: Option<&str>) -> String {
    mask_middle_half(value).unwrap_or_default()
}

/// Java `OrderItemVO#getSkuValue()`：`[{name,value}]` → `name:value|name:value`。
fn sku_value(item: &OrderItem) -> String {
    let Ok(rows) = serde_json::from_str::<Vec<serde_json::Value>>(&item.sku_data) else {
        return String::new();
    };
    rows.iter()
        .map(|row| {
            format!(
                "{}:{}",
                row.get("name").and_then(|v| v.as_str()).unwrap_or_default(),
                row.get("value").and_then(|v| v.as_str()).unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// 金额明文（等价 `BigDecimal#toPlainString`）。
fn plain(amount: Decimal) -> String {
    amount.to_string()
}

/// 预约时间：`reservationDate + " " + (timeSlotDescription || startTime[-endTime])`。
fn reservation_time(reservation: Option<&OrderReservations>) -> String {
    let Some(r) = reservation else {
        return String::new();
    };
    let mut slot = r.time_slot_description.clone();
    if slot.trim().is_empty() {
        slot = r.start_time.clone();
        if !r.end_time.trim().is_empty() {
            slot.push('-');
            slot.push_str(&r.end_time);
        }
    }
    format!("{} {}", r.reservation_date, slot)
        .trim()
        .to_string()
}

fn non_blank(value: &str) -> Option<&str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

// ————————————————————— 显示宽度 / 排版 —————————————————————

fn is_wide(c: char) -> bool {
    matches!(c,
        '\u{4e00}'..='\u{9fa5}'
        | '\u{3000}'..='\u{303f}'
        | '\u{ff01}'..='\u{ff60}'
        | '\u{ffe0}'..='\u{ffe6}')
}

fn display_width(s: &str) -> usize {
    s.chars().map(|c| if is_wide(c) { 2 } else { 1 }).sum()
}

fn pad_right_display(s: &str, width: usize) -> String {
    let mut out = s.to_string();
    out.push_str(&" ".repeat(width.saturating_sub(display_width(s))));
    out
}

fn pad_left_display(s: &str, width: usize) -> String {
    let mut out = " ".repeat(width.saturating_sub(display_width(s)));
    out.push_str(s);
    out
}

fn center_display(s: &str, width: usize) -> String {
    let w = display_width(s);
    if w >= width {
        return s.to_string();
    }
    let padding = width - w;
    let left = padding / 2;
    let right = padding - left;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
}

/// Java `FeieyunApiUtils#centerString`：按 char 数居中（58mm 历史行为）。
fn center_chars(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.to_string();
    }
    let padding = width - len;
    let left = padding / 2;
    let right = padding - left;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
}

fn split_by_display_width(s: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut current = 0usize;
    for c in s.chars() {
        let w = if is_wide(c) { 2 } else { 1 };
        if current + w > max_width {
            lines.push(std::mem::take(&mut line));
            line.push(c);
            current = w;
        } else {
            line.push(c);
            current += w;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// 飞鹅排版（58mm 走历史字面宽度，80mm 按 48 列）。
#[derive(Clone, Copy)]
struct Layout {
    paper: i32,
}

impl Layout {
    fn new(paper_size: i32) -> Self {
        Self {
            paper: if paper_size == PAPER_SIZE_80 {
                PAPER_SIZE_80
            } else {
                PAPER_SIZE_58
            },
        }
    }

    fn is_58(self) -> bool {
        self.paper == PAPER_SIZE_58
    }

    fn chars_per_line(self) -> usize {
        if self.is_58() { 32 } else { 48 }
    }

    fn multipart_right_width(self) -> usize {
        if self.is_58() { 28 } else { 44 }
    }

    fn divider(self) -> String {
        "-".repeat(self.chars_per_line())
    }

    /// 三列商品行（价格 / 数量 / 小计）。
    fn format_line_3(self, price: &str, quantity: i32, subtotal: &str) -> String {
        if self.is_58() {
            let price_str = format!("{price:<10}");
            let quantity_str = center_chars(&quantity.to_string(), 10);
            let subtotal_str = format!("{subtotal:>10}");
            return format!("{price_str}{quantity_str}{subtotal_str}");
        }
        let col = (self.chars_per_line() / 3).max(6);
        format!(
            "{}{}{}",
            pad_right_display(price, col),
            center_display(&quantity.to_string(), col),
            pad_left_display(subtotal, col)
        )
    }

    /// 两列行（左标签 / 右值），保留 Java 对 `<BOLD>` 的透传。
    fn format_line_2(self, left: &str, right: &str) -> String {
        let contains_bold = right.contains("<BOLD>");
        let right_text = right.replace("<BOLD>", "");
        let (left_width, right_width) = if self.is_58() {
            (10, 20)
        } else {
            let left_width = (self.chars_per_line() / 3).max(6);
            (left_width, self.chars_per_line() - left_width)
        };
        let bold = if contains_bold { "<BOLD>" } else { "" };
        format!(
            "{}{}{}",
            pad_right_display(left, left_width),
            bold,
            pad_left_display(&right_text, right_width)
        )
    }

    /// 左标签 + 多行右值。
    fn format_multipart(self, left: &str, right: &str) -> String {
        let left_width = display_width(left);
        let available = self.multipart_right_width().saturating_sub(left_width).max(8);
        let lines = split_by_display_width(right, available);
        let mut result = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                result.push_str(left);
                result.push_str(line);
            } else {
                result.push_str("<BR>");
                result.push_str(&" ".repeat(left_width));
                result.push_str(line);
            }
        }
        result
    }

    fn product_header(self) -> String {
        if self.is_58() {
            return "单价          数量          小计".to_string();
        }
        let col = self.chars_per_line() / 3;
        format!(
            "{}{}{}",
            pad_right_display("单价", col),
            center_display("数量", col),
            pad_left_display("小计", col)
        )
    }

    /// 单行键值：58mm 保持历史拼接；80mm 按纸宽撑满。
    fn kv_line(self, label: &str, value: &str) -> String {
        if self.is_58() {
            return format!("{label}{value}");
        }
        let gap = self
            .chars_per_line()
            .saturating_sub(display_width(label) + display_width(value));
        if gap == 0 {
            format!("{label}{value}")
        } else {
            format!("{label}{}{value}", " ".repeat(gap))
        }
    }
}

// ————————————————————— 飞鹅云渲染器 —————————————————————

/// 飞鹅云小票（富文本标签）。
#[must_use]
pub fn render_feieyun(ctx: &ReceiptContext<'_>) -> String {
    let layout = Layout::new(ctx.paper_size);
    let mut out = String::new();
    for item in ctx.template {
        let options = item.options.as_ref();
        match item.title.as_deref() {
            Some("小票头部") => receipt_header(&mut out, ctx, options),
            Some("配送信息") => delivery_info(&mut out, ctx, options, layout),
            Some("买家备注") => buyer_note(&mut out, ctx, options, layout),
            Some("商品信息") => product_info(&mut out, ctx, options, layout),
            Some("运费信息") => shipping_info(&mut out, ctx, options, layout),
            Some("优惠信息") => discount_info(&mut out, ctx, options, layout),
            Some("支付信息") => payment_info(&mut out, ctx, options, layout),
            Some("客户信息") => customer_info(&mut out, ctx, options, layout),
            Some("其他订单信息") => other_order_info(&mut out, ctx, options, layout),
            Some("二维码") => code_info(&mut out, ctx, options),
            Some("底部公告") => footer(&mut out, options),
            _ => {}
        }
    }
    out
}

fn receipt_header(out: &mut String, ctx: &ReceiptContext<'_>, options: Option<&Vec<PrintTemplateOption>>) {
    let shop = non_blank(ctx.shop_title.unwrap_or_default()).unwrap_or(ctx.default_shop_name);
    if is_choose(options, "商家名称") {
        out.push_str(&format!("<C>{shop}</C><BR>"));
    }
    if is_choose(options, "门店名称") {
        out.push_str(&format!("<CB>{shop}</CB><BR>"));
    }
}

fn delivery_info(
    out: &mut String,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    if is_pos_order(ctx.order) {
        return;
    }
    if is_reservation_order(ctx.order) {
        reservation_info(out, ctx, options, layout);
        return;
    }
    let pickup = ctx.order.shipping_method == 2;
    if pickup {
        if is_choose(options, "配送方式") {
            out.push_str(&format!("{}<BR>", layout.kv_line("配送方式：", "到店自提")));
            // Order 表没有 expectPickupTime 字段，Java 该值为空串时同样输出空值。
            out.push_str(&format!("{}<BR>", layout.kv_line("自提时间：", "")));
        }
    } else if is_choose(options, "配送方式") {
        out.push_str(&format!(
            "{}<BR>",
            layout.kv_line("配送方式：", &ctx.order.shipping_type_name)
        ));
    }

    if is_choose(options, "商配&同城-收货信息") {
        let mobile = if is_choose(options, "手机号模糊") {
            masked(Some(ctx.order.mobile.as_str()))
        } else {
            ctx.order.mobile.clone()
        };
        let address = user_address(ctx.order);
        if pickup {
            out.push_str(&format!(
                "{}<BR>",
                layout.format_multipart(
                    "自提信息：",
                    &format!("{}，{}，{}", ctx.order.consignee, mobile, address)
                )
            ));
        } else if ctx.order.shipping_type_id == 3 {
            out.push_str(&format!(
                "{}<BR>",
                layout.format_multipart(
                    "配送信息：",
                    &format!("{}，{}，{}", ctx.order.consignee, mobile, address)
                )
            ));
        } else {
            out.push_str(&format!(
                "{}<BR>",
                layout.format_multipart(
                    "收货信息：",
                    &format!("{}，{}，{}", ctx.order.consignee, mobile, address)
                )
            ));
        }
    }
    if is_choose_any(options, &["配送方式", "同城配送-收货信息"]) {
        out.push_str(&format!("{}<BR>", layout.divider()));
    }
}

fn reservation_info(
    out: &mut String,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    let at_store = is_reservation_at_store(ctx.order);
    if is_choose(options, "配送方式") {
        let way = if at_store { "预约到店" } else { "预约上门" };
        out.push_str(&format!("{}<BR>", layout.kv_line("配送方式：", way)));
        let time = reservation_time(ctx.reservation);
        let time = if time.trim().is_empty() { "-" } else { time.as_str() };
        out.push_str(&format!("{}<BR>", layout.kv_line("预约时间：", time)));
    }
    if is_choose(options, "商配&同城-收货信息") {
        let mut mobile = ctx.order.mobile.clone();
        if is_choose(options, "手机号模糊") {
            mobile = masked(Some(mobile.as_str()));
        }
        let address = if at_store {
            let title = ctx.shop_title.unwrap_or_default();
            let detailed = ctx.shop_address.unwrap_or_default();
            format!("{title} {detailed}").trim().to_string()
        } else {
            user_address(ctx.order)
        };
        let consignee = non_blank(&ctx.order.consignee).unwrap_or("-");
        let mobile = non_blank(&mobile).unwrap_or("-");
        let address = non_blank(&address).unwrap_or("-");
        out.push_str(&format!(
            "{}<BR>",
            layout.format_multipart(
                "预约信息：",
                &format!("{consignee}，{mobile}，{address}")
            )
        ));
    }
    if is_choose_any(options, &["配送方式", "商配&同城-收货信息"]) {
        out.push_str(&format!("{}<BR>", layout.divider()));
    }
}

fn buyer_note(
    out: &mut String,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    if is_choose(options, "买家备注") {
        let note = non_blank(&ctx.order.buyer_note).unwrap_or("无");
        out.push_str(&format!(
            "<BOLD>{}</BOLD><BR>",
            layout.format_multipart("买家备注：", note)
        ));
        out.push_str(&format!("{}<BR>", layout.divider()));
    }
}

fn product_info(
    out: &mut String,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    let currency = ctx.currency;
    if is_choose_any(options, &["商品基础信息", "规格编码"]) {
        out.push_str("商品<BR>");
        out.push_str(&format!("{}<BR>", layout.product_header()));
        out.push_str(&format!("{}<BR>", layout.divider()));
    }
    for item in ctx.items {
        if is_choose(options, "商品基础信息") {
            let sku = sku_value(item);
            if sku.is_empty() {
                out.push_str(&format!("{}<BR>", item.product_name));
            } else {
                out.push_str(&format!("{}（{sku}）<BR>", item.product_name));
            }
        }
        if is_choose(options, "规格编码") {
            out.push_str(&format!("商品编码：{}<BR>", item.product_sn));
        }
        if is_choose(options, "商品基础信息") {
            let subtotal = item.price * Decimal::from(item.quantity);
            out.push_str(&format!(
                "{}<BR>",
                layout.format_line_3(
                    &format!("{currency}{}", plain(item.price)),
                    item.quantity,
                    &format!("{currency}{}", plain(subtotal))
                )
            ));
        }
    }
    let sum: i32 = ctx.items.iter().map(|i| i.quantity).sum();
    if is_choose_any(options, &["商品基础信息", "规格编码"]) {
        out.push_str(&format!("{}<BR>", layout.divider()));
        let total_line = layout.format_line_2(
            &format!("共{sum}件"),
            &format!("<BOLD>合计：{currency}{}", plain(ctx.order.product_amount)),
        );
        out.push_str(&format!("{total_line}</BOLD><BR>"));
        out.push_str(&format!("{}<BR>", layout.divider()));
    }
}

fn shipping_info(
    out: &mut String,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    if !is_pos_order(ctx.order) && is_choose(options, "运费") {
        out.push_str(&format!(
            "{}<BR>",
            layout.format_line_2("运费", &format!("{}{}", ctx.currency, plain(ctx.order.shipping_fee)))
        ));
        out.push_str(&format!("{}<BR>", layout.divider()));
    }
}

fn discount_info(
    out: &mut String,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    let currency = ctx.currency;
    let order = ctx.order;
    if is_choose(options, "优惠明细") {
        out.push_str(&format!(
            "{}<BR>",
            layout.format_line_2(
                "优惠",
                &format!("优惠券：-{currency}{}", plain(order.coupon_amount))
            )
        ));
        out.push_str(&format!(
            "{}<BR>",
            layout.format_line_2(
                "",
                &format!("积分抵扣：-{currency}{}", plain(order.points_amount))
            )
        ));
        out.push_str(&format!(
            "{}<BR>",
            layout.format_line_2(
                "",
                &format!("活动优惠：-{currency}{}", plain(order.discount_amount))
            )
        ));
    }
    let total = order.coupon_amount + order.points_amount + order.discount_amount;
    let pos_order = is_pos_order(order);
    if is_choose(options, "优惠总计") && !is_choose(options, "优惠明细") {
        let label = if pos_order { "优惠总计：-" } else { "总计：-" };
        let line = layout.format_line_2(
            "优惠",
            &format!("<BOLD>{label}{currency}{}", plain(total)),
        );
        out.push_str(&format!("{line}</BOLD><BR>"));
    }
    if is_choose(options, "优惠总计") && is_choose(options, "优惠明细") {
        let label = if pos_order { "优惠总计：-" } else { "总计：-" };
        let line = layout.format_line_2(
            "",
            &format!("<BOLD>{label}{currency}{}", plain(total)),
        );
        out.push_str(&format!("{line}</BOLD><BR>"));
    }
    if is_choose_any(options, &["优惠明细", "优惠总计"]) {
        out.push_str(&format!("{}<BR>", layout.divider()));
    }
}

fn payment_info(
    out: &mut String,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    let currency = ctx.currency;
    let order = ctx.order;
    let paylog = ctx.paylog;
    if is_choose(options, "支付方式") {
        if let Some(p) = paylog {
            out.push_str(&format!(
                "{}<BR>",
                layout.kv_line("支付方式：", &pay_method_name(&p.pay_code))
            ));
        } else {
            // Java PaymentType：3 线下支付 / 2 货到付款。
            if order.pay_type_id == 3 {
                out.push_str(&format!("{}<BR>", layout.kv_line("支付方式：", "线下支付")));
            }
            if order.pay_type_id == 2 {
                out.push_str(&format!("{}<BR>", layout.kv_line("支付方式：", "货到付款")));
            }
            if order.balance > Decimal::ZERO {
                out.push_str(&format!("{}<BR>", layout.kv_line("支付方式：", "余额支付")));
            }
        }
    }
    if is_choose(options, "实收金额") {
        out.push_str(&format!(
            "{}<BR>",
            layout.kv_line("实收金额：", &format!("{currency}{}", plain(order.total_amount)))
        ));
    }
    if is_choose(options, "第三方支付单号") {
        let transaction = paylog.map_or("无", |p| p.transaction_id.as_str());
        out.push_str(&format!(
            "{}<BR>",
            layout.kv_line("第三方支付单号：", transaction)
        ));
    }
    if is_choose_any(options, &["支付方式", "实收金额", "第三方支付单号"]) {
        out.push_str(&format!("{}<BR>", layout.divider()));
    }
}

fn customer_info(
    out: &mut String,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    if is_choose(options, "客户信息") {
        out.push_str(&format!(
            "{}<BR>",
            layout.kv_line("客户姓名：", &masked(ctx.user_name))
        ));
        out.push_str(&format!(
            "{}<BR>",
            layout.kv_line("手机号：", &masked(ctx.user_mobile))
        ));
        out.push_str(&format!("{}<BR>", layout.divider()));
    }
}

fn other_order_info(
    out: &mut String,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    let order = ctx.order;
    if is_choose(options, "下单时间") {
        let add_time = nextrade_core::models::logistics::print_config::format_time(Some(order.add_time));
        out.push_str(&format!("{}<BR>", layout.kv_line("下单时间：", &add_time)));
    }
    if is_choose(options, "支付时间") {
        let pay_time = nextrade_core::models::logistics::print_config::format_time(Some(order.pay_time));
        let pay_time = if pay_time.is_empty() { "-" } else { pay_time.as_str() };
        out.push_str(&format!("{}<BR>", layout.kv_line("支付时间：", pay_time)));
    }
    if is_choose(options, "渠道类型") {
        out.push_str(&format!(
            "{}<BR>",
            layout.kv_line("渠道类型：", &order.order_source)
        ));
    }
    if is_choose_any(options, &["下单时间", "支付时间", "渠道类型"]) {
        out.push_str(&format!("{}<BR>", layout.divider()));
    }
}

fn code_info(
    out: &mut String,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
) {
    if is_choose(options, "订单二维码") {
        let sn = &ctx.order.order_sn;
        out.push_str(&format!("订单编号：{sn}<BR><C><BC128_C>{sn}</BC128_C></C><BR>"));
    } else {
        out.push_str("<BR><BR>");
    }
}

fn footer(out: &mut String, options: Option<&Vec<PrintTemplateOption>>) {
    if is_choose(options, "底部公告") {
        out.push_str(&format!("<C>{}</C><BR>", first_option_value(options)));
    }
    out.push_str("<BR><BR><CUT>");
}

// ————————————————————— 商米云渲染器（ESC/POS hex） —————————————————————

const ESC_INIT: [u8; 2] = [0x1B, 0x40];
const ESC_ALIGN_LEFT: [u8; 3] = [0x1B, 0x61, 0x00];
const ESC_ALIGN_CENTER: [u8; 3] = [0x1B, 0x61, 0x01];
const ESC_BOLD_ON: [u8; 3] = [0x1B, 0x45, 0x01];
const ESC_BOLD_OFF: [u8; 3] = [0x1B, 0x45, 0x00];
const LF: u8 = 0x0A;
const GS_CUT_FULL: [u8; 3] = [0x1D, 0x56, 0x00];

struct SunmiBuilder {
    buf: Vec<u8>,
}

impl SunmiBuilder {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn write(&mut self, bytes: &[u8]) -> &mut Self {
        self.buf.extend_from_slice(bytes);
        self
    }

    fn text(&mut self, s: &str) -> &mut Self {
        if !s.is_empty() {
            self.buf.extend_from_slice(s.as_bytes());
        }
        self
    }

    fn text_line(&mut self, s: &str) -> &mut Self {
        self.text(s).lf()
    }

    fn bold(&mut self, s: &str) -> &mut Self {
        self.write(&ESC_BOLD_ON).text(s).write(&ESC_BOLD_OFF)
    }

    fn spaces(&mut self, n: usize) -> &mut Self {
        self.write(" ".repeat(n).as_bytes())
    }

    fn lf(&mut self) -> &mut Self {
        self.buf.push(LF);
        self
    }

    fn align_left(&mut self) -> &mut Self {
        self.write(&ESC_ALIGN_LEFT)
    }

    fn align_center(&mut self) -> &mut Self {
        self.write(&ESC_ALIGN_CENTER)
    }

    fn barcode128(&mut self, data: &str) -> &mut Self {
        if data.is_empty() {
            return self;
        }
        let payload = data.as_bytes();
        self.write(&[0x1D, 0x48, 0x02]);
        self.write(&[0x1D, 0x68, 80]);
        self.write(&[0x1D, 0x77, 0x02]);
        self.write(&[0x1D, 0x6B, 0x49, payload.len() as u8]);
        self.write(payload);
        self.lf()
    }

    fn to_hex(&self) -> String {
        let mut out = String::with_capacity(self.buf.len() * 2);
        for b in &self.buf {
            out.push_str(&format!("{b:02x}"));
        }
        out
    }
}

impl Layout {
    /// 商米三列行：58mm 也使用中文感知宽度（与飞鹅 58mm 历史行为不同）。
    fn sunmi_price_line(self, price: &str, quantity: i32, subtotal: &str) -> String {
        let col = (self.chars_per_line() / 3).max(6);
        format!(
            "{}{}{}",
            pad_right_display(price, col),
            center_display(&quantity.to_string(), col),
            pad_left_display(subtotal, col)
        )
    }
}

/// 商米云小票（hex）。
#[must_use]
pub fn render_sunmi(ctx: &ReceiptContext<'_>) -> String {
    let layout = Layout::new(ctx.paper_size);
    let mut sb = SunmiBuilder::new();
    sb.write(&ESC_INIT).align_left();

    for item in ctx.template {
        let options = item.options.as_ref();
        match item.title.as_deref() {
            Some("小票头部") => sunmi_receipt_header(&mut sb, ctx, options),
            Some("配送信息") => sunmi_delivery_info(&mut sb, ctx, options, layout),
            Some("买家备注") => sunmi_buyer_note(&mut sb, ctx, options, layout),
            // Java 源码字面量为 "商品信"（缺 "息"），为保证逐行迁移一致此处保留。
            Some("商品信") => sunmi_product_info(&mut sb, ctx, options, layout),
            Some("运费信息") => sunmi_shipping_info(&mut sb, ctx, options, layout),
            Some("优惠信息") => sunmi_discount_info(&mut sb, ctx, options, layout),
            Some("支付信息") => sunmi_payment_info(&mut sb, ctx, options, layout),
            Some("客户信息") => sunmi_customer_info(&mut sb, ctx, options, layout),
            Some("其他订单信息") => sunmi_other_order_info(&mut sb, ctx, options, layout),
            Some("二维码") => sunmi_code_info(&mut sb, ctx, options),
            Some("底部公告") => sunmi_footer(&mut sb, options),
            _ => {}
        }
    }

    sb.lf().lf().write(&GS_CUT_FULL);
    sb.to_hex()
}

fn sunmi_receipt_header(
    sb: &mut SunmiBuilder,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
) {
    let shop = non_blank(ctx.shop_title.unwrap_or_default()).unwrap_or(ctx.default_shop_name);
    if is_choose(options, "商家名称") {
        sb.align_center().text_line(shop).align_left();
    }
    if is_choose(options, "门店名称") {
        sb.align_center().bold(shop).lf().align_left();
    }
}

fn sunmi_delivery_info(
    sb: &mut SunmiBuilder,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    if is_pos_order(ctx.order) {
        return;
    }
    if is_reservation_order(ctx.order) {
        sunmi_reservation_info(sb, ctx, options, layout);
        return;
    }
    let pickup = ctx.order.shipping_method == 2;
    if pickup {
        if is_choose(options, "配送方式") {
            sb.text_line(&layout.kv_line("配送方式：", "到店自提"));
            sb.text_line(&layout.kv_line("自提时间：", ""));
        }
    } else if is_choose(options, "配送方式") {
        sb.text_line(&layout.kv_line("配送方式：", &ctx.order.shipping_type_name));
    }

    if is_choose(options, "商配&同城-收货信息") {
        let mut mobile = ctx.order.mobile.clone();
        if is_choose(options, "手机号模糊") {
            mobile = masked(Some(mobile.as_str()));
        }
        let (label, address) = if pickup {
            ("自提信息：", user_address(ctx.order))
        } else if ctx.order.shipping_type_id == 3 {
            ("配送信息：", user_address(ctx.order))
        } else {
            ("收货信息：", user_address(ctx.order))
        };
        sb.text_line(&format!(
            "{label}{}，{}，{}",
            ctx.order.consignee, mobile, address
        ));
    }
    if is_choose_any(options, &["配送方式", "同城配送-收货信息"]) {
        sb.text_line(&layout.divider());
    }
}

fn sunmi_reservation_info(
    sb: &mut SunmiBuilder,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    let at_store = is_reservation_at_store(ctx.order);
    if is_choose(options, "配送方式") {
        let way = if at_store { "预约到店" } else { "预约上门" };
        sb.text_line(&layout.kv_line("配送方式：", way));
        let time = reservation_time(ctx.reservation);
        let time = if time.trim().is_empty() { "-" } else { time.as_str() };
        sb.text_line(&layout.kv_line("预约时间：", time));
    }
    if is_choose(options, "商配&同城-收货信息") {
        let mut mobile = ctx.order.mobile.clone();
        if is_choose(options, "手机号模糊") {
            mobile = masked(Some(mobile.as_str()));
        }
        let address = if at_store {
            let title = ctx.shop_title.unwrap_or_default();
            let detailed = ctx.shop_address.unwrap_or_default();
            format!("{title} {detailed}").trim().to_string()
        } else {
            user_address(ctx.order)
        };
        let consignee = non_blank(&ctx.order.consignee).unwrap_or("-");
        let mobile = non_blank(&mobile).unwrap_or("-");
        let address = non_blank(&address).unwrap_or("-");
        sb.text_line(&format!("预约信息：{consignee}，{mobile}，{address}"));
    }
    if is_choose_any(options, &["配送方式", "商配&同城-收货信息"]) {
        sb.text_line(&layout.divider());
    }
}

fn sunmi_buyer_note(
    sb: &mut SunmiBuilder,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    if is_choose(options, "买家备注") {
        let note = non_blank(&ctx.order.buyer_note).unwrap_or("无");
        sb.bold(&format!("买家备注：{note}")).lf();
        sb.text_line(&layout.divider());
    }
}

fn sunmi_product_info(
    sb: &mut SunmiBuilder,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    let currency = ctx.currency;
    if is_choose_any(options, &["商品基础信息", "规格编码"]) {
        sb.text_line("商品");
        sb.text_line(&layout.product_header());
        sb.text_line(&layout.divider());
    }
    for item in ctx.items {
        if is_choose(options, "商品基础信息") {
            let sku = sku_value(item);
            if sku.is_empty() {
                sb.text_line(&item.product_name);
            } else {
                sb.text_line(&format!("{}（{sku}）", item.product_name));
            }
        }
        if is_choose(options, "规格编码") {
            sb.text_line(&format!("商品编码：{}", item.product_sn));
        }
        if is_choose(options, "商品基础信息") {
            let subtotal = item.price * Decimal::from(item.quantity);
            sb.text_line(&layout.sunmi_price_line(
                &format!("{currency}{}", plain(item.price)),
                item.quantity,
                &format!("{currency}{}", plain(subtotal)),
            ));
        }
    }
    let sum: i32 = ctx.items.iter().map(|i| i.quantity).sum();
    if is_choose_any(options, &["商品基础信息", "规格编码"]) {
        sb.text_line(&layout.divider());
        let left = format!("共{sum}件");
        let right = format!("合计：{currency}{}", plain(ctx.order.product_amount));
        let gap = if layout.is_58() {
            6
        } else {
            (48usize.saturating_sub(display_width(&left) + display_width(&right))).max(1)
        };
        sb.bold(&left).spaces(gap).bold(&right).lf();
        sb.text_line(&layout.divider());
    }
}

fn sunmi_shipping_info(
    sb: &mut SunmiBuilder,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    if is_pos_order(ctx.order) || !is_choose(options, "运费") {
        return;
    }
    let value = format!("{}{}", ctx.currency, plain(ctx.order.shipping_fee));
    let line = if layout.is_58() {
        format!("运费          {value}")
    } else {
        layout.format_line_2("运费", &value)
    };
    sb.text_line(&line);
    sb.text_line(&layout.divider());
}

fn sunmi_discount_info(
    sb: &mut SunmiBuilder,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    let currency = ctx.currency;
    let order = ctx.order;
    let pos_order = is_pos_order(order);
    let total = order.coupon_amount + order.points_amount + order.discount_amount;

    if layout.is_58() {
        if is_choose(options, "优惠明细") {
            sb.text_line(&format!("优惠  优惠券：-{currency}{}", plain(order.coupon_amount)));
            sb.text_line(&format!("      积分抵扣：-{currency}{}", plain(order.points_amount)));
            sb.text_line(&format!("      活动优惠：-{currency}{}", plain(order.discount_amount)));
        }
        let label = if pos_order { "优惠总计：-" } else { "总计：-" };
        if is_choose(options, "优惠总计") && !is_choose(options, "优惠明细") {
            sb.text("优惠  ").bold(&format!("{label}{currency}{}", plain(total))).lf();
        }
        if is_choose(options, "优惠总计") && is_choose(options, "优惠明细") {
            sb.text("      ").bold(&format!("{label}{currency}{}", plain(total))).lf();
        }
        if is_choose_any(options, &["优惠明细", "优惠总计"]) {
            sb.text_line(&layout.divider());
        }
        return;
    }

    if is_choose(options, "优惠明细") {
        sb.text_line(&layout.format_line_2("优惠", &format!("优惠券：-{currency}{}", plain(order.coupon_amount))));
        sb.text_line(&layout.format_line_2("", &format!("积分抵扣：-{currency}{}", plain(order.points_amount))));
        sb.text_line(&layout.format_line_2("", &format!("活动优惠：-{currency}{}", plain(order.discount_amount))));
    }
    let label = if pos_order { "优惠总计：-" } else { "总计：-" };
    if is_choose(options, "优惠总计") && !is_choose(options, "优惠明细") {
        let right = format!("{label}{currency}{}", plain(total));
        let gap = 48usize.saturating_sub(display_width("优惠") + display_width(&right));
        sb.bold("优惠").spaces(gap).bold(&right).lf();
    }
    if is_choose(options, "优惠总计") && is_choose(options, "优惠明细") {
        let right = format!("{label}{currency}{}", plain(total));
        let gap = 48usize.saturating_sub(display_width(&right));
        sb.spaces(gap).bold(&right).lf();
    }
    if is_choose_any(options, &["优惠明细", "优惠总计"]) {
        sb.text_line(&layout.divider());
    }
}

fn sunmi_payment_info(
    sb: &mut SunmiBuilder,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    let currency = ctx.currency;
    let order = ctx.order;
    if is_choose(options, "支付方式") {
        if let Some(p) = ctx.paylog {
            sb.text_line(&layout.kv_line("支付方式：", &pay_method_name(&p.pay_code)));
        } else {
            if order.pay_type_id == 3 {
                sb.text_line(&layout.kv_line("支付方式：", "线下支付"));
            }
            if order.pay_type_id == 2 {
                sb.text_line(&layout.kv_line("支付方式：", "货到付款"));
            }
            if order.balance > Decimal::ZERO {
                sb.text_line(&layout.kv_line("支付方式：", "余额支付"));
            }
        }
    }
    if is_choose(options, "实收金额") {
        sb.text_line(&layout.kv_line("实收金额：", &format!("{currency}{}", plain(order.total_amount))));
    }
    if is_choose(options, "第三方支付单号") {
        let transaction = ctx.paylog.map_or("无", |p| p.transaction_id.as_str());
        sb.text_line(&layout.kv_line("第三方支付单号：", transaction));
    }
    if is_choose_any(options, &["支付方式", "实收金额", "第三方支付单号"]) {
        sb.text_line(&layout.divider());
    }
}

fn sunmi_customer_info(
    sb: &mut SunmiBuilder,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    if is_choose(options, "客户信息") {
        sb.text_line(&layout.kv_line("客户姓名：", &masked(ctx.user_name)));
        sb.text_line(&layout.kv_line("手机号：", &masked(ctx.user_mobile)));
        sb.text_line(&layout.divider());
    }
}

fn sunmi_other_order_info(
    sb: &mut SunmiBuilder,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
    layout: Layout,
) {
    let order = ctx.order;
    if is_choose(options, "下单时间") {
        let add_time = nextrade_core::models::logistics::print_config::format_time(Some(order.add_time));
        sb.text_line(&layout.kv_line("下单时间：", &add_time));
    }
    if is_choose(options, "支付时间") {
        let pay_time = nextrade_core::models::logistics::print_config::format_time(Some(order.pay_time));
        let pay_time = if pay_time.is_empty() { "-" } else { pay_time.as_str() };
        sb.text_line(&layout.kv_line("支付时间：", pay_time));
    }
    if is_choose(options, "渠道类型") {
        sb.text_line(&layout.kv_line("渠道类型：", &order.order_source));
    }
    if is_choose_any(options, &["下单时间", "支付时间", "渠道类型"]) {
        sb.text_line(&layout.divider());
    }
}

fn sunmi_code_info(
    sb: &mut SunmiBuilder,
    ctx: &ReceiptContext<'_>,
    options: Option<&Vec<PrintTemplateOption>>,
) {
    if is_choose(options, "订单二维码") {
        sb.text_line(&format!("订单编号：{}", ctx.order.order_sn));
        sb.align_center();
        sb.barcode128(&ctx.order.order_sn);
        sb.align_left();
        sb.lf();
    } else {
        sb.lf().lf();
    }
}

fn sunmi_footer(sb: &mut SunmiBuilder, options: Option<&Vec<PrintTemplateOption>>) {
    if is_choose(options, "底部公告") {
        let value = first_option_value(options);
        sb.align_center().text_line(&value).align_left();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_width_counts_full_width_chars() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("中文"), 4);
        assert_eq!(display_width("￥12"), 4);
    }

    #[test]
    fn divider_matches_paper() {
        assert_eq!(Layout::new(58).divider().len(), 32);
        assert_eq!(Layout::new(80).divider().len(), 48);
        assert_eq!(Layout::new(0).divider().len(), 32);
    }

    #[test]
    fn fifty_eight_line_matches_java_literals() {
        let layout = Layout::new(58);
        // 价格左 10 / 数量居中 10 / 小计右 10
        let line = layout.format_line_3("￥1.00", 2, "￥2.00");
        assert_eq!(line.chars().count(), 30);
        assert!(line.starts_with("￥1.00"));
        assert!(line.ends_with("￥2.00"));
        // 两列：左 10 宽 + 右 20 宽
        let line = layout.format_line_2("优惠", "优惠券：-￥1.00");
        assert_eq!(display_width(&line), 30);
        assert!(line.starts_with("优惠"));
        assert!(line.ends_with("优惠券：-￥1.00"));
        assert_eq!(layout.product_header(), "单价          数量          小计");
    }

    #[test]
    fn eighty_line_uses_forty_eight_columns() {
        let layout = Layout::new(80);
        let line = layout.format_line_2("运费", "￥10.00");
        assert_eq!(display_width(&line), 48);
    }

    #[test]
    fn choose_helpers_follow_java() {
        let options = vec![PrintTemplateOption {
            choose_title: Some("统计".into()),
            choose: Some(true),
            value: Some("".into()),
        }];
        assert!(is_choose(Some(&options), "统计"));
        assert!(!is_choose(Some(&options), "统计2"));
        assert!(is_choose_any(Some(&options), &["统计", "统计2"]));
        assert!(!is_choose(None, "统计"));
    }
}
