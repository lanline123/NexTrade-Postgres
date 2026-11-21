/**
 * 商品分组 预览（移植自 view/admin editor/modules/mobilePro/productGrouping/Preview.vue）
 *
 * 选项卡（productGroups 手动组 / groupList 分组组）+ 当前组商品网格 + 底部查看更多。
 * 手动组取数同原版：组内演示项（productId===0）+ 按 productList 的 ids 拉取真实商品；
 * 分组组按原版参数 {productGroupId, size} 调同一商品列表接口（useDecorateProductList
 * 不支持 productGroupId 参数，故本模块以 apiGet 等价承接，接口路径不变）。
 * 原版 Preview 的 currentIndex 无点击入口恒为 0、激活色亦按 index===0 写死；本实现
 * 允许点选切换分组，激活样式随当前组（初始同为第 1 组，初始渲染与原版一致）。
 * 当前组配置了 bannerPic 时，首张卡为海报填充卡（原版 __isBanner 口径）。
 */
import { useEffect, useMemo, useState } from "react";
import { cssText, productImage } from "../../../shared/format";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { mergeDefaultModule, formatCommonStyle, gradientMap } from "../../../contract/sharedDefaults";
import BuyButton, { useResolvedBuyButton } from "../../../shared/BuyButton";
import { apiGet } from "../../../../../api";
import { productGroupingDefaults } from "./index";
import "./Preview.css";

const picScaleMap: Record<string, string> = {
    "3:2": "calc(100% / 3 * 2)",
    "1:1": "calc(100% / 1 * 1)",
    "3:4": "calc(100% / 3 * 4)",
    "16:9": "calc(100% / 16 * 9)"
};

