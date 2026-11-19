import { useMemo, useRef } from "react";
import { View, Text, Image } from "@tarojs/components";

import { getCategoryProduct, getStoreProduct } from "@/api/search/search";
import useList from "@/hooks/useList";
import { useProductStyle } from "@/hooks";
import { mergeConfig } from "@/components/modules/search-result/config";
import type { SearchResultModuleData } from "@/components/modules/search-result/types";
import MainHeader from "@/components/mainHeader";
import ProductItem from "@/components/masonry/ProductItem";
import StoreSearchCard from "@/components/storeSearchCard";
import { staticResource } from "@/utils";
import { t } from "@/utils/i18n";

import HeaderDomestic from "@/pages/search/components/search-result/HeaderDomestic";
import HeaderOverseas from "@/pages/search/components/search-result/header-overseas";
import SearchProductList from "@/pages/search/components/search-result/List";
import "@/pages/search/result.scss";

/**
 * 搜索页（pageType=6）整页预览分支（2026-11-19）。
 *
 * 与真实搜索结果页（pages/search/result.tsx）同一呈现路径：编辑器下发的
 * mobileSearchResult module 经前台同一 mergeConfig 合成配置，头部 / 列表 /
 * 门店卡全部复用结果页自己的子组件，商品与门店走结果页同一接口取真实数据。
 * 结果页本体是整页组件（路由参数 + TigLayout + fixed 头 + 筛选抽屉），无法在
 * 嵌入预览里独立挂载，故按其渲染结构用子组件拼装（见报告差距说明）。
 *
 * 样例关键词固定「电池」：admin 兼容渲染 Preview 的约定是「卫衣」+mock 数据，
 * 但真实渲染走真实搜索，本站商品库无「卫衣」结果（恒空态、版式无从验收）；
 * 「电池」为本站在售商品的真实高频词（实测 19 条），故改口径（我方改口径）。
 * （admin Preview 写死演示关键词）；区别是 admin 侧商品为静态 mock、本分支
 * 取真实搜索结果（关键词相同、数据为真）。
 *
 * 已知继承的线上现状（非本分支引入）：结果页国内版排序行 FilterDomestic、
 * 海外版头部 HeaderOverseas、门店卡 StoreSearchCard 在当前 H5 移植中均为
 * 未移植的 null 桩，真实搜索页同样不渲染——预览与线上一致，不另造形态。
 */

/** 样例关键词：与 admin mobileSearchResult Preview 的演示关键词一致 */
export const SEARCH_PREVIEW_KEYWORD = "电池";

/** 后台按 px 配置，uniapp 按 rpx 渲染，换算比例固定 1:2（同 result.tsx） */
const toRpx = (px: number | undefined, fallback: number) => `${(px ?? fallback) * 2}rpx`;

