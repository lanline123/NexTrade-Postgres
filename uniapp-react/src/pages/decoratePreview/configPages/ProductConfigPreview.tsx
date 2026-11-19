import { useEffect, useMemo, useState } from "react";
import { View, Text } from "@tarojs/components";

import { getCategoryProduct } from "@/api/search/search";
import { useConfigStore } from "@/store/config";
import { isMerchant } from "@/utils";
import { t } from "@/utils/i18n";

import { useProductDetail, useProductStyle } from "@/pages/product/composables";
import ProductImg from "@/pages/product/src/productImg";
import ProductImgDecorate from "@/pages/product/src/productImgDecorate";
import ProductTitleInfo from "@/pages/product/src/productTitleInfo";
import ServiceCard from "@/pages/product/src/serviceCard";
import MerchantShopInfo from "@/pages/product/src/merchantShopInfo";
import AttrCard from "@/pages/product/src/attrCard";
import EvaluateCard from "@/pages/product/src/evaluateCard";
import ProductComment from "@/pages/product/src/productComment";
import AfterSaleCard from "@/pages/product/src/afterSaleCard";
import ProductDescription from "@/pages/product/src/productDescription";
import DescriptionCard from "@/pages/product/src/descriptionCard";
import ProductContent from "@/pages/product/src/productContent";
import ProductActionBar from "@/pages/product/components/ProductActionBar";
import "@/pages/product/index.scss";

/**
 * 商品详情页（pageType=8）整页预览分支（2026-11-19）。
 *
 * 与真实商品页（pages/product/index.tsx）同一呈现路径：编辑器下发的
 * productDetailA1/A2/A3 module 作装修配置（type 决定 A 版式，口径同详情页
 * fetchDecorateConfig 的 decorateStyle 回填），商品本体用详情页自己的数据
 * Hook（useProductDetail）与子组件树（主图 / 标题价格 / 服务 / 店铺 / 规格 /
 * 评价 / 售后 / 详情 / 底部购买栏）拼装。详情页本体是整页组件（路由商品 id、
 * 页面生命周期、分享、规格弹窗、TigLayout 页壳），无法在嵌入预览里独立
 * 挂载，故按其渲染结构拼装（见报告差距说明）。
 *
 * 样例商品口径（确定性）：商品列表接口（与搜索页同一 getCategoryProduct）
 * 默认排序取第一条在售商品，再走详情接口取全量详情；取不到商品时渲染
 * 明确占位而非空白。admin 兼容渲染 Preview 的样例是写死演示数据（竖条纹
 * 衬衫），本分支取真实商品，口径差异见报告。
 */

/** 装修字段归一（同详情页：布尔兼容 "false"/0/"0"，数字兼容字符串） */
const toDecorateBool = (val: unknown) => {
    if (val === undefined || val === null || val === "") return undefined;
    return !(val === false || val === "false" || val === 0 || val === "0");
};
const toDecorateNumber = (val: unknown) => {
    if (val === undefined || val === null || val === "") return undefined;
    return Number(val);
};

