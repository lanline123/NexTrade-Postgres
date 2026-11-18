/**
 * 首焦轮播 预览（移植自 view/admin editor/modules/mobilePro/banner/Preview.vue）
 *
 * 商城头部型：弧度渐变层（随当前轮播图 gradientColorA/B 与 gradientType/height/backgroundRadius）+
 * logo + 搜索条（菜单图标/搜索框/签到图标）+ 热词行 + Swiper 轮播（指示色 = swiperPageColor）。
 * 菜单/签到默认图标取原版 defaultIconImage 的 OSS 图（menu 7 款 / sign 4 款），自定义图优先。
 * 搜索图标为 iconfont-h5 icon-sousuo1，以等尺寸内联 SVG 承接。
 * 轮播定高（2026-11-18）：原版自定义 Swiper 按内容自适应高度，NutUI Swiper 不定高会
 * 随图片自然高度漂移——改按首图比例定高（与前台 useImgInfo 同口径）+ cover 填充。
 */
import { useMemo, useState } from "react";
import { Swiper, SwiperItem } from "@nutui/nutui-react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle, gradientMap } from "../../../contract/sharedDefaults";
import { bannerDefaults } from "./index";
import "./Preview.css";

const MENU_ICONS = [
    "https://oss.tigshop.com/img/gallery/202411/1731044058k3D2oAM6v4j1NUqnd4.png",
    "https://oss.tigshop.com/img/gallery/202411/1731044058yv4ngGH3BtQalK2Ews.png",
    "https://oss.tigshop.com/img/gallery/202411/17310440580AWfxX5dX8tq2iC91X.png",
    "https://oss.tigshop.com/img/gallery/202411/1731044058oBfYyYzrYBK3daMTiS.png",
    "https://oss.tigshop.com/img/gallery/202411/17310440582tapohBoU1mliQxGgT.png",
    "https://oss.tigshop.com/img/gallery/202411/1731044058Xxnsx3an6igjmg9OZh.png",
    "https://oss.tigshop.com/img/gallery/202411/1731044060czIoPbXd4pO0GteMVZ.png"
];
const SIGN_ICONS = [
    "https://oss.tigshop.com/img/gallery/202411/1731044060WUMf13Ls2YfzTFHVYR.png",
    "https://oss.tigshop.com/img/gallery/202411/17310440604yFr67HdqrIIk5UCzm.png",
    "https://oss.tigshop.com/img/gallery/202411/1731044060gYalEs84jlX6pMBE0p.png",
    "https://oss.tigshop.com/img/gallery/202411/1731044060cZffgle3HfRaQtiH3V.png"
];

function SearchIcon() {
    return (
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true" style={{ display: "block" }}>
            <circle cx="10.5" cy="10.5" r="6.5" fill="none" stroke="currentColor" strokeWidth="2" />
            <path d="M15.3 15.3 L21 21" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
    );
}

