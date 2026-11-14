/**
 * 商品详情(样式A1) 预览（契约占位：详情页为固定渲染，此预览仅表达模块契约字段的排版意图）
 */
import { useMemo } from "react";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { productDetailA1Defaults } from "./index";
import "./Preview.css";

export default function ProductDetailA1Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, productDetailA1Defaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const picUrl = productImage(module.pic);
    return (
        <div className="module-ad-con module-product_detail" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={{ ...(cssText(frameFormat.backgroundColor, frameFormat.boxRadius) as Record<string, string>), ...(module.backgroundColor ? { backgroundColor: module.backgroundColor } : {}) }}>
                <div className={`dec-product-detail dec-product-detail--style${module.style ?? 1}`}>
                    <div className="dec-product-detail__pic">
                        {picUrl ? <img src={picUrl} alt="" /> : <div className="dec-product-detail__pic-ph">商品图</div>}
                    </div>
                    <div className="dec-product-detail__info">
                        {Number(module.showName ?? 1) !== 0 ? <div className="dec-product-detail__name" style={{ color: module.nameColor || undefined }}>{module.productTitle}</div> : null}
                        {Number(module.showBrief ?? 1) !== 0 ? <div className="dec-product-detail__brief">{module.description}</div> : null}
                        <div className="dec-product-detail__buy-row">
                            {Number(module.showPrice ?? 1) !== 0 ? (
                                <div className="dec-product-detail__price-box">
                                    <span className="dec-product-detail__price" style={{ color: module.priceColor || undefined }}>¥{priceFormat(module.price)}</span>
                                    {Number(module.showMarketPrice ?? 1) !== 0 ? <span className="dec-product-detail__market-price">¥{priceFormat(module.marketPrice)}</span> : null}
                                </div>
                            ) : <span />}
                            {Number(module.showButton ?? 1) !== 0 ? <span className="dec-product-detail__btn" style={{ background: module.buttonColor || undefined }}>{module.buttonText}</span> : null}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
