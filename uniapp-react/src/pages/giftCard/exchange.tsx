import type { CSSProperties } from "react";
import { useEffect, useState } from "react";
import { Input, Text, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import TigButton2 from "@/components/tig/tig-button2/TigButton2";
import TigLayout from "@/components/tig/tig-layout/TigLayout";
import TigNavbar from "@/components/tig/tig-navbar/TigNavbar";
import { giftCardExchange, useGiftCard } from "@/api/giftCard/giftCard";
import type { GiftCardExchangeParams } from "@/types/giftCard/giftCard";
import { useUserStore } from "@/store/user";
import { redirect, staticResource } from "@/utils";
import { handleLogin } from "@/utils/handleLogin";
import { t } from "@/utils/i18n";

import "./exchange.scss";
import { usePageGuards } from "@/utils/pageMixin";

/**
 * 礼品卡兑换页（自旧 pages/giftCard/exchange.vue 移植）
 * - 输入卡号/卡密，调用兑换接口把卡绑进我的卡包
 * - 储值卡（D4 原版口径）：只入卡包、不入会员余额，结算时抵扣卡内余额 → 跳我的礼品卡
 * - 兑换卡固定组合：useCard 核销后跳结算 flowType=12；任选：跳卡详情打开选品弹窗
 * - 旧 uni-forms/uni-easyinput → Taro Input + 自绘表单行（必填校验等价）
 */
export default function GiftCardExchange() {
    usePageGuards();
    const userToken = useUserStore((s) => s.token);
    const [formState, setFormState] = useState<GiftCardExchangeParams>({
        cardNumber: "",
        cardPassword: ""
    });
    const [errors, setErrors] = useState<{ cardNumber: string; cardPassword: string }>({
        cardNumber: "",
        cardPassword: ""
    });
    const [loading, setLoading] = useState(false);

    const hasToken = userToken || Taro.getStorageSync("token");

    const backgroundImg = `url(${staticResource("giftCard/exchange_bg.png")}) center no-repeat`;

    /**
     * 礼品卡兑换
     */
    const exchange = async () => {
        try {
            if (loading) {
                return;
            }
            setLoading(true);
            const result: any = await giftCardExchange(formState);
            // 兑换响应字段宽松读取（类型/模式/卡 id/SN 可能在顶层或卡对象内）
            const card = result?.card ?? result ?? {};
            const groupType = Number(card.groupType ?? result?.groupType ?? 0);
            const exchangeMode = Number(card.exchangeMode ?? result?.exchangeMode ?? 0);
            const giftCardsId = Number(card.giftCardsId ?? result?.giftCardsId ?? result?.exchangeProductToCart?.giftCardsId ?? 0);
            const giftCardSn = String(card.giftCardSn ?? result?.giftCardSn ?? "");

            if (groupType === 2 && exchangeMode === 2) {
                // 任选：跳卡详情打开选品弹窗（无卡 id 时回卡包）
                redirect({
                    url: giftCardsId ? `/pages/giftCard/myCardDetail?giftCardsId=${giftCardsId}` : "/pages/giftCard/myCards"
                });
                return;
            }
            if (groupType === 2) {
                // 固定组合：先 useCard 核销，再进结算（flowType=12）
                if (giftCardsId) {
                    await useGiftCard({ giftCardsId });
                }
                Taro.showToast({ title: t("兑换成功"), icon: "success" });
                redirect({
                    url: `/pages/order/check?flowType=12${giftCardSn ? `&giftCardSn=${encodeURIComponent(giftCardSn)}` : ""}`
                });
                return;
            }
            if (groupType === 1) {
                // 储值卡：已存入我的礼品卡，结算时抵扣卡内余额（不入会员余额）
                Taro.showToast({ title: t("已存入我的礼品卡，下单结算时可抵扣卡内余额"), icon: "none" });
                redirect({ url: "/pages/giftCard/myCards" });
                return;
            }
            // 响应未带类型字段时的兼容：旧 exchangeProductToCart.flowType 直达结算，否则按入卡包口径回卡包
            if (result?.exchangeProductToCart?.flowType) {
                redirect({ url: `/pages/order/check?flowType=${result.exchangeProductToCart?.flowType}` });
            } else {
                Taro.showToast({ title: t("已存入我的礼品卡，下单结算时可抵扣卡内余额"), icon: "none" });
                redirect({ url: "/pages/giftCard/myCards" });
            }
        } catch (error: any) {
            Taro.showToast({
                title: t(error.message),
                icon: "none"
            });
            console.error(error);
        } finally {
            setLoading(false);
        }
    };

    const handleExchange = () => {
        // 旧 uni-forms.validate()：按 rules 逐项校验，错误信息展示在表单项下方
        const nextErrors = {
            cardNumber: formState.cardNumber ? "" : t("请输入卡号"),
            cardPassword: formState.cardPassword ? "" : t("请输入卡密")
        };
        setErrors(nextErrors);
        if (nextErrors.cardNumber || nextErrors.cardPassword) {
            console.log("表单错误信息：", nextErrors);
            return;
        }
        void exchange();
    };

    // 页面加载（旧 onLoad）
    useEffect(() => {
        // 需要登录
        if (!hasToken) {
            handleLogin();
            return;
        }
        // if (options?.cardNumber) {
        //     setFormState((prev) => ({ ...prev, cardNumber: options.cardNumber }));
        // }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // 旧 onShow → nextTick(() => formRef.setRules(rules))；rules 恒为上方校验规则，无需重复注入
    useEffect(() => {
        // TODO(MIGRATION)：uni-forms 的 setRules 动态注入在自绘表单中由 validate() 内联等价
    }, []);

    return (
        <View>
            <TigLayout>
                <TigNavbar background="transparent" />
                <View className="bg" style={{ "--exchange-bg": backgroundImg } as CSSProperties} />
                <View className="container">
                    <View className="card-title">
                        <Text className="title">{t("礼品卡兑换")}</Text>
                    </View>
                    <View className="card-subtitle">
                        <View className="title">
                            <Text>{t("一卡在手，好礼不愁")}</Text>
                        </View>
                    </View>
                    <View className="gift-card">
                        <View className="card-box exchange-form">
                            <View className="card-edit-content">
                                <View className="form-item">
                                    <View className="form-label">{t("卡号")}</View>
                                    <Input
                                        className="input"
                                        value={formState.cardNumber}
                                        placeholder={t("请输入卡号")}
                                        placeholderStyle="color:#bfbfbf;font-size:28rpx"
                                        onInput={(e) => setFormState((prev) => ({ ...prev, cardNumber: e.detail.value }))}
                                    />
                                    {errors.cardNumber ? <View className="form-error">{errors.cardNumber}</View> : null}
                                </View>
                                <View className="form-item">
                                    <View className="form-label">{t("卡密")}</View>
                                    <Input
                                        className="input"
                                        password
                                        value={formState.cardPassword}
                                        placeholder={t("请输入卡密")}
                                        placeholderStyle="color:#bfbfbf;font-size:28rpx"
                                        onInput={(e) => setFormState((prev) => ({ ...prev, cardPassword: e.detail.value }))}
                                    />
                                    {errors.cardPassword ? <View className="form-error">{errors.cardPassword}</View> : null}
                                </View>
                                <View className="action">
                                    <TigButton2 className="btn" loading={loading} onClick={handleExchange}>
                                        {t("立即兑换")}
                                    </TigButton2>
                                </View>
                            </View>
                        </View>
                        <View className="card-box">
                            <View className="card-content">
                                <View className="title">{t("兑换说明")}</View>
                                <View className="tips">
                                    <View>{t("1.储值卡：输入卡密完成兑换，卡片自动存入「我的礼品卡」，下单结算时可直接抵扣卡内余额。")}</View>
                                    <View>{t("2.兑换卡：输入卡密完成兑换，卡片自动存入卡包；选择心仪商品下单，即可自动核销使用。")}</View>
                                </View>
                            </View>
                        </View>
                    </View>
                </View>
            </TigLayout>
        </View>
    );
}