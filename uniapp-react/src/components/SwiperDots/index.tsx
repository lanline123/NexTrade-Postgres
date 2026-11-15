import { View } from "@tarojs/components";

import "./index.scss";

interface Props {
    /** 总页数 */
    count: number;
    /** 当前页下标（0 起） */
    current: number;
    /**
     * 原版 tig-swiper 的 picPageType 指示点形态：
     * 1 = mode "default" 圆点；2 = mode "dot" 圆点；3 = mode "dot" + uni-swiper-dot-box 胶囊底盒
     */
    picPageType?: number;
    /** 未选中色（原 tig-swiper :color；picPageType==3 时各模块传 "#fff"） */
    color?: string;
    /** 选中色（原 tig-swiper :select-color，一般为模块 swiperPageColor） */
    selectColor?: string;
    /** 底部偏移（原 tig-swiper :bottom），不传时用样式表默认 20rpx */
    bottom?: string | number;
    /** 横条形态：仅 imageNav picPageType==1 使用（原版其 bullet 被模块 CSS 改为 24rpx×4rpx 横条） */
    variant?: "dots" | "bars";
}

/**
 * 自绘轮播指示点（替代 Taro 原生 indicatorDots）。
 * 原版各轮播模块经 tig-swiper 渲染 uni-swiper-dot：picPageType=1 有默认圆点、=3 有深色胶囊底盒；
 * 原生 indicatorDots 在 =1 时整体消失、=3 时只剩居中裸圆点，故按原版三式自绘（审计 S3）。
 * 仅展示、不拦截手势（点击跳页未还原，与 bannerA2 既有自绘指示点口径一致）。
 * 使用方需保证最近的定位祖先存在（可给直接父容器加 swiper-dots-anchor 类）。
 */
export default function SwiperDots({ count, current, picPageType = 1, color, selectColor, bottom, variant = "dots" }: Props) {
    if (!count) {
        return null;
    }
    const items = Array.from({ length: count });
    const rootStyle = bottom !== undefined ? { bottom: typeof bottom === "number" ? `${bottom}px` : bottom } : undefined;

    if (picPageType === 3) {
        // 原 App.vue:154-162 .uni-swiper-dot-box .uni-swiper__dots-box 胶囊底盒
        return (
            <View className="swiper-dots swiper-dots--capsule" style={rootStyle}>
                <View className="swiper-dots__box">
                    {items.map((_, i) => (
                        <View
                            key={i}
                            className="swiper-dots__dot"
                            style={{ backgroundColor: i === current ? selectColor : color || "#fff" }}
                        />
                    ))}
                </View>
            </View>
        );
    }

    if (variant === "bars") {
        // 原 imageNav index.vue:218-223：24rpx×4rpx 横条、未选 #000/opacity .2、选中 swiperPageColor
        return (
            <View className="swiper-dots swiper-dots--bars" style={rootStyle}>
                {items.map((_, i) => (
                    <View
                        key={i}
                        className={`swiper-dots__dot${i === current ? " active" : ""}`}
                        style={i === current ? { backgroundColor: selectColor } : undefined}
                    />
                ))}
            </View>
        );
    }

    // 圆点（picPageType 1/2）：16rpx 圆点、间距 0 8rpx（与 bannerA2 既有自绘指示点同口径）
    return (
        <View className="swiper-dots" style={rootStyle}>
            {items.map((_, i) => (
                <View
                    key={i}
                    className="swiper-dots__dot"
                    style={{ backgroundColor: i === current ? selectColor : color || "rgba(0, 0, 0, 0.2)" }}
                />
            ))}
        </View>
    );
}
