import { useEffect, useMemo, useRef, useState } from "react";
import { Image, Swiper, SwiperItem, Text, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import ImageSearchEntry from "@/components/imageSearchEntry";
import SwiperDots from "@/components/SwiperDots";
import { designPxToDevicePx, designPxToRpx, formatCommonStyle, gradientMap } from "@/components/modules";
import TigSelectLang from "@/components/tig/tig-select-lang/TigSelectLang";
import { useImgInfo } from "@/hooks/useDecorate";
import { useSaveTopBoxHeight, useScrollTop } from "@/hooks/index";
import { useConfigStore } from "@/store/config";
import { useImageSearchStore } from "@/store/imageSearch";
import { redirect, getElementRect, isOverseas } from "@/utils";
import { imageFormat, urlFormat } from "@/utils/format";
import { t } from "@/utils/i18n";

import { LANG_PARK_ICON, MENU_PARK_ICONS, SIGN_PARK_ICONS, parkIconSrc, resolveBannerIcon } from "./parkIcons";
import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
}

// 菜单/签到默认图标已改走 IconPark 内嵌字形（见 ./parkIcons.ts，2026-11-19 用户拍板路线）；
// 原 defaultIconImage 的 tigshop 图库 PNG 不再引用（主机改写后文件不在我方桶、必然 404）。

