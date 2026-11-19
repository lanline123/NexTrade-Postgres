import { useEffect, useState } from "react";
import { Image, Text, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import TigLayout from "@/components/tig/tig-layout/TigLayout";
import EmptyBox from "@/components/empty-box/EmptyBox";
import TigLoadingPage from "@/components/tig/tig-loadingpage";
import FormatPrice from "@/components/format-price/FormatPrice";
import { getMyAuctionDeposits, getMyAuctionJoins } from "@/api/auction/auction";
import type { AuctionDepositItem, AuctionMyJoinItem } from "@/types/auction/auction";
import { useUserStore } from "@/store/user";
import { t } from "@/utils/i18n";
import { usePageGuards } from "@/utils/pageMixin";
import { formatCountdown } from "./list";

import "./mylist.scss";

/**
 * 我的拍卖（我方新增功能，一期）
 * - 我的出价（my/joins）/ 保证金记录（my/deposits）/ 违约记录（保证金扣罚流水口径）
 * - 后端一期无独立违约记录接口，违约页签以保证金状态 4/5（已扣罚/部分扣罚后退回）呈现
 */

type TabKey = "joins" | "deposits" | "penalty";

const DEPOSIT_STATUS_TEXT: Record<number, string> = {
    0: "未冻结",
    1: "冻结中",
    2: "已解冻",
    3: "已抵扣",
    4: "已扣罚",
    5: "部分扣罚后退回"
};

function money(v: string | number | null | undefined): number {
    const n = Number(v ?? 0);
    return Number.isFinite(n) ? n : 0;
}

export default function AuctionMyList() {
    usePageGuards();
    const token = useUserStore((s) => s.token);
    const [tab, setTab] = useState<TabKey>("joins");
    const [joins, setJoins] = useState<AuctionMyJoinItem[]>([]);
    const [deposits, setDeposits] = useState<AuctionDepositItem[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        if (!token) {
            setLoading(false);
            return;
        }
        (async () => {
            try {
                const [j, d] = await Promise.all([getMyAuctionJoins(), getMyAuctionDeposits(1, 50)]);
                setJoins(Array.isArray(j) ? j : []);
                setDeposits(d.records || []);
            } catch {
                /* 保持空态 */
            } finally {
                setLoading(false);
            }
        })();
    }, [token]);

    const penaltyRecords = deposits.filter((d) => d.status === 4 || d.status === 5);

    const toDetail = (itemId: number) => Taro.navigateTo({ url: `/pages/auction/detail?id=${itemId}` });

    const renderDepositRow = (d: AuctionDepositItem) => (
        <View key={d.depositId} className="auction-dep-row" onClick={() => toDetail(d.itemId)}>
            <View className="auction-dep-main">
                <Text className="auction-dep-title">{d.itemTitle}</Text>
                <Text className="auction-dep-time">
                    {d.addTimeText}
                    {d.handleTimeText ? ` · ${t("处理")} ${d.handleTimeText}` : ""}
                </Text>
            </View>
            <View className="auction-dep-side">
                <Text className="auction-dep-amount">¥{money(d.amount).toLocaleString()}</Text>
                <Text className={`auction-dep-status is-${d.status}`}>{d.statusName}</Text>
                {money(d.penaltyAmount) > 0 ? (
                    <Text className="auction-dep-penalty">
                        {t("扣罚")} ¥{money(d.penaltyAmount).toLocaleString()}
                    </Text>
                ) : null}
            </View>
        </View>
    );

    return (
        <TigLayout>
            <View className="auction-my">
                <View className="auction-tabs">
                    <View className={`auction-tab${tab === "joins" ? " is-active" : ""}`} onClick={() => setTab("joins")}>
                        <Text>{t("我的出价")}</Text>
                    </View>
                    <View
                        className={`auction-tab${tab === "deposits" ? " is-active" : ""}`}
                        onClick={() => setTab("deposits")}
                    >
                        <Text>{t("保证金记录")}</Text>
                    </View>
                    <View
                        className={`auction-tab${tab === "penalty" ? " is-active" : ""}`}
                        onClick={() => setTab("penalty")}
                    >
                        <Text>
                            {t("违约记录")}
                            {penaltyRecords.length ? ` (${penaltyRecords.length})` : ""}
                        </Text>
                    </View>
                </View>

                {!token ? (
                    <View className="auction-my-login">
                        <Text>{t("请先登录")}</Text>
                        <View className="auction-my-login-btn" onClick={() => Taro.navigateTo({ url: "/pages/login/index" })}>
                            <Text>{t("去登录")}</Text>
                        </View>
                    </View>
                ) : loading ? (
                    <TigLoadingPage />
                ) : tab === "joins" ? (
                    joins.length ? (
                        <View className="auction-my-list">
                            {joins.map((j) => {
                                const payRemain =
                                    j.myStatus === "won" && j.payDeadline
                                        ? Math.max(0, j.payDeadline - Math.floor(Date.now() / 1000))
                                        : null;
                                return (
                                    <View key={j.itemId} className="auction-my-row" onClick={() => toDetail(j.itemId)}>
                                        <Image className="auction-my-thumb" src={j.picThumb || j.picUrl} mode="aspectFill" />
                                        <View className="auction-my-body">
                                            <Text className="auction-my-title">{j.title}</Text>
                                            <Text className="auction-my-meta">
                                                {t("我的最高出价")} ¥{money(j.myHighestBid).toLocaleString()} ·{" "}
                                                {t("保证金")}：{t(DEPOSIT_STATUS_TEXT[j.myDepositStatus] || "-")}
                                            </Text>
                                            <View className="auction-my-price">
                                                <Text className="auction-my-price-label">
                                                    {j.status === 3 ? t("成交价") : t("当前价")}
                                                </Text>
                                                <FormatPrice priceData={j.status === 3 && j.finalPrice ? j.finalPrice : j.currentPrice} />
                                            </View>
                                            {payRemain !== null ? (
                                                <Text className="auction-my-warn">
                                                    {t("请在")} {formatCountdown(payRemain)} {t("内完成支付")}
                                                </Text>
                                            ) : null}
                                        </View>
                                        <Text className={`auction-tag is-${j.myStatus}`}>{j.myStatusName}</Text>
                                    </View>
                                );
                            })}
                        </View>
                    ) : (
                        <EmptyBox icon="salesman/no_order.png" text={t("暂无参拍记录")} />
                    )
                ) : tab === "deposits" ? (
                    deposits.length ? (
                        <View className="auction-my-list">{deposits.map(renderDepositRow)}</View>
                    ) : (
                        <EmptyBox icon="salesman/no_order.png" text={t("暂无保证金记录")} />
                    )
                ) : penaltyRecords.length ? (
                    <View className="auction-my-list">{penaltyRecords.map(renderDepositRow)}</View>
                ) : (
                    <EmptyBox icon="salesman/no_order.png" text={t("暂无违约扣罚记录")} />
                )}
            </View>
        </TigLayout>
    );
}
