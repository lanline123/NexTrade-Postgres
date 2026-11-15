/**
 * 倒计时 预览（移植自 view/admin editor/modules/mobilePro/countdown/Preview.vue）
 *
 * 标题区（大标题 20px/700 + 小标题 + 右侧提示）+ 商品横排（每项 flex 0 0 100/rowNum%，
 * 图 padding-top 按 picScale，价格行 + 划线价）+ 底部倒计时图层（背景图 + CommonCountdown 绝对居中叠加）。
 * 商品直接用 pruductList 快照渲染（原版对 productId=0 演示项走 getDemoList 同口径；真实商品的接口刷新不在编辑器预览内做）。
 * 注：原版价格 font-weight 绑定的是不存在的 module.headlines 字段（恒 undefined），此处照源码不额外加粗，
 * priceWeight 仅在 Settings 可编辑（原版 Preview 未消费）。
 */
import { useMemo } from "react";
import { cssText, imageFormat, productImage } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import H5Countdown from "../../../shared/H5Countdown";
import { countdownDefaults } from "./index";
import "./Preview.css";

const PIC_SCALE_MAP: Record<string, string> = {
    "3:2": "calc(100% / 3 * 2)",
    "1:1": "calc(100% / 1 * 1)",
    "3:4": "calc(100% / 3 * 4)",
    "16:9": "calc(100% / 16 * 9)"
};
const FORMAT_MAP: Record<string, string> = { 天: "DD", 时: "HH", 分: "mm", 秒: "ss" };
const TEXT_MAP: Record<string, string> = { DD: "天", HH: "时", mm: "分", ss: "秒" };

export default function CountdownPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, countdownDefaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const list: any[] = Array.isArray(module.pruductList) ? module.pruductList : [];
    const moduleColor = module.moduleColor || {};
    const moduleTitle = module.moduleTitle || {};
    const countdownData = module.countdownData || {};
    const formatText = (countdownData.timeContent || [])
        .map((c: string) => (FORMAT_MAP[c] ? ` ${FORMAT_MAP[c]} ${TEXT_MAP[FORMAT_MAP[c]]}` : ""))
        .join("");
    const countdownPicUrl =
        typeof countdownData.countdownBackgroundPic === "string"
            ? countdownData.countdownBackgroundPic
            : countdownData.countdownBackgroundPic?.picUrl || "";

    return (
        <div
            className="module-ad-con h5-countdown-mod"
            style={cssText(
                commonStyle.moduleStyle.boxPaddingBottom,
                commonStyle.moduleStyle.boxPaddingTop,
                commonStyle.moduleStyle.boxPadding,
                commonStyle.moduleStyle.backgroundSize,
                commonStyle.moduleStyle.backgroundImage
            )}
        >
            <div
                className="module-ad-content"
                style={cssText(
                    commonStyle.moduleContentStyle.backgroundImage,
                    commonStyle.moduleContentStyle.backgroundSize,
                    commonStyle.moduleContentStyle.boxRadius2,
                    commonStyle.moduleContentStyle.boxShadow,
                    commonStyle.moduleContentStyle.innerPadding
                )}
            >
                <div className="countdow-warp">
                    <div className="mk-top">
                        <div className="mk-top-left">
                            <div className="mk-title line1" style={{ color: moduleColor.titleColor }}>
                                {moduleTitle.title}
                            </div>
                            <div className="mk-stitle line1" style={{ color: moduleColor.subTitleColor }}>
                                {moduleTitle.subTitle}
                            </div>
                        </div>
                        <div>
                            <div className="mk-notice" style={{ color: moduleColor.tipsColor }}>
                                {moduleTitle.moreText}
                            </div>
                        </div>
                    </div>
                    <div className="itemBox">
                        {list.map((item, index) => (
                            <div className="li" key={index} style={{ flex: `0 0 ${100 / (Number(module.rowNum) || 3)}%` }}>
                                <div
                                    className="item"
                                    style={{
                                        padding: `${module.productPadding}px`,
                                        backgroundColor: moduleColor.productBackgroundColor,
                                        borderRadius: `${module.productRadius}px`
                                    }}
                                >
                                    <div className="imgBox" style={{ paddingTop: PIC_SCALE_MAP[module.picScale] || "100%" }}>
                                        <div
                                            className="img"
                                            style={{
                                                borderRadius:
                                                    Number(module.picBottomRadius) === 1
                                                        ? `${module.productRadius}px ${module.productRadius}px 0 0`
                                                        : `${module.productRadius}px`,
                                                backgroundSize: module.picFillType,
                                                backgroundImage: `url(${productImage(item)})`
                                            }}
                                        />
                                    </div>
                                    <div className="goods-info" style={{ paddingBottom: 0, textAlign: module.textAlignment }}>
                                        {Number(module.showProductName) === 1 ? (
                                            <div className="title line2" style={{ color: moduleColor.productNameColor }}>
                                                {item.productName ? item.productName : "商品名称"}
                                            </div>
                                        ) : null}
                                        {Number(module.showProductPrice) === 1 ? (
                                            <div className="priceBox" style={{ justifyContent: module.textAlignment }}>
                                                <div className="price" style={{ color: moduleColor.productPriceColor, fontFamily: "inherit" }}>
                                                    ￥{item.productPrice}
                                                </div>
                                                {Number(module.crossedOutPrice) === 1 ? (
                                                    <div className="origin" style={{ color: moduleColor.crossedOutPriceColor }}>
                                                        {item.marketPrice}
                                                    </div>
                                                ) : null}
                                            </div>
                                        ) : null}
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
                {Number(countdownData.showCountdown) === 1 ? (
                    <div className="countdown" style={{ marginTop: `-${countdownData.countdownPicBottom}px`, borderRadius: "0px 0px 6px 6px" }}>
                        <img className="countdown-img" src={countdownPicUrl ? imageFormat(countdownPicUrl) : undefined} alt="" />
                        <div className="count-down" style={{ top: `${countdownData.marginTop}%` }}>
                            <H5Countdown
                                countdownType={countdownData.countdownType || "dailyCycle"}
                                valueStyle={{ color: moduleColor.countdownColor, fontSize: "16px", display: "block" }}
                                format={formatText}
                                dailyTime={countdownData.dailyTime}
                                specifyTime={countdownData.specifyTime}
                            />
                        </div>
                    </div>
                ) : null}
            </div>
        </div>
    );
}
