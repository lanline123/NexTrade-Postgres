/**
 * 商品详情 moduleSortList 编辑侧归一（2026-11-21 方案二接通，原版有·我方补齐）
 *
 * 与前台消费口径（uniapp-react pages/product/sortBlocks.ts）严格镜像：
 * 设置面板展示的顺序/显隐必须与前台生效结果一致，故此处把任意存量数据
 * 归一为「生效列表」再喂给排序编辑器——
 * - 未配置（无字段/空）→ 页面自然序（券→活动→规格→评价→售后）、全显；
 * - 出厂默认序（activity/spec/review/afterSale/coupon 五键齐全且全显，
 *   存量模板的原始形态，前台视为未配置）→ 同样归一为页面自然序；
 * - 其余存量按字面顺序生效，缺失键按自然序补齐到末尾（默认文案、全显）。
 * 商家在面板里一经保存，写回的即归一后的完整列表，前台按字面消费，
 * 所见即所得；存量模板未经编辑保存时，前台渲染不变（红线口径）。
 */

export interface PdSortItem {
    key: string;
    label: string;
    visible: boolean;
}

const PD_LABELS: Record<string, string> = {
    activity: "活动信息",
    spec: "规格参数",
    review: "评价入口",
    afterSale: "售后入口",
    coupon: "领劵/优惠"
};

const PD_KEYS = Object.keys(PD_LABELS);

/** 页面自然序（五区块在详情页中的相对位置，与前台 sortBlocks 一致） */
const PD_PAGE_ORDER = ["coupon", "activity", "spec", "review", "afterSale"];

/** 出厂默认序（编辑器旧 defaults / 原版内置默认模板的原始顺序） */
const PD_FACTORY_ORDER = ["activity", "spec", "review", "afterSale", "coupon"];

const isVisibleValue = (value: unknown) => !(value === false || value === "false" || value === 0 || value === "0");

/** 任意存量 moduleSortList → 生效列表（顺序 + 显隐 + 文案齐全） */
export function normalizePdSortList(raw: unknown): PdSortItem[] {
    const entries = Array.isArray(raw) ? raw : [];
    const seen = new Set<string>();
    const stored: PdSortItem[] = [];
    for (const entry of entries) {
        const key = (entry as { key?: unknown } | null)?.key;
        if (typeof key !== "string" || !PD_KEYS.includes(key) || seen.has(key)) continue;
        seen.add(key);
        const label = (entry as { label?: unknown }).label;
        stored.push({
            key,
            label: typeof label === "string" && label ? label : PD_LABELS[key],
            visible: isVisibleValue((entry as { visible?: unknown }).visible)
        });
    }
    const isFactoryDefault =
        stored.length === PD_KEYS.length &&
        stored.every((item) => item.visible) &&
        stored.every((item, i) => item.key === PD_FACTORY_ORDER[i]);
    const makeItem = (key: string): PdSortItem => ({ key, label: PD_LABELS[key], visible: true });
    if (stored.length === 0 || isFactoryDefault) {
        return PD_PAGE_ORDER.map(makeItem);
    }
    const result = [...stored];
    for (const key of PD_PAGE_ORDER) {
        if (!seen.has(key)) result.push(makeItem(key));
    }
    return result;
}
