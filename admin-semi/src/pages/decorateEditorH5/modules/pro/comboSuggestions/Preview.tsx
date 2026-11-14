/**
 * 组合建议 预览（类名对齐 uniapp-react comboSuggestions：top/list/item/pic-box/goods-price）
 */
import { useMemo } from "react";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { comboSuggestionsDefaults } from "./index";
import "./Preview.css";

const DEMO_PRODUCTS = [
    { productId: 9001, productName: "演示商品一", productPrice: 99, marketPrice: 129, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png" },
    { productId: 9002, productName: "演示商品二", productPrice: 59, marketPrice: 79, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png" },
    { productId: 9003, productName: "演示商品三", productPrice: 39, marketPrice: 49, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png" },
];

export default function ComboSuggestionsPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, comboSuggestionsDefaults);
        return merged;
    }, [data]);
    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const fetched = useDecorateProductList(module.products);
    const productList = fetched.length ? fetched : DEMO_PRODUCTS;
    const groups: any[] = module.comboGroups?.length ? module.comboGroups : [{ title: "超值搭配", subTitle: "" }];
    return (
        <div className="module-ad-con" style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content" style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}>
                {groups.map((group, gi) => (
                    <div className="main" key={gi} style={{ borderRadius: `${module.groupRadius}px`, boxShadow: Number(module.groupShadow) === 1 ? "0 2px 8px rgba(0,0,0,0.06)" : "none", marginBottom: 10 }}>
                        <div className="top"><div className="top-tit" style={{ color: module.groupTitleColor }}>
                            <span className="big-title line1">{group.title}</span>
                            {group.subTitle ? <span className="desc">{group.subTitle}</span> : null}
                        </div></div>
                        <div className="list">
                            {productList.map((item: any, i: number) => (
                                <div className="item" key={i}>
                                    <div className="pic-box"><img className="pic-img" src={productImage(item)} alt="" style={{ borderRadius: `${module.imageRadius}px` }} /></div>
                                    <div className="title line1">{item.productName}</div>
                                    <div className="goods-price"><span className="price-text">¥{priceFormat(item.productPrice)}</span>{item.marketPrice ? <span className="goods-img" style={{ backgroundColor: module.regularPriceBgColor, color: module.regularPriceColor, borderRadius: `${module.regularPriceRadius}px`, fontSize: `${module.regularPriceSize}px` }}>¥{priceFormat(item.marketPrice)}</span> : null}</div>
                                </div>
                            ))}
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
}
