import { useEffect, useMemo, useRef, useState } from "react";
import { Image, Swiper, SwiperItem, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import { designPxToRpx, formatFrame } from "@/components/modules";
import SwiperDots from "@/components/SwiperDots";
import { redirect, getElementRect } from "@/utils";
import { imageFormat, urlFormat } from "@/utils/format";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
}

export default function ImageNav({ module }: Props) {
    // 原版 View.vue 口径：渲染前先 mergeDefaultModule 把缺失字段按契约默认值
    // 补齐（navType/navStyle/rowNum/colNum/picList/imgPadding/radioStyle/
    // picPageType/swiperPageColor），其后才裸读。本组件此前未合并：数据缺
    // picList 且 navType≠1 时 getSwiperList 读 picList.length 直接抛错、被模块
    // 错误边界降级为「渲染失败」占位；rowNum/colNum 缺失时 itemNum=0 还会让
    // 分页循环跑飞。合并只补 undefined/null/""（与原版 mergeDefaultModule 同
    // 口径），picList 非数组时一并归一为空数组。
    const mod = useMemo(() => {
        const merged: any = { ...(module ?? {}) };
        const defaults: Record<string, any> = {
            navType: 1,
            navStyle: 1,
            rowNum: 4,
            colNum: 2,
            picList: [],
            imgPadding: 8,
            radioStyle: 1,
            picPageType: 1,
            swiperPageColor: ""
        };
        for (const k of Object.keys(defaults)) {
            if (merged[k] === undefined || merged[k] === null || merged[k] === "") {
                merged[k] = defaults[k];
            }
        }
        if (!Array.isArray(merged.picList)) merged.picList = [];
        return merged;
    }, [module]);
    const { frame } = mod;
    const instanceRef = useRef<any>(null);

    const frameFormat = useMemo(() => {
        return formatFrame(frame ?? {});
    }, [frame]);

    const imageGap = useMemo(() => designPxToRpx(mod.imgPadding), [mod.imgPadding]);

    const selectColor = useMemo(() => {
        return mod.swiperPageColor || "#333";
    }, [mod.swiperPageColor]);

    // 自绘指示点当前页（原 tig-swiper 内部 current，现由 onChange 回写驱动 SwiperDots）
    const [current, setCurrent] = useState(0);

    const [swiperList, setSwiperList] = useState<any[]>([]);
    const getSwiperList = () => {
        if (mod.navType == 1) return;
        const itemNum = mod.rowNum && mod.colNum ? mod.rowNum * mod.colNum : 0;
        // rowNum/colNum 被显式写 0 时 itemNum=0：原版此处会算出 Infinity 页数
        // 循环跑飞，归一为单页全量（编辑器下拉只产 3-6 / 1-3，此为数据兜底）
        if (itemNum <= 0) {
            setSwiperList(mod.picList.length > 0 ? [mod.picList] : []);
            return;
        }
        const swiperListNum = Math.ceil(mod.picList.length / itemNum);
        const next: any[] = [];
        for (let index = 0; index < swiperListNum; index++) {
            next.push(mod.picList.slice(index * itemNum, (index + 1) * itemNum));
        }
        setSwiperList(next);
    };
    // 对应旧 watchEffect：按 rowNum * colNum 把图片分组，每组一页轮播
    useEffect(() => {
        getSwiperList();
    }, [mod.navType, mod.rowNum, mod.colNum, mod.picList]);

    const [swiperHeight, setSwiperHeight] = useState(150);

    const gteSwiperHeight = async () => {
        try {
            const res = await getElementRect(".swiper-main", instanceRef.current);
            if (res) {
                setSwiperHeight(res.height);
            }
        } catch (error) {
            console.error(error);
        }
    };

    useEffect(() => {
        // 对应旧 onMounted + nextTick
        const timer = setTimeout(() => {
            gteSwiperHeight();
        }, 0);
        return () => clearTimeout(timer);
    }, []);

    useEffect(() => {
        const windowResizeCallback = () => {
            gteSwiperHeight();
        };
        Taro.onWindowResize(windowResizeCallback);
        return () => {
            Taro.offWindowResize(windowResizeCallback);
        };
    }, []);

    if (!(mod.picList && mod.picList.length > 0)) {
        return null;
    }

    return (
        <View
            ref={instanceRef as any}
            className={`module-ad-con module-ad-con- module-image_nav ad-nav_type__${mod.navType} ad-nav_style__${mod.navStyle} ad-row_num__${mod.rowNum} ad-col_num__${mod.colNum} ad-pic_page_type__${mod.picPageType} ad-radio_style__${mod.radioStyle}`}
            style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom}
        >
            <View className="module-ad-content" style={frameFormat.innerPadding + frameFormat.backgroundColor + frameFormat.boxRadius}>
                {mod.navType == 2 ? (
                    <View className="image-ad-warp">
                        <View className="image-ad-con swiper-dots-anchor">
                            {/* 旧 tig-swiper（v-model="swiperList"）等价展开：
                                circular、autoplay 默认 true、interval 默认 3000、duration 默认 500、height 为字符串时按原单位使用；
                                指示点改自绘 SwiperDots（picPageType=1 为原版横条式、=3 为胶囊底盒），原生 indicatorDots 在 =1 时消失、=3 时无底盒 */}
                            <Swiper
                                current={0}
                                className={mod.picPageType == 3 ? "uni-swiper-dot-box" : ""}
                                style={{ height: `${swiperHeight}px`, "--swiper-page-color": selectColor } as Record<string, string | number>}
                                circular
                                indicatorDots={false}
                                autoplay
                                interval={3000}
                                duration={500}
                                onChange={(e: any) => setCurrent(e.detail.current)}
                            >
                                {swiperList.map((item: any[], index: number) => (
                                    <SwiperItem key={index}>
                                        <View className="swiper-main" style={{ flexWrap: mod.colNum > 1 ? "wrap" : "nowrap" }}>
                                            {item.map((subItem: any, subIndex: number) => (
                                                <View
                                                    key={subIndex}
                                                    className="swiper-item"
                                                    style={{ flexGrow: 0, flexBasis: `${100 / mod.rowNum}%`, padding: designPxToRpx(mod.imgPadding) }}
                                                    onClick={() => {
                                                        subItem.picLink && redirect({ url: urlFormat(subItem.picLink, subItem.picTitle) });
                                                    }}
                                                >
                                                    <View className="swiper-item-box" style={`background-image: url(${imageFormat(subItem.picUrl)});`} />
                                                    {subItem.picTitle && mod.navStyle !== 2 && (
                                                        <View className="imagenav-item-text">{t(subItem.picTitle)}</View>
                                                    )}
                                                </View>
                                            ))}
                                        </View>
                                    </SwiperItem>
                                ))}
                            </Swiper>
                            <SwiperDots
                                count={swiperList.length}
                                current={current}
                                picPageType={mod.picPageType}
                                color={mod.picPageType == 3 ? "#fff" : ""}
                                selectColor={selectColor}
                                variant={mod.picPageType == 1 ? "bars" : "dots"}
                            />
                        </View>
                    </View>
                ) : (
                    <View
                        className="imagenav-main"
                        // 原 scoped 样式中的 v-bind("mod.rowNum") / v-bind("mod.colNum") / v-bind("imageGap") 改为内联样式
                        style={{
                            gridTemplateColumns: `repeat(${mod.rowNum}, 1fr)`,
                            gridTemplateRows: `repeat(${mod.colNum}, 1fr)`,
                            gridGap: imageGap
                        }}
                    >
                        {mod.picList.map((item: any, index: number) => (
                            <View
                                key={index}
                                className="imagenav-main-item"
                                onClick={() => {
                                    item.picLink && redirect({ url: urlFormat(item.picLink, item.picTitle) });
                                }}
                            >
                                <View className="item-img-a">
                                    {mod.navStyle == 2 || mod.navStyle == 1 ? (
                                        <Image
                                            className={`imagenav-item-img ${mod.navStyle === 2 ? "img-height" : ""}`}
                                            src={imageFormat(item.picUrl)}
                                            mode="widthFix"
                                        />
                                    ) : null}
                                </View>
                                <View className="item-text-a">
                                    {item.picTitle && mod.navStyle != 2 && (
                                        <View className="imagenav-item-text line1">{t(item.picTitle ?? "")}</View>
                                    )}
                                </View>
                            </View>
                        ))}
                    </View>
                )}
            </View>
        </View>
    );
}
