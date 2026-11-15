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
}

export default function CommonTitle({ module }: Props) {
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
        return null;
    }

    return (
        <View
            className={`image-ad-title ad-title_style__${module.titleStyle}`}
            style={titleFormat.titleBackground + titleFormat.titleRadius}
        >
            <View className="image-ad-title-l">
                {titleBackgroundImage && (
                    <View className="title-pic-box">
                        <Image className="title-pic" src={titleBackgroundImage} />
                    </View>
                )}
                <View className="image-ad-title-h" style={"color: " + module.titleColor}>
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
