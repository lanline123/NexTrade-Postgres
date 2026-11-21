/**
 * 商品A1 预览（移植自 view/admin editor/modules/mobilePro/productA1/Preview.vue）
 *
 * 共享 goods-ad 类（全局 module.css）+ 本模块 scoped 规则（Preview.css，pa1- 作用域）。
 * 商品按 products 来源拉取，无有效来源/无结果时以在售商品池随机真实商品
 * 兜底展示（共享 demoProductFill 的 usePreviewDemoProducts，纯显示层不落盘）；
 * 池空/取数失败时走下方既有空态，不再写死演示商品。
 * buyBtnStyle 历史草稿可能为数字：非对象时回退默认对象（原版 Preview 同口径）。
 * 购买钮用共享 BuyButton（CommonBuyButton 口径）。
 */
import { useMemo } from "react";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { mergeDefaultModule, cloneDeep } from "../../../contract/sharedDefaults";
import BuyButton from "../../../shared/BuyButton";
import { usePreviewDemoProducts, demoTargetCount } from "../../../../../components/demoProductFill";
import { productA1Defaults } from "./index";
import "./Preview.css";

export default function ProductA1Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, productA1Defaults);
        if (!merged.buyBtnStyle || typeof merged.buyBtnStyle !== "object") {
            merged.buyBtnStyle = cloneDeep(productA1Defaults.buyBtnStyle);
        }
        return merged;
    }, [data]);

    const fetched = useDecorateProductList(module.products);
    const demoFallback = usePreviewDemoProducts(fetched.length === 0, demoTargetCount("productA1"));
    const productList = fetched.length > 0 ? fetched : demoFallback;

    const allFormat = {
        backgroundColor: module.backgroundColor ? `background-color:${module.backgroundColor};` : "",
        boxPadding: `margin-left:${module.boxPadding}px;margin-right:${module.boxPadding}px;`,
        boxPaddingBottom: `margin-bottom:${module.boxPaddingBottom}px;`,
        boxPaddingTop: `margin-top:${module.boxPaddingTop}px;`,
        boxRadius: `border-radius:${module.boxRadius}px;`,
        innerPadding: `padding:${module.innerPadding}px;`
    };

    const goodsRadius = module.goodsRadius ?? 0;
    const isListStyle = Number(module.style) === 4;
    const itemConRadiusStyle = goodsRadius ? `border-radius:${goodsRadius}px;overflow:hidden;` : "";
    const itemPhotoRadiusStyle = !goodsRadius
        ? ""
        : isListStyle
          ? `border-radius:${goodsRadius}px 0 0 ${goodsRadius}px;overflow:hidden;`
          : `border-radius:${goodsRadius}px ${goodsRadius}px 0 0;overflow:hidden;`;
    const itemInfoRadiusStyle = !goodsRadius
        ? ""
        : isListStyle
          ? `border-radius:0 ${goodsRadius}px ${goodsRadius}px 0;`
          : `border-radius:0 0 ${goodsRadius}px ${goodsRadius}px;`;

    const styleClassMap: Record<number, string> = { 1: "ad-style__1", 2: "ad-style__2", 3: "ad-style__3", 4: "ad-style__5" };
    const styleClass = styleClassMap[Number(module.style)] || "ad-style__1";

    const labelTagStyle = (() => {
        const r = module.labelRadius ?? 0;
        const mode = module.labelRadiusStyle ?? 0;
        let radiusCss = "";
        if (mode === 0) radiusCss = `border-radius:${r}px;`;
        else if (mode === 1) radiusCss = `border-radius:${r}px 0 0 ${r}px;`;
        else if (mode === 2) radiusCss = `border-radius:0 ${r}px ${r}px 0;`;
        else if (mode === 3) radiusCss = `border-top-left-radius:${r}px;border-bottom-right-radius:${r}px;`;
        else if (mode === 4) radiusCss = `border-top-right-radius:${r}px;border-bottom-left-radius:${r}px;`;
        const offsetX = module.labelOffsetX ?? 0;
        const offsetY = module.labelOffsetY ?? 0;
        const posMap: Record<string, string> = {
            "top-left": `left:${offsetX}px;top:${offsetY}px;right:auto;bottom:auto;`,
            "top-right": `right:${offsetX}px;top:${offsetY}px;left:auto;bottom:auto;`,
            "bottom-left": `left:${offsetX}px;bottom:${offsetY}px;right:auto;top:auto;`,
            "bottom-right": `right:${offsetX}px;bottom:${offsetY}px;left:auto;top:auto;`
        };
        return cssText(
            module.labelBackgroundColor ? `background-color:${module.labelBackgroundColor};` : "",
            module.labelTextColor ? `color:${module.labelTextColor};` : "",
            radiusCss,
            module.labelFontSize !== undefined ? `font-size:${module.labelFontSize}px;` : "",
            posMap[module.labelPosition || "top-left"] || posMap["top-left"]
        );
    })();

    const itemGap = module.itemGap ?? 0;
    const itemGapStyle = itemGap > 0 ? cssText(`padding:${itemGap / 2}px;`) : undefined;
    const goodsPadding = module.goodsPadding ?? 0;
    const goodsPaddingStyle = goodsPadding > 0 ? `padding:${goodsPadding}px;` : "";

    const nameAlignStyle = cssText(module.nameAlign === 2 ? "text-align:center;" : "text-align:left;");
    const briefListAlignStyle = cssText(module.nameAlign === 2 ? "justify-content:center;" : "justify-content:flex-start;");
    const priceAlignStyle = cssText(module.priceAlign === 2 ? "justify-content:center;" : "justify-content:flex-start;");
    const actionAlignStyle = cssText(module.priceAlign === 2 ? "justify-content:center;" : "");

    const briefTags = (idx: number): string[] => {
        const raw = module.briefList?.[idx] || "";
        return String(raw).split("\n").map((s) => s.trim()).filter(Boolean);
    };
    const briefTagStyle = (() => {
        const s = module.briefStyle || {};
        return cssText(
            s.titleColor ? `color:${s.titleColor};` : "",
            s.titleFontSize ? `font-size:${s.titleFontSize}px;` : "",
            s.titleFontBold ? `font-weight:${s.titleFontBold};` : "",
            module.briefBackgroundColor ? `background-color:${module.briefBackgroundColor};` : ""
        );
    })();

    return (
        <div
            className={`module-ad-con module-goods_ad module-product_a1 ${styleClass} ad-goods_style__${module.goodsStyle} ad-text_weight__${module.textWeight} ad-goods_name_row__${module.goodsNameRow} ad-goods_name_padding__${module.goodsNamePadding}`}
            style={cssText(allFormat.boxPadding, allFormat.boxPaddingTop, allFormat.boxPaddingBottom)}
        >
            <div className="module-ad-content" style={cssText(allFormat.backgroundColor, allFormat.boxRadius, allFormat.innerPadding)}>
                {!productList || productList.length === 0 ? (
                    <div className="module-ad-empty empty-image_ad">
                        <div className="image-empty-goods-bg" />
                        <div className="desc">点击添加商品</div>
                    </div>
                ) : null}
                <div className="goods-ad-warp">
                    <div className="goods-ad-con">
                        {productList.map((item: any, idx: number) => (
                            <div className="goods-ad-item" key={`${item.productId ?? 0}_${idx}`} style={itemGapStyle}>
                                <div className="item-content">
                                    <div className="item-con" style={cssText(itemConRadiusStyle, goodsPaddingStyle)}>
                                        <div className="item-photo" style={cssText(itemPhotoRadiusStyle)}>
                                            <a className="item-image-a">
                                                <img src={productImage(item)} alt="" />
                                            </a>
                                            {Number(module.showProductLabel) === 1 && module.tagList?.[idx] ? (
                                                <span className="pa1-item-label-tag" style={labelTagStyle}>
                                                    {module.tagList[idx]}
                                                </span>
                                            ) : null}
                                        </div>
                                        <div className="item-info" style={cssText(itemInfoRadiusStyle)}>
                                            <div className="item-name" style={nameAlignStyle}>
                                                {Number(module.showName) === 1 ? <a className="item-name-a">{item.productName}</a> : null}
                                                {Number(module.showBrief) === 1 && briefTags(idx).length ? (
                                                    <div className="pa1-item-brief-list" style={briefListAlignStyle}>
                                                        {briefTags(idx).map((brief, bi) => (
                                                            <span key={bi} className="pa1-item-brief-tag" style={briefTagStyle}>
                                                                {brief}
                                                            </span>
                                                        ))}
                                                    </div>
                                                ) : null}
                                            </div>
                                            {Number(module.showPrice) === 1 ? (
                                                <div className="item-action" style={actionAlignStyle}>
                                                    <div className="item-price" style={priceAlignStyle}>
                                                        <b className="price_format">{priceFormat(Number(item.productPrice))}</b>
                                                        {Number(module.showMarketPrice) === 1 ? (
                                                            <span className="market_price">
                                                                <b>{priceFormat(Number(item.marketPrice))}</b>
                                                            </span>
                                                        ) : null}
                                                    </div>
                                                    {module.buyBtnStyle?.showBtn ? (
                                                        <div className="item-buy">
                                                            <BuyButton styleOptions={module.buyBtnStyle} />
                                                        </div>
                                                    ) : null}
                                                </div>
                                            ) : null}
                                        </div>
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
}
