/**
 * 分组滑动 预览（类名对齐 uniapp-react groupSliding：groupsliding/slide-box/goods-item/title_box）
 */
import { useMemo } from "react";
import BuyButton from "../../../shared/BuyButton";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { groupSlidingDefaults } from "./index";
import "./Preview.css";

const DEMO_PRODUCTS = [
    { productId: 9001, productName: "演示商品一", productPrice: 99, marketPrice: 129, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png" },
    { productId: 9002, productName: "演示商品二", productPrice: 59, marketPrice: 79, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png" },
    { productId: 9003, productName: "演示商品三", productPrice: 39, marketPrice: 49, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png" },
];

export default function GroupSlidingPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, groupSlidingDefaults);
        return merged;
    }, [data]);
    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const fetched = useDecorateProductList(module.products);
    const productList = fetched.length ? fetched : DEMO_PRODUCTS;
    const groups: any[] = module.groupList?.length ? module.groupList : [{ title: "热销榜", subTitle: "" }];
    const colors = module.moduleColor || {};
    return (
        <div className="module-ad-con" style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content" style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}>
                <div className="groupsliding">
                    {groups.map((group, gi) => (
                        <div className="li" key={gi} style={{ marginBottom: 12 }}>
                            <div className="title_box">
                                {colors.titlePic?.picUrl ? <img className="bg_title" src={colors.titlePic.picUrl} alt="" /> : null}
                                <span className="span line1" style={{ color: colors.titleColor, fontSize: `${colors.titleSize ?? 16}px` }}>{group.title}</span>
                                {group.subTitle ? <span className="p line1" style={{ color: colors.subTitleColor, fontSize: `${colors.subTitleSize ?? 12}px` }}>{group.subTitle}</span> : null}
                            </div>
                            <div className="slide-box"><div className="item-box" style={{ display: "flex", gap: 8, overflowX: "auto" }}>
                                {productList.map((item: any, i: number) => (
                                    <div className="goods-item" key={i} style={{ flex: `0 0 ${module.groupWidth ?? 85}%`, backgroundColor: colors.productBackgroundColor, borderRadius: `${module.groupRadius}px` }}>
                                        <img className="goods-img" src={productImage(item)} alt="" style={{ borderRadius: `${module.productPicRadius}px` }} />
                                        <div className="goods-info">
                                            {Number(module.showRanking) === 1 ? <span className="ranking">{i + 1}</span> : null}
                                            {Number(module.showProductName) === 1 ? <div className="goods-title" style={{ fontSize: `${module.productNameSize}px`, fontWeight: module.productNameWeight }}>{item.productName}</div> : null}
                                            {Number(module.showProductPrice) === 1 ? (<div className="goods-price-box" style={{ marginTop: `${module.productPriceMarginTop}px` }}><span className="price" style={{ color: colors.productPriceColor }}><span className="price-icon">¥</span>{priceFormat(item.productPrice)}</span>{Number(module.crossedOutPrice) === 1 && item.marketPrice ? <span className="origin"><span className="price-origin-icon">¥</span><span className="price-origin-sale">{priceFormat(item.marketPrice)}</span></span> : null}</div>) : null}
                                            {Number(module.buyBtnStyle?.showBtn) === 1 ? <BuyButton styleOptions={module.buyBtnStyle} /> : null}
                                        </div>
                                    </div>
                                ))}
                            </div></div>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
}
