import type { ReactNode } from "react";
import { View } from "@tarojs/components";

import { useConfigStore } from "@/store/config";
import { getPx } from "@/utils";

import "./index.scss";

interface Props {
    /** 导航栏高度（旧默认 45，经 getPx 换算；有胶囊时以胶囊高度+4 为准，与旧 computed 一致） */
    height?: number | string;
    fixed?: boolean;
    background?: string;
    /** fixed 时是否渲染同高占位块（旧默认 true） */
    placeholder?: boolean;
    /** 是否需要状态栏占位与胶囊避让（旧默认 true） */
    needStatusBar?: boolean;
    offsetTop?: number;
    zIndex?: number | string;
    children?: ReactNode;
}

/**
 * 内容区导航容器（自旧 components/navbarContent/index.vue 整体移植，恢复共享组件）。
 * 此前同逻辑有两处内联副本：bannerA2 模块内的本地 NavbarContent 与
 * cart 的 CartHeard（文件内 TODO 注明等本组件整体移植），结构与计算口径以
 * 旧组件为准：statusBarTop = 胶囊 top-2（回落安全区 top）；
 * navbarHeight = 有胶囊时胶囊高+4，否则 getPx(height)；胶囊右侧避让 = 胶囊宽+5。
 */
export default function NavbarContent({
    height = 45,
    fixed = false,
    background = "transparent",
    placeholder = true,
    needStatusBar = true,
    offsetTop = 0,
    zIndex,
    children
}: Props) {
    const menuButtonInfo = useConfigStore((s) => s.menuButtonInfo);
    const safeAreaInsets = useConfigStore((s) => s.safeAreaInsets);

    const statusBarTop = menuButtonInfo.top ? menuButtonInfo.top - 2 : safeAreaInsets.top;
    const navbarHeight = menuButtonInfo.top ? menuButtonInfo.height + 4 : Number(getPx(height as any, false));
    const paddingRight = menuButtonInfo.width ? menuButtonInfo.width + 5 : 0;

    const totalHeight = (needStatusBar ? statusBarTop : 0) + navbarHeight + offsetTop;

    return (
        <>
            <View
                className="navbar-box"
                style={{
                    position: fixed ? "fixed" : "static",
                    top: fixed ? `${offsetTop}px` : undefined,
                    left: fixed ? 0 : undefined,
                    right: fixed ? 0 : undefined,
                    background,
                    zIndex
                }}
            >
                {needStatusBar && <View className="status-bar" style={{ paddingTop: `${statusBarTop}px` }} />}
                <View
                    className="navbar"
                    style={{ height: `${navbarHeight}px`, paddingRight: needStatusBar ? `${paddingRight}px` : undefined }}
                >
                    {children}
                </View>
            </View>
            {fixed && placeholder ? <View style={{ height: `${totalHeight}px` }} /> : null}
        </>
    );
}
