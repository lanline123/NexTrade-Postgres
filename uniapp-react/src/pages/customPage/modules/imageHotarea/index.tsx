import { useMemo } from "react";
import { Text, View } from "@tarojs/components";

import { designPxToRpx, formatFrame } from "@/components/modules";
import TigImage from "@/components/tig/tig-image/TigImage";
import CommonTitle from "@/pages/customPage/modules/commonTitle/index";
import { redirect } from "@/utils";
import { urlFormat } from "@/utils/format";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
}

/**
 * 是否处于后台编辑器嵌入态（decoratePreview iframe）。
 * 2026-11-20 症状三：热图未选图时本模块此前无任何在流占位——空 picList 只剩
 * frame padding、空 picUrl 条目 TigImage 塌成 0 高，编辑器 decorate-layout
 * 如实上报 ~0 高，覆盖槽位与下一模块挤在同一带、视觉重叠。按 PC 编辑器
 * HotareaPreview 口径，仅在嵌入态补在流占位（空态块 / 40% 比例块）；
 * 前台真实访问（非 iframe）行为与此前完全一致，不显示编辑占位。
 */
const inEditorEmbed = (): boolean => {
    try {
        if (typeof window === "undefined") return false;
        return window.self !== window.top || window.location.href.indexOf("decoratePreview") >= 0;
    } catch {
        return false;
    }
};

export default function ImageHotarea({ module }: Props) {
    const frameFormat = useMemo(() => {
        return formatFrame(module.frame ?? {});
    }, [module.frame]);

    const format = useMemo(() => {
        return {
            imgPadding: "padding:" + designPxToRpx(module?.imgPadding) + ";"
        };
    }, [module?.imgPadding]);

    const embed = inEditorEmbed();
    const picListEmpty = !module.picList || module.picList.length === 0;

    function convertPxToRpx(position: string): string {
        return position.replace(/(\d+\.?\d*)px/g, (match, num) => {
            return parseFloat(num) * 2 + "rpx";
        });
    }

    return (
        <View
            className={`module-ad-con decorate-image-hotarea module-ad-con- ad-radio_style__${module.radioStyle}`}
            style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom}
        >
            <View className="module-ad-content" style={frameFormat.backgroundColor + " " + frameFormat.boxRadius}>
                {module.title.showTitle == 1 && <CommonTitle module={module.title} />}
                {embed && picListEmpty ? (
                    /* 空态在流占位（PC 口径：paddingTop 30 / paddingBottom 50 / #f2f4f6 /
                       100×100 占位盒 + 文案，总高约 210 设计 px） */
                    <View className="image-hotarea-empty">
                        <View className="image-hotarea-empty__box" />
                        <Text className="image-hotarea-empty__text">点击添加热图绘制</Text>
                    </View>
                ) : null}
                {module.picList && (
                    <View className="image-ad-warp image-hotarea-warp" style={frameFormat.innerPadding}>
                        <View className="image-ad-con">
                            {module.picList.map((item: any, itemIndex: number) => (
                                <View className="image-ad-item" key={item.picId ?? itemIndex}>
                                    <View className="item-content" style={format.imgPadding}>
                                        <View className="item-content-hotatrea">
                                            {/* TigImage 不接收 className，旧 <tig-image class="item-img-bg"> 的样式由外层 View 承接 */}
                                            {embed && !item.picUrl ? (
                                                /* 未选图条目在流比例占位（PC 同口径 paddingTop:40%） */
                                                <View className="item-img-bg image-hotarea-img-placeholder" />
                                            ) : (
                                                <View className="item-img-bg">
                                                    <TigImage src={item.picUrl} mode="widthFix" />
                                                </View>
                                            )}
                                            {item.hotarea &&
                                                item.hotarea.map((subItem: any, index: number) => (
                                                    <View
                                                        key={index}
                                                        className="image-hotarea-link"
                                                        style={convertPxToRpx(subItem.position)}
                                                        onClick={() => {
                                                            subItem.link && redirect({ url: urlFormat(subItem.link) });
                                                        }}
                                                    >
                                                        <Text className="area_box_con_text">{subItem.link?.name ?? ""}</Text>
                                                    </View>
                                                ))}
                                        </View>
                                    </View>
                                </View>
                            ))}
                        </View>
                    </View>
                )}
            </View>
        </View>
    );
}
