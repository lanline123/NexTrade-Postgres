import { View } from "@tarojs/components";

import TigImage from "@/components/tig/tig-image/TigImage";
import { imageFormat } from "@/utils/format";

import "./imageBox.scss";

interface Props {
    src?: string;
}

export default function ImageBox({ src }: Props) {
    return (
        // image-square-box 为本组件专属作用域类：原 imageBox.vue 的 scoped 样式靠它收回作用域，
        // 避免 .item-img{opacity:0} 全局泄漏到 picType 1-4 分支（见 imageBox.scss）
        <View className="item-img-a image-square-box">
            <View className="item-img-bg" style={`background-image: url(${imageFormat(src ?? "")});`} />
            {/* TigImage 不接收 className，旧 <tig-image class="item-img"> 的样式由外层 View 承接 */}
            <View className="item-img">
                <TigImage mode="widthFix" src={src} />
            </View>
        </View>
    );
}
