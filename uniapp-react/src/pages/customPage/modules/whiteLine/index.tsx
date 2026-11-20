import { useMemo } from "react";
import { View } from "@tarojs/components";

import { formatFrame } from "@/components/modules";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
}

export default function WhiteLine({ module }: Props) {
    const { frame } = module;

    // 分割类型口径与编辑器 whiteLineDefaults 对齐（2026-11-21 改口径）：仅 1=空白，
    // 其余（含存量缺字段）一律按 2=分割线。此前缺字段时类名落 ad-style__undefined，
    // 靠「未命中 __1 的 border:none」偶然显线，口径不可靠，现显式归一。
    const lineType = Number(module.lineType) === 1 ? 1 : 2;

    const frameFormat = useMemo(() => {
        return formatFrame(frame ?? {});
    }, [frame]);

    return (
        <View
            className={`module-ad-con  module-white_line ad-style__${lineType} ad-line_style__${module.lineStyle}`}
        >
            <View className="white-line-warp" style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom}>
                <View className="white-line-con" style={frameFormat.backgroundColor + " " + frameFormat.boxRadius}>
                    <View className="white-line-item">
                        <View className="item-content">
                            <View className="item-line" />
                        </View>
                    </View>
                </View>
            </View>
        </View>
    );
}
