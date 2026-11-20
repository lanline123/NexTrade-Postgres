/**
 * 商品详情 moduleSortList 消费口径（2026-11-21 方案二接通，原版有·我方补齐）。
 *
 * 背景：moduleSortList（活动信息/规格参数/评价入口/售后入口/领劵优惠 五区块
 * 的显隐与排序）在原版即为死字段——默认值里有、编辑器无入口、前台无消费；
 * 我方此前照搬同病。本文件把消费口径收口一处，详情页本体与编辑器真实渲染
 * 预览（ProductConfigPreview）共用，保证两处行为一致。
 *
 * 口径：
 * - 页面自然序 = 五区块在详情页源码流中的相对位置（券条 → 活动 → 规格 →
 *   评价 → 售后）；未配置（无该字段）时按自然序、全显，与接通前渲染一致。
 * - 出厂默认序（activity/spec/review/afterSale/coupon，五键齐全且全显）是
 *   编辑器 defaults 与原版内置默认模板的原始形态，存量数据（含现网生效
 *   模板 10105）多为此形态且从未被消费过——视为「未配置」按自然序生效，
 *   保证接通消费后存量渲染零变化。商家经新设置入口改过的顺序/显隐一律
 *   按存量字面生效（唯一例外：刻意把顺序改回出厂序且全显时与未配置不可
 *   区分，仍按未配置处理，属兼容口径的已知边界）。
 * - 部分键缺失的存量：已列键按存量序，缺失键按自然序补齐到末尾、全显。
 * - visible 兼容 boolean / "false" / 0 / "0"（装修字段可能以字符串下发）。
 */

export const PD_SORT_KEYS = ["activity", "spec", "review", "afterSale", "coupon"] as const;
export type PdSortKey = (typeof PD_SORT_KEYS)[number];

/** 页面自然序（五区块在详情页中的相对位置） */
export const PD_PAGE_ORDER: PdSortKey[] = ["coupon", "activity", "spec", "review", "afterSale"];

/** 出厂默认序（编辑器 defaults / 原版内置默认模板的原始顺序） */
const PD_FACTORY_ORDER: PdSortKey[] = ["activity", "spec", "review", "afterSale", "coupon"];

export interface PdSortResolution {
    /** 全部已知键的生效顺序（缺失键已按页面自然序补齐到末尾） */
    order: PdSortKey[];
    /** 隐藏键集合 */
    hidden: Set<PdSortKey>;
}

const isKnownKey = (key: unknown): key is PdSortKey =>
    typeof key === "string" && (PD_SORT_KEYS as readonly string[]).includes(key);

const isVisibleValue = (value: unknown) => !(value === false || value === "false" || value === 0 || value === "0");

/** 解析装修配置中的 moduleSortList → 生效顺序 + 隐藏集合 */
export function resolvePdSortBlocks(raw: unknown): PdSortResolution {
    const entries = Array.isArray(raw) ? raw : [];
    const seen = new Set<PdSortKey>();
    const storedOrder: PdSortKey[] = [];
    const hidden = new Set<PdSortKey>();
    for (const entry of entries) {
        const key = (entry as { key?: unknown } | null)?.key;
        if (!isKnownKey(key) || seen.has(key)) continue;
        seen.add(key);
        storedOrder.push(key);
        if (!isVisibleValue((entry as { visible?: unknown }).visible)) hidden.add(key);
    }
    const isFactoryDefault =
        storedOrder.length === PD_SORT_KEYS.length &&
        hidden.size === 0 &&
        storedOrder.every((key, i) => key === PD_FACTORY_ORDER[i]);
    let order: PdSortKey[];
    if (storedOrder.length === 0 || isFactoryDefault) {
        order = [...PD_PAGE_ORDER];
    } else {
        order = [...storedOrder];
        for (const key of PD_PAGE_ORDER) {
            if (!order.includes(key)) order.push(key);
        }
    }
    return { order, hidden };
}

/**
 * 在场区块按生效顺序排序：在场键须以页面自然序传入，返回过滤隐藏后的
 * 生效渲染序。详情页据此把排序结果逐个填回各区块的原锚点位置——默认
 * （未配置）时排序结果与在场序 identical，渲染与接通前逐字一致。
 */
export function sortPdBlocks(resolution: PdSortResolution, presentInPageOrder: PdSortKey[]): PdSortKey[] {
    const rank = new Map<PdSortKey, number>(resolution.order.map((key, i) => [key, i] as const));
    return presentInPageOrder
        .filter((key) => !resolution.hidden.has(key))
        .sort((a, b) => (rank.get(a) ?? 99) - (rank.get(b) ?? 99));
}