export default function ProductConfigPreview({ module, type }: { module: any; type: string }) {
    const config = (module ?? {}) as Record<string, any>;
    // 接口/编辑器未回 decorateStyle 时按模块 type 回填（同详情页 fetchDecorateConfig）
    const decorateStyle = config.decorateStyle || (type === "productDetailA1" ? "A1" : type === "productDetailA2" ? "A2" : type === "productDetailA3" ? "A3" : undefined);
    const { productStyle, isVortmallStyle, isOverseasStyle } = useProductStyle({ decorateStyle, moduleType: type });
    /** A1 / A3 使用装修版主图，A2 使用全宽主图（同详情页） */
    const useDecorateImg = isVortmallStyle || isOverseasStyle;

    const productCommentEnabled = useConfigStore((s) => s.productCommentEnabled);
    const showSelledCount = useConfigStore((s) => s.showSelledCount);

    // ===== 样例商品：列表默认排序第一条在售商品 =====
    const [sampleId, setSampleId] = useState(0);
    const [sampleEmpty, setSampleEmpty] = useState(false);
    useEffect(() => {
        let alive = true;
        void (async () => {
            try {
                // 参数口径对齐搜索结果页的实际请求：brand 为逗号拼接字符串
                // （传数组后端反序列化 400）；max/min 空串同样 400，故不传。
                const res: any = await getCategoryProduct({
                    page: 1,
                    size: 1,
                    sort: "",
                    order: "asc",
                    keyword: "",
                    cat: 0,
                    couponId: 0,
                    brand: "",
                    intro: "",
                    pageType: "search"
                } as any);
                if (!alive) return;
                const first = (res?.records ?? [])[0];
                if (first?.productId) setSampleId(Number(first.productId));
                else setSampleEmpty(true);
            } catch {
                if (alive) setSampleEmpty(true);
            }
        })();
        return () => {
            alive = false;
        };
    }, []);

    const detail = useProductDetail({});
    const { product, picList, videoList, attrList, serviceList, descArr, productPrice, productStock, promotionList, shopInfo, shopId, activityBgImage, detailLoaded, failLoad } = detail;
    useEffect(() => {
        if (sampleId > 0) void detail.getProductDetail(sampleId);
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [sampleId]);

    const [descShowMore, setDescShowMore] = useState(false);

    // 页面底色 / 内容底色 / 标题与价格色变量（同详情页 pageStyle）
    const pageStyle = useMemo<Record<string, string>>(() => {
        const configuredPageBackground = config.backgroundColor as string | undefined;
        const pageBackground =
            productStyle === "A2" && (!configuredPageBackground || configuredPageBackground === "transparent") ? "#f5f5f5" : configuredPageBackground;
        const contentBackground = config.contentBackgroundColor || "#ffffff";
        const fallbackPageBackground = isVortmallStyle ? "#f7f7f7" : "#f5f5f5";
        const style: Record<string, string> = {
            "--product-page-bg": pageBackground || fallbackPageBackground,
            "--product-content-bg": contentBackground,
            "--product-title-color": config.titleColor || "#1a1a1a",
            "--product-price-color": config.priceColor || "#2a3145",
            "--product-market-price-color": config.marketPriceColor || "#999"
        };
        return pageBackground ? { ...style, backgroundColor: pageBackground } : style;
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [config, productStyle, isVortmallStyle]);

    /** 底栏装修配置（同详情页 actionBarDecorate） */
    const actionBarDecorate = useMemo(
        () => ({
            buyBarStyle: toDecorateNumber(config.buyBarStyle) as 1 | 2 | undefined,
            buttonRadius: toDecorateNumber(config.buttonRadius),
            showHome: toDecorateBool(config.showHome),
            showService: toDecorateBool(config.showService),
            showCart: toDecorateBool(config.showCart)
        }),
        // eslint-disable-next-line react-hooks/exhaustive-deps
        [config]
    );

    const showMerchantShop =
        isMerchant() &&
        toDecorateBool(config.showShop) !== false &&
        !!shopInfo &&
        Object.keys(shopInfo).length > 0 &&
        Number.isInteger(shopInfo.shopId) &&
        shopInfo.shopId > 0;

    if (sampleEmpty) {
        return (
            <View className="decorate-preview__config-note">
                <Text>{t("暂无可预览的在售商品")}</Text>
            </View>
        );
    }
    if (failLoad) {
        return (
            <View className="decorate-preview__config-note">
                <Text>{t("样例商品加载失败")}</Text>
            </View>
        );
    }
    if (!detailLoaded) {
        return (
            <View className="decorate-preview__config-note">
                <Text>{t("正在加载样例商品…")}</Text>
            </View>
        );
    }

    return (
        <View className={`product-detail-page product-detail-page--${productStyle}`} style={pageStyle as any}>
            {/* 跨境风格（A3）：标题与已售置于主图上方（同详情页 overseas-head，预览从简：不做展开/预售态） */}
            {isOverseasStyle && (
                <View className="overseas-head">
                    <View className="overseas-head__name">
                        <View className="overseas-head__name-inner">{product.productName}</View>
                    </View>
                    {!!showSelledCount && (
                        <View className="overseas-head__sold">
                            {t("已售")}
                            {product.virtualSales || 0}
                            {t("件")}
                        </View>
                    )}
                </View>
            )}

            <View className="product-area-wrapper">
                <View className="product-area">
                    {useDecorateImg ? (
                        <ProductImgDecorate
                            videoList={videoList}
                            picList={picList}
                            productInfo={product}
                            productStock={productStock}
                            detailLoaded={detailLoaded}
                            imageRatio={config.imageRatio}
                            imageMargin={config.imageMargin}
                            imageMode={config.imageMode}
                            imageRadiusStyle={config.imageRadiusStyle}
                        />
                    ) : (
                        <ProductImg
                            videoList={videoList}
                            picList={picList}
                            productInfo={product}
                            productStock={productStock}
                            detailLoaded={detailLoaded}
                            imageRatio={config.imageRatio}
                            imageMargin={config.imageMargin}
                            imageMode={config.imageMode}
                            imageRadiusStyle={config.imageRadiusStyle}
                        />
                    )}
                </View>
            </View>

            <View className={`productDetail-content ${isVortmallStyle ? "productDetail-content--vortmall" : ""} ${isOverseasStyle ? "productDetail-content--overseas" : ""}`}>
                <ProductTitleInfo
                    layoutStyle={productStyle}
                    titleColor={config.titleColor || ""}
                    priceColor={config.priceColor || ""}
                    marketPriceColor={config.marketPriceColor || ""}
                    titleSize={config.titleSize || ""}
                    titleMaxLines={config.titleMaxLines || 0}
                    promotionList={promotionList}
                    productInfo={product}
                    productPrice={productPrice}
                    productStock={productStock}
                    activityBgImage={activityBgImage}
                    soldCount={product.virtualSales || 0}
                />

                {isVortmallStyle && serviceList.length > 0 && <ServiceCard serviceList={serviceList} />}

                {showMerchantShop && (
                    <MerchantShopInfo
                        shopInfo={shopInfo}
                        variant={isVortmallStyle ? "vortmall" : isOverseasStyle ? "overseas" : "o2o"}
                        onRefreshShopDetail={(id) => void detail.getShopInfo(id)}
                    />
                )}

                {(isVortmallStyle || isOverseasStyle) && attrList.normal?.length > 0 && (
                    <View className="attr-card-box">
                        <AttrCard normal={attrList.normal} overseas={isOverseasStyle} />
                    </View>
                )}

                {productCommentEnabled === 1 ? (
                    isVortmallStyle || isOverseasStyle ? (
                        <View className="evaluate-area">
                            <EvaluateCard productId={product.productId} overseas={isOverseasStyle} />
                        </View>
                    ) : (
                        <View className="evaluate-area">
                            <ProductComment productId={product.productId} shopId={shopId > -1 ? shopId : undefined} />
                        </View>
                    )
                ) : null}

                {isOverseasStyle ? (
                    <>
                        <AfterSaleCard id={product.productId} overseas={true} />
                        <ProductDescription product={product} overseas={true} />
                        <View className="product-detail-area">
                            <DescriptionCard
                                modelValue={descShowMore}
                                onUpdateModelValue={setDescShowMore}
                                isBuy={product.isBuy}
                                paidContent={product.paidContent as any}
                                productType={product.productType}
                                descArr={descArr as any}
                                productId={product.productId}
                                overseas={true}
                            />
                        </View>
                    </>
                ) : isVortmallStyle ? (
                    <View className="product-detail-area">
                        <DescriptionCard
                            modelValue={descShowMore}
                            onUpdateModelValue={setDescShowMore}
                            isBuy={product.isBuy}
                            paidContent={product.paidContent as any}
                            productType={product.productType}
                            descArr={descArr as any}
                            productId={product.productId}
                        />
                    </View>
                ) : (
                    <View className="product-detail-area">
                        <ProductContent
                            isBuy={product.isBuy}
                            paidContent={product.paidContent as any}
                            productType={product.productType}
                            descArr={descArr as any}
                            productId={product.productId}
                        />
                    </View>
                )}
            </View>

            {/* 底部购买栏：fixed 定位在预览 scss 中改回文档流（见 decoratePreview/index.scss） */}
            <ProductActionBar
                product={product}
                shopInfo={shopInfo}
                productStock={productStock}
                type=""
                detailLoaded={detailLoaded}
                cartCount={0}
                productId={String(product.productId ?? "")}
                layoutStyle={productStyle}
                decorateConfig={actionBarDecorate as any}
                onShowSpecification={() => {}}
                onToHome={() => {}}
                onToProductList={() => {}}
                onToCart={() => {}}
            />
        </View>
    );
}
