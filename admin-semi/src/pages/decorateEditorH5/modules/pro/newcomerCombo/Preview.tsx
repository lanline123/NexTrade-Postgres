/**
 * 新人专享 预览（类名对齐 uniapp-react newcomerCombo：combo-wrap/combo-header/combo-card/product-item/coupon-item）
 */
import { useMemo } from "react";
import { cssText, productImage, priceFormat, imageFormat, pickPic } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { newcomerComboDefaults } from "./index";
import "./Preview.css";

const DEMO_PRODUCTS = [
    { productId: 9001, productName: "演示商品一", productPrice: 99, marketPrice: 129, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png" },
    { productId: 9002, productName: "演示商品二", productPrice: 59, marketPrice: 79, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png" },
    { productId: 9003, productName: "演示商品三", productPrice: 39, marketPrice: 49, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png" },
];
const DEMO_COUPONS = [
    { couponName: "新人专享券", couponMoney: 10, minOrderAmount: 99 },
    { couponName: "满减券", couponMoney: 20, minOrderAmount: 199 },
    { couponName: "尝鲜券", couponMoney: 5, minOrderAmount: 49 },
];

export default function NewcomerComboPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, newcomerComboDefaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const fetched = useDecorateProductList(module.products);
    const productList = fetched.length ? fetched : DEMO_PRODUCTS;
    const coupons: any[] = module.coupons?.length ? module.coupons : DEMO_COUPONS;
    const bg = pickPic(module.containerBgImage);
    return (
        <div className="module-ad-con module-newcomer_combo" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="combo-wrap" style={{ background: `linear-gradient(to right, ${module.boxGradientStart}, ${module.boxGradientEnd})`, backgroundColor: module.decorationBgColor, borderRadius: `${module.frame?.boxRadius ?? 8}px` }}>
                {bg ? <img src={imageFormat(bg)} alt="" style={{ width: "100%", display: "block" }} /> : null}
                {Number(module.showHeader) === 1 ? (<div className="combo-header"><div className="header-title">{module.headerTitle}</div><div className="header-sub">{module.headerSubTitle}</div></div>) : null}
                <div style={{ backgroundColor: module.contentBgColor, borderRadius: 8, padding: 8 }}>
                {Number(module.showSection1) === 1 ? (
                    <div className="combo-card combo-card-section1">
                        <div className="card-head"><span className="card-badge">{module.section1Badge}</span><span className="card-title">{module.section1Title}</span><span className="more-link more-link-pill">更多</span></div>
                        <div className="product-scroll"><div className="product-track" style={{ gap: `${module.productItemGap ?? 8}px` }}>
                            {productList.map((item: any, i: number) => (
                                <div className="product-item" key={i} style={{ flex: `0 0 ${module.productPercent ?? 28}%`, padding: `${module.productItemPadding ?? 6}px` }}>
                                    <div className="product-photo"><div className="product-img-wrap"><img src={productImage(item)} alt="" style={{ borderRadius: `${module.productImgRadius ?? 8}px`, objectFit: module.productPicFillType ?? "cover", width: "100%", aspectRatio: "1", display: "block" }} /></div></div>
                                    {module.productBriefList?.[i] ? <div className="product-brief-overlap" style={{ background: `linear-gradient(to right, ${module.productBriefGradientStart}, ${module.productBriefGradientEnd})`, color: module.productBriefTextColor }}>{module.productBriefList[i]}</div> : null}
                                    {Number(module.showProductName) === 1 ? <div className="product-name" style={{ color: module.productTitleColor }}>{item.productName}</div> : null}
                                    <div className="product-bottom"><div className="product-prices"><span className="price-current" style={{ color: module.productPriceColor }}>¥{priceFormat(item.productPrice)}</span>{Number(module.showMarketPrice) === 1 && item.marketPrice ? <span className="price-market" style={{ color: module.productMarketPriceColor }}>¥{priceFormat(item.marketPrice)}</span> : null}</div>
                                    <span className="iconfont-h5 icon-gouwuche" style={{ background: `linear-gradient(to right, ${module.buyBtnGradientStart}, ${module.buyBtnGradientEnd})`, color: module.buyBtnTextColor, borderRadius: "50%", padding: 4, fontSize: 11 }}>🛒</span></div>
                                </div>
                            ))}
                        </div></div>
                    </div>
                ) : null}
                {Number(module.showSection2) === 1 ? (
                    <div className="combo-card combo-card-section2">
                        <div className="card-head"><span className="card-badge">{module.section2Badge}</span><span className="card-title">{module.section2Title}</span><span className="more-link">更多</span></div>
                        <div className="coupon-row coupon-row-scroll" style={{ display: "grid", gridTemplateColumns: `repeat(${module.couponPerRow ?? 3}, 1fr)`, gap: 6 }}>
                            {coupons.map((c: any, i: number) => (
                                <div className="coupon-item" key={i} style={{ background: `linear-gradient(to bottom, ${module.couponBgGradientStart}, ${module.couponBgGradientEnd})` }}>
                                    <div className="coupon-amount"><span className="amount-prefix" style={{ color: module.couponAmountColor }}>¥</span><span className="amount-num" style={{ color: module.couponAmountColor }}>{c.couponMoney ?? c.money ?? 10}</span></div>
                                    <div className="coupon-condition" style={{ color: module.couponConditionColor }}>满{c.minOrderAmount ?? c.threshold ?? 0}可用</div>
                                    <div className="cur-prefix" style={{ color: module.couponBtnTextColor, fontSize: 11 }}>立即领取</div>
                                </div>
                            ))}
                        </div>
                    </div>
                ) : null}
                </div>
            </div>
        </div>
    );
}