export default function ProductGroupingPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, productGroupingDefaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const gt = module.groupTitle ?? {};
    const pc = module.productColor ?? {};

    const isGroupSource = module.productSource === "productGroup";
    // 全部商品（我方新增）：不分选项卡，仅借首组的标题/横幅外观，商品全站自动取数
    const isAllSource = module.productSource === "all";
    const tabList: Array<Record<string, any>> = (isGroupSource ? module.groupList : isAllSource ? (module.productGroups ?? []).slice(0, 1) : module.productGroups) ?? [];
    const [currentIndex, setCurrentIndex] = useState(0);
    const safeIndex = Math.min(currentIndex, Math.max(0, tabList.length - 1));
    const activeGroup = tabList[safeIndex] ?? null;

    // 手动组：按 ids 拉取
    const isManualSource = !isGroupSource && !isAllSource;
    const manualIds = isManualSource && activeGroup ? (activeGroup.productList ?? []).map((i: any) => i.productId).filter(Boolean) : [];
    const manualFetched = useDecorateProductList({ productSelectType: manualIds.length ? 1 : 0, productIds: manualIds });
    const manualDemo = isManualSource && activeGroup ? (activeGroup.productList ?? []).filter((i: any) => i.productId === 0) : [];

    // 全部商品：无过滤拉取（size 口径同店面渲染端）
    const [allFetched, setAllFetched] = useState<Array<Record<string, any>>>([]);
    const allSize = Number(module.productNum) > 0 ? Number(module.productNum) : 100;
    useEffect(() => {
        if (!isAllSource) {
            setAllFetched([]);
            return;
        }
        let alive = true;
        apiGet<{ records?: Array<Record<string, any>>; list?: Array<Record<string, any>> }>("/product/product/list", {
            size: allSize
        })
            .then((result) => {
                if (alive) setAllFetched(result.records || result.list || []);
            })
            .catch(() => {
                if (alive) setAllFetched([]);
            });
        return () => {
            alive = false;
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [isAllSource, allSize]);

    // 分组组：按 productGroupId 拉取
    const [groupFetched, setGroupFetched] = useState<Array<Record<string, any>>>([]);
    const groupKey = isGroupSource && activeGroup ? `${activeGroup.productGroupId ?? ""}:${activeGroup.productNumType === "number" ? activeGroup.productNum : 100}` : "";
    useEffect(() => {
        if (!isGroupSource || !activeGroup?.productGroupId) {
            setGroupFetched([]);
            return;
        }
        let alive = true;
        apiGet<{ records?: Array<Record<string, any>>; list?: Array<Record<string, any>> }>("/product/product/list", {
            productGroupId: activeGroup.productGroupId,
            size: activeGroup.productNumType === "number" ? activeGroup.productNum : 100
        })
            .then((result) => {
                if (alive) setGroupFetched(result.records || result.list || []);
            })
            .catch(() => {
                if (alive) setGroupFetched([]);
            });
        return () => {
            alive = false;
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [groupKey]);

    const list = isAllSource ? allFetched : isGroupSource ? groupFetched : [...manualDemo, ...manualFetched];

    const activeBannerPic = activeGroup?.bannerPic && (activeGroup.bannerPic.picThumb || activeGroup.bannerPic.picUrl) ? activeGroup.bannerPic : null;
    const displayList = useMemo(() => {
        if (!activeBannerPic) return list;
        const placeholder = list[0] ?? { productName: "_", productPrice: "0", marketPrice: "0" };
        return [
            {
                __isBanner: true,
                picUrl: activeBannerPic.picUrl || activeBannerPic.picThumb,
                picThumb: activeBannerPic.picThumb || activeBannerPic.picUrl,
                productName: placeholder.productName,
                productPrice: placeholder.productPrice,
                marketPrice: placeholder.marketPrice
            },
            ...list
        ];
    }, [list, activeBannerPic]);

    // 版式判断（文字是否换行）按生效按钮值走：跟随全局时 text 来自全局 objStyle，
    // 与 BuyButton 本体的解析同口径。
    const resolvedBuyBtn = useResolvedBuyButton(module.buyBtnStyle);
    const hasBuyBtnText = Number(resolvedBuyBtn?.showBtn) === 1 && !!resolvedBuyBtn?.text && Number(resolvedBuyBtn?.wrap) === 1;

    const goodsItemStyle = (() => {
        let str = `margin:${(module.cardMargin ?? 0) / 2}px;padding:${module.cardPadding}px;border:${Number(module.productCardStyle) === 3 ? 1 : 0}px solid rgb(238, 238, 238);border-radius:${module.cardRadius}px;`;
        if (Number(module.productCardStyle) !== 4) str += `background-color:${pc.backgroundColor};`;
        if (Number(module.productCardStyle) === 2) str += `box-shadow:rgba(93, 113, 127, 0.08) 0px 2px 8px;`;
        return str;
    })();

    const tabTitleStyle = (index: number) =>
        cssText(`color:${gt.titleColor};`, `font-size:${gt.bigTitleFontSize}px;`, index === safeIndex ? `color:${gt.activeTitleColor};` : "");
    const tabSubStyle = (index: number) =>
        cssText(
            `color:${gt.subTitleColor};`,
            `font-size:${gt.titleFontSize}px;`,
            `border-radius:${gt.subTitleRadius}px;`,
            index === safeIndex
                ? `color:${gt.activeSubTitleColor};background-image:linear-gradient(${gradientMap[gt.subTitleGradientType] ?? gradientMap.diagonal}, ${gt.gradientColorA}, ${gt.gradientColorB});`
                : ""
        );

    const pricePicUrl = typeof module.productPricePic === "string" ? module.productPricePic : module.productPricePic?.picUrl;

    return (
        <div
            className="module-ad-con module-product_grouping"
            style={cssText(
                commonStyle.moduleStyle.boxPaddingBottom,
                commonStyle.moduleStyle.boxPaddingTop,
                commonStyle.moduleStyle.boxPadding,
                commonStyle.moduleStyle.backgroundSize,
                commonStyle.moduleStyle.backgroundImage
            )}
        >
            <div
                className="module-ad-content"
                style={cssText(
                    commonStyle.moduleContentStyle.backgroundImage,
                    commonStyle.moduleContentStyle.backgroundSize,
                    commonStyle.moduleContentStyle.boxRadius,
                    commonStyle.moduleContentStyle.boxShadow,
                    commonStyle.moduleContentStyle.innerPadding
                )}
            >
                <div className="pg-productgrouping">
                    {Number(gt.showTitle) === 1 || Number(gt.showSubTitle) === 1 ? (
                        <div className="pg-tablist">
                            {tabList.map((item, index) => (
                                <div className="pg-tablist-item" key={index} onClick={() => setCurrentIndex(index)}>
                                    <div className="pg-tab-text">
                                        <div className="pg-nav-item">
                                            {index > 0 ? <div className="pg-line" style={cssText(`border-color:${gt.splitLineColor};`)} /> : null}
                                            <div className="pg-nav-item-box" style={cssText(`padding:0px ${gt.marginLeftRight ?? 0}px;`)}>
                                                {Number(gt.showTitle) === 1 ? (
                                                    <div className="pg-tab-title" style={tabTitleStyle(index)}>{item.title}</div>
                                                ) : null}
                                                {Number(gt.showSubTitle) === 1 && item.subTitle ? (
                                                    <div className="pg-tab-sub" style={tabSubStyle(index)}>{item.subTitle}</div>
                                                ) : null}
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            ))}
                        </div>
                    ) : null}

                    <div className="pg-productlist">
                        <div className="pg-productlist-item">
                            <div style={{ marginTop: 5 }}>
                                <div className="pg-item-box">
                                    {displayList.map((item: any, index: number) => {
                                        const widthPct = Number(module.listStyle) < 5 ? 100 / Number(module.listStyle) : index % 3 === 0 ? 100 : 50;
                                        return (
                                            <div className="pg-li" key={index} style={cssText("opacity:1;", `width:${widthPct}%;`)}>
                                                <div className="pg-goods-item" style={cssText(goodsItemStyle, item.__isBanner ? "position:relative;" : "")}>
                                                    {item.__isBanner ? (
                                                        <div
                                                            className="pg-banner-fill"
                                                            style={cssText(
                                                                "position:absolute;top:0;right:0;bottom:0;left:0;",
                                                                `background-image:url(${productImage(item)});`,
                                                                `background-size:${module.picFillType};`,
                                                                "background-position:center;",
                                                                "background-repeat:no-repeat;",
                                                                `border-radius:${module.cardRadius}px;`,
                                                                "z-index:1;"
                                                            )}
                                                        />
                                                    ) : null}
                                                    <div className="pg-goods-img" style={cssText(`padding-top:${picScaleMap[module.picScale] ?? picScaleMap["1:1"]};`)}>
                                                        {!item.__isBanner ? (
                                                            <div
                                                                className="pg-img"
                                                                style={cssText(
                                                                    `border-radius:${module.picBottomRadius ? `${module.cardRadius}px` : `${module.cardRadius}px ${module.cardRadius}px 0 0`};`,
                                                                    `background-size:${module.picFillType};`,
                                                                    `background-image:url(${productImage(item)});`
                                                                )}
                                                            />
                                                        ) : null}
                                                    </div>
                                                    <div
                                                        className="pg-goods-info"
                                                        style={cssText(
                                                            "text-align:left;",
                                                            `padding:${pc.infoTopPadding}px ${pc.infoLeftRightPadding}px ${pc.infoBottomPadding}px;`,
                                                            item.__isBanner ? "visibility:hidden;" : ""
                                                        )}
                                                    >
                                                        {Number(module.showProductName) === 1 ? (
                                                            <div
                                                                className="pg-goods-title"
                                                                style={cssText(
                                                                    "margin-top:3px;",
                                                                    `height:${Number(module.productNameRow) * 18}px;`,
                                                                    "line-height:18px;",
                                                                    `font-weight:${module.productNameWeight};`,
                                                                    `color:${pc.titleColor};`,
                                                                    `font-size:${module.productNameSize}px;`
                                                                )}
                                                            >
                                                                {item.productName}
                                                            </div>
                                                        ) : null}
                                                        <div
                                                            className="pg-goods-price-box"
                                                            style={cssText(
                                                                `margin-top:${module.productPriceMarginTop}px;`,
                                                                "justify-content:left;",
                                                                "display:flex;",
                                                                hasBuyBtnText ? "flex-wrap:wrap;" : ""
                                                            )}
                                                        >
                                                            <div className="pg-price-origin-sale" style={cssText("justify-content:left;", hasBuyBtnText ? "flex-basis:100%;" : "")}>
                                                                <div className="pg-price-origin-icon">
                                                                    <div className="pg-price-icon">
                                                                        {pricePicUrl ? <img className="pg-icon" src={pricePicUrl} alt="" /> : null}
                                                                        {Number(module.showProductPrice) === 1 ? (
                                                                            <div
                                                                                className="pg-price"
                                                                                style={cssText(
                                                                                    `color:${pc.priceColor};`,
                                                                                    `font-weight:${module.productPriceWeight};`,
                                                                                    "font-family:inherit;",
                                                                                    module.productPriceSize ? `font-size:${module.productPriceSize}px;` : ""
                                                                                )}
                                                                            >
                                                                                ￥{item.productPrice}
                                                                            </div>
                                                                        ) : null}
                                                                        {Number(module.crossedOutPrice) === 1 ? (
                                                                            <div className="pg-origin" style={cssText(`color:${pc.crossedOutPriceColor};`)}>
                                                                                ￥{item.marketPrice}
                                                                            </div>
                                                                        ) : null}
                                                                    </div>
                                                                </div>
                                                            </div>
                                                            {Number(module.buyBtnStyle?.showBtn) === 1 ? (
                                                                <div style={{ margin: hasBuyBtnText ? "5px auto 0" : "0px" }}>
                                                                    <BuyButton styleOptions={module.buyBtnStyle} />
                                                                </div>
                                                            ) : null}
                                                        </div>
                                                    </div>
                                                </div>
                                            </div>
                                        );
                                    })}
                                </div>
                                {module.bottomMoreBtn !== "close" ? (
                                    <div className="pg-more-box">
                                        {module.bottomMoreBtnText && module.bottomMoreBtn === "text" ? (
                                            <div
                                                className="pg-more-txt"
                                                style={cssText(
                                                    "margin:10px 3px;",
                                                    `background-color:${pc.moreBtnBgColor};`,
                                                    `border-color:${pc.moreBtnBorderColor};`,
                                                    `color:${pc.moreBtnTextColor};`
                                                )}
                                            >
                                                {module.bottomMoreBtnText}
                                            </div>
                                        ) : null}
                                    </div>
                                ) : null}
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
