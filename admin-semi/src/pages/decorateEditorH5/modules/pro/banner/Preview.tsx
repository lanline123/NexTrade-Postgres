/**
 * 轮播Banner 预览（类名对齐 uniapp-react banner：banner/image-ad-warp/image-ad-con/image-ad-item/search）
 */
import { useMemo, useState } from "react";
import { Swiper, SwiperItem } from "@nutui/nutui-react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { bannerDefaults } from "./index";
import "./Preview.css";

export default function BannerPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, bannerDefaults);
        return merged;
    }, [data]);
    const commonStyle = formatCommonStyle(module.moduleStyle ?? {});
    const picList: any[] = module.bannerContent?.picList || [];
    const [realIndex, setRealIndex] = useState(0);
    const cur = picList[realIndex] || picList[0] || {};
    return (
        <div className={`banner module-ad-con ad-pic_type__2 ad-pic_page_type__${module.bannerContent?.picPageType ?? 1}`} style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content">
                <div className="banner">
                    <div className="circle" style={{ backgroundImage: cur.gradientColorA ? `linear-gradient(to right, ${cur.gradientColorA}, ${cur.gradientColorB || cur.gradientColorA})` : undefined }} />
                    {Number(module.searchContent?.showSearch) === 1 ? (
                        <div className="search-top">
                            <div className="search" style={{ backgroundColor: module.bannerStyle?.searchBackgroundColor, height: `${module.bannerStyle?.searchHeight ?? 34}px`, borderRadius: `${module.bannerStyle?.searchRadius ?? 20}px` }}>
                                <span className="search-icon" style={{ color: module.bannerStyle?.searchIconColor }}>🔍</span>
                                <div className="search-input" style={{ color: module.bannerStyle?.searchFontColor }}>{module.searchContent?.searchText}</div>
                                <div className="search-btn">搜索</div>
                            </div>
                        </div>
                    ) : null}
                    {module.logoPic?.picUrl ? <img className="catnav-logo" src={imageFormat(module.logoPic.picUrl)} style={{ height: `${module.logoHeight ?? 32}px` }} alt="" /> : null}
                    <div className="image-ad-warp">
                        <div className="image-ad-con" style={{ borderRadius: `${module.bannerContent?.picRadius ?? 0}px`, overflow: "hidden" }}>
                            {picList.length === 0 ? (
                                <div className="module-ad-empty empty-image_ad"><div className="image-empty-bg" /><div className="desc">点击添加图片</div></div>
                            ) : (
                                <Swiper autoplay={3000} loop={picList.length > 1} onChange={(i: number) => setRealIndex(i)} indicator={false}>
                                    {picList.map((item, i) => (
                                        <SwiperItem key={i}><div className="image-ad-item"><img className="item-img" src={imageFormat(item.picUrl)} alt="" /></div></SwiperItem>
                                    ))}
                                </Swiper>
                            )}
                            {picList.length > 1 ? <div className="banner-dots">{picList.map((_, i) => (<i key={i} className={i === realIndex ? "on" : ""} style={{ background: i === realIndex ? module.swiperPageColor || "#fff" : "rgba(255,255,255,0.5)" }} />))}</div> : null}
                        </div>
                    </div>
                    {Number(module.navContent?.showNav) === 1 && Array.isArray(module.navContent?.wordList) ? (
                        <div className="word-list" style={{ justifyContent: module.bannerStyle?.textAlignment }}>{module.navContent.wordList.map((w: any, i: number) => (<span className="word" key={i}><span className="tit" style={{ color: module.bannerStyle?.navFontColor, backgroundColor: module.bannerStyle?.navBackgroundColor, fontWeight: module.bannerStyle?.navFontBold, fontSize: `${module.bannerStyle?.navFontSize ?? 12}px` }}>{w.title || w}</span></span>))}</div>
                    ) : null}
                </div>
            </div>
        </div>
    );
}
