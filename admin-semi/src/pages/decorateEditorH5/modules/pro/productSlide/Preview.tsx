/**
 * 商品滑动 预览（类名对齐 uniapp-react productSlide：ps-wrap/ps-scroll/ps-item/ps-hero/ps-prices）
 */
import { useMemo } from "react";
import { cssText, productImage, priceFormat, imageFormat, pickPic } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { productSlideDefaults } from "./index";
import "./Preview.css";

const DEMO_PRODUCTS = [
    { productId: 9001, productName: "演示商品一", productPrice: 99, marketPrice: 129, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png" },
    { productId: 9002, productName: "演示商品二", productPrice: 59, marketPrice: 79, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png" },
    { productId: 9003, productName: "演示商品三", productPrice: 39, marketPrice: 49, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png" },
];

export default function ProductSlidePreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, productSlideDefaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const fetched = useDecorateProductList(module.products);
    const productList = fetched.length ? fetched : DEMO_PRODUCTS;
    const bg = pickPic(module.moduleBgImage);
    const titles: string[] = module.cardTitleList || [];
    const starts: string[] = module.cardGradientStartList || [];
    const ends: string[] = module.cardGradientEndList || [];
    return (
        <div className="module-ad-con module-product_slide" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            {bg ? <img className="ps-module-bg" src={imageFormat(bg)} alt="" /> : null}
            <div className="ps-wrap" style={{ backgroundColor: module.innerBgColor, borderRadius: `${module.cardRadius ?? 16}px`, padding: 8 }}>
                <div className="ps-scroll"><div className="ps-track" style={{ gap: `${module.productItemGap ?? 10}px` }}>
                    {productList.map((item: any, i: number) => (
                        <div className="ps-item" key={i} style={{ flex: `0 0 ${module.productPercent ?? 42}%` }}>
                            <div className="ps-hero" style={{ background: `linear-gradient(to bottom, ${starts[i % Math.max(starts.length, 1)] || "#B6A8F5"}, ${ends[i % Math.max(ends.length, 1)] || "#8E7BF0"})`, borderRadius: `${module.cardRadius ?? 16}px`, padding: 8 }}>
                                <div style={{ color: module.cardTitleColor, fontWeight: module.textWeight, textAlign: module.textAlign ?? "center", marginBottom: `${module.layerGap ?? 10}px` }}>{titles[i % Math.max(titles.length, 1)] || ""}</div>
                                <div className="ps-photo"><div className="ps-img-wrap"><img className="ps-img ps-img-ratio" src={productImage(item)} alt="" style={{ borderRadius: `${module.productImgRadius ?? 12}px`, objectFit: module.productPicFillType ?? "cover", width: "100%", aspectRatio: "1", display: "block" }} /></div></div>
                            </div>
                            {Number(module.showProductName) === 1 ? <div className="ps-name">{item.productName}</div> : null}
                            {Number(module.showFeature) === 1 && item.productFeature ? <div className="ps-feature">{item.productFeature}</div> : null}
                            {Number(module.showBrief) === 1 && item.productBrief ? <div className="ps-feature" style={{ color: module.briefColor }}>{item.productBrief}</div> : null}
                            <div className="ps-bottom"><div className="ps-prices">
                                {Number(module.showPrice) === 1 ? <span className="price-current" style={{ color: module.productPriceColor === "#FFFFFF" ? "#e1251b" : module.productPriceColor, fontSize: `${module.priceFontSize ?? 15}px`, fontWeight: Number(module.priceBold) === 1 ? 700 : 400 }}>¥{priceFormat(item.productPrice)}</span> : null}
                                {Number(module.showMarketPrice) === 1 && item.marketPrice ? <span className="price-market">¥{priceFormat(item.marketPrice)}</span> : null}
                            </div>
                            {Number(module.showSales) === 1 && item.sales ? <span className="ps-sales">已售{item.sales}</span> : null}
                            {Number(module.buyBtnStyle) !== 0 ? <span className="buy-btn-text" style={{ background: `linear-gradient(to right, ${module.buyBtnGradientStart}, ${module.buyBtnGradientEnd})`, color: module.buyBtnTextColor }}>抢购</span> : null}
                            </div>
                            {item.badge ? <span className="ps-badge" style={{ backgroundColor: module.badgeColor }}>{item.badge}</span> : null}
                        </div>
                    ))}
                </div></div>
            </div>
        </div>
    );
}
