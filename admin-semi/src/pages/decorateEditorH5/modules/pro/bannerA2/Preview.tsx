/**
 * 首焦轮播A2 预览（移植自 view/admin editor/modules/mobilePro/bannerA2/Preview.vue）
 *
 * 背景 = 当前轮播图放大模糊 + 渐变遮罩；顶部定位 + 搜索框；下方圆角轮播。
 */
import { useMemo, useState } from "react";
import { Swiper, SwiperItem } from "@nutui/nutui-react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { bannerA2Defaults } from "./index";
import "./Preview.css";

export default function BannerA2Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, bannerA2Defaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {});
    const bannerContent = module.bannerContent || {};
    const bannerStyle = module.bannerStyle || {};
    const picList: any[] = bannerContent.picList || [];
    const [realIndex, setRealIndex] = useState(0);
    const bgUrl = picList[realIndex]?.picUrl || picList[0]?.picUrl || "";

    // 原版 searchStyle：仅 height/border-radius/background-color 三项内联（其余走 CSS）
    const searchStyle: React.CSSProperties = {
        height: `${bannerStyle.searchHeight ?? 34}px`,
        borderRadius: `${bannerStyle.searchRadius ?? 20}px`,
        backgroundColor: bannerStyle.searchBackgroundColor || undefined,
    };
    // 原版 searchTnputStyle：height/line-height = searchHeight、color/background 取 bannerStyle
    const searchInputStyle: React.CSSProperties = {
        height: `${bannerStyle.searchHeight ?? 34}px`,
        lineHeight: `${bannerStyle.searchHeight ?? 34}px`,
        color: bannerStyle.searchFontColor || undefined,
        backgroundColor: bannerStyle.searchBackgroundColor || undefined,
    };

    return (
        <div className="module-ad-con ad-pic_type__2">
            {bgUrl ? (
                <>
                    <div className="banner-bg">
                        <img src={imageFormat(bgUrl)} alt="" />
                    </div>
                    <div className="bg-gradient" />
                </>
            ) : null}
            <div
                style={{
                    ...cssText(
                        commonStyle.moduleStyle.boxPaddingBottom,
                        commonStyle.moduleStyle.boxPaddingTop,
                        commonStyle.moduleStyle.boxPadding,
                        commonStyle.moduleStyle.backgroundSize,
                        commonStyle.moduleStyle.backgroundImage,
                    ),
                    position: "relative",
                }}
            >
                <div className="module-ad-content" style={{ marginTop: `${module.moduleStyle?.bannerMarginTop ?? 0}px` }}>
                    <div className="Banner">
                        <div className="bar">
                            <div className="search-top">
                                <div className="area-select-box">
                                    <div>北京市</div>
                                    {/* 原版 icon-a-gengduo20：9px、rotate(90deg) 的更多字形，以等形 SVG 承接 */}
                                    <svg className="area-more-icon" width="9" height="9" viewBox="0 0 10 10" style={{ transform: "rotate(90deg)", flexShrink: 0 }} aria-hidden="true">
                                        <path d="M3 1 L7 5 L3 9" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
                                    </svg>
                                </div>
                                {Number(module.searchContent?.showSearch) === 1 ? (
                                    <div className="search" style={searchStyle}>
                                        <div className="search-icon" style={{ color: bannerStyle.searchIconColor }}>
                                            {/* 原版 iconfont-h5 icon-sousuo1 搜索字形，以等形 SVG 承接 */}
                                            <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true" style={{ display: "block" }}>
                                                <circle cx="10.5" cy="10.5" r="6.5" fill="none" stroke="currentColor" strokeWidth="2" />
                                                <path d="M15.3 15.3 L21 21" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                                            </svg>
                                        </div>
                                        <form className="search-form" action="" onSubmit={(e) => e.preventDefault()}>
                                            <div className="search-input" style={searchInputStyle}>
                                                <div className="search-placeholder">{module.searchContent?.searchText}</div>
                                            </div>
                                        </form>
                                    </div>
                                ) : null}
                            </div>
                        </div>
                        <div className="image-ad-warp">
                            <div
                                className={`image-ad-con pagination-bullet${bannerContent.picPageType}`}
                                style={{ borderRadius: `${bannerContent.picRadius}px`, overflow: "hidden" }}
                            >
                                <Swiper
                                    autoplay={3000}
                                    loop={picList.length > 1}
                                    onChange={(i: number) => setRealIndex(i)}
                                    indicator={false}
                                >
                                    {picList.map((item, i) => (
                                        <SwiperItem key={i}>
                                            <div className="image-ad-item" style={{ borderRadius: `${bannerContent.picRadius}px`, overflow: "hidden" }}>
                                                <img className="item-img" src={imageFormat(item.picUrl)} alt="" />
                                            </div>
                                        </SwiperItem>
                                    ))}
                                </Swiper>
                                {picList.length > 1 ? (
                                    <div className="banner-a2-dots">
                                        {picList.map((_, i) => (
                                            <i
                                                key={i}
                                                className={i === realIndex ? "on" : ""}
                                                style={{
                                                    // 原版 Swiper 分页色：激活 = swiperPageColor（兜底 #fff），默认 = indicatorColor（兜底 rgba(0,0,0,0.2)）
                                                    background: i === realIndex ? bannerContent.swiperPageColor || "#fff" : bannerContent.indicatorColor || "rgba(0, 0, 0, 0.2)",
                                                }}
                                            />
                                        ))}
                                    </div>
                                ) : null}
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
