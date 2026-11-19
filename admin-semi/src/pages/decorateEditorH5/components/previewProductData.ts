/**
 * 真实渲染画布的商品真实价格下发【我方新增】（2026-11-20）。
 *
 * 背景：真实渲染画布是 iframe 嵌入前台 decoratePreview 页，嵌入页内各商品
 * 模块以匿名身份调公开商品列表接口。isIdentity 价格门控开启后，公开接口
 * 把价格遮罩为 null，嵌入页按兜底渲染成 0.00——编辑器里商品价格全显示 0
 * （页面 10019 实证，2026-11-20 用户报）。
 *
 * 口径：编辑器是管理面、预览必须显示真实价格。父页面（本文件所在的管理
 * 端，管理员登录口径）按页面内各商品来源——与兼容渲染各 Preview 的取数
 * 口径完全一致（useDecorateProductList 同参数）——经管理端商品列表接口
 * 预取真实价格，只保留价格字段（与后端 price_gate 的 PRICE_KEYS 同名单），
 * 按商品 id 组表，随 decorate-data 的 productData 字段下发；嵌入页在商品
 * 列表响应上按 id 回填被遮罩的价格。价格只走已鉴权的管理端通道，不经
 * 任何公开接口，店面门控（匿名/未认证遮价、已认证明文）不受影响。
 */
import { apiGet } from "../../../api";
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

export interface PreviewProductSource {
    productSelectType: number;
    productIds?: Array<string | number>;
    productCategoryId?: any;
    productTag?: string;
    productNumber?: number;
}

const isObj = (v: any): boolean => !!v && typeof v === "object";

/** 从 pruductList 形态的数组里解析商品 id（纯 id / {productId|id} 两种形态） */
function idsFromList(list: any[]): Array<string | number> {
    return list
        .map((x) => (isObj(x) ? Number(x.productId ?? x.id) : Number(x)))
        .filter((n) => Number.isFinite(n) && n > 0);
}
/**
 * 递归收集模块数据里的全部商品来源，与兼容渲染各 Preview 的取数集合一致：
 * - 带 productSelectType 的对象（product/product2/productA1/productSlide/
 *   limitedSpecial/newcomerCombo 等模块的 products 配置）；
 * - 带 pruductList 的对象（customProduct 的条目表、mixedProductDisplay 的
 *   productGroups 分组），按其 id 集合合成指定商品来源。
 * 快照型模块（productGrouping 等，价格已存页面数据）不经接口取数，其快照
 * 对象不带上述两种形态，不会被误收。
 */
export function collectProductSources(moduleList: any[]): PreviewProductSource[] {
    const seen = new Set<string>();
    const sources: PreviewProductSource[] = [];
    const push = (s: PreviewProductSource) => {
        const key = JSON.stringify(s);
        if (seen.has(key)) return;
        seen.add(key);
        sources.push(s);
    };
    const visit = (node: any, depth: number) => {
        if (!isObj(node) || depth > 8) return;
        if (Array.isArray(node)) {
            node.forEach((n) => visit(n, depth + 1));
            return;
        }
        const type = Number(node.productSelectType || 0);
        if (type === 1) {
            const ids = Array.isArray(node.productIds)
                ? node.productIds.filter((v: any) => v !== "" && v !== null && v !== undefined)
                : [];
            if (ids.length) push({ productSelectType: 1, productIds: ids });
        } else if (type === 2) {
            if (
                node.productCategoryId !== null &&
                node.productCategoryId !== undefined &&
                node.productCategoryId !== 0 &&
                node.productCategoryId !== ""
            ) {
                push({
                    productSelectType: 2,
                    productCategoryId: node.productCategoryId,
                    productNumber: node.productNumber
                });
            }
        } else if (type === 3) {
            if (node.productTag) {
                push({ productSelectType: 3, productTag: node.productTag, productNumber: node.productNumber });
            }
        }
        if (Array.isArray(node.pruductList)) {
            const ids = idsFromList(node.pruductList);
            if (ids.length) push({ productSelectType: 1, productIds: ids });
        }
        Object.values(node).forEach((v) => visit(v, depth + 1));
    };
    (moduleList || []).forEach((item: any) => visit(item?.module ?? item, 0));
    return sources;
}
/** 来源 → 管理端列表参数（与 useDecorateProductList 的参数口径逐项一致） */
function buildSourceParams(source: PreviewProductSource): Record<string, any> | null {
    const type = Number(source.productSelectType || 0);
    if (type === 1) {
        const ids = (source.productIds || []).filter((v) => v !== "" && v !== null && v !== undefined);
        if (!ids.length) return null;
        return { ids: ids.join(","), size: ids.length || 15 };
    }
    if (type === 2) {
        const leaf = Array.isArray(source.productCategoryId)
            ? Number(source.productCategoryId[source.productCategoryId.length - 1])
            : Number(source.productCategoryId);
        if (!Number.isFinite(leaf) || leaf === 0) return null;
        return { page: 1, categoryId: leaf, size: source.productNumber ?? 15 };
    }
    if (type === 3) {
        if (!source.productTag || String(source.productTag).trim() === "") return null;
        return { page: 1, introType: source.productTag, size: source.productNumber ?? 15 };
    }
    return null;
}

/** 只保留 id + 价格字段，控制 postMessage 载荷体积 */
function trimPriceRecord(record: any): Record<string, any> | null {
    const id = record?.productId ?? record?.id;
    if (id === null || id === undefined) return null;
    const out: Record<string, any> = { productId: Number(id) };
    for (const k of PREVIEW_PRICE_KEYS) {
        if (record[k] !== null && record[k] !== undefined) out[k] = record[k];
    }
    return out;
}
/**
 * 按来源集合经管理端商品列表接口预取真实价格，组装 { recordsById } 表。
 * 单个来源失败只跳过该来源，不阻塞其余来源与画布渲染。
 */
export async function fetchPreviewProductData(
    sources: PreviewProductSource[],
    shopId: number
): Promise<{ recordsById: Record<string, Record<string, any>> }> {
    const recordsById: Record<string, Record<string, any>> = {};
    await Promise.all(
        sources.map(async (source) => {
            const param = buildSourceParams(source);
            if (!param) return;
            try {
                const result = await apiGet<{ records?: any[]; list?: any[] }>("/product/product/list", {
                    productStatus: 1,
                    checkStatus: 1,
                    ...(shopId > 0 ? { shopId } : {}),
                    ...param
                });
                const records = result?.records || result?.list || [];
                records.forEach((r) => {
                    const trimmed = trimPriceRecord(r);
                    if (trimmed && !(String(trimmed.productId) in recordsById)) {
                        recordsById[String(trimmed.productId)] = trimmed;
                    }
                });
            } catch {
                /* 单个来源取数失败不阻塞其余来源与画布渲染 */
            }
        })
    );
    return { recordsById };
}
