import { useState } from "react";
import type { ReactNode } from "react";
import { Swiper, SwiperItem, View } from "@tarojs/components";

import "./TigSwiper.scss";

interface Props {
    /** 轮播数据（旧 modelValue） */
    list?: any[];
    /** 指示点选中色（旧 selectColor，优先于 color） */
    selectColor?: string;
    color?: string;
    /** 指示模式（旧 mode：dot） */
    mode?: string;
    height?: number | string;
    /** 指示点距底部（旧 bottom，px） */
    bottom?: number;
    dot?: boolean;
    displayMultipleItems?: number;
    interval?: number;
    autoplay?: boolean;
    previousMargin?: string;
    vertical?: boolean;
    duration?: number;
    onChange?: (index: number) => void;
    renderItem?: (item: any, index: number) => ReactNode;
    children?: ReactNode;
}

/**
 * 轮播（自旧 tig-swiper.vue 等价封装：uni-swiper-dot + swiper）。
 * 此前各装修模块均直接展开 Taro 原生 Swiper 并自理指示点；本组件按旧
 * props 收口（数据 list + renderItem，或直接传 SwiperItem children），
 * 指示点行为与旧版一致（dot 模式、selectColor 选中色）。
 */
export default function TigSwiper({
    list,
    selectColor = "",
    color = "",
    mode = "dot",
    height = 150,
    bottom = 10,
    dot = true,
    displayMultipleItems = 1,
    interval = 3000,
    autoplay = true,
    previousMargin = "0",
    vertical = false,
    duration = 500,
    onChange,
    renderItem,
    children
}: Props) {
    const [current, setCurrent] = useState(0);
    const items = list || [];
    const count = items.length || (children ? 1 : 0);
    const activeColor = selectColor || color || "#fff";

    return (
        <View className="tig-swiper" style={{ height: typeof height === "number" ? `${height}px` : height }}>
            <Swiper
                className="tig-swiper__body"
                current={current}
                autoplay={autoplay}
                interval={interval}
                duration={duration}
                circular={count > 1}
                vertical={vertical}
                displayMultipleItems={displayMultipleItems}
                previousMargin={previousMargin}
                onChange={(e) => {
                    setCurrent(e.detail.current);
                    onChange?.(e.detail.current);
                }}
            >
                {children
                    ? children
                    : items.map((item, index) => <SwiperItem key={index}>{renderItem ? renderItem(item, index) : null}</SwiperItem>)}
            </Swiper>
            {dot && mode === "dot" && count > 1 ? (
                <View className="tig-swiper__dots" style={{ bottom: `${bottom}px` }}>
                    {Array.from({ length: count }).map((_, i) => (
                        <View
                            key={i}
                            className="tig-swiper__dot"
                            style={i === current ? { background: activeColor, opacity: 1 } : undefined}
                        />
                    ))}
                </View>
            ) : null}
        </View>
    );
}
