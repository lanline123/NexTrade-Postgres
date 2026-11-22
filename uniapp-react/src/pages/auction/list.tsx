import { useCallback, useEffect, useRef, useState } from "react";
import { Image, ScrollView, Text, View } from "@tarojs/components";
import Taro, { useReachBottom } from "@tarojs/taro";

import TigLayout from "@/components/tig/tig-layout/TigLayout";
import EmptyBox from "@/components/empty-box/EmptyBox";
import LoadingBox from "@/components/loading-box";
import TigLoadingPage from "@/components/tig/tig-loadingpage";
import FormatPrice from "@/components/format-price/FormatPrice";
import { getAuctionItems, getAuctionSessions } from "@/api/auction/auction";
import type { AuctionItem, AuctionSessionItem } from "@/types/auction/auction";
import { t } from "@/utils/i18n";
import { usePageGuards } from "@/utils/pageMixin";

import "./list.scss";

/**
 * 拍卖频道页（我方新增功能，一期）
 * - 场次切换 + 状态页签（全部/即将开始/竞价中/已结束）+ 排序
 * - 卡片：当前价/起拍价、倒计时（服务端 remainSeconds 锚定）、出价次数、状态角标
 */

const TABS = [
    { key: "all", label: "全部" },
    { key: "preview", label: "即将开始" },
    { key: "running", label: "竞价中" },
    { key: "finished", label: "已结束" }
];

const SORTS = [
    { key: "ending", label: "即将结束" },
    { key: "newest", label: "最新上架" },
    { key: "priceAsc", label: "价格↑" },
    { key: "priceDesc", label: "价格↓" }
];

const PAGE_SIZE = 20;

function pad(n: number) {
    return String(n).padStart(2, "0");
}

