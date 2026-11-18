import { useMemo } from "react";
import { Image, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import { useTabbarStore } from "@/store/tabbar";
import { useConfigStore } from "@/store/config";
import { staticResource } from "@/utils";

import "./index.scss";

interface Props {
    right?: number;
    bottom?: number;
    duration?: number;
    scrollTop?: number | string;
    zIndex?: number | string;
    /** uview u-back-top 的 mode 字段，视觉统一为圆形按钮（保留 prop 兼容旧调用） */
    mode?: string;
    iconStyle?: Record<string, any>;
    customStyle?: Record<string, any>;
}

/**
 * 返回顶部（自旧 tig/tig-back-top/tig-back-top.vue 移植）：
 * - 旧实现基于 uview u-back-top，滚动超过一屏后出现，点击回到顶部；
 * - uview 源码不在本仓库内，按钮视觉按其默认圆形图标样式复刻。
 */
export default function TigBackTop({
    right = 15,
    bottom = 20,
    duration = 500,
    scrollTop = 0,
    zIndex = 999,
    mode: _mode = "circle",
    iconStyle = { fontSize: "32rpx", color: "#333" },
    customStyle = { backgroundColor: "#fff" }
}: Props) {
    const tabbarHeightNum = useTabbarStore((s) => s.tabbarHeightNum);
    const safeBottom = useConfigStore((s) => s.safeAreaInsets.bottom);

    // 对应旧 computed styleFormat
    // H5 下 right 改按窗口宽换算成 px：rpx 字符串在 H5 内联样式里一旦转换失效，
    // 按钮会失去定位落到左边缘半出屏遮挡正文（2026-11-17 实测）；bottom 各分量做数值兜底防 NaNpx
    const styleFormat = useMemo(() => {
        const bottomPx = (Number(bottom) || 0) + (Number(tabbarHeightNum) || 0) + (Number(safeBottom) || 0);
        if (process.env.TARO_ENV === "h5") {
            let windowWidth = 375;
            try {
                windowWidth = Taro.getSystemInfoSync().windowWidth || 375;
            } catch {
                // 取不到窗口宽时按设计宽兜底
            }
            return {
                right: `${Math.round(((Number(right) || 0) * windowWidth) / 750)}px`,
                bottom: `${bottomPx}px`
            };
        }
        return {
            right: `${right}rpx`,
            bottom: `${bottomPx}px`
        };
    }, [right, bottom, tabbarHeightNum, safeBottom]);

    // uview 默认行为：滚动超过一屏后显示
    const show = useMemo(() => Number(scrollTop) > 500, [scrollTop]);
    if (!show) return null;

    const handleClick = () => {
        Taro.pageScrollTo({ scrollTop: 0, duration });
    };

    return (
        <View className="tig-back-top" style={{ ...styleFormat, zIndex, ...customStyle } as any} onClick={handleClick}>
            {/* 本仓库无 uview 内置上箭头资源，复用右箭头图旋转 -90° 呈现向上箭头 */}
            <Image className="tig-back-top__icon" src={staticResource("common/right_arrows.png")} mode="widthFix" style={iconStyle as any} />
        </View>
    );
}