export default function SearchConfigPreview({ module }: { module: any }) {
    const config: SearchResultModuleData = useMemo(() => mergeConfig(module ?? {}), [module]);
    const isStoreStyle = Number(config.searchResultType) === 2;
    /** 布局样式：1-国内版，2-海外版；门店聚合搜索固定国内版搜索头（同 result.tsx） */
    const layoutStyle = isStoreStyle ? 1 : Number(config.layoutStyle ?? 1);
    const listStyle = Number(config.listStyle ?? 1);

    // 与 result.tsx 同构的请求参数（useList 会原地维护 page/size，须用 ref 持有同一对象）
    const filterParams = useRef({
        page: 1,
        size: 10,
        sort: "",
        order: "asc",
        keyword: SEARCH_PREVIEW_KEYWORD,
        max: "",
        min: "",
        cat: 0,
        couponId: 0,
        brand: [] as any[],
        attrs: [] as any[],
        intro: "",
        pageType: "search"
    }).current;

    const {
        data: productList,
        promotionList,
        total,
        isLoading
    } = useList(getCategoryProduct as any, {
        params: filterParams,
        path: { dataKey: "records" },
        needPromotionList: true,
        needReachBottom: false,
        immediate: !isStoreStyle
    });

    const storeParams = useRef({ page: 1, size: 5 }).current;
    const { data: storeList, isLoading: storeIsLoading } = useList(getStoreProduct as any, {
        params: storeParams,
        path: { dataKey: "records" },
        needReachBottom: false,
        immediate: isStoreStyle
    });

    const { styleMap } = useProductStyle(productList);

    // ===== 样式计算（自 result.tsx 同名 computed 移植，hasDecorate 恒真：配置由编辑器实时下发） =====
    const waiting = isStoreStyle ? storeIsLoading : isLoading;
    const themeVars = (() => {
        const style: Record<string, string> = {
            "--search-theme": config.themeColor || "var(--general)",
            "--search-price-color": config.priceColor || config.themeColor || "var(--general)",
            "--search-item-bg": config.searchItemBackgroundColor || "#ffffff",
            "--search-item-padding-lr": toRpx(config.searchItemPaddingLeftRight, 10)
        };
        if (!waiting && config.backgroundColor) style.backgroundColor = config.backgroundColor;
        return style;
    })();

    const goodsContainerStyle = {
        margin: toRpx(config.margin, 0),
        padding: `${toRpx(config.paddingTopBottom ?? config.padding, 10)} ${toRpx(config.paddingLeftRight ?? config.padding, 10)}`,
        borderRadius: toRpx(config.moduleRadius, 0),
        backgroundColor: config.backgroundColor || ""
    };

    const storeContainerStyle = {
        margin: toRpx(config.margin, 0),
        padding: `${toRpx(config.paddingTopBottom ?? config.padding, 10)} ${toRpx(config.paddingLeftRight ?? config.padding, 10)}`,
        rowGap: toRpx(config.storeGap, 10)
    };

    /** 透传给通用瀑布流的装修字段（同 result.tsx listDecorate） */
    const listDecorate = {
        productTitleLine: config.productTitleLine,
        buyBtnStyle: config.buyBtnStyle,
        showCoupon: config.showCoupon,
        priceColor: config.priceColor,
        productTitleColor: config.productTitleColor,
        productImageRadius: config.productImageRadius,
        itemBackgroundColor: config.itemBackgroundColor,
        themeColor: config.themeColor
    };

    /** 透传给门店卡片的装修字段（同 result.tsx storeDecorate） */
    const storeDecorate = {
        showRating: config.storeShowRating,
        showMinOrder: config.storeShowMinOrder,
        showDeliveryFee: config.storeShowDeliveryFee,
        showDistance: config.storeShowDistance,
        showFollow: config.storeShowFollow,
        showProductName: config.storeShowProductName,
        productNameRow: config.storeProductNameRow,
        showProductPrice: config.storeShowProductPrice,
        productGap: config.storeProductGap,
        shopLogoRadius: config.storeLogoRadius,
        cardRadius: config.storeCardRadius,
        cardPaddingTopBottom: config.storePaddingTopBottom,
        cardPaddingLeftRight: config.storePaddingLeftRight,
        shopTitleColor: config.storeShopTitleColor,
        shopTitleFontSize: config.storeShopTitleFontSize,
        shopTitleFontBold: config.storeShopTitleFontBold,
        itemBackgroundColor: config.itemBackgroundColor,
        themeColor: config.themeColor,
        productImageRadius: config.productImageRadius
    };

    const evenColumn = productList.filter((_: any, index: number) => index % 2 === 0);
    const oddColumn = productList.filter((_: any, index: number) => index % 2 === 1);
    const renderMasonryColumn = (list: any[]) =>
        list.map((item: any) => (
            <ProductItem
                key={item.productId}
                item={item}
                promotionList={promotionList}
                styleMap={styleMap}
                productTitleLine={listDecorate.productTitleLine}
                buyBtnStyle={listDecorate.buyBtnStyle}
                showCoupon={listDecorate.showCoupon}
                priceColor={listDecorate.priceColor}
                productTitleColor={listDecorate.productTitleColor}
                productImageRadius={listDecorate.productImageRadius}
                itemBackgroundColor={listDecorate.itemBackgroundColor}
                themeColor={listDecorate.themeColor}
                onClickItem={() => {}}
                onClickBuy={() => {}}
            />
        ));

    return (
        <View className="pageMain productSort mobile-search-results" style={themeVars as any}>
            {layoutStyle === 2 ? (
                <>
                    <MainHeader position="sticky" />
                    <HeaderOverseas keyword={SEARCH_PREVIEW_KEYWORD} scrollLeft={0} onToSearch={() => {}} />
                </>
            ) : (
                <HeaderDomestic
                    keyword={SEARCH_PREVIEW_KEYWORD}
                    filterParams={filterParams as any}
                    showImageSearch={!isStoreStyle}
                    onToSearch={() => {}}
                    onBack={() => {}}
                />
            )}

            {isStoreStyle ? (
                <View className="store-container" style={storeContainerStyle as any}>
                    {!storeIsLoading &&
                        storeList.length > 0 &&
                        storeList.map((item: any) => (
                            <StoreSearchCard
                                key={item.shopId}
                                store={item}
                                manageListGap
                                hasLocation={false}
                                showRating={storeDecorate.showRating}
                                showMinOrder={storeDecorate.showMinOrder}
                                showDeliveryFee={storeDecorate.showDeliveryFee}
                                showDistance={storeDecorate.showDistance}
                                showFollow={storeDecorate.showFollow}
                                showProductName={storeDecorate.showProductName}
                                productNameRow={storeDecorate.productNameRow}
                                showProductPrice={storeDecorate.showProductPrice}
                                productGap={storeDecorate.productGap}
                                shopTitleColor={storeDecorate.shopTitleColor}
                                shopLogoRadius={storeDecorate.shopLogoRadius}
                                cardRadius={storeDecorate.cardRadius}
                                cardPaddingTopBottom={storeDecorate.cardPaddingTopBottom}
                                cardPaddingLeftRight={storeDecorate.cardPaddingLeftRight}
                                shopTitleFontSize={storeDecorate.shopTitleFontSize}
                                shopTitleFontBold={storeDecorate.shopTitleFontBold}
                                itemBackgroundColor={storeDecorate.itemBackgroundColor}
                                themeColor={storeDecorate.themeColor}
                                productImageRadius={storeDecorate.productImageRadius}
                                applyCardStyle
                            />
                        ))}
                    {!storeIsLoading && storeList.length === 0 && (
                        <View className="empty-container">
                            <Image className="empty-image" src={staticResource("empty/emptyProduct.png")} mode="aspectFit" />
                            <Text>{t("没有找到相关的门店")}</Text>
                        </View>
                    )}
                    {storeIsLoading && <View className="decorate-placeholder" />}
                </View>
            ) : (
                <>
                    {!isLoading && total > 0 && (
                        <View className="goods-container" style={goodsContainerStyle as any}>
                            {listStyle === 2 ? (
                                <SearchProductList productList={productList} promotionList={promotionList} config={config} />
                            ) : (
                                <View className="masonry-two-col">
                                    <View className="masonry-col">{renderMasonryColumn(evenColumn)}</View>
                                    <View className="masonry-col">{renderMasonryColumn(oddColumn)}</View>
                                </View>
                            )}
                        </View>
                    )}
                    {!isLoading && total === 0 && (
                        <View className="empty-container">
                            <Image className="empty-image" src={staticResource("empty/emptyProduct.png")} mode="aspectFit" />
                            <Text>{t("没有找到相关的商品")}</Text>
                        </View>
                    )}
                    {isLoading && <View className="decorate-placeholder" />}
                </>
            )}
        </View>
    );
}
