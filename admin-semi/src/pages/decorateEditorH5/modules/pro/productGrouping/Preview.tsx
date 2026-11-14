/**
 * 商品分组 预览（类名对齐 uniapp-react productGrouping：productgrouping/tablist/tablist-item/productlist/goods-item）
 */
import { useMemo, useState } from "react";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { productGroupingDefaults } from "./index";
import "./Preview.css";

const DEMO_PRODUCTS = [
    { productId: 9001, productName: "演示商品一", productPrice: 99, marketPrice: 129, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png" },
    { productId: 9002, productName: "演示商品二", productPrice: 59, marketPrice: 79, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png" },
    { productId: 9003, productName: "演示商品三", productPrice: 39, marketPrice: 49, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png" },
];

export default function ProductGroupingPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, productGroupingDefaults);
        return merged;
    }, [data]);
    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {});
    const fetched = useDecorateProductList(module.products);
    const productList = fetched.length ? fetched : DEMO_PRODUCTS;
    const [currentIndex, setCurrentIndex] = useState(0);
    const tabs: any[] = module.productGroups?.length ? module.productGroups : module.groupList || [];
    const gt = module.groupTitle || {};
    const pc = module.productColor || {};
    return (
        <div className="module-ad-con" style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content" style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}>
                <div className="productgrouping">
                    <div className="tablist" style={{ margin: `0 ${gt.marginLeftRight ?? 10}px` }}>
                        {tabs.map((item, index) => (
                            <div className="tablist-item" key={index} onClick={() => setCurrentIndex(index)}>
                                {index > 0 ? <div className="line" style={{ borderColor: gt.splitLineColor }} /> : null}
                                <div className="nav-item-box"><div className="nav-item">
                                    {gt.showTitle ? <div className="tab-title" style={{ color: index === currentIndex ? gt.activeTitleColor : gt.titleColor, fontSize: `${index === currentIndex ? gt.bigTitleFontSize ?? gt.titleFontSize : gt.titleFontSize}px` }}>{item.title}</div> : null}
                                    {item.subTitle && gt.showSubTitle ? <div className="tab-sub" style={{ color: index === currentIndex ? gt.activeSubTitleColor : gt.subTitleColor, backgroundImage: index === currentIndex ? `linear-gradient(to right, ${gt.gradientColorA}, ${gt.gradientColorB})` : "none", borderRadius: `${gt.subTitleRadius}px` }}>{item.subTitle}</div> : null}
                                </div></div>
                            </div>
                        ))}
                    </div>
                    <div className="productlist" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: `${module.cardMargin ?? 8}px` }}>
                        {productList.map((item: any, i: number) => (
                            <div className="productlist-item" key={i}><div className="goods-item product-style-border" style={{ backgroundColor: pc.backgroundColor, borderRadius: `${module.cardRadius}px`, padding: `${module.cardPadding}px` }}>
                                <img className="goods-img" src={productImage(item)} alt="" style={{ width: "100%", aspectRatio: "1", display: "block", objectFit: module.picFillType ?? "cover", borderRadius: `${module.cardRadius}px` }} />
                                <div className="goods-info">
                                    {Number(module.showProductName) === 1 ? <div className="goods-title" style={{ color: pc.titleColor, fontSize: `${module.productNameSize}px`, fontWeight: module.productNameWeight }}>{item.productName}</div> : null}
                                    {Number(module.showProductPrice) === 1 ? (<div className="goods-price-box" style={{ marginTop: `${module.productPriceMarginTop}px` }}><span className="price" style={{ color: pc.priceColor, fontSize: `${module.productPriceSize}px`, fontWeight: module.productPriceWeight }}><span className="price-icon">¥</span>{priceFormat(item.productPrice)}</span>{Number(module.crossedOutPrice) === 1 && item.marketPrice ? <span className="origin" style={{ color: pc.crossedOutPriceColor }}><span className="price-origin-icon">¥</span><span className="price-origin-sale">{priceFormat(item.marketPrice)}</span></span> : null}</div>) : null}
                                </div>
                            </div></div>
                        ))}
                    </div>
                    {Number(module.bottomMoreBtn) === 1 ? <div className="more-box"><span className="more-txt" style={{ backgroundColor: pc.moreBtnBgColor, borderColor: pc.moreBtnBorderColor, color: pc.moreBtnTextColor }}>{module.bottomMoreBtnText}</span></div> : null}
                    {module.productPricePic?.picUrl ? <img className="banner-fill" src={module.productPricePic.picUrl} alt="" /> : null}
                </div>
            </div>
        </div>
    );
}
