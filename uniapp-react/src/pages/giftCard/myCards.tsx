import { useEffect, useState } from "react";
import { Image, Text, View } from "@tarojs/components";
import Taro, { usePullDownRefresh, useReachBottom } from "@tarojs/taro";

import TigLayout from "@/components/tig/tig-layout/TigLayout";
import LoadingBox from "@/components/loading-box";
import TigNavbar from "@/components/tig/tig-navbar/TigNavbar";
import { getMyGiftCards } from "@/api/giftCard/giftCard";
import type { MyGiftCardItem } from "@/api/giftCard/giftCard";
import { redirect } from "@/utils";
import { t } from "@/utils/i18n";
import { usePageGuards } from "@/utils/pageMixin";

import "./myCards.scss";

const PAGE_SIZE = 10;

/**
 * 我的礼品卡（P1 新增）
 * - 顶部筛选：全部/储值卡/兑换卡 × 有效/已失效
 * - 数据：promotion/giftCardGroups/myCards（卡号脱敏，状态/来源后端派生）
 */
export default function MyGiftCards() {
    usePageGuards();
    const [type, setType] = useState(0);
    const [state, setState] = useState("valid");
    const [list, setList] = useState<MyGiftCardItem[]>([]);
    const [page, setPage] = useState(1);
    const [total, setTotal] = useState(0);
    const [loading, setLoading] = useState(false);

    const load = async (targetPage: number, targetType: number, targetState: string, append = false) => {
        if (loading) return;
        setLoading(true);
        try {
            const res: any = await getMyGiftCards({
                page: targetPage,
                size: PAGE_SIZE,
                type: targetType === 0 ? undefined : targetType,
                state: targetState
            });
            const records: MyGiftCardItem[] = res?.records || res?.list || [];
            setTotal(res?.total || 0);
            setList(append ? [...list, ...records] : records);
            setPage(targetPage);
        } catch (error: any) {
            Taro.showToast({ title: error?.message || t("加载失败"), icon: "none" });
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        void load(1, type, state);
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [type, state]);

    usePullDownRefresh(() => {
        void load(1, type, state).finally(() => Taro.stopPullDownRefresh());
    });

    useReachBottom(() => {
        if (list.length < total) {
            void load(page + 1, type, state, true);
        }
    });

    const fmtDate = (ts?: number) => (ts ? new Date(ts * 1000).toLocaleDateString() : "");

    return (
        <TigLayout>
            <TigNavbar title={t("我的礼品卡")} />
            <View className="my-gift-cards">
                {/* 原版口径：页头「购买礼品卡」= 搜索过滤 onlyGiftCardProduct=1（无专属频道页） */}
                <View
                    className="buy-entry"
                    style={{ display: "flex", justifyContent: "flex-end", marginBottom: "16rpx" }}
                    onClick={() => redirect({ url: "/pages/search/result?onlyGiftCardProduct=1" })}
                >
                    <Text style={{ color: "var(--general)", fontSize: "26rpx" }}>{t("购买礼品卡")}</Text>
                </View>
                <View className="tabs">
                    {[
                        { key: 0, label: t("全部") },
                        { key: 1, label: t("储值卡") },
                        { key: 2, label: t("兑换卡") }
                    ].map((item) => (
                        <View
                            key={item.key}
                            className={`tab${type === item.key ? " active" : ""}`}
                            onClick={() => setType(item.key)}
                        >
                            {item.label}
                        </View>
                    ))}
                </View>
                <View className="tabs sub">
                    {[
                        { key: "valid", label: t("有效卡") },
                        { key: "invalid", label: t("已失效") }
                    ].map((item) => (
                        <View
                            key={item.key}
                            className={`tab${state === item.key ? " active" : ""}`}
                            onClick={() => setState(item.key)}
                        >
                            {item.label}
                        </View>
                    ))}
                </View>
                {list.length === 0 && !loading ? (
                    <View className="empty">
                        {t("暂无礼品卡")}
                        <View
                            style={{ marginTop: "20rpx", color: "var(--general)", fontSize: "28rpx" }}
                            onClick={() => redirect({ url: "/pages/search/result?onlyGiftCardProduct=1" })}
                        >
                            {t("去购买礼品卡")}
                        </View>
                    </View>
                ) : (
                    list.map((card) => (
                        <View
                            key={card.giftCardsId}
                            className={`card-item${card.isValid ? "" : " invalid"}`}
                            onClick={() => redirect({ url: `/pages/giftCard/myCardDetail?giftCardsId=${card.giftCardsId}` })}
                        >
                            {card.imageUrl ? <Image className="card-img" src={card.imageUrl} mode="aspectFill" /> : null}
                            <View className="card-main">
                                <View className="card-name">{card.cardGroupName}</View>
                                <View className="card-sub">
                                    {t("卡号")}：{card.cardNumber}
                                </View>
                                <View className="card-sub">
                                    {card.groupType === 1
                                        ? `${t("面额")} ¥${card.faceAmount ?? "-"} · ${t("余额")} ¥${card.balance ?? "-"}`
                                        : t("兑换卡")}
                                    {" · "}
                                    {card.sourceText}
                                    {" · "}
                                    {card.expiredTime ? `${t("有效期至")} ${fmtDate(card.expiredTime)}` : t("永久有效")}
                                </View>
                                {card.groupType === 1 && card.scopeDesc ? (
                                    <View className="card-sub">
                                        {t("适用范围")}：{t(card.scopeDesc)}
                                    </View>
                                ) : null}
                            </View>
                            <Text className={`card-state${card.isValid ? "" : " off"}`}>{card.stateText}</Text>
                        </View>
                    ))
                )}
                {/* 触底加载/终态行（此前翻页零反馈）：page>1 显示转圈，全部加载完显示「没有更多了」 */}
                <LoadingBox
                    modelValue={loading}
                    page={page}
                    length={list.length}
                    finished={!loading && total > PAGE_SIZE && list.length >= total}
                />
            </View>
        </TigLayout>
    );
}
