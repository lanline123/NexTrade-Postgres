import { useEffect, useMemo, useState } from "react";
import { Image, Swiper, SwiperItem, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import { designPxToRpx, formatFrame } from "@/components/modules";
import SwiperDots from "@/components/SwiperDots";
import CommonTitle from "@/pages/customPage/modules/commonTitle/index";
import { redirect } from "@/utils";
import { imageFormat, urlFormat } from "@/utils/format";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
}

export default function ImageAd({ module }: Props) {
    const frameFormat = useMemo(() => {
        return formatFrame(module.frame ?? {});
    }, [module.frame]);

    const format = useMemo(() => {
        return {
            imgPadding: "padding:" + designPxToRpx(module?.imgPadding) + ";"
        };
    }, [module?.imgPadding]);

    const selectColor = useMemo(() => {
        return module.swiperPageColor || "#333";
    }, [module.swiperPageColor]);

    // 自绘指示点当前页（原 tig-swiper 内部 current，现由 onChange 回写驱动 SwiperDots）
    const [current, setCurrent] = useState(0);

    const columns = useMemo(() => {
        return module.swiperPreView || 1;
    }, [module.swiperPreView]);

    // 高度一律设备 px 口径（与 imageAd2 的 safeHeightPx 一致）：React style 对象里的 rpx 值在 H5 会被
    // 直接丢弃（字符串口径才走 rpx 转换层），宿主无定高会让整条 height:100% 链塌陷成 0×0（图面纯白）。
    // 初值 150rpx 按当前窗口宽换算成 px。
    const [swiperHeight, setSwiperHeight] = useState(() => Math.round((150 * Taro.getSystemInfoSync().windowWidth) / 750));
    const [loadEnd, setLoadEnd] = useState(false);

    const loadImage = (e: any) => {
        const { height, width } = e.detail;
        computeImgHeight(height, width);
    };

    const computeImgHeight = (height: number, width: number) => {
        const screenWidth = Taro.getSystemInfoSync().windowWidth;
        // 统一按宽高比计算，将图片等比例缩放到屏幕宽度后的高度（设备 px）
        const scaledHeight = (screenWidth * height) / width;
        // 取所有图片中的最大高度，确保所有图片都能完整显示
        setSwiperHeight((prev) => {
            if (scaledHeight > prev) {
                return scaledHeight;
            }
            return prev;
        });
        setLoadEnd(true);
    };

    const [swiperList, setSwiperList] = useState<any[]>([]);
    const getSwiperList = () => {
        const itemNum = module.swiperPreView || 1;
        const swiperListNum = Math.ceil(module.picList.length / itemNum);
        const next: any[] = [];
        for (let index = 0; index < swiperListNum; index++) {
            next.push(module.picList.slice(index * itemNum, (index + 1) * itemNum));
        }
        setSwiperList(next);
    };
    // 对应旧 watchEffect：多列预览时按 columns 把图片分组，每组一页轮播
    useEffect(() => {
        if (columns > 1 && module.picList && module.picList.length > 0) {
            getSwiperList();
        }
    }, [columns, module.picList]);

    useEffect(() => {
        const windowResizeCallback = () => {
            Taro.getImageInfo({
                src: imageFormat(module.picList?.[0]?.picUrl),
                success: (e: any) => {
                    computeImgHeight(e.height, e.width);
                }
            }).catch(() => {});
        };
        Taro.onWindowResize(windowResizeCallback);
        return () => {
            Taro.offWindowResize(windowResizeCallback);
        };
    }, [module]);

    if (!module.picList?.length) {
        return null;
    }

    return (
        <View
            className={`module-ad-con decorate-image-ad module-image_ad  ad-pic_type__${module.picType} ad-pic_page_type__${module.picPageType} ad-radio_style__${module.picRadioStyle} `}
            style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom}
        >
            <View className="module-ad-content" style={frameFormat.backgroundColor + " " + frameFormat.boxRadius}>
                {module.title.showTitle == 1 && <CommonTitle module={module.title} />}
                {module.picList && module.picList.length > 0 && (
                    <>
                        {module.picType == 2 ? (
                            <View className="image-ad-warp" style={frameFormat.innerPadding}>
                                <View className="image-ad-con swiper-dots-anchor">
                                    {columns == 1 ? (
                                        // 旧 tig-swiper（v-model="module.picList"）等价展开：
                                        // circular、autoplay 默认 true、interval 默认 3000、duration 默认 500、height 为设备 px（见 swiperHeight 注）；
                                        // 指示点改自绘 SwiperDots（原 uni-swiper-dot 三式），原生 indicatorDots 在 picPageType=1 时消失、=3 时无胶囊底盒
                                        <Swiper
                                            current={0}
                                            className={module.picPageType == 3 ? "uni-swiper-dot-box" : ""}
                                            style={{ height: swiperHeight + "px" }}
                                            circular
                                            indicatorDots={false}
                                            autoplay
                                            interval={3000}
                                            duration={500}
                                            onChange={(e: any) => setCurrent(e.detail.current)}
                                        >
                                            {module.picList.map((item: any, index: number) => (
                                                <SwiperItem key={index}>
                                                    <View style={{ height: swiperHeight + "px" }} className="image-ad-item">
                                                        <View
                                                            className="item-content"
                                                            style={format.imgPadding}
                                                            onClick={() => {
                                                                item.picLink && redirect({ url: urlFormat(item.picLink) });
                                                            }}
                                                        >
                                                            <View className="item-img-a">
                                                                <Image
                                                                    style={{ width: "100%", height: "100%" }}
                                                                    src={imageFormat(item.picUrl)}
                                                                    mode="aspectFit"
                                                                    onLoad={loadImage}
                                                                />
                                                            </View>
                                                        </View>
                                                    </View>
                                                </SwiperItem>
                                            ))}
                                        </Swiper>
                                    ) : (
                                        <Swiper
                                            current={0}
                                            className={module.picPageType == 3 ? "uni-swiper-dot-box" : ""}
                                            style={{ height: swiperHeight + "px" }}
                                            circular
                                            indicatorDots={false}
                                            autoplay
                                            interval={3000}
                                            duration={500}
                                            onChange={(e: any) => setCurrent(e.detail.current)}
                                        >
                                            {swiperList.map((item: any[], index: number) => (
                                                <SwiperItem key={index}>
                                                    <View
                                                        className="swiper-main"
                                                        style={{ height: swiperHeight + "px", gridTemplateColumns: `repeat(${columns}, 1fr)` }}
                                                    >
                                                        {item.map((subItem: any, subIndex: number) => (
                                                            <View key={subIndex} className="image-ad-item">
                                                                <View className="item-content" style={format.imgPadding}>
                                                                    <View className="item-img-a">
                                                                        <Image
                                                                            style={{ width: "100%", height: "100%" }}
                                                                            src={imageFormat(subItem.picUrl)}
                                                                            onLoad={loadImage}
                                                                        />
                                                                    </View>
                                                                </View>
                                                            </View>
                                                        ))}
                                                    </View>
                                                </SwiperItem>
                                            ))}
                                        </Swiper>
                                    )}
                                    <SwiperDots
                                        count={columns == 1 ? module.picList.length : swiperList.length}
                                        current={current}
                                        picPageType={module.picPageType}
                                        color={module.picPageType == 3 ? "#fff" : ""}
                                        selectColor={selectColor}
                                    />
                                </View>
                            </View>
                        ) : (
                            <View className="image-ad-warp" style={frameFormat.innerPadding}>
                                <View className="image-ad-con">
                                    {module.picList.map((pic: any, picIndex: number) => (
                                        <View className="image-ad-item" key={picIndex}>
                                            <View
                                                className="item-content"
                                                style={format.imgPadding}
                                                onClick={() => {
                                                    pic.picLink && redirect({ url: urlFormat(pic.picLink) });
                                                }}
                                            >
                                                <View className="item-img-a">
                                                    <Image className="item-img" src={imageFormat(pic.picUrl)} mode="widthFix" />
                                                </View>
                                            </View>
                                        </View>
                                    ))}
                                </View>
                            </View>
                        )}
                    </>
                )}
            </View>
        </View>
    );
}
