/**
 * 混合商品展示 预览（类名对齐 uniapp-react mixedProductDisplay：comp/cont/item-box/mk-title/price-box）
 */
import { useMemo } from "react";
import H5Countdown from "../../../shared/H5Countdown";
import { cssText, productImage, priceFormat, imageFormat, pickPic } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { mixedProductDisplayDefaults } from "./index";
import "./Preview.css";

const DEMO_PRODUCTS = [
    { productId: 9001, productName: "演示商品一", productPrice: 99, marketPrice: 129, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png" },
    { productId: 9002, productName: "演示商品二", productPrice: 59, marketPrice: 79, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png" },
    { productId: 9003, productName: "演示商品三", productPrice: 39, marketPrice: 49, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png" },
];

export default function MixedProductDisplayPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, mixedProductDisplayDefaults);
        return merged;
    }, [data]);
    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const fetched = useDecorateProductList(module.products);
    const productList = fetched.length ? fetched : DEMO_PRODUCTS;
    const groups: any[] = module.productGroups?.length ? module.productGroups : [{ title: "精选好物", subTitle: "" }];
    const colors = module.moduleColor || {};
    const rightPic = pickPic(module.rightPic);
    return (
        <div className="module-ad-con" style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content" style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}>
                <div className="comp">
                    {groups.map((group, gi) => (
                        <div className="cont" key={gi} style={{ borderColor: colors.lineColor, marginBottom: 10 }}>
                            <div className="mk-title" style={{ color: colors.titleColor }}>{group.title}</div>
                            {group.subTitle ? <div className="mk-stitle line1">{group.subTitle}</div> : null}
                            {group.sellingPoint ? <div className="line1" style={{ color: colors.sellingPointColor, fontSize: 12 }}>{group.sellingPoint}</div> : null}
                            {Number(module.showCountdown) === 1 ? (<div className="time" style={{ backgroundColor: colors.countdownBackgroundColor, color: colors.countdownColor }}><H5Countdown countdownType={module.countdownType || "dailyCycle"} dailyTime={module.dailyTime} specifyTime={Number(module.specifyTime) || Date.now() + 3600 * 1000} format="HH:mm:ss" /></div>) : null}
                            {rightPic ? <img className="img" src={imageFormat(rightPic)} alt="" /> : null}
                            <div className="item-box">
                                {productList.map((item: any, i: number) => (
                                    <div className="item" key={i}>
                                        <img className="goods-img" src={productImage(item)} alt="" style={{ borderRadius: `${module.productRadius}px`, objectFit: module.picFillType ?? "cover" }} />
                                        <div className="line1" style={{ fontSize: 12 }}>{item.productName}</div>
                                        {Number(module.showProductPrice) === 1 ? (<div className="price-box" style={{ backgroundColor: colors.priceBackgroundColor }}><span className="price" style={{ color: colors.priceColor }}>¥{priceFormat(item.productPrice)}</span>{Number(module.crossedOutPrice) === 1 && item.marketPrice ? <span className="origin" style={{ color: colors.crossedOutPriceColor }}>¥{priceFormat(item.marketPrice)}</span> : null}</div>) : null}
                                    </div>
                                ))}
                            </div>
                            <div className="line-box"><div className="line" style={{ borderColor: colors.lineColor }} /></div>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
}
