import { useMemo } from "react";
import { Image, View } from "@tarojs/components";

import { designPxToRpx, gradientMap } from "@/components/modules";
import { useConfigStore } from "@/store/config";
import { resolveObjBuyButton } from "@/utils/buyStyle";
import { imageFormat, pickPic } from "@/utils/format";
import { t } from "@/utils/i18n";

import "./index.scss";

interface StyleOptions {
    showBtn?: number; // 是否显示按钮
    followGlobal?: number; // 1 = 跟随全局「购买按钮样式」的对象型字段组（读时解析，不回写）
    btnType?: number; // 按钮类型
    text?: string; // 按钮文字
    textColor?: string; // 按钮文字颜色
    iconColor?: string; // 按钮图标颜色
    gradientType?: string; // 按钮背景渐变类型
    gradientColorA?: string; // 按钮背景渐变颜色A
    gradientColorB?: string; // 按钮背景渐变颜色B
    radius?: number; // 按钮圆角
    paddingTopBottom?: number; // 按钮高度
    paddingLeftRight?: number; // 按钮宽度
    iconSize?: number;
    textSize?: number;
    iconImage?: any;
}

interface Props {
    styleOptions?: StyleOptions;
}

/** 通用购买按钮（自旧 commonBuyButton/index.vue 移植，供 productA1 等装修模块复用） */
export default function CommonBuyButton({
    styleOptions = {
        showBtn: 1, // 是否显示按钮
        btnType: 1, // 按钮类型
        text: "", // 按钮文字
        textColor: "#ffffff", // 按钮文字颜色
        iconColor: "#ffffff", // 按钮图标颜色
        gradientType: "leftRight", // 按钮背景渐变类型
        gradientColorA: "#333333", // 按钮背景渐变颜色A
        gradientColorB: "#333333", // 按钮背景渐变颜色B
        radius: 16, // 按钮圆角
        paddingTopBottom: 0, // 按钮高度
        paddingLeftRight: 10 // 按钮宽度
    }
}: Props) {
    // 跟随全局解析（读时、不回写模块数据）：followGlobal=1 时 13 个外观字段取
    // 全局 objStyle，showBtn/wrap/wrapAlign 等模块语境字段保留模块值；未开启
    // 跟随时 resolveObjBuyButton 原样返回入参，渲染与此前逐字一致。全局配置
    // 未拉到时回落默认组（= 出厂行为），到达后自动重渲。
    const globalBuyStyle = useConfigStore((s) => s.buyStyle);
    const options = resolveObjBuyButton(styleOptions, globalBuyStyle);
    // 新增字段对历史草稿做兜底：旧 buyBtnStyle 对象可能缺这些子字段（与 admin commonBuyButton 同源）
    const iconSizeVal = options.iconSize ?? 14;
    const textSizeVal = options.textSize ?? 12;
    const iconBoxWidth = Math.max(iconSizeVal, 24);
    const iconStyle = `color: ${options.iconColor}; font-weight: 400; font-size: ${designPxToRpx(iconSizeVal)};`;
    const iconImageSrc = useMemo(() => {
        const img = options.iconImage;
        if (!img || typeof img !== "object") return "";
        return pickPic(img);
    }, [options.iconImage]);

    const background = useMemo(() => {
        let str = "";
        // 历史默认底色 #333333（defaultBuyButtonStyle 出厂值）视为「未自定义」：跟随主题色 --general，
        // 消除默认黑圈；用户在设置里改过任一端渐变色（非历史默认值）时仍按字段值原样渲染。
        const isLegacyBg = (v?: string) => {
            const s = String(v ?? "")
                .trim()
                .toLowerCase();
            return s === "" || s === "#333333" || s === "#333";
        };
        const colorA = options.gradientColorA;
        const colorB = options.gradientColorB;
        if (options.gradientType !== "purity") {
            const followTheme = isLegacyBg(colorA) && isLegacyBg(colorB);
            const a = followTheme ? "var(--general)" : colorA;
            const b = followTheme ? "var(--general)" : colorB;
            str = `background-image: linear-gradient(${gradientMap[options.gradientType]}, ${a}, ${b});`;
        } else {
            str = `background-color:${isLegacyBg(colorA) ? "var(--general)" : colorA};`;
        }
        return str;
    }, [options.gradientType, options.gradientColorA, options.gradientColorB]);

    const paddingLeft = useMemo(() => {
        let str = "padding-left:0";
        if (options.text) {
            str = `padding-left: ${designPxToRpx(options.paddingLeftRight)};`;
            if (!options.btnType) {
                str = `padding-left: ${designPxToRpx(Number(options.paddingLeftRight || 0) + 10)};`;
            }
        }
        return str;
    }, [options.text, options.btnType, options.paddingLeftRight]);

    if (!(options.btnType || options.text)) {
        return null;
    }

    return (
        <View
            className="goods-btn decorate-common-buy-button"
            style={
                `height: ${designPxToRpx(Number(options.paddingTopBottom || 0) + 24)}; padding-right: ${
                    options.text ? designPxToRpx(Number(options.paddingLeftRight || 0) + 10) : 0
                }; border-radius: ${designPxToRpx(options.radius)}; color: ${options.textColor}; font-weight: 400;` +
                background +
                paddingLeft
            }
        >
            {options.btnType ? (
                <View
                    className="btn-icon"
                    style={`margin-left: ${options.text ? "10rpx" : 0}; margin-right: ${
                        options.text ? "-4rpx" : 0
                    }; width: ${designPxToRpx(iconBoxWidth)}; background: none;`}
                >
                    {/* color/font-size 必须打在 .iconfont-h5 上：父级 .item-buy .iconfont-h5 与 iconfont.css 会覆盖继承样式 */}
                    {options.btnType == 1 && <View className="iconfont-h5 icon-gouwuche" style={iconStyle} />}
                    {options.btnType == 2 && <View className="iconfont-h5 icon-gouwuche1" style={iconStyle} />}
                    {options.btnType == 3 && <View className="iconfont-h5 icon-jia" style={iconStyle} />}
                    {options.btnType == 4 && <View className="iconfont-h5 icon-jia1" style={iconStyle} />}
                    {options.btnType == 5 && iconImageSrc && (
                        <Image
                            className="btn-icon-img"
                            src={imageFormat(iconImageSrc)}
                            mode="aspectFit"
                            style={`width: ${designPxToRpx(iconSizeVal)}; height: ${designPxToRpx(iconSizeVal)};`}
                        />
                    )}
                </View>
            ) : null}
            {options.text ? (
                <View className="btn-text" style={`font-size: ${designPxToRpx(textSizeVal)};`}>
                    {t(options.text)}
                </View>
            ) : null}
        </View>
    );
}