export default function Banner({ module, index }: Props) {
    // React 版用组件根节点 ref 替代 Vue getCurrentInstance()，供选择器查询圈定范围
    const rootRef = useRef<any>(null);

    const logoFormat = useMemo(() => {
        return {
            logoHeight: "height:" + designPxToRpx(module.logoHeight) + ";",
            logoPic: module.logoPic
        };
    }, [module.logoHeight, module.logoPic]);

    const { scrollTop } = useScrollTop();
    // usePageScroll 回调闭包拿不到最新值，用 ref 镜像给异步测量回调读取
    const scrollTopRef = useRef(0);
    scrollTopRef.current = scrollTop;

    const { height: saveHeight, statusBarHeight } = useSaveTopBoxHeight();
    const saveHeightNum = useMemo(() => {
        return logoFormat.logoPic?.picUrl ? statusBarHeight : saveHeight;
    }, [logoFormat.logoPic, statusBarHeight, saveHeight]);

    const configStore = useConfigStore();
    /** 海外版 mainHeader 占位高度：吸顶栏 top 与阈值都要让开 */
    const headerOffset = useMemo(() => (isOverseas() ? configStore.commonHeaderHeight || 0 : 0), [configStore.commonHeaderHeight]);

    // 以图搜图可用性：原版读 imageSearchStore.available（store 已迁移，入口组件挂载时自动拉取开关）
    const imageSearchAvailable = useImageSearchStore((s) => s.available);

    const { imgratio, height, getImgratio, getHeght, updateImgInfo } = useImgInfo();

    const isCeiling = useMemo(() => module.bannerStyle?.effectType == "ceiling", [module.bannerStyle]);
    /** 未测到前用 Infinity，避免阈值默认 0 导致「动一点就吸顶」 */
    const [ceilingThreshold, setCeilingThreshold] = useState(Number.POSITIVE_INFINITY);

    const fixed = useMemo(() => {
        if (!isCeiling) {
            return false;
        }
        return scrollTop > ceilingThreshold;
    }, [isCeiling, scrollTop, ceilingThreshold]);

    const measureCeilingThreshold = async () => {
        if (!isCeiling) {
            setCeilingThreshold(Number.POSITIVE_INFINITY);
            return;
        }
        // Vue nextTick → 等 DOM 提交后再测（setTimeout 0）
        await new Promise((resolve) => setTimeout(resolve, 0));
        // 吸顶栏自上而下：mainHeader + moduleStyle.boxPaddingTop + saveHeight（安全区）+ 搜索栏
        const boxPaddingTop = designPxToDevicePx(Number(module.moduleStyle?.boxPaddingTop) || 0, Taro.getWindowInfo().windowWidth);
        const safePad = index == 0 ? Number(saveHeight) || 0 : 0;
        const fixedSearchOffset = headerOffset + boxPaddingTop + safePad;
        const searchRes = await getElementRect(".banner-search-anchor", rootRef.current);
        if (searchRes && typeof searchRes.top === "number") {
            // 常驻搜索滚到与吸顶后搜索同一位置时再 fix（含装修 padding-top，避免「搜索框挨边才吸顶」）
            setCeilingThreshold(Math.max(0, scrollTopRef.current + searchRes.top - fixedSearchOffset));
            return;
        }
        // 测量失败：回落为吸顶偏移本身，绝不回落 0
        setCeilingThreshold(Math.max(1, fixedSearchOffset || Number(statusBarHeight) || 40));
    };

    const [realIndex, setRealIndex] = useState(0);
    const selectColor = useMemo(() => {
        return module.swiperPageColor || "#fff";
    }, [module.swiperPageColor]);
    const commonStyle = useMemo(() => {
        return formatCommonStyle(module.moduleStyle);
    }, [module.moduleStyle]);

    const swiperChange = (e: any) => {
        setRealIndex(e);
    };

    // 取图（原版口径）：自定义图 picUrl 优先，否则按样式编号取 IconPark 字形；null = 不渲染
    const menuChoice = useMemo(
        () => resolveBannerIcon(module.searchContent?.menuIcon, module.searchContent?.menuPic?.picUrl, MENU_PARK_ICONS),
        [module.searchContent]
    );
    const menuIconSrc = menuChoice ? (menuChoice.type === "image" ? imageFormat(menuChoice.url) : parkIconSrc(menuChoice.name)) : "";

    const signChoice = useMemo(
        () => resolveBannerIcon(module.searchContent?.signIcon, module.searchContent?.signPic?.picUrl, SIGN_PARK_ICONS),
        [module.searchContent]
    );
    const signIconSrc = signChoice ? (signChoice.type === "image" ? imageFormat(signChoice.url) : parkIconSrc(signChoice.name)) : "";

    const circleStyle = useMemo(() => {
        const designHeight = designPxToRpx(module.bannerContent.height);
        const backgroundHeight = designHeight && index == 0 ? `calc(${designHeight} + ${Number(saveHeight)}px)` : designHeight;
        return `background-image: linear-gradient(${gradientMap[module.bannerContent.gradientType]} ,${module.bannerContent.picList[realIndex]?.gradientColorA},${
            module.bannerContent.picList[realIndex].gradientColorB
        });height:${backgroundHeight}; border-radius: 0 0 ${module.bannerContent.backgroundRadius}% ${module.bannerContent.backgroundRadius}%;`;
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [module.bannerContent, realIndex, index, saveHeight]);

    const searchStyle = useMemo(() => {
        return `height: ${designPxToRpx(module.bannerStyle?.searchHeight)}; border-radius: ${designPxToRpx(module.bannerStyle?.searchRadius)}; background-color: ${
            module.bannerStyle?.searchBackgroundColor
        }`;
    }, [module.bannerStyle]);

    const searchTnputStyle = useMemo(() => {
        return `height: ${designPxToRpx(module.bannerStyle?.searchHeight)}; line-height: ${designPxToRpx(module.bannerStyle?.searchHeight)}; color: ${
            module.bannerStyle?.searchFontColor
        };`;
    }, [module.bannerStyle]);

    const searchBtnStyle = useMemo(() => {
        return `height: ${designPxToRpx(module.bannerStyle?.searchHeight - 6)}; line-height: ${designPxToRpx(
            module.bannerStyle?.searchHeight - 6
        )}; color: #fff; background-color: ${module.bannerContent.picList[realIndex]?.gradientColorA};border-radius: ${designPxToRpx(
            module.bannerStyle?.searchRadius
        )};right:6rpx;top:6rpx`;
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [module.bannerStyle, module.bannerContent, realIndex]);

    const searchWordStyle = useMemo(() => {
        return `margin-top: 16rpx; justify-content: ${module.bannerStyle?.textAlignment}; font-size: ${designPxToRpx(module.bannerStyle?.navFontSize)}`;
    }, [module.bannerStyle]);

    const wordStyle = useMemo(() => {
        const labelMargin = designPxToRpx(module.bannerStyle?.navLabelMargin);
        return `${labelMargin ? `margin: 0 ${labelMargin};` : ""}color: ${module.bannerStyle?.navFontColor}; background-color: ${
            module.bannerStyle?.navBackgroundColor
        };padding: 0 16rpx;border-radius: 40rpx;font-weight: ${module.bannerStyle?.navFontBold};`;
    }, [module.bannerStyle]);

    // 旧版通过 selectLangRef 调用 tig-select-lang 的 handleclickLang 打开语言弹层（show-lang-btn=false 隐藏内置触发器）
    const selectLangRef = useRef<any>(null);
    const handleclickLang = () => {
        selectLangRef.current && selectLangRef.current.handleclickLang && selectLangRef.current.handleclickLang();
    };

    const windowResizeCallback = async () => {
        await updateImgInfo(module.bannerContent.picList[0].picUrl, ".image-ad-item", rootRef);
        // 仅在页顶附近重测，避免滚动中重测把阈值算歪
        if (scrollTopRef.current <= 2) {
            await measureCeilingThreshold();
        }
    };

    // 旧 useImgInfo 默认 immediate：挂载时先测比例再测高度
    useEffect(() => {
        updateImgInfo(module.bannerContent.picList[0].picUrl, ".image-ad-item", rootRef);
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // uni.onWindowResize → Taro.onWindowResize；onUnmounted 清理
    useEffect(() => {
        Taro.onWindowResize(windowResizeCallback);
        return () => {
            Taro.offWindowResize(windowResizeCallback);
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // 对应旧 onMounted：先测吸顶阈值，布局未稳定时补测一次
    useEffect(() => {
        measureCeilingThreshold();
        const timer = setTimeout(() => {
            measureCeilingThreshold();
        }, 100);
        return () => {
            clearTimeout(timer);
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // 对应旧 watch(headerOffset)：mainHeader 异步测高后，吸顶阈值与 top 需重算
    useEffect(() => {
        if (isCeiling) {
            measureCeilingThreshold();
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [headerOffset]);

    // 常驻栏视觉（logo/搜索/导航词）在原版与吸顶副本中逐行重复，这里同样保留两份 JSX；
    // 常驻（非吸顶）栏的 search-top 挂 banner-search-anchor 类，作为吸顶阈值测量锚点
    const renderSearchTop = (fixedBar: boolean) => (
        <View className={fixedBar ? "search-top" : "search-top banner-search-anchor"}>
            {/* 原版真值门控（v-if="searchContent.menuIcon"）：0 隐藏，1–7 样式与 'custom' 都显示；
                移植时曾被收窄为 == 1，换样式即消失。现由 resolveBannerIcon 统一判定：
                有可渲染图标（自定义图或编号字形）才渲染，天然避开 React 数值 0 印字问题 */}
            {menuChoice && (
                <View
                    className="menu"
                    onClick={() => {
                        urlFormat(module.searchContent.menuLink) && redirect({ url: urlFormat(module.searchContent.menuLink) });
                    }}
                >
                    <Image
                        className="menu-icon"
                        src={menuIconSrc}
                        style={`width: ${designPxToRpx(module.bannerStyle?.searchIconSize)};height:${designPxToRpx(module.bannerStyle?.searchIconSize)}`}
                    />
                </View>
            )}
            {module.searchContent?.showSearch == 1 && (
                <View
                    className="search"
                    style={searchStyle}
                    onClick={() => {
                        redirect({ url: "/pages/search/index" });
                    }}
                >
                    <View className="search-icon" style={`color:${module.bannerStyle?.searchFontColor}`}>
                        <Text className="iconfont-h5 icon-sousuo1" />
                    </View>
                    <View className={`search-form ${imageSearchAvailable ? "search-form--with-entry" : ""}`}>
                        <View className="search-input" style={searchTnputStyle}>
                            <View className="search-placeholder">{t(module.searchContent?.searchText)}</View>
                        </View>
                        <ImageSearchEntry color={module.bannerStyle?.searchFontColor} />
                        {(module.bannerStyle?.showSearchBtn ?? 1) == 1 && (
                            <View className="search-btn" style={searchBtnStyle}>
                                {t(module.bannerStyle?.searchText)}
                            </View>
                        )}
                    </View>
                </View>
            )}
            {signChoice && (
                <View
                    className="sign"
                    onClick={() => {
                        urlFormat(module.searchContent.signLink) && redirect({ url: urlFormat(module.searchContent.signLink) });
                    }}
                >
                    <Image
                        className="sign-icon"
                        src={signIconSrc}
                        style={`width: ${designPxToRpx(module.bannerStyle?.searchIconSize - 2)};height:${designPxToRpx(module.bannerStyle?.searchIconSize - 2)}`}
                    />
                </View>
            )}
            {/* 多语言地球【我方新增消费】：管理端新增 searchContent.showLang 开关（原版无此字段）；
                存量数据无此字段，必须 ?? 1 缺省显示，只有显式 0 才隐藏 */}
            {isOverseas() && (module.searchContent?.showLang ?? 1) == 1 && (
                // 旧版点击 globe 调 selectLangRef.handleclickLang（tig-select-lang 已移植 imperative 句柄）
                // 地球字形改 IconPark earth 内嵌 SVG（布局数值仍按 TigSelectLang 对齐，见 index.scss）
                <View onClick={handleclickLang}>
                    <View className="lang-icon-box">
                        <View className="park-earth">
                            <Image className="park-earth-img" src={parkIconSrc(LANG_PARK_ICON)} />
                            <Text className="iconfont-h5 icon-sanjiaoright" />
                        </View>
                    </View>
                </View>
            )}
        </View>
    );

    const renderSearchWord = () => (
        <View className="search-word" style={searchWordStyle}>
            <View className="tit" style={`color: ${module.bannerStyle?.navFontColor};font-weight: ${module.bannerStyle?.navFontBold};`}>
                {t(module.bannerStyle?.navPrefix)}
            </View>
            <View className="word-list">
                {(module.navContent?.navList || []).map((item: any, itemIndex: number) => (
                    <View
                        className="item"
                        key={item.picTitle ?? itemIndex}
                        onClick={() => {
                            urlFormat(item.picLink) && redirect({ url: urlFormat(item.picLink) });
                        }}
                    >
                        <View className="word" style={wordStyle}>
                            {t(item.picTitle ?? "")}
                        </View>
                    </View>
                ))}
            </View>
        </View>
    );

    const renderSwiper = () => (
        <View className="image-ad-warp">
            <View className="image-ad-con swiper-dots-anchor">
                {/* 旧 tig-swiper（v-model="module.bannerContent.picList"）等价展开：
                    circular、autoplay 默认 true、interval 默认 3000、duration 默认 500；height 为 px 字符串。
                    指示点改自绘 SwiperDots（原 uni-swiper-dot 三式）：原生 indicatorDots 在 picPageType=1 时消失、=3 时无胶囊底盒 */}
                <Swiper
                    current={0}
                    className={module.bannerContent.picPageType == 3 ? "uni-swiper-dot-box" : ""}
                    style={{ height: height + "px" }}
                    circular
                    indicatorDots={false}
                    autoplay
                    interval={3000}
                    duration={500}
                    onChange={(e: any) => swiperChange(e.detail.current)}
                >
                    {module.bannerContent.picList.map((item: any, picIndex: number) => (
                        <SwiperItem key={picIndex}>
                            <View
                                className="image-ad-item"
                                style={`border-radius: ${designPxToRpx(module.bannerContent.picRadius)};overflow: hidden;`}
                                onClick={() => {
                                    item.picLink && redirect({ url: urlFormat(item.picLink) });
                                }}
                            >
                                <View
                                    className="item-content"
                                    style={`background-image:url(${imageFormat(item.picUrl)});padding-top:${imgratio * 100}%;`}
                                />
                            </View>
                        </SwiperItem>
                    ))}
                </Swiper>
                <SwiperDots
                    count={module.bannerContent.picList.length}
                    current={realIndex}
                    picPageType={module.bannerContent.picPageType}
                    color={module.bannerContent.picPageType == 3 ? "#fff" : ""}
                    selectColor={selectColor}
                />
            </View>
        </View>
    );

    return (
        <View
            ref={rootRef}
            className={`banner module-ad-con decorate-banner ad-pic_type__2  ad-pic_page_type__${module.bannerContent.picPageType}`}
            style={
                commonStyle.moduleStyle.boxPaddingBottom +
                commonStyle.moduleStyle.boxPaddingTop +
                commonStyle.moduleStyle.boxPadding +
                commonStyle.moduleStyle.backgroundSize +
                commonStyle.moduleStyle.backgroundImage +
                `margin-top:${designPxToRpx(module.moduleStyle?.bannerMarginTop)};`
            }
        >
            <View className="module-ad-content">
                <View className="banner">
                    <View className="circle" style={circleStyle} />
                    {/* 吸顶开启时隐藏常驻栏视觉，保留占位，避免与吸顶副本叠成双头 */}
                    <View className={`bar ${fixed ? "is-ceiling-hidden" : ""}`}>
                        <View style={`padding-top: ${index == 0 ? saveHeightNum : 0}px`} />
                        {logoFormat.logoPic?.picUrl && (
                            <Image className="catnav-logo" mode="aspectFit" style={logoFormat.logoHeight} src={imageFormat(logoFormat.logoPic?.picUrl || "")} />
                        )}
                        {renderSearchTop(false)}
                        {module.navContent?.showNav == 1 && renderSearchWord()}
                    </View>

                    <View
                        className={`bar ${fixed ? "fixed" : ""}`}
                        style={
                            commonStyle.moduleStyle.boxPaddingTop +
                            commonStyle.moduleStyle.boxPadding +
                            `z-index: ${fixed ? 99 : 9};display:${fixed ? "block" : "none"};` +
                            (fixed ? `top:${headerOffset}px;` : "")
                        }
                    >
                        <View style={`padding-top: ${index == 0 ? saveHeight : 0}px`} />
                        <View className="circle" style={circleStyle} />
                        {/* 旧版此处为 <form action="">，Taro 无 form 组件等价改为 View */}
                        {renderSearchTop(true)}
                        {module.navContent?.showNav == 1 && renderSearchWord()}
                    </View>

                    {renderSwiper()}
                </View>
            </View>
            {/* 旧 <tig-select-lang ref="selectLangRef" :show-lang-btn="false" />：弹层挂载，内置触发器隐藏 */}
            {isOverseas() && <TigSelectLang ref={selectLangRef} showLangBtn={false} />}
        </View>
    );
}