export function formatCountdown(totalSeconds: number): string {
    const s = Math.max(0, Math.floor(totalSeconds));
    const d = Math.floor(s / 86400);
    const h = Math.floor((s % 86400) / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = s % 60;
    if (d > 0) return `${d}${"天"} ${pad(h)}:${pad(m)}:${pad(sec)}`;
    return `${pad(h)}:${pad(m)}:${pad(sec)}`;
}

function statusCls(status: number): string {
    if (status === 2) return "is-running";
    if (status === 1) return "is-preview";
    if (status === 3) return "is-sold";
    return "is-ended";
}

export default function AuctionList() {
    usePageGuards();
    const [sessions, setSessions] = useState<AuctionSessionItem[]>([]);
    const [sessionId, setSessionId] = useState<number>(0);
    const [tab, setTab] = useState("all");
    const [sort, setSort] = useState("ending");
    const [items, setItems] = useState<AuctionItem[]>([]);
    const [total, setTotal] = useState(0);
    const [loading, setLoading] = useState(true);
    /** 首屏失败与真空态此前同形（catch 清空 items 落 EmptyBox）：分开落错误态给重试；上拉态用 state 驱动页脚反馈 */
    const [loadError, setLoadError] = useState(false);
    const [loadingMore, setLoadingMore] = useState(false);
    const pageRef = useRef(1);
    const anchorRef = useRef(Date.now());
    const [now, setNow] = useState(() => Date.now());
    const loadingMoreRef = useRef(false);

    const load = useCallback(
        async (page: number, append: boolean) => {
            try {
                const res = await getAuctionItems({
                    tab,
                    sessionId: sessionId || undefined,
                    sort,
                    page,
                    pageSize: PAGE_SIZE
                });
                anchorRef.current = Date.now();
                setTotal(res.total || 0);
                setItems((prev) => (append ? [...prev, ...res.records] : res.records));
                pageRef.current = page;
                if (!append) setLoadError(false);
            } catch {
                if (!append) {
                    setItems([]);
                    setLoadError(true);
                }
            } finally {
                setLoading(false);
                loadingMoreRef.current = false;
                setLoadingMore(false);
            }
        },
        [tab, sessionId, sort]
    );

    useEffect(() => {
        getAuctionSessions()
            .then((res) => setSessions(Array.isArray(res) ? res : []))
            .catch(() => setSessions([]));
    }, []);

    useEffect(() => {
        setLoading(true);
        load(1, false);
    }, [load]);

    useEffect(() => {
        const timer = setInterval(() => setNow(Date.now()), 1000);
        return () => clearInterval(timer);
    }, []);

    useReachBottom(() => {
        if (loadingMoreRef.current || items.length >= total) return;
        loadingMoreRef.current = true;
        setLoadingMore(true);
        load(pageRef.current + 1, true);
    });

    const remainOf = (item: AuctionItem) =>
        Math.max(0, item.remainSeconds - Math.floor((now - anchorRef.current) / 1000));

    const toDetail = (item: AuctionItem) => {
        Taro.navigateTo({ url: `/pages/auction/detail?id=${item.itemId}` });
    };

    return (
        <TigLayout>
            <View className="auction-list">
                {sessions.length > 0 && (
                    <ScrollView scrollX className="auction-sessions">
                        <View
                            className={`auction-session${sessionId === 0 ? " is-active" : ""}`}
                            onClick={() => setSessionId(0)}
                        >
                            <Text>{t("全部场次")}</Text>
                        </View>
                        {sessions.map((s) => (
                            <View
                                key={s.sessionId}
                                className={`auction-session${sessionId === s.sessionId ? " is-active" : ""}`}
                                onClick={() => setSessionId(s.sessionId)}
                            >
                                <Text className="auction-session-name">{s.sessionName}</Text>
                                <Text className="auction-session-time">
                                    {s.startTimeText} ~ {s.endTimeText}
                                </Text>
                            </View>
                        ))}
                    </ScrollView>
                )}

                <View className="auction-tabs">
                    {TABS.map((x) => (
                        <View
                            key={x.key}
                            className={`auction-tab${tab === x.key ? " is-active" : ""}`}
                            onClick={() => setTab(x.key)}
                        >
                            <Text>{t(x.label)}</Text>
                        </View>
                    ))}
                </View>
                <View className="auction-sorts">
                    {SORTS.map((x) => (
                        <Text
                            key={x.key}
                            className={`auction-sort${sort === x.key ? " is-active" : ""}`}
                            onClick={() => setSort(x.key)}
                        >
                            {t(x.label)}
                        </Text>
                    ))}
                </View>

                {loading ? (
                    <TigLoadingPage />
                ) : items.length === 0 && loadError ? (
                    <EmptyBox
                        height="auto"
                        text={t("拍品加载失败，请检查网络后重试")}
                        actionText={t("重新加载")}
                        onAction={() => {
                            setLoading(true);
                            void load(1, false);
                        }}
                    />
                ) : items.length === 0 ? (
                    <EmptyBox icon="salesman/no_order.png" text={t("暂无拍品")} />
                ) : (
                    <View className="auction-grid">
                        {items.map((item) => {
                            const remain = remainOf(item);
                            const live = item.status === 1 || item.status === 2;
                            return (
                                <View key={item.itemId} className="auction-card" onClick={() => toDetail(item)}>
                                    <View className="auction-card-img">
                                        <Image src={item.picThumb || item.picUrl} mode="aspectFill" />
                                        <Text className={`auction-badge ${statusCls(item.status)}`}>
                                            {item.statusName}
                                        </Text>
                                        {item.buyoutPrice ? (
                                            <Text className="auction-buyout-tag">{t("一口价")}</Text>
                                        ) : null}
                                    </View>
                                    <View className="auction-card-body">
                                        <Text className="auction-card-title">{item.title}</Text>
                                        <View className="auction-card-price">
                                            <Text className="auction-card-price-label">
                                                {item.status === 3
                                                    ? t("成交价")
                                                    : item.hasBid
                                                      ? t("当前价")
                                                      : t("起拍价")}
                                            </Text>
                                            <FormatPrice
                                                priceData={
                                                    item.status === 3 && item.finalPrice
                                                        ? item.finalPrice
                                                        : item.currentPrice
                                                }
                                            />
                                        </View>
                                        <View className="auction-card-meta">
                                            <Text>
                                                {t("出价")} {item.bidCount} {t("次")}
                                            </Text>
                                            {live ? (
                                                <Text
                                                    className={`auction-card-count${
                                                        item.status === 2 && remain <= 300 ? " is-urgent" : ""
                                                    }`}
                                                >
                                                    {item.status === 1 ? t("距开始") : t("距结束")}{" "}
                                                    {formatCountdown(remain)}
                                                </Text>
                                            ) : item.status === 3 && item.winnerMaskedName ? (
                                                <Text>
                                                    {t("中标")} {item.winnerMaskedName}
                                                </Text>
                                            ) : null}
                                        </View>
                                    </View>
                                </View>
                            );
                        })}
                    </View>
                )}
                {!loading && items.length > 0 && (
                    <LoadingBox
                        modelValue={loadingMore}
                        page={Math.max(2, pageRef.current)}
                        length={items.length}
                        finished={!loadingMore && items.length >= total && total > PAGE_SIZE}
                    />
                )}
            </View>
        </TigLayout>
    );
}
