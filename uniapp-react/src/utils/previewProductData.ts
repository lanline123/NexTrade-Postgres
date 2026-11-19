/**
 * 编辑器嵌入预览的真实价格注册表【我方新增】（2026-11-20）。
 *
 * 背景：装修编辑器的真实渲染画布是 iframe 嵌入本应用的 decoratePreview 页，
 * 各商品模块在嵌入页内以匿名身份调公开商品列表接口。isIdentity 价格门控
 * 开启后，公开接口把价格键遮罩为 null 并打 priceGate 标记，模块按
 * FormatPrice 的兜底把 null 渲染成 0.00——编辑器里所有商品价格显示为 0
 * （页面 10019 实证，2026-11-20 00:42 用户报）。
 *
 * 口径：编辑器是管理面，预览必须显示真实价格。父页面（管理端，管理员登录
 * 口径）按页面内各商品来源预取真实价格，随 decorate-data 下发 productData
 * （按商品 id 的价格字段表，与后端 price_gate 的 PRICE_KEYS 同名单），
 * 本模块保存该表；api/common 的 getProductsList 在响应返回后按 id 回填
 * 被遮罩的价格字段，再交给各模块渲染。价格只从管理端已鉴权通道下发，
 * 不经任何公开接口，店面门控（匿名/未认证遮价、已认证明文）不受影响。
 *
 * 武装条件：只有 decoratePreview 页在收到来源受检（origin 白名单）的
 * decorate-data 且其中带 productData 字段时才调用 setPreviewPriceData。
 * 真实店面的任何页面都不经过该页、注册表恒为空，getProductsList 原样
 * 返回，行为与此前完全一致。
 */
/** 与后端 price_gate PRICE_KEYS 同名单（37 键，含批发价键与 snake_case 变体） */
export const PREVIEW_PRICE_KEYS: string[] = [
    "productPrice", "marketPrice", "costPrice", "minPricedPrice", "price",
    "orgProductPrice", "storeProductPrice", "skuPrice", "promotePrice",
    "seckillPrice", "grouponPrice", "minGrouponPrice", "groupPrice",
    "originPrice", "memberPrice", "wholesalePrice", "firstWholesalePrice",
    "maxPrice", "minPrice",
    "product_price", "market_price", "cost_price", "min_priced_price",
    "org_product_price", "store_product_price", "sku_price", "promote_price",
    "seckill_price", "groupon_price", "min_groupon_price", "group_price",
    "origin_price", "member_price", "wholesale_price", "first_wholesale_price",
    "max_price", "min_price"
];

let priceMap: Map<string, Record<string, any>> | null = null;
let priceSig = "";
/**
 * 装入/清除父页面下发的真实价格表。内容有变化时返回 true（调用方据此
 * 整树重挂一次，让已挂载的商品模块重新取数并回填价格）；无变化返回 false。
 * data 为 null/undefined 时清除注册表（编辑器切到无商品来源的页面时，
 * 旧价格表不得残留武装）。
 */
export function setPreviewPriceData(
    data: { recordsById?: Record<string, Record<string, any>> } | null | undefined
): boolean {
    const recordsById = data && typeof data === "object" ? data.recordsById : undefined;
    if (!recordsById || typeof recordsById !== "object") {
        if (priceMap === null) return false;
        priceMap = null;
        priceSig = "";
        return true;
    }
    const sig = JSON.stringify(recordsById);
    if (sig === priceSig) return false;
    priceSig = sig;
    priceMap = new Map(Object.entries(recordsById));
    return true;
}
/**
 * 按商品 id 回填一条商品记录中被门控遮罩（null/缺失）的价格字段。
 * 只补空缺、绝不覆盖已有值：已认证视角下公开接口本就返回明文价格，
 * 注册表未武装或表中无此商品时原样返回。
 */
export function repairPreviewPrices<T extends Record<string, any>>(record: T): T {
    if (!priceMap || !record || typeof record !== "object") return record;
    const rec = record as Record<string, any>;
    const id = rec.productId ?? rec.id;
    if (id === null || id === undefined) return record;
    const real = priceMap.get(String(id));
    if (!real) return record;
    let touched = false;
    for (const key of PREVIEW_PRICE_KEYS) {
        if (
            (rec[key] === null || rec[key] === undefined) &&
            real[key] !== null &&
            real[key] !== undefined
        ) {
            rec[key] = real[key];
            touched = true;
        }
    }
    if (touched) {
        if (rec.priceVisible === false) rec.priceVisible = true;
        if ("priceGate" in rec) delete rec.priceGate;
    }
    return record;
}
/** 批量回填（原地修改并返回同一数组） */
export function repairPreviewPriceList<T extends Record<string, any>>(records: T[]): T[] {
    if (!priceMap || !Array.isArray(records)) return records;
    records.forEach((r) => repairPreviewPrices(r));
    return records;
}
