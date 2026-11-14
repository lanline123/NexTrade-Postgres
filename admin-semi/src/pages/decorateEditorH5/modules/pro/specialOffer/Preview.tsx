/**
 * 特价专区 预览（类名对齐 uniapp-react specialOffer：mk-top/mk-title/mk-countdown/lists/item/price-box）
 */
import { useMemo } from "react";
import H5Countdown from "../../../shared/H5Countdown";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { specialOfferDefaults } from "./index";
import "./Preview.css";

const DEMO_PRODUCTS = [
    { productId: 9001, productName: "演示商品一", productPrice: 99, marketPrice: 129, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png" },
    { productId: 9002, productName: "演示商品二", productPrice: 59, marketPrice: 79, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png" },
    { productId: 9003, productName: "演示商品三", productPrice: 39, marketPrice: 49, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png" },
];

export default function SpecialOfferPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, specialOfferDefaults);
        return merged;
    }, [data]);
    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const fetched = useDecorateProductList(module.products);
    const explicit: any[] = Array.isArray(module.productList) && module.productList.length ? module.productList : [];
    const productList = fetched.length ? fetched : explicit.length ? explicit : DEMO_PRODUCTS;
    const mt = module.moduleTitle || {}; const colors = module.moduleColor || {};
    return (
        <div className="module-ad-con" style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content main" style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}>
                <div className="mk-top">
                    <div>
                        <div className="mk-title" style={{ color: colors.titleColor }}>{mt.title}</div>
                        {mt.subTitle ? <div className="mk-small-title" style={{ color: colors.subTitleColor }}>{mt.subTitle}</div> : null}
                    </div>
                    <div className="mk-countdown">
                        <span className="tit line1" style={{ color: colors.countdownTitleColor }}>{mt.countdownTitle}</span>
                        <span className="countdown" style={{ color: colors.countdownColor }}><H5Countdown countdownType={mt.countdownType || "dailyCycle"} dailyTime={mt.dailyTime} specifyTime={Number(mt.specifyTime) || Date.now() + 3600 * 1000} format="HH:mm:ss" /></span>
                    </div>
                    {mt.buttonText ? <span className="mk-action-btn" style={{ color: colors.buttonTextColor }}>{mt.buttonText}<i className="iconfont-h5 icon-youjiantou" /></span> : null}
                </div>
                {Number(module.showProdcutTitles) === 1 && Array.isArray(module.prodcutTitles) ? <div className="lists">{module.prodcutTitles.map((t: any, i: number) => (<span className="text line1" key={i}>{t.title || t}</span>))}</div> : null}
                <div className="lists">
                    {productList.map((item: any, i: number) => (
                        <div className="item" key={i} style={{ flex: `0 0 ${module.productWidth ?? 30}%` }}>
                            <div className="item-box product-style-border" style={{ borderRadius: `${module.productRadius}px` }}>
                                <div className="item-img-wrap"><img className="img" src={productImage(item)} alt="" style={{ objectFit: module.picFillType ?? "cover" }} /></div>
                                <div className="goods-info">
                                    {Number(module.showProductName) === 1 ? <div className="tit line1" style={{ color: colors.productNameColor, fontSize: `${module.productNameSize}px`, fontWeight: module.productNameWeight }}>{item.productName}</div> : null}
                                    {Number(module.showSellingPoints) === 1 && item.sellingPoints ? <div className="desc line1" style={{ color: colors.sellingPointsColor }}>{item.sellingPoints}</div> : null}
                                    {Number(module.showProductPrice) === 1 ? <div className="price-box"><span className="price">¥{priceFormat(item.productPrice)}</span></div> : null}
                                </div>
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
}
