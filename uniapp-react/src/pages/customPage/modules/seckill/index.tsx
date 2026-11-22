import { useEffect, useMemo, useRef, useState , type CSSProperties } from "react";
import { Image, Text, View } from "@tarojs/components";

import { designPxToRpx, formatFrame } from "@/components/modules";
import FormatPrice from "@/components/format-price/FormatPrice";
import TigImage from "@/components/tig/tig-image/TigImage";
import CommonTitle from "@/pages/customPage/modules/commonTitle/index";
import { getHomeSeckill } from "@/api/home/home";
import { useConfigStore } from "@/store/config";
import { resolveH5BuyBtnStyle } from "@/utils/buyStyle";
import { useProductStyle } from "@/hooks/useProductStyle";
import { pickPic } from "@/utils/format";
import { redirect } from "@/utils";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
}

/**
 * 旧 tig-countdown.vue + uni-countdown（uni-ui）等价展开：
 * - 旧 tig-countdown 解析 "YYYY-MM-DD HH:mm:ss" 结束时间，挂载时算一次差值（days/hours/minutes/seconds）；
 * - 差值交给 uni-countdown 内部定时器逐秒递减展示（show-day：days>0 才显示天段；uni-countdown 冒号模式
 *   格式为「DD 天 HH:MM:SS」——天段分隔符恒为「天」，时/分/秒段间才是 ":"，两位补零）；
 * - React 侧合并为「挂载算一次 + 每秒重算」，口径一致。
 */
function TigCountdown({ endTime }: { endTime: string }) {
    const [timeData, setTimeData] = useState({ days: 0, hours: 0, minutes: 0, seconds: 0 });
    const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

    useEffect(() => {
        /*转时间戳（后端 seckillEndTime 是 epoch 秒数字，先归一化，否则对数字 split 直接抛错） */
        const dateStringToTimestamp = (dateString: any) => {
            if (typeof dateString === "number" || /^\d+$/.test(String(dateString ?? "").trim())) {
                const n = Number(dateString);
                return n < 1e12 ? n * 1000 : n;
            }
            const dateParts = String(dateString).split(" ");
            const [year, month, day] = dateParts[0].split("-");
            const [hour, minute, second] = (dateParts[1] || "00:00:00").split(":");
            return new Date(Number(year), Number(month) - 1, Number(day), Number(hour), Number(minute), Number(second)).getTime();
        };
        const countdownInit = () => {
            const endTimestamp = dateStringToTimestamp(endTime);
            const nowTimestamp = new Date().getTime();
            // 计算差值（毫秒）
            const diffMilliseconds = endTimestamp - nowTimestamp;
            if (diffMilliseconds > 0) {
                setTimeData({
                    days: Math.floor(diffMilliseconds / (1000 * 60 * 60 * 24)),
                    hours: Math.floor(diffMilliseconds / (1000 * 60 * 60)) % 24,
                    minutes: Math.floor(diffMilliseconds / (1000 * 60)) % 60,
                    seconds: Math.floor(diffMilliseconds / 1000) % 60
                });
            } else {
                setTimeData({ days: 0, hours: 0, minutes: 0, seconds: 0 });
            }
        };
        countdownInit();
        timerRef.current = setInterval(countdownInit, 1000);
        return () => {
            if (timerRef.current) {
                clearInterval(timerRef.current);
                timerRef.current = null;
            }
        };
    }, [endTime]);

    const formatTime = (time: number) => time.toString().padStart(2, "0");

    // tig-countdown 默认 color/splitorColor #fff、fontSize 12（px），在此以容器内联样式还原
    return (
        <View className="tig-countdown" style={{ color: "#fff", fontSize: "12px" }}>
            {timeData.days > 0 && (
                <>
                    <Text className="uni-countdown__number">{formatTime(timeData.days)}</Text>
                    <Text className="uni-countdown__splitor">{t("天")}</Text>
                </>
            )}
            <Text className="uni-countdown__number">{formatTime(timeData.hours)}</Text>
            <Text className="uni-countdown__splitor">:</Text>
            <Text className="uni-countdown__number">{formatTime(timeData.minutes)}</Text>
            <Text className="uni-countdown__splitor">:</Text>
            <Text className="uni-countdown__number">{formatTime(timeData.seconds)}</Text>
        </View>
    );
}

