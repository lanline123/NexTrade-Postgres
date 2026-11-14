import { View } from "@tarojs/components";

import "./index.scss";

interface Props {
    /** 背景色（旧默认 #efefef） */
    bg?: string;
    width?: string;
    height?: string;
    /** 是否闪烁动画（旧 animated，class `shan`） */
    animated?: boolean;
}

/**
 * 骨架占位块（自旧 components/skeleton/index.vue 等价移植，补齐通用骨架屏入口）。
 * 此前全仓无通用骨架组件，仅图片/买家秀/分类导航有各自的局部占位样式；
 * 本组件按旧 props 提供单块占位，组合多块即可拼列表骨架。
 */
export default function Skeleton({ bg = "#efefef", width = "100px", height = "100px", animated = false }: Props) {
    return <View className={`skeleton-box${animated ? " shan" : ""}`} style={{ background: bg, width, height }} />;
}
