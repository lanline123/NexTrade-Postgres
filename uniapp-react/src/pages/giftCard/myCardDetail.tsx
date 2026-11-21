import { useEffect, useState } from "react";
import { Image, ScrollView, Text, View } from "@tarojs/components";
import Taro, { useRouter } from "@tarojs/taro";
import { Popup } from "@nutui/nutui-react-taro";

import TigButton2 from "@/components/tig/tig-button2/TigButton2";
import TigLayout from "@/components/tig/tig-layout/TigLayout";
import TigNavbar from "@/components/tig/tig-navbar/TigNavbar";
import { getMyGiftCardDetail, useGiftCard } from "@/api/giftCard/giftCard";
import type { MyGiftCardDetail } from "@/api/giftCard/giftCard";
import { redirect } from "@/utils";
import { t } from "@/utils/i18n";
import { usePageGuards } from "@/utils/pageMixin";

import "./myCardDetail.scss";

/**
 * 我的礼品卡详情（P1 新增）
 * - 数据：promotion/giftCardGroups/myCardDetail（归属校验在后端）
 * - 「去使用」分流（原版口径）：储值卡 → 搜索去购物结算抵扣；兑换卡任选 → 页内选品弹窗
 *   （按 SKU 计件、库存封顶、可选 X/已选 Y，useCard 提交后跳结算 flowType=12）；
 *   兑换卡固定组合 → useCard 后跳结算 flowType=12
 * - 有效卡可「转赠好友」；消费流水 P2/P3 才有数据
 */
