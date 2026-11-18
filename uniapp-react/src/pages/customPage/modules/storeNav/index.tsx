import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { Image, ScrollView, Text, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import { getStoreProduct } from "@/api/search/search";
import { shopCollection } from "@/api/shop/shop";
import { getStoreDetail } from "@/api/store/store";
import FormatPrice from "@/components/format-price/FormatPrice";
import type { ModuleStyle, ModuleContentStyle } from "@/types/decorate/decorate.d";
import { designPxToRpx, formatCommonStyle } from "@/components/modules";
import { useRegionStore } from "@/store/region";
import type { StoreSearchParams, StoreSearchProduct, StoreSearchRecord } from "@/types/search/storeSearch";
import { redirect, staticResource } from "@/utils";
import { formatPadZero, imageFormat, productImage } from "@/utils/format";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
    shopId?: number;
}

/** 标题样式组（subTitle / storeTitleStyle 共用形状，对齐编辑器 SubTitleSettingsField） */
interface TitleStyle {
    showTitle?: number;
    titleColor?: string;
    titleFontSize?: number;
    titleFontBold?: number;
}

/** 指定门店条目（原版编辑器 types.ts StoreNavShopItem） */
interface StoreNavShopItem {
    shopId: number;
    shopTitle: string;
    shopLogo?: string;
}

/** 门店导航模块数据（原版编辑器 mobilePro/storeNav 契约，本目录内收敛，不动全局类型） */
interface StoreNavData {
    subTitle: TitleStyle;
    storeTitleStyle: TitleStyle;
    titlePaddingY: number;
    titlePaddingX: number;
    storeStyle: number;
    storeSource: number;
    storeNum: number;
    storeList: StoreNavShopItem[];
    showRating: number;
    showFollow: number;
    showMinOrder: number;
    showDeliveryFee: number;
    showDistance: number;
    storeGap: number;
    storeBackgroundColor: string;
    storePaddingY: number;
    storePaddingX: number;
    storeRadius: number;
    productGap: number;
    showProductName: number;
    productNameRow: number;
    showProductPrice: number;
    moduleStyle: ModuleStyle;
    contentStyle: ModuleContentStyle;
}

/** 编辑器 defaults（admin mobilePro/storeNav/index.ts + sharedDefaults 同值，本地镜像用于缺字段兜底） */
const DEFAULT_SUB_TITLE: TitleStyle = { showTitle: 1, titleColor: "#454545", titleFontSize: 12, titleFontBold: 400 };
const DEFAULT_STORE_TITLE_STYLE: TitleStyle = { showTitle: 1, titleColor: "#1a1a1a", titleFontSize: 14, titleFontBold: 700 };
const DEFAULT_MODULE_STYLE: Record<string, any> = {
    boxPaddingTopBottom: 0,
    boxPaddingTop: 5,
    boxPaddingBottom: 0,
    boxPadding: 10,
    boxMargin: 0,
    backgroundColor: "",
    backgroundPic: { picUrl: "", picThumb: "" },
    backgroundPicFillType: "cover"
};
const DEFAULT_CONTENT_STYLE: Record<string, any> = {
    paddingTopBottom: 10,
    paddingTop: 0,
    paddingBottom: 0,
    paddingLeftRight: 0,
    padding: 0,
    boxRadiusTop: 0,
    boxRadiusBottom: 0,
    boxRadius: 0,
    gradientType: "upDown",
    gradientColorA: "",
    gradientColorB: "",
    backgroundColor: "#ffffff",
    backgroundPic: { picUrl: "", picThumb: "" },
    backgroundPicFillType: "cover",
    boxShadow: 0
};

/**
 * 当前定位参数（与 search/result.tsx 同一校验链：坐标合法且非零点才带上）；
 * 未定位时接口返回的距离不可信，hasLocation=false 时距离一律不展示（原版 storeSearchCard 同口径）。
 */
const getLocationContext = () => {
    const info = useRegionStore.getState().currentLocationInfo;
    const longitude = Number(info?.longitude);
    const latitude = Number(info?.latitude);
    const valid =
        Number.isFinite(longitude) &&
        Number.isFinite(latitude) &&
        longitude >= -180 &&
        longitude <= 180 &&
        latitude >= -90 &&
        latitude <= 90 &&
        (longitude !== 0 || latitude !== 0);
    return valid
        ? { params: { longitude, latitude } as StoreSearchParams, hasLocation: true }
        : { params: {} as StoreSearchParams, hasLocation: false };
};

