import { useRef } from "react";
import type { ReactNode } from "react";
import { View } from "@tarojs/components";

interface Props {
    /** 触发滑动回调的最小位移(px)，默认 40 */
    threshold?: number;
    onSlide?: (direction: "left" | "right") => void;
    onSlideLeft?: () => void;
    onSlideRight?: () => void;
    children?: ReactNode;
}

/**
 * 滑动容器（自旧 tig-slide 等价移植）。
 * 旧 .vue 本体是空壳（仅 <view><slot/></view>），实际滑动逻辑在
 * index.wxs（微信小程序 WXS 手势模块）里，WXS 无法移植到 Taro/React；
 * 本实现以 touch 事件等价承接其手势回调语义：水平滑动超阈值后
 * 按方向回调一次，纵向滚动不拦截。
 */
export default function TigSlide({ threshold = 40, onSlide, onSlideLeft, onSlideRight, children }: Props) {
    const startRef = useRef<{ x: number; y: number } | null>(null);
    const firedRef = useRef(false);

    return (
        <View
            onTouchStart={(e) => {
                const t = (e as any).touches[0];
                startRef.current = { x: t.clientX, y: t.clientY };
                firedRef.current = false;
            }}
            onTouchMove={(e) => {
                if (!startRef.current || firedRef.current) return;
                const t = (e as any).touches[0];
                const dx = t.clientX - startRef.current.x;
                const dy = t.clientY - startRef.current.y;
                if (Math.abs(dx) < threshold || Math.abs(dx) <= Math.abs(dy)) return;
                firedRef.current = true;
                const direction = dx < 0 ? "left" : "right";
                onSlide?.(direction);
                if (direction === "left") onSlideLeft?.();
                else onSlideRight?.();
            }}
            onTouchEnd={() => {
                startRef.current = null;
            }}
        >
            {children}
        </View>
    );
}
