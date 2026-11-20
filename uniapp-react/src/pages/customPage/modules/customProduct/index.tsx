import { useEffect, useMemo, useState } from "react";
import { Image, Text, View } from "@tarojs/components";

import { designPxToRpx, formatFrame } from "@/components/modules";
import FormatPrice from "@/components/format-price/FormatPrice";
import { getDecorateProducts } from "@/components/modules/useDecorateProducts";
import { redirect } from "@/utils";
import { imageFormat, productImage } from "@/utils/format";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
    shopId?: number;
}

/**
 * 个性商品装修模块（type=customProduct，本线新增渲染入口）。
 * 数据契约以 admin 精确版 customProductDefaults 为准，注意字段名
 * `pruductList` 是原契约拼写（非笔误）。admin 侧该模块 visible:false、
 * Settings 已注明“旧版已停用，建议改用商品/商品2”，本入口只保证
 * 存量数据可渲染：pruductList 条目兼容三种形态——商品对象（直接渲染）、
 * 纯 ID、{id/productId} 对象（后两者走标准取数 getDecorateProducts
 * 的 productSelectType=1 通道，不新写取数）。
 * picUrl/picLocation：头图与其位置。picLocation 在原版无任何渲染口径：
 * 原版两代编辑器对该模块只渲染占位文本、原版 uniapp 无此模块渲染器，
 * 字段仅存于数据契约（默认 right）。本渲染器不臆造左右布局，头图一律
 * 整宽置于商品区上方；`custom-product__pic--<location>` 修饰类仅作契约
 * 保留、无对应 SCSS 规则、无布局效果（2026-11-21 核查定性）。
 */
export default function CustomProduct({ module, shopId = -1 }: Props) {
    const d = useMemo(() => {
        const data = module?.data && typeof module.data === "object" ? module.data : {};
        return { ...module, ...data };
    }, [module]);

    const frameFormat = useMemo(() => formatFrame(d.frame ?? {}), [d]);
    const [products, setProducts] = useState<any[]>([]);

    useEffect(() => {
        let cancelled = false;
        (async () => {
            const raw: any[] = Array.isArray(d.pruductList) ? d.pruductList : [];
            // 快照对象（带商品名）直渲、纯 ID / {id} 形态才取数——旧实现把快照对象的 ID
            // 也收进取数集合再 [...direct, ...fetched] 拼接，同一商品会出两张卡；
            // 且拼接顺序与 pruductList 原顺序不符。现按原顺序逐项组装、快照不重复取数。
            const isDirect = (x: any) => x && typeof x === "object" && (x.productName || x.name);
            const ids = raw
                .filter((x) => !isDirect(x))
                .map((x) => (typeof x === "object" ? Number(x?.productId ?? x?.id) : Number(x)))
                .filter((x) => x > 0);
            let fetchedById = new Map<number, any>();
            if (ids.length) {
                try {
                    const fetched = await getDecorateProducts(
                        { productSelectType: 1, productIds: ids },
                        { shopId: shopId > 0 ? shopId : undefined }
                    );
                    fetchedById = new Map(
                        (fetched || []).map((p: any) => [Number(p?.productId ?? p?.id), p])
                    );
                } catch {
                    fetchedById = new Map();
                }
            }
            if (!cancelled) {
                const ordered = raw
                    .map((x) => {
                        if (isDirect(x)) return x;
                        const id = typeof x === "object" ? Number(x?.productId ?? x?.id) : Number(x);
                        return fetchedById.get(id);
                    })
                    .filter(Boolean);
                setProducts(ordered);
            }
        })();
        return () => {
            cancelled = true;
        };
    }, [d.pruductList, shopId]);

    const picUrl = d.picUrl?.picUrl || "";
    const colors = d.moduleColor || {};

    if (!products.length && !picUrl) return null;

    return (
        <View
            className="module-ad-con module-custom-product"
            style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom}
        >
            <View className="module-ad-content" style={frameFormat.innerPadding + frameFormat.backgroundColor + frameFormat.boxRadius}>
                {picUrl ? (
                    <View className={`custom-product__pic custom-product__pic--${d.picLocation || "right"}`}>
                        <Image src={imageFormat(picUrl)} mode="aspectFill" style={{ objectFit: (d.picFillType as any) || "cover" }} />
                    </View>
                ) : null}
                <View className="custom-product__grid">
                    {products.map((item, i) => (
                        <View
                            key={item.productId || item.id || i}
                            className="custom-product__item"
                            style={{ background: colors.productBackgroundColor || "#fff", borderRadius: designPxToRpx(d.productRadius ?? 5) }}
                            onClick={() => redirect({ url: `/pages/product/index?id=${item.productId || item.id}` })}
                        >
                            <Image className="custom-product__img" src={productImage(item)} mode="aspectFill" />
                            <Text className="custom-product__name" style={{ color: colors.productNameColor || "#333" }}>
                                {item.productName || item.name}
                            </Text>
                            <View className="custom-product__price-row">
                                <FormatPrice priceData={item.productPrice ?? item.price} />
                                {d.crossedOutPrice !== 0 && item.marketPrice ? (
                                    <Text className="custom-product__market" style={{ color: colors.crossedOutPriceColor || "#999" }}>
                                        ¥{item.marketPrice}
                                    </Text>
                                ) : null}
                            </View>
                        </View>
                    ))}
                </View>
            </View>
        </View>
    );
}