export default function Seckill({ module }: Props) {
    const { frame } = module;

    // 购买按钮「跟随全局」解析（读时、不回写）：buyBtnStyle=-1 时取全局样式
    // 降级到 0–8；其余值原样透传，渲染与此前逐字一致。全局 iconColor 非空时
    // 以 --general 局部变量作用于按钮宿主元素（不外溢到整模块）。
    const globalBuyStyle = useConfigStore((s) => s.buyStyle);
    const buyBtn = resolveH5BuyBtnStyle(module?.buyBtnStyle, globalBuyStyle);
    const resolvedBuyBtnStyle = buyBtn.style ?? module?.buyBtnStyle;
    const buyBtnVarStyle = buyBtn.iconColor ? ({ "--general": buyBtn.iconColor } as CSSProperties) : undefined;

    const frameFormat = useMemo(() => {
        return formatFrame(frame ?? {});
    }, [frame]);

    const allFormat = useMemo(() => {
        return {
            backgroundColor: `background-color: ${module.backgroundColor};`,
            boxPadding: `margin-left: ${designPxToRpx(module.boxPadding)};margin-right: ${designPxToRpx(module.boxPadding)};`,
            boxPaddingBottom: `margin-bottom: ${designPxToRpx(module.boxPaddingBottom)};`,
            boxPaddingTop: `margin-top: ${designPxToRpx(module.boxPaddingTop)};`,
            boxRadius: `border-radius: ${designPxToRpx(module.boxRadius)};`,
            innerPadding: `padding: ${designPxToRpx(module.innerPadding)};`,
            goodsPadding: `padding: ${designPxToRpx(module.goodsPadding)};`
        };
    }, [module]);

    // 模板兜底与编辑器新建默认对齐（2026-11-21 用户口径：一行两个）：
    // 存量数据 style 缺失时按 2 渲染，已存值逐字尊重、不做归一化。
    const styleValue = module.style ?? 2;

    const className = useMemo(() => {
        switch (styleValue) {
            case 1:
                return "big";
            case 2:
                return "small flex-end";
            case 5:
                return "list flex-end";
            case 6:
                return "small flex-end";
            default:
                return "";
        }
    }, [styleValue]);

    // 旧 uni.navigateTo → redirect（默认 navigateTo 模式）
    const buy = (id: any) => {
        redirect({ url: "/pages/product/index?id=" + id });
    };

    const [seckillList, setSeckillList] = useState<any[]>([]);
    const { styleMap } = useProductStyle(seckillList);
    const getData = async () => {
        try {
            const result = await getHomeSeckill();
            setSeckillList(result.records);
        } catch (error) {
            console.error(error);
        }
    };

    // 对应旧 onMounted
    useEffect(() => {
        void getData();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    return (
        seckillList && seckillList?.length > 0 && (
            <View
                className={
                    "module-ad-con decorate-seckill module-goods_ad module-seckill-goods_ad ad-style__" +
                    styleValue +
                    " ad-buy_btn_style__" +
                    resolvedBuyBtnStyle +
                    " ad-goods_style__" +
                    module?.goodsStyle +
                    " ad-goods_radio_style__" +
                    module?.goodsRadioStyle +
                    " ad-text_align__" +
                    module?.textAlign +
                    " ad-text_weight__" +
                    module?.textWeight +
                    " ad-goods_name_row__" +
                    module?.goodsNameRow +
                    " ad-goods_name_padding__" +
                    module?.goodsNamePadding
                }
                style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom}
            >
                <View
                    className="module-ad-content"
                    style={
                        frameFormat.backgroundColor +
                        " " +
                        frameFormat.boxRadius +
                        allFormat.backgroundColor +
                        allFormat.boxPadding +
                        allFormat.boxPaddingBottom +
                        allFormat.boxPaddingTop +
                        allFormat.boxRadius +
                        allFormat.innerPadding
                    }
                >
                    {module.title?.showTitle == 1 && <CommonTitle module={module.title} />}

                    <View className="goods-ad-warp">
                        <View className="goods-ad-con">
                            {seckillList.map((item: any) => (
                                <View key={item.productId} className="goods-ad-item">
                                    <View className="item-content" style={allFormat.goodsPadding}>
                                        <View className="item-con">
                                            <View className="item-photo">
                                                {/* 旧 navigator :url → View onClick + redirect */}
                                                <View
                                                    className="item-image-a"
                                                    onClick={() => redirect({ url: "/pages/product/index?id=" + item.productId })}
                                                >
                                                    <TigImage src={pickPic(item)} mode={styleValue == 6 ? "aspectFill" : "widthFix"} />
                                                </View>
                                                <View className={"cap-seckill-goods__tag " + className}>
                                                    {styleValue == 1 && <Text className="cap-seckill-goods__tag-title"> {t("秒杀")} </Text>}
                                                    {/* 后端 getSeckill 返回扁平记录（seckillEndTime 在顶层），嵌套 seckkillData 仅旧契约有；直接读嵌套会抛错并整页白屏 */}
                                                    <TigCountdown endTime={item.seckkillData?.seckillEndTime ?? item.seckillEndTime} />
                                                </View>
                                                {styleMap[item.productId]?.borderImage && (
                                                    <Image
                                                        className="product-style-border"
                                                        src={styleMap[item.productId].borderImage}
                                                        mode="scaleToFill"
                                                    />
                                                )}
                                            </View>
                                            <View className="item-info">
                                                {module.showName && (
                                                    <View className="item-name">
                                                        <View
                                                            className="item-name-a"
                                                            onClick={() => redirect({ url: "/pages/product/index?id=" + item.productId })}
                                                        >
                                                            {item.productName ?? ""}
                                                        </View>
                                                    </View>
                                                )}
                                                {module.showPrice ? (
                                                    <View className="item-action">
                                                        <View className="item-price">
                                                            <FormatPrice displayPrice
                                                                fontStyle={{ fontSize: "32rpx" }}
                                                                currencyStyle={{
                                                                    fontSize: "22rpx"
                                                                }}
                                                                decimalsStyle={{
                                                                    fontSize: "24rpx"
                                                                }}
                                                                priceData={item.seckillPrice}
                                                            />
                                                        </View>
                                                        <View className="item-buy" style={buyBtnVarStyle}>
                                                            <View className="buy-btn" onClick={() => buy(item.productId)}>
                                                                {buyBtn.style == 5 || buyBtn.style == 6 ? (
                                                                    <View>{t("购买")}</View>
                                                                ) : buyBtn.style == 7 || buyBtn.style == 8 ? (
                                                                    <View>{t("马上抢")}</View>
                                                                ) : (
                                                                    <View className="iconfont-h5" />
                                                                )}
                                                            </View>
                                                        </View>
                                                    </View>
                                                ) : (
                                                    <View className="blank" style={{ height: "20rpx" }} />
                                                )}
                                            </View>
                                        </View>
                                    </View>
                                </View>
                            ))}
                        </View>
                    </View>
                </View>
            </View>
        )
    );
}
