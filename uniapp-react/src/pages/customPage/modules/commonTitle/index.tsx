import { useMemo } from "react";
import { Image, Text, View } from "@tarojs/components";

import { designPxToRpx } from "@/components/modules";
import { redirect } from "@/utils";
import { productImage, urlFormat } from "@/utils/format";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
    /** 独立模块形态（分发器按 type 顶层分发时注入）：消费 titleAlign/titleWeight；各模块内嵌直引不传、渲染口径与改造前逐字一致 */
    standaloneTitle?: boolean;
    /** 编辑器嵌入预览态（仅 decoratePreview 经分发器注入）：独立形态 showTitle 关闭时出占位条；店面恒不传、恒零输出 */
    editorPreview?: boolean;
}

export default function CommonTitle({ module, standaloneTitle, editorPreview }: Props) {
    const titleBackgroundImage = useMemo(() => productImage(module?.titleBackgroundPic), [module?.titleBackgroundPic]);
    const titleFormat = useMemo(() => {
        const radius = designPxToRpx(module.titleRadius);
        let backgroundText = module.titleBackground2
            ? module.titleBackground2
                ? `background:linear-gradient(to right, ${module.titleBackground}, ${module.titleBackground2});`
                : ""
            : module.titleBackground
              ? "background-color:" + module.titleBackground + ";"
              : "";
        return {
            titleBackground: backgroundText,
            titleRadius: radius ? `border-radius:${radius} ${radius} 0 0;` : ""
        };
    }, [module]);

    if (module.showTitle != 1) {
        // 店面（含全部内嵌场景）恒零输出；仅编辑器嵌入预览里的独立形态出浅灰
        // 占位条，让隐藏的标题条在真实渲染画布上仍有槽位、可点选找回。
        if (standaloneTitle && editorPreview) {
            return (
                <View className="decorate-common-title-hidden">
                    <Text>{t("标题条已隐藏——在设置中开启「显示标题栏」恢复")}</Text>
                </View>
            );
        }
        return null;
    }

    return (
        <View
            className={`image-ad-title decorate-common-title ad-title_style__${module.titleStyle}${standaloneTitle ? ` ad-title_align__${module.titleAlign ?? 1}` : ""}`}
            style={titleFormat.titleBackground + titleFormat.titleRadius}
        >
            <View className="image-ad-title-l">
                {titleBackgroundImage && (
                    <View className="title-pic-box">
                        <Image className="title-pic" src={titleBackgroundImage} />
                    </View>
                )}
                <View
                    className="image-ad-title-h"
                    style={"color: " + module.titleColor + (standaloneTitle ? ";font-weight: " + (module.titleWeight || "normal") : "")}
                >
                    {t(module.titleText ?? "")}
                </View>
                <View className="image-ad-title-d" style={"color: " + module.descColor}>
                    {t(module.descText ?? "")}
                </View>
            </View>
            {/* 原版 commonTitle 无插槽出口：右侧只有 showMore==1 时的默认「查看更多」（旧 seckill2 传入的 #more 倒计时从未渲染，P1 已撤） */}
            {module.showMore == 1 && (
                <View className="image-ad-title-r">
                    <View
                        className="more-link"
                        style={"color: " + module.moreColor}
                        onClick={() => {
                            module.moreLink && redirect({ url: urlFormat(module.moreLink) });
                        }}
                    >
                        <Text>{t("查看更多")}</Text>
                        <Text className="iconfont-h5 icon-youjiantou" />
                    </View>
                </View>
            )}
        </View>
    );
}