/** 距离文案（原版 storeSearchCard：<1000 显示 m，否则两位小数 km） */
const formatDistance = (distance: unknown, hasLocation: boolean): string => {
    if (!hasLocation || distance == null) return "";
    const num = Number(distance);
    if (!Number.isFinite(num) || num < 0) return "";
    return num < 1000 ? `${Math.round(num)}m` : `${(num / 1000).toFixed(2)}km`;
};

/** 评分文案：接口有有效 shopStar 才显示（原版卡片缺星时回落 5，本模块按「数据缺失不显示」口径，见交付说明） */
const formatScore = (shopStar: unknown): string => {
    const star = Number(shopStar);
    if (!Number.isFinite(star) || star <= 0) return "";
    return formatPadZero(star, 1);
};

/**
 * 门店导航装修模块（type=storeNav）。
 * 契约以原版编辑器 mobilePro/storeNav（index.ts/types.ts）为准，布局对齐
 * 编辑器 Preview（样式一）与原版 uniapp 门店聚合卡 storeSearchCard（样式二）：
 * - 样式一：标题行（固定文案「附近门店」，subTitle 控样式与显隐）+ 门店列表
 *   （68px Logo、店名、评分、地址、距离胶囊）；
 * - 样式二：门店卡（星级标签/店名/关注钮、评分/起送/配送费/距离 meta 行）
 *   + 每店商品横滑行（名称/价格受开关控制）。
 * 取数统一走门店聚合通道 getStoreProduct（o2o/storeSearch/getStoreProduct，
 * 与搜索页门店样式同一接口）：自动模式按 storeNum（钳制 1~20）拉取；
 * 指定模式传 shopIds 按 storeList 顺序回排，接口未返回的门店再用
 * getStoreDetail 按 shopId 补拉详情，仍补不到就只渲染 storeList 原始条目
 * （shopId/shopTitle/shopLogo），不编造评分/距离/商品。
 * 关注钮走真实收藏接口 shopCollection（原版卡片同口径）。
 * 无门店数据（加载中/接口空/指定列表为空）时整模块不渲染（与 customProduct 等邻居模块一致）。
 */
