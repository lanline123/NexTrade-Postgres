import { useEffect, useRef, useState } from "react";
import { View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import TigLayout from "@/components/tig/tig-layout/TigLayout";
import Masonry from "@/components/masonry/Masonry";
import EmptyBox from "@/components/empty-box/EmptyBox";
import { exchangeCoupon, getExchangeList } from "@/api/exchange/exchange";
import useList from "@/hooks/useList";
import { useProductStyle } from "@/hooks/useProductStyle";
import { useConfigStore } from "@/store/config";
import { useI18nStore } from "@/store/i18n";
import { isOverseas, staticResource } from "@/utils";
import { setPageMeta } from "@/utils/seo";
import { t } from "@/utils/i18n";

import "./list.scss";
import { usePageGuards } from "@/utils/pageMixin";

/**
 * 积分兑换列表页（自旧 pages/exchange/list.vue 移植）
 * - 头部积分展示（国内/海外两套 bg）
 * - 瀑布流兑换商品列表 + 空状态
 */
export default function ExchangeList() {
    usePageGuards();
    const shopTitle = useConfigStore((s) => s.baseInfo.shopTitle);
    const integralName = useConfigStore((s) => s.integralName);
    const langCode = useI18nStore((s) => s.langCode);
    const isZh = isOverseas() ? (langCode ? langCode.includes("zh") : true) : true;

    // 对应旧 setup 阶段 uni.getStorageSync("userInfo")
    const [userInfo] = useState<Record<string, any>>(() => Taro.getStorageSync("userInfo") || {});
    // 当前积分（兑换券成功后按接口返回的剩余积分即时更新头部展示）
    const [points, setPoints] = useState<number>(Number(userInfo.points || 0));

    // 对应旧 onMounted（#ifdef H5）
    useEffect(() => {
        setPageMeta({
            title: `${t("积分兑换")} - ${shopTitle}`,
            description: `${shopTitle}${t("积分兑换商品列表")}`
        });
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [shopTitle]);

    const filterParamsRef = useRef({
        //初始化用于查询的参数
        page: 1,
        size: 20
    });

    const {
        isLoading,
        total,
        data: lsit
    } = useList(getExchangeList, {
        params: filterParamsRef.current,
        path: {
            dataKey: "records"
        },
        immediate: true
    });

    const { styleMap } = useProductStyle(lsit);

    // 券型兑换品（我方新增）与商品型分开陈列：券区在上、商品瀑布流在下
    const couponItems = (lsit || []).filter((item: any) => Number(item.exchangeType) === 2);
    const productItems = (lsit || []).filter((item: any) => Number(item.exchangeType) !== 2);

    /** 积分兑换优惠券：确认弹窗（券名/面额/所需积分/折算来源）→ 即时兑换 */
    const onExchangeCoupon = (item: any) => {
        const need = Number(item.exchangeIntegral || 0);
        const sourceNote = item.integralSource === "platform" ? `（${t("按平台统一比折算")}）` : "";
        Taro.showModal({
            title: t("确认兑换"),
            content: `${t("使用")} ${need} ${integralName}${t("兑换")}「${item.couponName || ""}」${sourceNote}？`,
            confirmText: t("确认兑换"),
            success: async (res) => {
                if (!res.confirm) return;
                try {
                    const result: any = await exchangeCoupon({ id: Number(item.id) });
                    const data = result?.data || {};
                    if (typeof data.remainingPoints === "number") {
                        setPoints(data.remainingPoints);
                    } else {
                        setPoints((prev) => Math.max(0, prev - need));
                    }
                    Taro.showToast({ title: t("兑换成功"), icon: "success" });
                } catch (error: any) {
                    Taro.showToast({ title: error?.message || t("兑换失败"), icon: "none" });
                }
            }
        });
    };

    /** 国内版积分兑换页头部背景图（带文字 banner） */
    const exchangeBg = `url(${staticResource("exchange/exchange-bg.png")})`;

    /** 海外版积分兑换页头部背景图（纯装饰，标题另叠加） */
    const overseasExchangeBg = `url(${staticResource("exchange/overseas-exchange-bg.png")})`;

    const headerBg = !isZh && isOverseas() ? overseasExchangeBg : exchangeBg;

    return (
        <TigLayout title={t("积分兑换")}>
            <View className="exchange-box">
                <View className="exchange-header">
                    <View className="header-bg" style={{ backgroundImage: headerBg }} />
                    <View className="header-text">
                        {" "}
                        {t("当前积分{0}", { 0: points })}{" "}
                    </View>
                    {/* 中文用带文字的国内 banner（不叠加标题）；非中文用纯装饰海外图，叠加标题文案 */}
                    {!isZh && (
                        <View className="header-title">{`${integralName}${t("兑换")}`}</View>
                    )}
                </View>
                {!isLoading && (
                    <>
                        {total > 0 && (
                            <View className="exchange-list">
                                {couponItems.length > 0 && (
                                    <View className="exchange-coupons">
                                        <View className="exchange-coupons-title">{t("积分兑券")}</View>
                                        {couponItems.map((item: any) => (
                                            <View className="exchange-coupon-card" key={item.id}>
                                                <View className="coupon-face">
                                                    <View className="coupon-face-money">
                                                        {Number(item.couponMoney || 0) > 0
                                                            ? `¥${Number(item.couponMoney)}`
                                                            : t("折扣券")}
                                                    </View>
                                                    <View className="coupon-face-threshold">
                                                        {Number(item.minOrderAmount || 0) > 0
                                                            ? `${t("满")}¥${Number(item.minOrderAmount)}${t("可用")}`
                                                            : t("无门槛")}
                                                    </View>
                                                </View>
                                                <View className="coupon-main">
                                                    <View className="coupon-name">{item.couponName}</View>
                                                    <View className="coupon-points">
                                                        {Number(item.exchangeIntegral || 0)} {integralName}
                                                        {item.integralSource === "platform" && (
                                                            <View className="coupon-source">{t("平台统一比")}</View>
                                                        )}
                                                    </View>
                                                </View>
                                                <View className="coupon-btn" onClick={() => onExchangeCoupon(item)}>
                                                    {t("兑换")}
                                                </View>
                                            </View>
                                        ))}
                                    </View>
                                )}
                                {productItems.length > 0 && (
                                    <>
                                        {/* TODO(MIGRATION)：旧 :is-exchange="true" 在旧 masonry 中未声明（等同透传空属性），此处省略 */}
                                        <Masonry commodityList={productItems} type="exchange" styleMap={styleMap} />
                                    </>
                                )}
                            </View>
                        )}
                        {total === 0 && (
                            <View className="empty-box">
                                <EmptyBox
                                    background="transparent"
                                    height="auto"
                                    icon="salesman/no_order.png"
                                    iconWidth="160rpx"
                                    iconHeight="160rpx"
                                    text={t("暂无数据")}
                                />
                            </View>
                        )}
                    </>
                )}
            </View>
        </TigLayout>
    );
}