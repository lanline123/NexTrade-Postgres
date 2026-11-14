/**
 * 倒计时 预览（类名对齐 uniapp-react countdown：countdow-warp/count-down/mk-top/itemBox；倒计时用 shared/H5Countdown）
 */
import { useMemo } from "react";
import H5Countdown from "../../../shared/H5Countdown";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { countdownDefaults } from "./index";
import "./Preview.css";

const DEMO_PRODUCTS = [
    { productId: 9001, productName: "演示商品一", productPrice: 99, marketPrice: 129, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png" },
    { productId: 9002, productName: "演示商品二", productPrice: 59, marketPrice: 79, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png" },
    { productId: 9003, productName: "演示商品三", productPrice: 39, marketPrice: 49, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png" },
];

export default function CountdownPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, countdownDefaults);
        return merged;
    }, [data]);
    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const fetched = useDecorateProductList(module.products);
    const legacy: any[] = Array.isArray(module.pruductList) && module.pruductList.length ? module.pruductList : [];
    const productList = fetched.length ? fetched : legacy.length ? legacy : DEMO_PRODUCTS;
    const cd = module.countdownData || {};
    const colors = module.moduleColor || {};
    return (
        <div className="module-ad-con" style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content" style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}>
                <div className="countdow-warp">
                    <div className="mk-top">
                        <div className="mk-top-left">
                            <div className="mk-title line1" style={{ color: colors.titleColor }}>{module.moduleTitle?.title}</div>
                            {module.moduleTitle?.subTitle ? <div className="mk-stitle line1" style={{ color: colors.subTitleColor }}>{module.moduleTitle.subTitle}</div> : null}
                        </div>
                        {Number(cd.showCountdown) === 1 ? (
                            <div className="count-down">
                                <H5Countdown countdownType={cd.countdownType || "dailyCycle"} dailyTime={cd.dailyTime} specifyTime={Number(cd.specifyTime) || Date.now() + 2 * 3600 * 1000} format="DD HH:mm:ss">
                                    {(time) => (<div className="countdown" style={{ color: colors.countdownColor }}>
                                        {Number(time.days) > 0 ? <span className="countdown-img">{time.days}天</span> : null}
                                        <span className="countdown-img">{time.hours}</span>:<span className="countdown-img">{time.minutes}</span>:<span className="countdown-img">{time.seconds}</span>
                                    </div>)}
                                </H5Countdown>
                            </div>
                        ) : null}
                    </div>
                    <div className="itemBox" style={{ backgroundColor: colors.productBackgroundColor, borderRadius: `${module.productRadius}px`, padding: `${module.productPadding}px` }}>
                        {productList.slice(0, Number(module.rowNum) || 3).map((item: any, i: number) => (
                            <div className="item" key={i}>
                                <div className="imgBox"><img className="img" src={productImage(item)} alt="" style={{ objectFit: module.picFillType ?? "cover" }} /></div>
                                <div className="goods-info">
                                    {Number(module.showProductName) === 1 ? <div className="title line2" style={{ color: colors.productNameColor, textAlign: module.textAlignment }}>{item.productName}</div> : null}
                                    {Number(module.showProductPrice) === 1 ? (<div className="priceBox"><span className="price" style={{ color: colors.productPriceColor, fontWeight: module.priceWeight }}>¥{priceFormat(item.productPrice)}</span>{Number(module.crossedOutPrice) === 1 && item.marketPrice ? <span className="origin" style={{ color: colors.crossedOutPriceColor }}>¥{priceFormat(item.marketPrice)}</span> : null}</div>) : null}
                                </div>
                            </div>
                        ))}
                    </div>
                    {module.moduleTitle?.moreText ? <div className="mk-notice" style={{ color: colors.tipsColor }}>{module.moduleTitle.moreText}</div> : null}
                </div>
            </div>
        </div>
    );
}
