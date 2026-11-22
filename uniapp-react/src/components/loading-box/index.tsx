import { View, Image } from "@tarojs/components";

import { staticResource } from "@/utils";

import "./index.scss";

interface Props {
    /** 是否处于触底加载中（对应旧 v-model） */
    modelValue: boolean;
    page: number;
    length: number;
    color?: string;
    /** 已全部加载完毕（列表非空且不在加载中）：渲染「没有更多了」终态行 */
    finished?: boolean;
    /** 首屏/首段加载也显示指示（默认 false：保持旧口径仅 page>1 触底时显示） */
    showFirstPage?: boolean;
}

/** 触底加载指示（自旧 components/loading-box/loading-box.vue 移植） */
export default function LoadingBox({ modelValue, page, length, color = "", finished = false, showFirstPage = false }: Props) {
    void color;
    if (modelValue && (page > 1 || showFirstPage)) {
        return (
            <View className="loading-box">
                <View className="bottomLoading">
                    <Image lazyLoad className="loading" src={staticResource("common/loading.gif")} />
                </View>
            </View>
        );
    }
    if (finished && !modelValue && length > 0) {
        return (
            <View className="loading-box">
                <View className="finished-text">没有更多了</View>
            </View>
        );
    }
    return null;
}
