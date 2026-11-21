import { useCallback, useMemo, useRef, useState } from "react";
import { Image, ScrollView, Text, View } from "@tarojs/components";
import Taro, { useLoad } from "@tarojs/taro";

import TigLayout from "@/components/tig/tig-layout/TigLayout";
import FormatPrice from "@/components/format-price/FormatPrice";
import { getShopCategory, getShopDetail } from "@/api/shop/shop";
import { getCategoryProduct } from "@/api/search/search";
import { addToCart, getProductDetail } from "@/api/product/product";
import { getCart, updateCartCheck } from "@/api/cart/cart";
import { useCartStore } from "@/store/cart";
import { useConfigStore } from "@/store/config";
import { imageFormat } from "@/utils/format";
import { usePageGuards } from "@/utils/pageMixin";
import { t } from "@/utils/i18n";
import type { ShopCategoryList, ShopDetailItem } from "@/types/shop/shop";
import type { SkuList } from "@/types/product/product.d";

import "./index.scss";

/** 点单页本地购物车行（结算时逐行写入现有购物车链，不另造下单通道） */
interface QuickCartLine {
    productId: number;
    /** 0 = 无规格 */
    skuId: number;
    skuValue: string;
    name: string;
    pic: string;
    price: number;
    quantity: number;
    /** 库存上限；<=0 表示售罄（不可加），undefined 表示未取到不设上限 */
    stock?: number;
}

interface QuickProduct {
    productId: number;
    productName: string;
    picUrl?: string;
    picThumb?: string;
    productPrice?: number | string;
    productStock?: number;
}

const PAGE_SIZE = 50;
const ALL_CATEGORY_ID = 0;

const lineKey = (productId: number, skuId: number) => `${productId}_${skuId}`;

const skuValueText = (sku: SkuList): string => {
    if (Array.isArray(sku.skuData) && sku.skuData.length > 0) {
        return sku.skuData.map((d) => d.value).filter(Boolean).join(" / ");
    }
    return sku.skuValue || "";
};

/**
 * 门店点单页（论坛帖 127834「快捷点单模块」完整点单模式，原版有·我方补齐）。
 *
 * 原版点单卡片（storeOrderCard）只是入口卡片，原版其后上线的「点单模式」是
 * 完整点单下单流；本页即该流：左侧店铺分类、右侧商品、底部购物车栏一体，
 * 顶部可切换堂食 / 自提。下单不另造通道：逐行 addToCart 写入现有购物车，
 * 再把勾选收敛到本单行，跳现有结算页 /pages/order/check 并以
 * deliveryType=2（到店自提）+ tabDeliveryTypes=1 锁定自提口径；
 * 堂食时另带 orderMode=dinein，结算页据此预填买家备注「堂食」（可改）。
 */
