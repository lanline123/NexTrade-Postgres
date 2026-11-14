/**
 * 商品A1 预览（类名对齐 uniapp-react productA1：goods-ad-warp/goods-ad-item/item-label-tag/item-brief-list）
 */
import { useMemo } from "react";
import BuyButton from "../../../shared/BuyButton";
import CommonTitle from "../../../shared/CommonTitle";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { productA1Defaults } from "./index";
import "./Preview.css";

const DEMO_PRODUCTS = [
    { productId: 9001, productName: "演示商品一", productPrice: 99, marketPrice: 129, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png" },
    { productId: 9002, productName: "演示商品二", productPrice: 59, marketPrice: 79, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png" },
    { productId: 9003, productName: "演示商品三", productPrice: 39, marketPrice: 49, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png" },
];

export default function ProductA1Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, productA1Defaults);
        return merged;
    }, [data]);
    const fetched = useDecorateProductList(module.products);
    const productList = fetched.length ? fetched : DEMO_PRODUCTS;
    const frameFormat = formatFrame(module.frame ?? {});
    const cols = Number(module.style) === 1 ? 1 : Number(module.style) === 3 ? 3 : 2;
    const tags: string[] = Array.isArray(module.tagList) ? module.tagList : [];
    const briefs: string[] = Array.isArray(module.briefList) ? module.briefList : [];
    return (
        <div className={`module-ad-con module-goods_ad ad-style__${module.style}`} style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={{ backgroundColor: module.backgroundColor || undefined, borderRadius: `${module.boxRadius ?? 0}px`, padding: `${module.innerPadding ?? 0}px` }}>
                {(module as any).title?.showTitle ? <CommonTitle data={(module as any).title} /> : null}
                <div className="goods-ad-warp"><div className="goods-ad-con" style={{ display: "grid", gridTemplateColumns: `repeat(${cols}, 1fr)`, gap: `${module.itemGap ?? module.goodsPadding ?? 0}px`, margin: `${module.boxPaddingTop ?? 0}px ${module.boxPadding ?? 0}px ${module.boxPaddingBottom ?? 0}px` }}>
                    {productList.map((item: any, i: number) => (
                        <div className="goods-ad-item" key={i}><div className="item-content" style={{ padding: `${module.goodsPadding ?? 0}px` }}><div className="item-con">
                            <div className="item-photo"><a className="item-image-a"><span className="item-image item-image-ratio" style={{ borderRadius: `${module.goodsRadius ?? 0}px`, overflow: "hidden", display: "block", position: "relative" }}><img src={productImage(item)} alt="" style={{ width: "100%", aspectRatio: "1", display: "block", objectFit: "cover" }} />
                            {Number(module.showProductLabel) === 1 && tags[0] ? <span className="item-label-tag" style={{ position: "absolute", backgroundColor: module.labelBackgroundColor, color: module.labelTextColor, borderRadius: `${module.labelRadius}px`, fontSize: `${module.labelFontSize}px`, left: `${module.labelOffsetX}px`, top: `${module.labelOffsetY}px`, padding: "1px 6px" }}>{tags[0]}</span> : null}
                            </span></a></div>
                            <div className="item-info">
                                {Number(module.showName) === 1 ? <div className="item-name" style={{ textAlign: module.nameAlign === 2 ? "center" : module.nameAlign === 3 ? "right" : "left" }}><a className="item-name-a">{item.productName}</a></div> : null}
                                {Number(module.showBrief) === 1 && briefs.length ? <div className="item-brief-list" style={{ backgroundColor: module.briefBackgroundColor }}>{briefs.map((b, bi) => (<span className="item-brief-tag" key={bi} style={{ color: module.briefStyle?.titleColor, fontSize: `${module.briefStyle?.titleFontSize ?? 11}px` }}>{b}</span>))}</div> : null}
                                {Number(module.showPrice) === 1 ? (<div className="item-action"><div className="item-price" style={{ textAlign: module.priceAlign === 2 ? "center" : module.priceAlign === 3 ? "right" : "left" }}><b className="price_format">¥{priceFormat(item.productPrice)}</b>{Number(module.showMarketPrice) === 1 && item.marketPrice ? <span className="market_price">¥{priceFormat(item.marketPrice)}</span> : null}</div><div className="item-buy">{module.buyBtnStyle ? <BuyButton styleOptions={module.buyBtnStyle} /> : null}</div></div>) : null}
                            </div>
                        </div></div></div>
                    ))}
                </div></div>
            </div>
        </div>
    );
}
