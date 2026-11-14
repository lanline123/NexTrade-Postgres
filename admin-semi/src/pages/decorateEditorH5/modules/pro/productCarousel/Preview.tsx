/**
 * 商品轮播 预览（类名对齐 uniapp-react productCarousel：pc-swiper/pc-cell/pc-photo/pc-info/pc-prices；露邻轮播）
 */
import { useMemo, useState } from "react";
import { Swiper, SwiperItem } from "@nutui/nutui-react";
import { cssText, imageFormat, priceFormat, pickPic } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { productCarouselDefaults } from "./index";
import { defaultProductCarouselDemoItems } from "../../../contract/sharedDefaults";
import "./Preview.css";

export default function ProductCarouselPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, productCarouselDefaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const list: any[] = module.productList?.length ? module.productList : defaultProductCarouselDemoItems;
    const [idx, setIdx] = useState(0);
    const bg = pickPic(module.moduleBgImage);
    return (
        <div className="module-ad-con module-product_carousel" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            {bg ? <img className="pc-module-bg" src={imageFormat(bg)} alt="" /> : null}
            <div className="pc-body" style={{ paddingTop: `${module.titleContentGap ?? 20}px` }}>
                {Number(module.showModuleTitle) === 1 && module.moduleTitle ? <div className="pc-module-title" style={{ color: module.moduleTitleColor }}>{module.moduleTitle}</div> : null}
                <div className="pc-swiper">
                    <Swiper autoplay={Number(module.autoplay) === 1 ? 3000 : 0} loop={list.length > 1} onChange={(i: number) => setIdx(i)} indicator={false}>
                        {list.map((item, i) => (
                            <SwiperItem key={i}>
                                <div className="pc-cell" style={{ width: `${module.cardWidthPercent ?? 80}%`, margin: "0 auto" }}>
                                    <div className="pc-photo" style={{ borderRadius: `${module.goodsRadius ?? 12}px`, boxShadow: module.cardStyle === "shadow" ? "0 4px 16px rgba(0,0,0,0.10)" : "none", background: "#fff", overflow: "hidden" }}>
                                        <div className="pc-img-box"><img className="pc-img pc-img-ratio" src={imageFormat(item.picThumb || item.picUrl || "")} alt="" style={{ width: "100%", aspectRatio: "1", display: "block", objectFit: "cover", borderRadius: Number(module.picBottomRadius) === 1 ? undefined : `${module.goodsRadius}px` }} /></div>
                                        <div className="pc-info" style={{ padding: `${module.innerPadding ?? 10}px`, textAlign: module.textAlign ?? "left" }}>
                                            {Number(module.showName) === 1 ? <div className="pc-name" style={{ color: module.productTitleColor, fontWeight: module.textWeight }}>{item.productName}</div> : null}
                                            {Number(module.showDesc) === 1 && item.productDesc ? <div className="pc-desc" style={{ color: module.descColor }}>{item.productDesc}</div> : null}
                                            <div className="pc-bottom">
                                                <div className="pc-prices">
                                                    {Number(module.showPrice) === 1 ? <span className="price-current" style={{ color: module.priceColor }}>¥{priceFormat(item.productPrice)}</span> : null}
                                                    {Number(module.showMarketPrice) === 1 && item.marketPrice ? <span className="price-market" style={{ color: module.marketPriceColor }}>¥{priceFormat(item.marketPrice)}</span> : null}
                                                </div>
                                                {Number(module.showSales) === 1 && item.sales ? <span className="pc-sales" style={{ color: module.salesColor }}>已售{item.sales}</span> : null}
                                                <span className="pc-buy">{module.buyBtnStyle?.text || "立即抢购"}</span>
                                            </div>
                                            <span style={{ display: "none" }}>{idx}</span>
                                        </div>
                                    </div>
                                </div>
                            </SwiperItem>
                        ))}
                    </Swiper>
                </div>
                <div className="pc-measure" />
            </div>
        </div>
    );
}
