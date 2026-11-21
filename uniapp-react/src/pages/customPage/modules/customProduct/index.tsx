import { useEffect, useMemo, useState } from "react";
import { Image, Text, View } from "@tarojs/components";

import { designPxToRpx, formatFrame } from "@/components/modules";
import FormatPrice from "@/components/format-price/FormatPrice";
import { getProductsList } from "@/api/common";
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
 * `pruductList` 是原契约拼写（非笔误）。原版 uniapp 无此模块渲染器、
 * 原版两代编辑器 Preview 均为占位文本，无原版渲染口径可对——本渲染器
 * 的口径基准 = 我方编辑器 Preview（compat 面）+ 本仓既有渲染器通例，
 * 2026-11-21 全面复核后收口（详见 customproduct-audit/REPORT.md）：
 * - 布局与编辑器 Preview 一致：海报按 picLocation 与商品区左右并置
 *   （海报宽 37% ≈ 原版海报规格 280/750），不再整宽堆叠于商品上方；
 * - pruductList 只渲染真实商品：productId=0 演示条目运行端不展示
 *   （productCarousel 同口径），不再点进 id=0 死页；
 * - 真实条目按 productCarousel 口径回拉最新商品数据整体覆盖快照
 *   （productStatus=1/checkStatus=1），拉取失败/未返回回退快照；
 * - crossedOutPrice 划线价：===1 且 marketPrice>0 才显示（市场价为
 *   "0"/0/缺失一律不显示，含字符串 "0" 的真值陷阱）；
 * - moduleColor.productPriceColor 经 FormatPrice 分部样式真实生效；
 *   picBottomRadius=2 时海报底部圆角（半径取 productRadius）；
 *   picFillType 以 Taro Image mode 承接（cover=aspectFill/contain=aspectFit）。
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
            // 运行端只渲染真实商品（演示占位 productId:0 不展示，productCarousel 同口径）
            const items = raw.filter((x) => {
                const id = x && typeof x === "object" ? Number(x.productId ?? x.id) : Number(x);
                return Number.isFinite(id) && id > 0;
            });
            if (!items.length) {
                if (!cancelled) setProducts([]);
                return;
            }
            // 回拉最新商品整体覆盖快照（同 productCarousel 口径）；失败回退快照
            const ids = items.map((x) => Number(x.productId ?? x.id));
            const freshById = new Map<number, any>();
            try {
                const param: Record<string, any> = {
                    productStatus: 1,
                    checkStatus: 1,
                    ids: ids.join(","),
                    size: ids.length,
                    page: 1
                };
                if (shopId > 0) param.shopId = shopId;
                const result = await getProductsList(param);
                (result?.records || []).forEach((r: any) => {
                    const id = Number(r?.productId ?? r?.id);
                    if (Number.isFinite(id) && id > 0) freshById.set(id, r);
                });
            } catch {
                // 拉取失败回退各条目快照
            }
            if (!cancelled) {
                setProducts(
                    items.map((x) => {
                        const id = Number(x.productId ?? x.id);
                        const fresh = freshById.get(id);
                        return fresh ? { ...x, ...fresh } : x;
                    })
                );
            }
        })();
        return () => {
            cancelled = true;
        };
    }, [d.pruductList, shopId]);

    const picUrl = d.picUrl?.picUrl || d.picUrl?.picThumb || "";
    const colors = d.moduleColor || {};
    const isLeft = d.picLocation === "left";
    const radius = designPxToRpx(d.productRadius ?? 5);
    const showStrike = Number(d.crossedOutPrice ?? 1) === 1;
    const priceColor = colors.productPriceColor || "";
    const priceStyle = priceColor ? { color: priceColor } : undefined;

    if (!products.length && !picUrl) return null;

    return (
        <View
            className="module-ad-con module-custom-product"
            style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom}
        >
            <View className="module-ad-content" style={frameFormat.innerPadding + frameFormat.backgroundColor + frameFormat.boxRadius}>
                <View className="custom-product__body" style={{ flexDirection: isLeft ? "row" : "row-reverse" }}>
                    {picUrl ? (
                        <View
                            className="custom-product__poster"
                            style={
                                Number(d.picBottomRadius) === 2
                                    ? { borderBottomLeftRadius: radius, borderBottomRightRadius: radius }
                                    : undefined
                            }
                        >
                            <Image src={imageFormat(picUrl)} mode={d.picFillType === "contain" ? "aspectFit" : "aspectFill"} />
                        </View>
                    ) : null}
                    {products.length ? (
                        <View className="custom-product__grid">
                            {products.map((item, i) => (
                                <View
                                    key={item.productId || item.id || i}
                                    className="custom-product__item"
                                    style={{ background: colors.productBackgroundColor || "#fff", borderRadius: radius }}
                                    onClick={() => redirect({ url: `/pages/product/index?id=${item.productId || item.id}` })}
                                >
                                    <Image className="custom-product__img" src={productImage(item)} mode="aspectFill" />
                                    <Text className="custom-product__name" style={{ color: colors.productNameColor || "#333" }}>
                                        {item.productName || item.name}
                                    </Text>
                                    <View className="custom-product__price-row">
                                        <FormatPrice
                                            priceData={item.productPrice ?? item.price}
                                            currencyStyle={priceStyle}
                                            fontStyle={priceStyle}
                                            integerStyle={priceStyle}
                                            decimalsStyle={priceStyle}
                                        />
                                        {showStrike && Number(item.marketPrice) > 0 ? (
                                            <Text className="custom-product__market" style={{ color: colors.crossedOutPriceColor || "#999" }}>
                                                ¥{item.marketPrice}
                                            </Text>
                                        ) : null}
                                    </View>
                                </View>
                            ))}
                        </View>
                    ) : null}
                </View>
            </View>
        </View>
    );
}