export default function QuickOrder() {
    usePageGuards();

    const shopIdRef = useRef(0);
    const [shopId, setShopId] = useState(0);
    const [shopInfo, setShopInfo] = useState<ShopDetailItem>({} as ShopDetailItem);
    /** dinein 堂食 / pickup 自提 */
    const [mode, setMode] = useState<"dinein" | "pickup">("pickup");
    const [categories, setCategories] = useState<ShopCategoryList[]>([]);
    const [activeCatId, setActiveCatId] = useState(ALL_CATEGORY_ID);
    const [products, setProducts] = useState<QuickProduct[]>([]);
    const [page, setPage] = useState(1);
    const [hasMore, setHasMore] = useState(false);
    const [loadingProducts, setLoadingProducts] = useState(false);
    const [lines, setLines] = useState<QuickCartLine[]>([]);
    const [drawerOpen, setDrawerOpen] = useState(false);
    const [skuPopup, setSkuPopup] = useState<{ product: QuickProduct; skus: SkuList[]; skuId: number; qty: number } | null>(null);
    const [submitting, setSubmitting] = useState(false);

    const activeCatRef = useRef(ALL_CATEGORY_ID);
    const skuCacheRef = useRef<Map<number, SkuList[]>>(new Map());
    const productsCacheRef = useRef<Map<number, { list: QuickProduct[]; page: number; hasMore: boolean }>>(new Map());

    const totalCount = useMemo(() => lines.reduce((sum, l) => sum + l.quantity, 0), [lines]);
    const totalAmount = useMemo(() => lines.reduce((sum, l) => sum + l.price * l.quantity, 0), [lines]);

    const linesOfProduct = useCallback((productId: number) => lines.filter((l) => l.productId === productId), [lines]);
    const productQty = useCallback((productId: number) => linesOfProduct(productId).reduce((s, l) => s + l.quantity, 0), [linesOfProduct]);

    const loadProducts = useCallback(async (catId: number, nextPage: number) => {
        if (loadingProducts) return;
        const cached = productsCacheRef.current.get(catId);
        if (nextPage === 1 && cached) {
            setProducts(cached.list);
            setPage(cached.page);
            setHasMore(cached.hasMore);
            return;
        }
        setLoadingProducts(true);
        try {
            const result: any = await getCategoryProduct({
                page: nextPage,
                size: PAGE_SIZE,
                shopId: shopIdRef.current,
                ...(catId > 0 ? { shopCategoryId: catId } : {})
            });
            const records: QuickProduct[] = Array.isArray(result?.records) ? result.records : [];
            const total = Number(result?.total ?? records.length);
            const merged = nextPage === 1 ? records : [...(cached?.list ?? []), ...records];
            const more = merged.length < total;
            productsCacheRef.current.set(catId, { list: merged, page: nextPage, hasMore: more });
            // 分类已切换时不回写旧分类结果（缓存已按分类落好，切回时直接命中）
            if (activeCatRef.current !== catId) return;
            setProducts((prev) => (nextPage === 1 ? merged : [...prev, ...records]));
            setPage(nextPage);
            setHasMore(more);
        } catch (error) {
            console.error(error);
        } finally {
            setLoadingProducts(false);
        }
    }, [loadingProducts]);

    const init = useCallback(async (id: number) => {
        try {
            const detail = await getShopDetail(id);
            setShopInfo(detail);
        } catch (error) {
            console.error(error);
        }
        try {
            const cats = await getShopCategory(id);
            setCategories(Array.isArray(cats) ? cats : []);
        } catch (error) {
            console.error(error);
        }
        void loadProducts(ALL_CATEGORY_ID, 1);
    }, [loadProducts]);

    useLoad((options: any) => {
        const id = Number(options?.shopId ?? options?.id ?? 0);
        if (options?.mode === "dinein") setMode("dinein");
        if (id > 0) {
            shopIdRef.current = id;
            setShopId(id);
            void init(id);
        }
    });

    const switchCategory = (catId: number) => {
        if (catId === activeCatId) return;
        activeCatRef.current = catId;
        setActiveCatId(catId);
        const cached = productsCacheRef.current.get(catId);
        if (cached) {
            setProducts(cached.list);
            setPage(cached.page);
            setHasMore(cached.hasMore);
        } else {
            setProducts([]);
            void loadProducts(catId, 1);
        }
    };

    const ensureSkus = useCallback(async (productId: number): Promise<SkuList[]> => {
        const cached = skuCacheRef.current.get(productId);
        if (cached) return cached;
        try {
            const detail: any = await getProductDetail(productId);
            const skus: SkuList[] = Array.isArray(detail?.skuList) ? detail.skuList : [];
            skuCacheRef.current.set(productId, skus);
            return skus;
        } catch (error) {
            console.error(error);
            return [];
        }
    }, []);

    const upsertLine = useCallback((line: QuickCartLine, delta: number) => {
        setLines((prev) => {
            const key = lineKey(line.productId, line.skuId);
            const idx = prev.findIndex((l) => lineKey(l.productId, l.skuId) === key);
            if (idx === -1) {
                if (delta <= 0) return prev;
                const qty = line.stock !== undefined && line.stock >= 0 ? Math.min(delta, Math.max(line.stock, 0)) : delta;
                if (qty <= 0) return prev;
                return [...prev, { ...line, quantity: qty }];
            }
            const next = [...prev];
            const cur = next[idx];
            let qty = cur.quantity + delta;
            if (cur.stock !== undefined && cur.stock >= 0) qty = Math.min(qty, cur.stock);
            if (qty <= 0) {
                next.splice(idx, 1);
            } else {
                next[idx] = { ...cur, quantity: qty };
            }
            return next;
        });
    }, []);

    const openSkuPopup = useCallback(
        async (product: QuickProduct) => {
            const skus = await ensureSkus(product.productId);
            if (skus.length === 0) return;
            const firstAvailable = skus.find((s) => (s.skuStock ?? 1) > 0) ?? skus[0];
            setSkuPopup({ product, skus, skuId: firstAvailable.skuId, qty: 1 });
        },
        [ensureSkus]
    );

    const handlePlus = useCallback(
        async (product: QuickProduct) => {
            const own = linesOfProduct(product.productId);
            if (own.length === 1) {
                upsertLine(own[0], 1);
                return;
            }
            if (own.length > 1) {
                setDrawerOpen(true);
                return;
            }
            const skus = await ensureSkus(product.productId);
            if (skus.length > 0) {
                const firstAvailable = skus.find((s) => (s.skuStock ?? 1) > 0) ?? skus[0];
                setSkuPopup({ product, skus, skuId: firstAvailable.skuId, qty: 1 });
                return;
            }
            upsertLine(
                {
                    productId: product.productId,
                    skuId: 0,
                    skuValue: "",
                    name: product.productName,
                    pic: product.picUrl || product.picThumb || "",
                    price: Number(product.productPrice) || 0,
                    quantity: 0,
                    stock: product.productStock
                },
                1
            );
        },
        [ensureSkus, linesOfProduct, upsertLine]
    );

    const handleMinus = useCallback(
        (product: QuickProduct) => {
            const own = linesOfProduct(product.productId);
            if (own.length === 1) {
                upsertLine(own[0], -1);
            } else if (own.length > 1) {
                setDrawerOpen(true);
            }
        },
        [linesOfProduct, upsertLine]
    );

    const confirmSku = () => {
        if (!skuPopup) return;
        const sku = skuPopup.skus.find((s) => s.skuId === skuPopup.skuId);
        if (!sku) return;
        upsertLine(
            {
                productId: skuPopup.product.productId,
                skuId: sku.skuId,
                skuValue: skuValueText(sku),
                name: skuPopup.product.productName,
                pic: skuPopup.product.picUrl || skuPopup.product.picThumb || "",
                price: Number(sku.skuPrice) || Number(skuPopup.product.productPrice) || 0,
                quantity: 0,
                stock: sku.skuStock
            },
            skuPopup.qty
        );
        setSkuPopup(null);
    };

    /** 下单：写入现有购物车 → 勾选收敛到本单行 → 跳现有结算页（自提口径锁定） */
    const handleSubmit = async () => {
        if (submitting) return;
        if (lines.length === 0) {
            Taro.showToast({ title: t("请先选择商品"), icon: "none" });
            return;
        }
        if (useConfigStore.getState().closeOrder === 1) {
            Taro.showToast({ title: t("商城已关闭下单"), icon: "none" });
            return;
        }
        setSubmitting(true);
        try {
            for (const line of lines) {
                await addToCart({
                    id: line.productId,
                    number: line.quantity,
                    skuId: line.skuId > 0 ? line.skuId : undefined,
                    shopId
                });
            }
            // 加购同款会合并既有购物车行；按商品+规格在本店分组内回查 cartId，
            // 再把全车勾选收敛到本单行，保证结算页只带点单商品（与购物车页
            // 直跳 /pages/order/check 的既有口径一致）。
            const cartData: any = await getCart();
            const groups: any[] = Array.isArray(cartData?.cartList) ? cartData.cartList : [];
            const wanted = new Set(lines.map((l) => lineKey(l.productId, l.skuId)));
            const checkData: { cartId: number; isChecked: number }[] = [];
            groups.forEach((group) => {
                (group.carts ?? []).forEach((item: any) => {
                    const hit = Number(group.shopId) === shopId && wanted.has(lineKey(Number(item.productId), Number(item.skuId ?? 0)));
                    checkData.push({ cartId: Number(item.cartId), isChecked: hit ? 1 : 0 });
                });
            });
            if (checkData.length > 0) {
                await updateCartCheck({ data: checkData });
            }
            void useCartStore.getState().fetchCartCount(true);
            const modeQuery = mode === "dinein" ? "&orderMode=dinein" : "";
            Taro.navigateTo({
                url: `/pages/order/check?deliveryType=2&tabDeliveryTypes=1${modeQuery}`
            });
        } catch (error: any) {
            Taro.showToast({ title: error?.message || t("下单失败，请重试"), icon: "none" });
        } finally {
            setSubmitting(false);
        }
    };

    const activeCatName = activeCatId === ALL_CATEGORY_ID ? t("全部商品") : categories.find((c) => c.categoryId === activeCatId)?.categoryName ?? "";

    const renderProduct = (product: QuickProduct) => {
        const qty = productQty(product.productId);
        const soldOut = product.productStock !== undefined && product.productStock !== null && Number(product.productStock) <= 0;
        const knownSkus = skuCacheRef.current.get(product.productId);
        const showSkuButton = qty === 0 && knownSkus !== undefined && knownSkus.length > 0;
        return (
            <View className="quick-order-product" key={product.productId}>
                {(product.picUrl || product.picThumb) ? (
                    <Image className="quick-order-product-img" src={imageFormat(product.picUrl || product.picThumb || "")} mode="aspectFill" />
                ) : (
                    <View className="quick-order-product-img" />
                )}
                <View className="quick-order-product-main">
                    <Text className="quick-order-product-name">{product.productName}</Text>
                    {product.productStock !== undefined && product.productStock !== null ? (
                        <Text className="quick-order-product-stock">
                            {t("库存")} {product.productStock}
                        </Text>
                    ) : null}
                    <View className="quick-order-product-bottom">
                        <FormatPrice priceData={product.productPrice} fontStyle={{ color: "#e64340", fontWeight: "600" }} />
                        {soldOut ? (
                            <Text className="quick-order-soldout">{t("已售罄")}</Text>
                        ) : showSkuButton ? (
                            <View className="quick-order-sku-btn" onClick={() => void openSkuPopup(product)}>
                                {t("选规格")}
                            </View>
                        ) : (
                            <View className="quick-order-stepper">
                                {qty > 0 ? (
                                    <>
                                        <View className="quick-order-step-btn minus" onClick={() => handleMinus(product)}>
                                            −
                                        </View>
                                        <Text className="quick-order-step-num">{qty}</Text>
                                    </>
                                ) : null}
                                <View className="quick-order-step-btn plus" onClick={() => void handlePlus(product)}>
                                    +
                                </View>
                            </View>
                        )}
                    </View>
                </View>
            </View>
        );
    };

    return (
        <TigLayout title={shopInfo.shopTitle || t("门店点单")} shopId={shopId}>
            <View className="quick-order-page">
                <View className="quick-order-head">
                    <View className="quick-order-shop-name">{shopInfo.shopTitle || t("门店点单")}</View>
                    {shopInfo.description ? <View className="quick-order-shop-desc">{shopInfo.description}</View> : null}
                    <View className="quick-order-mode">
                        <View className={`quick-order-mode-item ${mode === "dinein" ? "active" : ""}`} onClick={() => setMode("dinein")}>
                            {t("堂食")}
                        </View>
                        <View className={`quick-order-mode-item ${mode === "pickup" ? "active" : ""}`} onClick={() => setMode("pickup")}>
                            {t("自提")}
                        </View>
                    </View>
                </View>

                <View className="quick-order-body">
                    <ScrollView className="quick-order-cats" scrollY>
                        <View className={`quick-order-cat ${activeCatId === ALL_CATEGORY_ID ? "active" : ""}`} onClick={() => switchCategory(ALL_CATEGORY_ID)}>
                            {t("全部商品")}
                        </View>
                        {categories.map((cat) => (
                            <View
                                className={`quick-order-cat ${activeCatId === cat.categoryId ? "active" : ""}`}
                                key={cat.categoryId}
                                onClick={() => switchCategory(cat.categoryId)}
                            >
                                {cat.categoryName}
                            </View>
                        ))}
                    </ScrollView>
                    <ScrollView
                        className="quick-order-products"
                        scrollY
                        onScrollToLower={() => {
                            if (hasMore && !loadingProducts) void loadProducts(activeCatId, page + 1);
                        }}
                    >
                        <View className="quick-order-cat-title">{activeCatName}</View>
                        {products.map(renderProduct)}
                        {products.length === 0 && !loadingProducts ? <View className="quick-order-empty">{t("该分类暂无商品")}</View> : null}
                    </ScrollView>
                </View>

                <View className="quick-order-cartbar">
                    <View className="quick-order-cart-info" onClick={() => totalCount > 0 && setDrawerOpen(true)}>
                        <View className="quick-order-cart-count">
                            {t("已选")} {totalCount} {t("件")}
                        </View>
                        <View className="quick-order-cart-total">
                            <FormatPrice priceData={totalAmount} fontStyle={{ color: "#e64340", fontWeight: "600" }} />
                        </View>
                    </View>
                    <View className={`quick-order-submit ${totalCount === 0 || submitting ? "disabled" : ""}`} onClick={() => void handleSubmit()}>
                        {mode === "dinein" ? t("堂食下单") : t("去结算")}
                    </View>
                </View>

                {drawerOpen ? (
                    <>
                        <View className="quick-order-drawer-mask" onClick={() => setDrawerOpen(false)} />
                        <View className="quick-order-drawer">
                            <View className="quick-order-drawer-head">
                                <Text className="quick-order-drawer-title">{t("已选商品")}</Text>
                                <Text
                                    className="quick-order-drawer-clear"
                                    onClick={() => {
                                        setLines([]);
                                        setDrawerOpen(false);
                                    }}
                                >
                                    {t("清空")}
                                </Text>
                            </View>
                            <View className="quick-order-drawer-list">
                                {lines.map((line) => (
                                    <View className="quick-order-drawer-line" key={lineKey(line.productId, line.skuId)}>
                                        <View className="quick-order-drawer-line-main">
                                            <View className="quick-order-drawer-line-name">{line.name}</View>
                                            {line.skuValue ? <View className="quick-order-drawer-line-sku">{line.skuValue}</View> : null}
                                        </View>
                                        <View className="quick-order-drawer-line-price">
                                            <FormatPrice priceData={line.price * line.quantity} />
                                        </View>
                                        <View className="quick-order-stepper">
                                            <View className="quick-order-step-btn minus" onClick={() => upsertLine(line, -1)}>
                                                −
                                            </View>
                                            <Text className="quick-order-step-num">{line.quantity}</Text>
                                            <View className="quick-order-step-btn plus" onClick={() => upsertLine(line, 1)}>
                                                +
                                            </View>
                                        </View>
                                    </View>
                                ))}
                            </View>
                            <View className="quick-order-drawer-foot">
                                <View className="quick-order-cart-total">
                                    <FormatPrice priceData={totalAmount} fontStyle={{ color: "#e64340", fontWeight: "600" }} />
                                </View>
                                <View
                                    className="quick-order-drawer-confirm"
                                    onClick={() => {
                                        setDrawerOpen(false);
                                        void handleSubmit();
                                    }}
                                >
                                    {mode === "dinein" ? t("堂食下单") : t("去结算")}
                                </View>
                            </View>
                        </View>
                    </>
                ) : null}

                {skuPopup ? (
                    <>
                        <View className="quick-order-drawer-mask" onClick={() => setSkuPopup(null)} />
                        <View className="quick-order-drawer">
                            <View className="quick-order-drawer-head">
                                <Text className="quick-order-drawer-title">{skuPopup.product.productName}</Text>
                                <Text className="quick-order-drawer-clear" onClick={() => setSkuPopup(null)}>
                                    {t("关闭")}
                                </Text>
                            </View>
                            <View className="quick-order-sku-list">
                                {skuPopup.skus.map((sku) => {
                                    const out = (sku.skuStock ?? 1) <= 0;
                                    return (
                                        <View
                                            className={`quick-order-sku-item ${skuPopup.skuId === sku.skuId ? "active" : ""} ${out ? "disabled" : ""}`}
                                            key={sku.skuId}
                                            onClick={() => {
                                                if (!out) setSkuPopup({ ...skuPopup, skuId: sku.skuId });
                                            }}
                                        >
                                            <View>
                                                <View className="quick-order-sku-value">{skuValueText(sku)}</View>
                                                <View className="quick-order-sku-meta">
                                                    {t("库存")} {sku.skuStock ?? "-"}
                                                    {out ? ` · ${t("已售罄")}` : ""}
                                                </View>
                                            </View>
                                            <FormatPrice priceData={sku.skuPrice} />
                                        </View>
                                    );
                                })}
                            </View>
                            <View className="quick-order-drawer-foot">
                                <View className="quick-order-stepper">
                                    <View
                                        className="quick-order-step-btn minus"
                                        onClick={() => setSkuPopup({ ...skuPopup, qty: Math.max(1, skuPopup.qty - 1) })}
                                    >
                                        −
                                    </View>
                                    <Text className="quick-order-step-num">{skuPopup.qty}</Text>
                                    <View className="quick-order-step-btn plus" onClick={() => setSkuPopup({ ...skuPopup, qty: skuPopup.qty + 1 })}>
                                        +
                                    </View>
                                </View>
                                <View className="quick-order-drawer-confirm" onClick={confirmSku}>
                                    {t("加入购物车")}
                                </View>
                            </View>
                        </View>
                    </>
                ) : null}
            </View>
        </TigLayout>
    );
}
