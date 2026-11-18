import { useMemo } from "react";
import { View } from "@tarojs/components";

import { copyArray, designPxToRpx, formatFrame } from "@/components/modules";
import TigImage from "@/components/tig/tig-image/TigImage";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
}

export default function Notice({ module }: Props) {
    const { frame } = module;

    const frameFormat = useMemo(() => {
        return formatFrame(frame ?? {});
    }, [frame]);

    const itemFormat = useMemo(() => {
        return {
            itemBackgroundColor: `background-color:${module?.frame?.itemBackgroundColor};`,
            itemRadius: `border-radius:${designPxToRpx(module?.frame?.itemRadius)};`,
            itemHeight: `height:${designPxToRpx(module?.frame?.itemHeight)};`,
            textColor: `color:${module?.frame?.textColor};`
        };
    }, [module]);

    // 滚动时复制文本列表
    const list = useMemo(() => {
        if (module?.text) {
            return copyArray(module.text, 3);
        }
        return [];
    }, [module?.text]);

    return (
        <View
            className="module-ad-con module-notice"
            style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom}
        >
            <View className="notice-ad-warp">
                <View className="notice-con" style={frameFormat.backgroundColor + " " + frameFormat.boxRadius}>
                    <View className="notice-item">
                        <View className="item-content">
                            <View
                                className="item-notice-con"
                                style={itemFormat.itemBackgroundColor + itemFormat.itemHeight + itemFormat.itemRadius}
                            >
                                {/* 未配置公告图标时不渲染（空 src 会显示裂图，2026-11-17 实测） */}
                                {module.icoPic?.picUrl ? (
                                    <View className="item-notice-ico">
                                        <TigImage src={module.icoPic.picUrl} mode="widthFix" />
                                    </View>
                                ) : null}
                                <View className="item-notice-list">
                                    {module.nociceEffect === "scroll" && (
                                        <View className="item-notice-input scroll-animation" style={itemFormat.textColor}>
                                            {list.map((item, index) => (
                                                <View className="headlines-item" key={index}>
                                                    {t(item)}
                                                </View>
                                            ))}
                                        </View>
                                    )}
                                    {module.nociceEffect === "still" && (
                                        <View className="item-notice-input" style={itemFormat.textColor}>
                                            <View className="headlines-item">{t(module?.text ?? "")}</View>
                                        </View>
                                    )}
                                </View>
                            </View>
                        </View>
                    </View>
                </View>
            </View>
        </View>
    );
}