export default function BannerPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, bannerDefaults);
        if (merged.bannerStyle && (merged.bannerStyle.showSearchBtn === undefined || merged.bannerStyle.showSearchBtn === null)) {
            merged.bannerStyle.showSearchBtn = 1;
        }
        return merged;
    }, [data]);
    const [realIndex, setRealIndex] = useState(0);
    // 轮播区定高（2026-11-18 修复）：NutUI Swiper 无定高时按每张图的自然高度反复重测，
    // 方图/竖图会把模块撑到 600px+ 且位置随自动播放漂移、指示点被挤出视野。
    // 与前台（uniapp-react useImgInfo）同口径：以首图宽高比定高，测到前用原版演示图
    // 比例 750/360 兜底；图片以 cover 填满定高区，不再随图变形。
    const [imgRatio, setImgRatio] = useState(750 / 360);
    const boxPadding = Number(module.moduleStyle?.boxPadding ?? 10) || 0;
    const swiperHeight = Math.min(375, Math.max(80, Math.round((375 - boxPadding * 2) / imgRatio)));

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {});
    const bannerContent = module.bannerContent || {};
    const bannerStyle = module.bannerStyle || {};
    const searchContent = module.searchContent || {};
    const navContent = module.navContent || {};
    const picList: any[] = Array.isArray(bannerContent.picList) ? bannerContent.picList : [];
    const current = picList[realIndex] || picList[0] || {};
    const iconSize = Number(bannerStyle.searchIconSize) || 24;

    const menuIconUrl = searchContent.menuPic?.picUrl
        ? searchContent.menuPic.picUrl
        : MENU_ICONS[Number(searchContent.menuIcon) - 1] || "";
    const signIconUrl = searchContent.signPic?.picUrl
        ? searchContent.signPic.picUrl
        : SIGN_ICONS[Number(searchContent.signIcon) - 1] || "";

    const circleStyle: React.CSSProperties = {
        backgroundImage: `linear-gradient(${gradientMap[bannerContent.gradientType] || "to bottom"},${current.gradientColorA},${current.gradientColorB})`,
        height: `${bannerContent.height}px`,
        borderRadius: `0px 0px ${bannerContent.backgroundRadius}% ${bannerContent.backgroundRadius}%`
    };
    const searchHeight = Number(bannerStyle.searchHeight) || 34;

    return (
        <div
            className={`module-ad-con ad-pic_type__2 ad-picPageType__${bannerContent.picPageType} h5-banner`}
            style={{
                ...cssText(
                    commonStyle.moduleStyle.boxPaddingBottom,
                    commonStyle.moduleStyle.boxPaddingTop,
                    commonStyle.moduleStyle.boxPadding,
                    commonStyle.moduleStyle.backgroundSize,
                    commonStyle.moduleStyle.backgroundImage
                ),
                marginTop: `${module.moduleStyle?.bannerMarginTop ?? 0}px`,
                ["--h5-banner-page-color" as any]: bannerContent.swiperPageColor || "#fff",
                ["--h5-banner-swiper-h" as any]: `${swiperHeight}px`
            }}
        >
            <div className="module-ad-content">
                <div className="Banner">
                    <div className="circle" style={circleStyle} />
                    <div className="bar" style={{ margin: "0px 3px 10px" }}>
                        {module.logoPic?.picUrl ? (
                            <img className="catnav-logo" style={{ height: `${module.logoHeight}px` }} src={imageFormat(module.logoPic.picUrl)} alt="" />
                        ) : null}
                        <div className="search-top">
                            {searchContent.menuIcon ? (
                                <div className="menu">
                                    {menuIconUrl ? <img className="menu-icon" src={imageFormat(menuIconUrl)} style={{ width: iconSize, height: iconSize }} alt="" /> : null}
                                </div>
                            ) : null}
                            {searchContent.showSearch ? (
                                <div
                                    className="search"
                                    style={{ height: `${searchHeight}px`, borderRadius: `${bannerStyle.searchRadius}px`, backgroundColor: bannerStyle.searchBackgroundColor }}
                                >
                                    <div className="search-icon" style={{ color: bannerStyle.searchFontColor }}>
                                        <SearchIcon />
                                    </div>
                                    <form className="search-form" action="" onSubmit={(e) => e.preventDefault()}>
                                        <div className="search-input" style={{ height: `${searchHeight}px`, lineHeight: `${searchHeight}px`, color: bannerStyle.searchFontColor }}>
                                            <div className="search-placeholder">{searchContent.searchText}</div>
                                        </div>
                                        {bannerStyle.showSearchBtn ? (
                                            <div
                                                className="search-btn"
                                                style={{
                                                    height: `${searchHeight - 6}px`,
                                                    lineHeight: `${searchHeight - 6}px`,
                                                    color: "#fff",
                                                    backgroundColor: current.gradientColorA,
                                                    borderRadius: `${bannerStyle.searchRadius}px`,
                                                    right: 3,
                                                    top: 3
                                                }}
                                            >
                                                {bannerStyle.searchText}
                                            </div>
                                        ) : null}
                                    </form>
                                </div>
                            ) : null}
                            {searchContent.signIcon ? (
                                <div className="sign">
                                    {signIconUrl ? <img className="sign-icon" src={imageFormat(signIconUrl)} style={{ width: iconSize, height: iconSize }} alt="" /> : null}
                                </div>
                            ) : null}
                        </div>
                        {navContent.showNav ? (
                            <div
                                className="search-word"
                                style={{ marginTop: 8, justifyContent: bannerStyle.textAlignment, fontSize: `${bannerStyle.navFontSize}px` }}
                            >
                                <div className="tit" style={{ color: bannerStyle.navFontColor, fontWeight: bannerStyle.navFontBold }}>
                                    {bannerStyle.navPrefix}
                                </div>
                                <ul className="word-list">
                                    {(navContent.navList || []).map((item: any, i: number) => (
                                        <li className="item" key={i}>
                                            <div
                                                className="word"
                                                style={{
                                                    margin: `0px ${bannerStyle.navLabelMargin}px`,
                                                    color: bannerStyle.navFontColor,
                                                    backgroundColor: bannerStyle.navBackgroundColor,
                                                    padding: "0px 8px",
                                                    borderRadius: 20,
                                                    fontWeight: bannerStyle.navFontBold
                                                }}
                                            >
                                                {item.picTitle}
                                            </div>
                                        </li>
                                    ))}
                                </ul>
                            </div>
                        ) : null}
                    </div>
                    <div className="image-ad-warp">
                        <div className="image-ad-con">
                            {picList.length > 0 ? (
                                <Swiper autoplay={3000} loop={picList.length > 1} onChange={(i: number) => setRealIndex(i)} indicator={false}>
                                    {picList.map((item, i) => (
                                        <SwiperItem key={i}>
                                            <div className="image-ad-item" style={{ borderRadius: `${bannerContent.picRadius}px`, overflow: "hidden" }}>
                                                <img
                                                    className="item-img"
                                                    src={imageFormat(item.picUrl)}
                                                    alt=""
                                                    onLoad={
                                                        i === 0
                                                            ? (e) => {
                                                                  const img = e.currentTarget;
                                                                  if (img.naturalWidth > 0 && img.naturalHeight > 0) {
                                                                      setImgRatio(img.naturalWidth / img.naturalHeight);
                                                                  }
                                                              }
                                                            : undefined
                                                    }
                                                />
                                            </div>
                                        </SwiperItem>
                                    ))}
                                </Swiper>
                            ) : null}
                            {picList.length > 1 ? (
                                <div className="h5-banner-dots">
                                    {picList.map((_: any, i: number) => (
                                        <i key={i} className={i === realIndex ? "on" : ""} />
                                    ))}
                                </div>
                            ) : null}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