export default function StoreNav({ module }: Props) {
    const d = useMemo<StoreNavData>(() => {
        const raw: Record<string, any> =
            module?.data && typeof module.data === "object" ? { ...module, ...module.data } : { ...(module ?? {}) };
        return {
            subTitle: { ...DEFAULT_SUB_TITLE, ...(raw.subTitle ?? {}) },
            storeTitleStyle: { ...DEFAULT_STORE_TITLE_STYLE, ...(raw.storeTitleStyle ?? {}) },
            titlePaddingY: Number(raw.titlePaddingY ?? 0) || 0,
            titlePaddingX: Number(raw.titlePaddingX ?? 0) || 0,
            storeStyle: Number(raw.storeStyle ?? 1) || 1,
            storeSource: Number(raw.storeSource ?? 1) || 1,
            storeNum: Number(raw.storeNum ?? 10) || 10,
            storeList: Array.isArray(raw.storeList) ? raw.storeList : [],
            showRating: Number(raw.showRating ?? 1),
            showFollow: Number(raw.showFollow ?? 1),
            showMinOrder: Number(raw.showMinOrder ?? 1),
            showDeliveryFee: Number(raw.showDeliveryFee ?? 1),
            showDistance: Number(raw.showDistance ?? 1),
            storeGap: Number(raw.storeGap ?? 15) || 0,
            storeBackgroundColor: raw.storeBackgroundColor || "#ffffff",
            storePaddingY: Number(raw.storePaddingY ?? 0) || 0,
            storePaddingX: Number(raw.storePaddingX ?? 0) || 0,
            storeRadius: Number(raw.storeRadius ?? 0) || 0,
            productGap: Number(raw.productGap ?? 8) || 0,
            showProductName: Number(raw.showProductName ?? 1),
            productNameRow: Number(raw.productNameRow ?? 1) || 1,
            showProductPrice: Number(raw.showProductPrice ?? 1),
            moduleStyle: { ...DEFAULT_MODULE_STYLE, ...(raw.moduleStyle ?? {}) },
            contentStyle: { ...DEFAULT_CONTENT_STYLE, ...(raw.contentStyle ?? {}) }
        };
    }, [module]);

    const commonStyle = useMemo(
        () => formatCommonStyle(d.moduleStyle, d.contentStyle, d.subTitle),
        [d.moduleStyle, d.contentStyle, d.subTitle]
    );

    const [stores, setStores] = useState<StoreSearchRecord[] | null>(null);
    const [hasLocation, setHasLocation] = useState(false);
    const [collectOverrides, setCollectOverrides] = useState<Record<number, boolean>>({});
    const collectPendingRef = useRef<Set<number>>(new Set());

    const assignIdsKey = useMemo(
        () =>
            d.storeList
                .map((item) => Number(item?.shopId))
                .filter((id) => Number.isFinite(id) && id >= 0)
                .join(","),
        [d.storeList]
    );

    useEffect(() => {
        let cancelled = false;
        (async () => {
            const { params: locationParams, hasLocation: located } = getLocationContext();
            if (!cancelled) setHasLocation(located);
            try {
                if (d.storeSource === 2) {
                    // 指定模式：按 storeList 顺序展示，聚合接口一次补齐详情与商品
                    const items = d.storeList.filter((item) => Number.isFinite(Number(item?.shopId)) && Number(item?.shopId) >= 0);
                    if (!items.length) {
                        if (!cancelled) setStores([]);
                        return;
                    }
                    const ids = items.map((item) => Number(item.shopId));
                    const res: any = await getStoreProduct({
                        shopIds: ids.join(","),
                        page: 1,
                        size: ids.length,
                        ...locationParams
                    });
                    const byId = new Map<number, StoreSearchRecord>();
                    (Array.isArray(res?.records) ? res.records : []).forEach((rec: StoreSearchRecord) => {
                        if (rec && Number.isFinite(Number(rec.shopId))) byId.set(Number(rec.shopId), rec);
                    });
                    const merged = await Promise.all(
                        items.map(async (item) => {
                            const id = Number(item.shopId);
                            const hit = byId.get(id);
                            if (hit) {
                                // 聚合记录缺标题/Logo 时以装修数据兜底，不覆盖接口实测字段
                                return { ...hit, shopTitle: hit.shopTitle || item.shopTitle, shopLogo: hit.shopLogo || item.shopLogo };
                            }
                            // 聚合未返回（无在售商品等）：按 shopId 补拉门店详情，仍失败则只用原始条目
                            try {
                                const detail: any = await getStoreDetail({ shopId: id });
                                if (detail && typeof detail === "object") {
                                    const address = [...(detail.shopRegionNames ?? []), detail.shopDetailedAddress]
                                        .filter(Boolean)
                                        .join(" ");
                                    return {
                                        shopId: id,
                                        shopTitle: detail.shopTitle || item.shopTitle,
                                        shopLogo: detail.shopLogo || item.shopLogo,
                                        shopStar: detail.shopStar,
                                        address: address || undefined,
                                        productList: []
                                    } as StoreSearchRecord;
                                }
                            } catch {
                                // 补拉失败回落原始条目
                            }
                            return {
                                shopId: id,
                                shopTitle: item.shopTitle,
                                shopLogo: item.shopLogo,
                                productList: []
                            } as StoreSearchRecord;
                        })
                    );
                    if (!cancelled) setStores(merged);
                    return;
                }
                // 自动模式：按 storeNum 拉门店聚合列表（原版编辑器口径钳制 1~20）
                const num = Math.min(Math.max(Math.round(d.storeNum) || 1, 1), 20);
                const res: any = await getStoreProduct({ page: 1, size: num, ...locationParams });
                if (!cancelled) setStores(Array.isArray(res?.records) ? res.records : []);
            } catch {
                if (!cancelled) setStores([]);
            }
        })();
        return () => {
            cancelled = true;
        };
        // storeList 内容变化以 assignIdsKey 为准（顺序/条目由其驱动回排）
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [d.storeSource, d.storeNum, assignIdsKey]);

    if (!stores || stores.length === 0) return null;

    const sts = d.storeTitleStyle;
    const storeTitleStyle: CSSProperties = {
        fontSize: designPxToRpx(sts.titleFontSize ?? 14),
        color: sts.titleColor || "#1a1a1a",
        fontWeight: sts.titleFontBold ?? 700
    };
    const showStoreTitle = Number(sts.showTitle ?? 1) !== 0;
    const showRating = d.showRating === 1;
    const showFollow = d.showFollow === 1;

    const boxBase = (index: number): CSSProperties => ({
        backgroundColor: d.storeBackgroundColor || "#ffffff",
        padding: `${designPxToRpx(d.storePaddingY)} ${designPxToRpx(d.storePaddingX)}`,
        borderRadius: designPxToRpx(d.storeRadius),
        overflow: "hidden",
        ...(index > 0 ? { marginTop: designPxToRpx(d.storeGap) } : {})
    });

    const toStore = (shopId: number) => {
        if (Number.isFinite(shopId)) redirect({ url: `/pages/shop/index?shopId=${shopId}` });
    };

    const toProduct = (e: any, product: StoreSearchProduct, shopId: number) => {
        e?.stopPropagation?.();
        if (product?.productId) redirect({ url: `/pages/product/index?id=${product.productId}&shopId=${shopId}` });
    };

    /** 关注/取消关注（原版 storeSearchCard 同口径：shopCollection 切换 + toast） */
    const handleCollect = async (e: any, store: StoreSearchRecord) => {
        e?.stopPropagation?.();
        const id = Number(store.shopId);
        if (!Number.isFinite(id) || collectPendingRef.current.has(id)) return;
        collectPendingRef.current.add(id);
        try {
            await shopCollection({ shopId: id });
            const next = !(collectOverrides[id] ?? Boolean(store.collectShop));
            setCollectOverrides((prev) => ({ ...prev, [id]: next }));
            Taro.showToast({ title: t(next ? "关注成功" : "取消关注成功"), icon: "none" });
        } catch (err: any) {
            Taro.showToast({ title: err?.message || t("操作失败"), icon: "none" });
        } finally {
            collectPendingRef.current.delete(id);
        }
    };

    const renderStyleOne = () => (
        <View className="store-nav__style-one">
            {Number(d.subTitle.showTitle ?? 1) !== 0 ? (
                <View
                    className="store-nav__header"
                    style={{ padding: `${designPxToRpx(d.titlePaddingY)} ${designPxToRpx(d.titlePaddingX)}` }}
                >
                    <Text
                        className="store-nav__header-title"
                        style={commonStyle.moduleSubTitle.textFontSize + commonStyle.moduleSubTitle.textColor + commonStyle.moduleSubTitle.textBlod}
                    >
                        {t("附近门店")}
                    </Text>
                    <View className="store-nav__header-more">
                        <Text>{t("更多")}</Text>
                        <Text className="store-nav__header-more-arrow">›</Text>
                    </View>
                </View>
            ) : null}
            <View>
                {stores.map((store, index) => {
                    const score = showRating ? formatScore(store.shopStar) : "";
                    const distance = formatDistance(store.distance, hasLocation);
                    return (
                        <View key={store.shopId ?? index} className="store-nav__list-item" style={boxBase(index)} onClick={() => toStore(Number(store.shopId))}>
                            <View className="store-nav__list-logo">
                                {store.shopLogo ? <Image src={imageFormat(store.shopLogo)} mode="aspectFit" /> : null}
                            </View>
                            <View className="store-nav__list-main">
                                {showStoreTitle ? (
                                    <Text className="store-nav__shop-title store-nav__line1" style={storeTitleStyle}>
                                        {store.shopTitle || ""}
                                    </Text>
                                ) : null}
                                {score ? <Text className="store-nav__list-score">{score}</Text> : null}
                                {store.address || distance ? (
                                    <View className="store-nav__list-address-row">
                                        <Text className="store-nav__list-address store-nav__line1">{store.address || ""}</Text>
                                        {distance ? <Text className="store-nav__distance-pill">{distance}</Text> : null}
                                    </View>
                                ) : null}
                            </View>
                        </View>
                    );
                })}
            </View>
        </View>
    );

    const renderStyleTwo = () => (
        <View className="store-nav__style-two">
            {stores.map((store, index) => {
                const score = showRating ? formatScore(store.shopStar) : "";
                const distance = formatDistance(store.distance, hasLocation);
                const isCollect = collectOverrides[Number(store.shopId)] ?? Boolean(store.collectShop);
                const products: StoreSearchProduct[] = Array.isArray(store.productList) ? store.productList : [];
                // 起送/配送费仅在门店开启同城配送时展示（原版 storeSearchCard 口径）
                const deliveryOn = Boolean(store.localDeliveryEnabled);
                const showMinOrder = d.showMinOrder === 1 && deliveryOn && store.minOrderAmount != null;
                const showDeliveryFee = d.showDeliveryFee === 1 && deliveryOn;
                const isFreeDelivery = Number(store.deliveryFee ?? 0) === 0;
                return (
                    <View key={store.shopId ?? index} className="store-nav__card" style={boxBase(index)} onClick={() => toStore(Number(store.shopId))}>
                        <View className="store-nav__card-header">
                            <View className="store-nav__card-logo">
                                {store.shopLogo ? <Image src={imageFormat(store.shopLogo)} mode="aspectFill" /> : null}
                            </View>
                            <View className="store-nav__card-main">
                                <View className="store-nav__title-row">
                                    {showRating ? (
                                        <View
                                            className="store-nav__tag"
                                            style={{ backgroundImage: `url(${staticResource("store/tag-bg.png")})` }}
                                        >
                                            <Text>{t("星级门店")}</Text>
                                        </View>
                                    ) : null}
                                    {showStoreTitle ? (
                                        <Text className="store-nav__shop-title store-nav__line1" style={storeTitleStyle}>
                                            {store.shopTitle || ""}
                                        </Text>
                                    ) : (
                                        <Text className="store-nav__shop-title-flex" />
                                    )}
                                    {showFollow ? (
                                        <View
                                            className={`store-nav__collect-btn${isCollect ? " store-nav__collect-btn--collected" : ""}`}
                                            onClick={(e) => void handleCollect(e, store)}
                                        >
                                            <Text className="store-nav__collect-text">{isCollect ? t("已关注") : t("关注")}</Text>
                                        </View>
                                    ) : null}
                                </View>
                                <View className="store-nav__meta-row">
                                    {score ? <Text className="store-nav__star">{score}{t("分")}</Text> : null}
                                    {showMinOrder ? (
                                        <View className="store-nav__meta-item store-nav__meta-price">
                                            <Text>{t("起送")}</Text>
                                            <FormatPrice
                                                priceData={store.minOrderAmount as any}
                                                fontStyle={{ fontSize: "22rpx", color: "#999" }}
                                                currencyStyle={{ fontSize: "22rpx", color: "#999" }}
                                                decimalsStyle={{ fontSize: "22rpx", color: "#999" }}
                                            />
                                        </View>
                                    ) : null}
                                    {showDeliveryFee ? (
                                        isFreeDelivery ? (
                                            <Text className="store-nav__meta-item">{t("免配送费")}</Text>
                                        ) : (
                                            <View className="store-nav__meta-item store-nav__meta-price">
                                                <Text>{t("配送费")}</Text>
                                                <FormatPrice
                                                    priceData={store.deliveryFee as any}
                                                    fontStyle={{ fontSize: "22rpx", color: "#999" }}
                                                    currencyStyle={{ fontSize: "22rpx", color: "#999" }}
                                                    decimalsStyle={{ fontSize: "22rpx", color: "#999" }}
                                                />
                                            </View>
                                        )
                                    ) : null}
                                    {d.showDistance === 1 && distance ? (
                                        <Text className="store-nav__meta-item">
                                            {t("距离")} {distance}
                                        </Text>
                                    ) : null}
                                </View>
                            </View>
                        </View>
                        {products.length > 0 ? (
                            <ScrollView className="store-nav__product-scroll" scrollX>
                                <View className="store-nav__product-row" style={{ columnGap: designPxToRpx(d.productGap) }}>
                                    {products.map((item) => (
                                        <View key={item.productId} className="store-nav__product-item" onClick={(e) => toProduct(e, item, Number(store.shopId))}>
                                            <View className="store-nav__product-image">
                                                <Image src={productImage(item as any)} mode="aspectFill" />
                                            </View>
                                            {d.showProductName === 1 ? (
                                                <Text className={`store-nav__product-name ${d.productNameRow === 2 ? "store-nav__line2" : "store-nav__line1"}`}>
                                                    {item.productName}
                                                </Text>
                                            ) : null}
                                            {d.showProductPrice === 1 ? (
                                                <View className="store-nav__product-price">
                                                    <FormatPrice
                                                        priceData={(item.price ?? item.productPrice) as any}
                                                        fontStyle={{ fontSize: "26rpx", fontWeight: "bold", color: "#ff4a1e" }}
                                                        currencyStyle={{ fontSize: "20rpx", fontWeight: "bold", color: "#ff4a1e" }}
                                                        decimalsStyle={{ fontSize: "22rpx", fontWeight: "bold", color: "#ff4a1e" }}
                                                    />
                                                </View>
                                            ) : null}
                                        </View>
                                    ))}
                                </View>
                            </ScrollView>
                        ) : null}
                    </View>
                );
            })}
        </View>
    );

    return (
        <View
            className="module-ad-con module-store-nav"
            style={commonStyle.moduleStyle.boxPaddingTopBottom + commonStyle.moduleStyle.boxPadding + commonStyle.moduleStyle.backgroundColor}
        >
            <View
                className="module-ad-content"
                style={
                    commonStyle.moduleContentStyle.backgroundColor +
                    commonStyle.moduleContentStyle.backgroundImage +
                    commonStyle.moduleContentStyle.backgroundSize +
                    commonStyle.moduleContentStyle.boxRadius2 +
                    commonStyle.moduleContentStyle.boxShadow +
                    commonStyle.moduleContentStyle.paddingTopBottom +
                    commonStyle.moduleContentStyle.paddingLeftRight
                }
            >
                {d.storeStyle === 2 ? renderStyleTwo() : renderStyleOne()}
            </View>
        </View>
    );
}