export default function MyGiftCardDetailPage() {
    usePageGuards();
    const router = useRouter();
    const giftCardsId = Number(router.params.giftCardsId || 0);
    const [detail, setDetail] = useState<MyGiftCardDetail | null>(null);
    const [loading, setLoading] = useState(false);
    // 任选选品弹窗：按 productId-skuId 记数量
    const [showSelect, setShowSelect] = useState(false);
    const [qtyMap, setQtyMap] = useState<Record<string, number>>({});
    const [submitting, setSubmitting] = useState(false);

    useEffect(() => {
        if (!giftCardsId) return;
        setLoading(true);
        getMyGiftCardDetail(giftCardsId)
            .then((res: any) => setDetail(res))
            .catch((error: any) => Taro.showToast({ title: error?.message || t("加载失败"), icon: "none" }))
            .finally(() => setLoading(false));
    }, [giftCardsId]);

    const fmtTime = (ts?: number) => (ts ? new Date(ts * 1000).toLocaleString() : "-");
    const card = detail ?? undefined;

    // 任选口径：件数取 selectCount（兼容 maxSelectCount 兜底），商品池来自详情 giftCardProducts
    const selectCount = Number(detail?.selectCount || detail?.maxSelectCount || 1);
    const selectProducts = detail?.giftCardProducts ?? [];
    const selectedTotal = Object.values(qtyMap).reduce((sum, n) => sum + n, 0);
    const stockOf = (p: NonNullable<MyGiftCardDetail["giftCardProducts"]>[number]) =>
        Number(p.skuStock ?? p.productStock ?? p.stock ?? 0);

    const changeQty = (p: NonNullable<MyGiftCardDetail["giftCardProducts"]>[number], delta: number) => {
        const key = `${p.productId}-${p.skuId}`;
        setQtyMap((prev) => {
            const current = prev[key] || 0;
            let next = current + delta;
            if (next < 0) next = 0;
            if (next > stockOf(p)) {
                if (delta > 0) Taro.showToast({ title: t("商品库存不足"), icon: "none" });
                next = stockOf(p);
            }
            const totalOthers = Object.entries(prev)
                .filter(([k]) => k !== key)
                .reduce((sum, [, n]) => sum + n, 0);
            if (totalOthers + next > selectCount) next = Math.max(0, selectCount - totalOthers);
            return { ...prev, [key]: next };
        });
    };

    /** 「去使用」分流（原版口径，见文件头） */
    const goUse = async () => {
        if (!detail) return;
        if (detail.groupType === 1) {
            redirect({ url: "/pages/search/result" });
            return;
        }
        if (Number(detail.exchangeMode || 0) === 2) {
            if (!selectProducts.length) {
                Taro.showToast({ title: t("暂无可兑换商品"), icon: "none" });
                return;
            }
            setShowSelect(true);
            return;
        }
        try {
            await useGiftCard({ giftCardsId: detail.giftCardsId });
            redirect({
                url: `/pages/order/check?flowType=12${detail.giftCardSn ? `&giftCardSn=${encodeURIComponent(detail.giftCardSn)}` : ""}`
            });
        } catch (error: any) {
            Taro.showToast({ title: error?.message || t("兑换失败"), icon: "none" });
        }
    };

    /** 任选确认：件数须恰等于 selectCount，useCard 提交后跳结算 flowType=12 */
    const confirmSelect = async () => {
        if (!detail || selectedTotal !== selectCount || submitting) return;
        setSubmitting(true);
        try {
            const items = selectProducts
                .map((p) => ({ productId: p.productId, skuId: p.skuId, quantity: qtyMap[`${p.productId}-${p.skuId}`] || 0 }))
                .filter((item) => item.quantity > 0);
            await useGiftCard({ giftCardsId: detail.giftCardsId, items });
            setShowSelect(false);
            redirect({
                url: `/pages/order/check?flowType=12${detail.giftCardSn ? `&giftCardSn=${encodeURIComponent(detail.giftCardSn)}` : ""}`
            });
        } catch (error: any) {
            Taro.showToast({ title: error?.message || t("兑换失败"), icon: "none" });
        } finally {
            setSubmitting(false);
        }
    };

    return (
        <TigLayout>
            <TigNavbar title={t("礼品卡详情")} />
            <View className="my-gift-card-detail">
                {!card && !loading ? (
                    <View className="empty">{t("礼品卡不存在")}</View>
                ) : card ? (
                    <>
                        <View className="card-head">
                            {card.imageUrl ? <Image className="card-img" src={card.imageUrl} mode="aspectFill" /> : null}
                            <View className="card-info">
                                <View className="card-name">
                                    {card.cardGroupName}
                                    <Text className={`card-state${card.isValid ? "" : " off"}`}>{card.stateText}</Text>
                                </View>
                                <View className="card-sub">
                                    {t("卡号")}：{card.cardNumber}
                                </View>
                                {card.groupType === 1 ? (
                                    <View className="card-sub">
                                        {t("面额")} ¥{card.faceAmount ?? "-"} · {t("当前余额")} ¥{card.balance ?? "-"}
                                    </View>
                                ) : (
                                    <View className="card-sub">{t("兑换卡")}</View>
                                )}
                                <View className="card-sub">
                                    {card.sourceText} · {card.expiredTime ? `${t("有效期至")} ${fmtTime(card.expiredTime)}` : t("永久有效")}
                                </View>
                                {card.groupType === 1 && card.scopeDesc ? (
                                    <View className="card-sub">
                                        {t("适用范围")}：{t(card.scopeDesc)}
                                    </View>
                                ) : null}
                            </View>
                        </View>
                        {card.isValid ? (
                            <View className="actions">
                                <TigButton2 onClick={() => void goUse()}>{t("去使用")}</TigButton2>
                                {card.giftCardSn ? (
                                    <View
                                        className="transfer-btn"
                                        onClick={() => redirect({ url: `/pages/giftCard/transfer?giftCardSn=${card.giftCardSn}` })}
                                    >
                                        {t("转赠好友")}
                                    </View>
                                ) : null}
                            </View>
                        ) : null}
                        {detail && detail.exchangeOrderIds.length > 0 ? (
                            <View className="section">
                                <View className="section-title">{t("兑换订单")}</View>
                                <View className="section-body">{detail.exchangeOrderIds.map((id) => `#${id}`).join("、")}</View>
                            </View>
                        ) : null}
                        <View className="section">
                            <View className="section-title">{t("消费记录")}</View>
                            {detail && detail.usageLogs.length > 0 ? (
                                detail.usageLogs.map((log) => (
                                    <View key={log.id} className="usage-row">
                                        <Text>{log.typeName}</Text>
                                        <Text>
                                            ¥{log.amount} · {fmtTime(log.addTime)}
                                        </Text>
                                    </View>
                                ))
                            ) : (
                                <View className="section-body">{t("暂无消费记录")}</View>
                            )}
                        </View>
                    </>
                ) : null}
            </View>

            {/* 任选选品弹窗（原版 detail 页内 selectProduct 组件口径） */}
            <Popup
                visible={showSelect}
                position="bottom"
                zIndex={999}
                closeable={false}
                style={{ backgroundColor: "#fff" }}
                onClose={() => setShowSelect(false)}
            >
                <View className="gift-select-pop">
                    <View className="gift-select-head">
                        {t("可选")} {selectCount} / {t("已选")} {selectedTotal}
                    </View>
                    <ScrollView scrollY className="gift-select-list">
                        {selectProducts.map((p) => {
                            const key = `${p.productId}-${p.skuId}`;
                            const qty = qtyMap[key] || 0;
                            const stock = stockOf(p);
                            return (
                                <View key={key} className="gift-select-row">
                                    {p.picUrl ? <Image className="gift-select-img" src={p.picUrl} mode="aspectFill" /> : null}
                                    <View className="gift-select-info">
                                        <View className="gift-select-name">{p.productName}</View>
                                        {p.skuData ? <View className="gift-select-sub">{p.skuData}</View> : null}
                                        <View className="gift-select-sub">{stock > 0 ? `${t("库存")} ${stock}` : t("无库存")}</View>
                                    </View>
                                    <View className="gift-select-stepper">
                                        <View className={`step-btn${qty <= 0 ? " off" : ""}`} onClick={() => changeQty(p, -1)}>
                                            -
                                        </View>
                                        <Text className="step-num">{qty}</Text>
                                        <View
                                            className={`step-btn${stock <= 0 || qty >= stock ? " off" : ""}`}
                                            onClick={() => changeQty(p, 1)}
                                        >
                                            +
                                        </View>
                                    </View>
                                </View>
                            );
                        })}
                    </ScrollView>
                    <View className="gift-select-actions">
                        <View className="gift-select-cancel" onClick={() => setShowSelect(false)}>
                            {t("取消")}
                        </View>
                        <View
                            className={`gift-select-confirm${selectedTotal === selectCount && !submitting ? "" : " off"}`}
                            onClick={() => void confirmSelect()}
                        >
                            {t("确认兑换")}
                        </View>
                    </View>
                </View>
            </Popup>
        </TigLayout>
    );
}
