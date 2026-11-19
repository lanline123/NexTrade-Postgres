import { useCallback, useEffect, useRef, useState } from "react";
import { Image, Input, RichText, Text, View } from "@tarojs/components";
import Taro, { useRouter } from "@tarojs/taro";

import TigLayout from "@/components/tig/tig-layout/TigLayout";
import TigLoadingPage from "@/components/tig/tig-loadingpage";
import FormatPrice from "@/components/format-price/FormatPrice";
import {
    freezeAuctionDeposit,
    getAuctionBids,
    getAuctionDetail,
    getAuctionPoll,
    placeAuctionBid,
    quitAuction,
    remindAuction
} from "@/api/auction/auction";
import type { AuctionBidItem, AuctionItemDetail } from "@/types/auction/auction";
import { useUserStore } from "@/store/user";
import { t } from "@/utils/i18n";
import { usePageGuards } from "@/utils/pageMixin";
import { formatCountdown } from "./list";

import "./detail.scss";

/**
 * 拍品详情页（我方新增功能，一期；PRD §7.2 移动端形态）
 * - 底部固定出价栏 + 保证金半屏弹层（金额/冻结与解冻规则/余额冻结确认）
 * - 竞价中 5 秒轮询 poll 接口；倒计时以服务端 remainSeconds 锚定
 */

function money(v: string | number | null | undefined): number {
    const n = Number(v ?? 0);
    return Number.isFinite(n) ? n : 0;
}

function errText(e: unknown): string {
    const m = (e as { message?: string })?.message;
    return m || t("操作失败，请重试");
}

export default function AuctionDetail() {
    usePageGuards();
    const router = useRouter();
    const itemId = Number(router.params.id || 0);
    const token = useUserStore((s) => s.token);

    const [detail, setDetail] = useState<AuctionItemDetail | null>(null);
    const [loading, setLoading] = useState(true);
    const [bidInput, setBidInput] = useState("");
    const [busy, setBusy] = useState("");
    const [showDepositSheet, setShowDepositSheet] = useState(false);
    const [extraBids, setExtraBids] = useState<AuctionBidItem[]>([]);
    const [bidPage, setBidPage] = useState(1);
    const [bidsDone, setBidsDone] = useState(false);
    const anchorRef = useRef({ remain: 0, at: Date.now() });
    const [now, setNow] = useState(() => Date.now());

    const applyDetail = useCallback((d: AuctionItemDetail) => {
        setDetail(d);
        anchorRef.current = { remain: d.remainSeconds, at: Date.now() };
        setExtraBids([]);
        setBidPage(1);
        setBidsDone(false);
    }, []);

    const refresh = useCallback(async () => {
        if (!itemId) return;
        try {
            const d = await getAuctionDetail(itemId);
            applyDetail(d);
        } catch {
            /* 静默，下轮重试 */
        } finally {
            setLoading(false);
        }
    }, [itemId, applyDetail]);

    useEffect(() => {
        refresh();
    }, [refresh, token]);

    useEffect(() => {
        const timer = setInterval(() => setNow(Date.now()), 1000);
        return () => clearInterval(timer);
    }, []);

    useEffect(() => {
        if (!detail || (detail.status !== 1 && detail.status !== 2)) return;
        const timer = setInterval(async () => {
            try {
                const p = await getAuctionPoll(itemId);
                setDetail((prev) =>
                    prev
                        ? {
                              ...prev,
                              status: p.status,
                              statusName: p.statusName,
                              currentPrice: p.currentPrice,
                              hasBid: p.hasBid,
                              bidCount: p.bidCount,
                              realEndTime: p.realEndTime,
                              remainSeconds: p.remainSeconds,
                              extendCount: p.extendCount,
                              isLeader: p.isLeader,
                              minNextBid: p.minNextBid,
                              stepAmount: p.stepAmount,
                              bids: p.latestBids.length ? p.latestBids : prev.bids
                          }
                        : prev
                );
                anchorRef.current = { remain: p.remainSeconds, at: Date.now() };
                if (p.status !== 1 && p.status !== 2) refresh();
            } catch {
                /* 静默 */
            }
        }, 5000);
        return () => clearInterval(timer);
    }, [detail?.status, itemId, refresh]);

    if (loading || !detail) {
        return (
            <TigLayout>
                <TigLoadingPage />
            </TigLayout>
        );
    }

    const remain = Math.max(0, anchorRef.current.remain - Math.floor((now - anchorRef.current.at) / 1000));
    const isRunning = detail.status === 2;
    const isPreview = detail.status === 1;
    const isFinished = detail.status >= 3;
    const frozen = detail.myDepositStatus === 1;
    const deposit = money(detail.depositAmount);
    const step = Math.max(1, money(detail.stepAmount));
    const minNext = money(detail.minNextBid);
    const bidValue = Number(bidInput) || 0;
    const bids = [...detail.bids, ...extraBids];

    const toast = (title: string) => Taro.showToast({ title, icon: "none" });

    const doFreeze = async () => {
        setBusy("freeze");
        try {
            await freezeAuctionDeposit(itemId);
            setShowDepositSheet(false);
            toast(t("保证金已冻结"));
            await refresh();
        } catch (e) {
            toast(errText(e));
        } finally {
            setBusy("");
        }
    };

    const doBid = async (price: number, isBuyout = false) => {
        setBusy(isBuyout ? "buyout" : "bid");
        try {
            const market = money(detail.productPrice);
            const needConfirm = !isBuyout && market > 0 && price > market;
            if (needConfirm) {
                const r = await Taro.showModal({
                    title: t("出价高于市场价"),
                    content: t("您的出价高于该商品市场价，确认继续出价吗？")
                });
                if (!r.confirm) return;
            }
            const res = await placeAuctionBid(itemId, price, { isBuyout, confirmHigh: needConfirm });
            toast(isBuyout || res.isBuyout ? t("一口价成交成功") : t("出价成功"));
            await refresh();
        } catch (e) {
            toast(errText(e));
        } finally {
            setBusy("");
        }
    };

    const doQuit = async () => {
        const r = await Taro.showModal({
            title: t("退出参拍"),
            content: t("退出后保证金将解冻退回，确认退出？")
        });
        if (!r.confirm) return;
        setBusy("quit");
        try {
            await quitAuction(itemId);
            toast(t("已退出参拍"));
            await refresh();
        } catch (e) {
            toast(errText(e));
        } finally {
            setBusy("");
        }
    };

    const doRemind = async () => {
        setBusy("remind");
        try {
            await remindAuction(itemId, !detail.reminded);
            setDetail((p) => (p ? { ...p, reminded: !p.reminded } : p));
        } catch (e) {
            toast(errText(e));
        } finally {
            setBusy("");
        }
    };

    const loadMoreBids = async () => {
        const next = bidPage + 1;
        try {
            const res = await getAuctionBids(itemId, next, 20);
            if (!res.records.length) {
                setBidsDone(true);
                return;
            }
            setExtraBids((p) => [...p, ...res.records]);
            setBidPage(next);
            if (res.records.length < 20) setBidsDone(true);
        } catch {
            setBidsDone(true);
        }
    };

    const goLogin = () => Taro.navigateTo({ url: "/pages/login/index" });
    const goMyAuction = () => Taro.navigateTo({ url: "/pages/auction/mylist" });

    const renderBar = () => {
        if (!token && !isFinished) {
            return (
                <View className="auction-bar">
                    <View className="auction-bar-btn is-primary" onClick={goLogin}>
                        <Text>{t("登录后参与竞价")}</Text>
                    </View>
                </View>
            );
        }
        if (detail.banReason) {
            return (
                <View className="auction-bar">
                    <Text className="auction-bar-blocked">{detail.banReason}</Text>
                </View>
            );
        }
        if (isPreview) {
            return (
                <View className="auction-bar">
                    <View className="auction-bar-btn" onClick={doRemind}>
                        <Text>{detail.reminded ? t("取消提醒") : t("开拍提醒")}</Text>
                    </View>
                    {frozen ? (
                        <View className="auction-bar-btn is-disabled">
                            <Text>{t("保证金已冻结")}</Text>
                        </View>
                    ) : detail.requireCertification && !detail.myCertified ? (
                        <View
                            className="auction-bar-btn is-primary"
                            onClick={() => toast(t("本场需企业认证，请在 PC 端完成认证后参拍"))}
                        >
                            <Text>{t("需企业认证")}</Text>
                        </View>
                    ) : (
                        <View className="auction-bar-btn is-primary" onClick={() => setShowDepositSheet(true)}>
                            <Text>
                                {t("提前冻结保证金")} ¥{deposit.toLocaleString()}
                            </Text>
                        </View>
                    )}
                </View>
            );
        }
        if (isRunning) {
            if (detail.requireCertification && !detail.myCertified && !frozen) {
                return (
                    <View className="auction-bar">
                        <View
                            className="auction-bar-btn is-primary"
                            onClick={() => toast(t("本场需企业认证，请在 PC 端完成认证后参拍"))}
                        >
                            <Text>{t("本场需企业认证后参拍")}</Text>
                        </View>
                    </View>
                );
            }
            if (!frozen) {
                return (
                    <View className="auction-bar">
                        <View className="auction-bar-btn is-primary" onClick={() => setShowDepositSheet(true)}>
                            <Text>
                                {t("冻结保证金")} ¥{deposit.toLocaleString()} {t("参拍")}
                            </Text>
                        </View>
                    </View>
                );
            }
            if (detail.isLeader) {
                return (
                    <View className="auction-bar">
                        <Text className="auction-bar-leader">
                            {t("您已出价")} ¥{money(detail.myHighestBid).toLocaleString()}，{t("当前领先")}
                        </Text>
                        <View className="auction-bar-btn" onClick={doQuit}>
                            <Text>{t("退出参拍")}</Text>
                        </View>
                    </View>
                );
            }
            return (
                <View className="auction-bar is-bid">
                    <View className="auction-stepper">
                        <View
                            className="auction-step-btn"
                            onClick={() => setBidInput(String(Math.max(minNext, (bidValue || minNext) - step)))}
                        >
                            <Text>−</Text>
                        </View>
                        <Input
                            className="auction-bid-input"
                            type="number"
                            value={bidInput || String(minNext)}
                            onInput={(e) => setBidInput(e.detail.value)}
                        />
                        <View
                            className="auction-step-btn"
                            onClick={() => setBidInput(String((bidValue || minNext) + step))}
                        >
                            <Text>+</Text>
                        </View>
                    </View>
                    <View
                        className={`auction-bar-btn is-primary${bidValue > 0 && bidValue < minNext ? " is-disabled" : ""}`}
                        onClick={() => {
                            if (busy) return;
                            const price = bidValue || minNext;
                            if (price < minNext) {
                                toast(`${t("出价不能低于")} ¥${minNext.toLocaleString()}`);
                                return;
                            }
                            doBid(price);
                        }}
                    >
                        <Text>{t("出价")}</Text>
                    </View>
                </View>
            );
        }
        // 已结束
        return (
            <View className="auction-bar">
                <Text className="auction-bar-ended">
                    {detail.status === 3
                        ? detail.isWinner
                            ? t("恭喜中标！请及时完成订单支付")
                            : t("本场已成交")
                        : detail.statusName}
                </Text>
                {detail.orderId ? (
                    <View
                        className="auction-bar-btn is-primary"
                        onClick={() => Taro.navigateTo({ url: `/pages/user/order/info?id=${detail.orderId}` })}
                    >
                        <Text>{t("查看订单")}</Text>
                    </View>
                ) : (
                    <View className="auction-bar-btn" onClick={goMyAuction}>
                        <Text>{t("我的拍卖")}</Text>
                    </View>
                )}
            </View>
        );
    };

    return (
        <TigLayout>
            <View className="auction-detail">
                <View className="auction-detail-img">
                    <Image src={detail.picUrl || detail.picThumb} mode="aspectFill" />
                    <Text className={`auction-badge ${isRunning ? "is-running" : isPreview ? "is-preview" : "is-sold"}`}>
                        {detail.statusName}
                    </Text>
                </View>

                <View className="auction-detail-head">
                    <Text className="auction-detail-title">{detail.title}</Text>
                    {detail.productBrief ? <Text className="auction-detail-brief">{detail.productBrief}</Text> : null}
                    <View className="auction-detail-price">
                        <Text className="auction-detail-price-label">
                            {isFinished && detail.status === 3 ? t("成交价") : detail.hasBid ? t("当前价") : t("起拍价")}
                        </Text>
                        <FormatPrice priceData={detail.hasBid || detail.status === 3 ? detail.currentPrice : detail.startPrice} />
                        {detail.isLeader && isRunning ? <Text className="auction-tag is-leader">{t("最高出价者")}</Text> : null}
                        {detail.isWinner ? <Text className="auction-tag is-leader">{t("您已中标")}</Text> : null}
                    </View>
                    <View className="auction-detail-sub">
                        <Text>
                            {t("起拍价")} ¥{money(detail.startPrice).toLocaleString()}
                        </Text>
                        <Text>
                            {t("市场价")} ¥{money(detail.productPrice).toLocaleString()}
                        </Text>
                        <Text>
                            {t("出价")} {detail.bidCount} {t("次")}
                        </Text>
                    </View>
                    {(isRunning || isPreview) && (
                        <View className={`auction-detail-count${isRunning && remain <= 300 ? " is-urgent" : ""}`}>
                            <Text>
                                {isPreview ? t("距开拍") : t("距结束")} {formatCountdown(remain)}
                                {detail.extendCount > 0 ? ` · ${t("已延时")} ${detail.extendCount} ${t("次")}` : ""}
                            </Text>
                        </View>
                    )}
                    {isFinished && detail.status === 3 && detail.winnerMaskedName ? (
                        <View className="auction-detail-count">
                            <Text>
                                {t("中标")} {detail.winnerMaskedName}
                            </Text>
                        </View>
                    ) : null}
                </View>

                <View className="auction-detail-facts">
                    <View className="auction-fact">
                        <Text className="auction-fact-k">{t("参拍保证金")}</Text>
                        <Text className="auction-fact-v">¥{deposit.toLocaleString()}</Text>
                    </View>
                    <View className="auction-fact">
                        <Text className="auction-fact-k">{t("加价幅度")}</Text>
                        <Text className="auction-fact-v">¥{step.toLocaleString()}</Text>
                    </View>
                    <View className="auction-fact">
                        <Text className="auction-fact-k">{t("开拍时间")}</Text>
                        <Text className="auction-fact-v">{detail.startTimeText}</Text>
                    </View>
                    <View className="auction-fact">
                        <Text className="auction-fact-k">{t("结束时间")}</Text>
                        <Text className="auction-fact-v">{detail.endTimeText}</Text>
                    </View>
                    {detail.buyoutPrice ? (
                        <View className="auction-fact">
                            <Text className="auction-fact-k">{t("一口价")}</Text>
                            <Text className="auction-fact-v">¥{money(detail.buyoutPrice).toLocaleString()}</Text>
                        </View>
                    ) : null}
                </View>

                {isRunning && frozen && !detail.isLeader && detail.buyoutPrice ? (
                    <View className="auction-buyout-row">
                        <View
                            className="auction-buyout-btn"
                            onClick={async () => {
                                const r = await Taro.showModal({
                                    title: t("一口价成交"),
                                    content: `${t("确认以")} ¥${money(detail.buyoutPrice).toLocaleString()} ${t("直接成交？")}`
                                });
                                if (r.confirm) doBid(money(detail.buyoutPrice), true);
                            }}
                        >
                            <Text>
                                {t("一口价")} ¥{money(detail.buyoutPrice).toLocaleString()} {t("直接成交")}
                            </Text>
                        </View>
                        <View className="auction-quit-btn" onClick={doQuit}>
                            <Text>{t("退出参拍")}</Text>
                        </View>
                    </View>
                ) : null}

                <View className="auction-detail-section">
                    <Text className="auction-detail-section-title">
                        {t("出价记录")}（{detail.bidCount}）
                    </Text>
                    {bids.length ? (
                        bids.map((b) => (
                            <View key={b.bidId} className={`auction-bid-row${b.isLeader ? " is-leader" : ""}`}>
                                <Text className="auction-bid-name">
                                    {b.maskedName}
                                    {b.isMine ? `（${t("我")}）` : ""}
                                    {b.isBuyout ? ` · ${t("一口价")}` : ""}
                                </Text>
                                <Text className="auction-bid-price">¥{money(b.bidPrice).toLocaleString()}</Text>
                                <Text className="auction-bid-time">{b.addTimeText}</Text>
                            </View>
                        ))
                    ) : (
                        <Text className="auction-detail-empty">{t("暂无出价")}</Text>
                    )}
                    {bids.length >= 20 && !bidsDone ? (
                        <View className="auction-more" onClick={loadMoreBids}>
                            <Text>{t("加载更多")}</Text>
                        </View>
                    ) : null}
                </View>

                <View className="auction-detail-section">
                    <Text className="auction-detail-section-title">{t("竞拍规则")}</Text>
                    <Text className="auction-rules-text">
                        {t("参拍前需从账户余额冻结保证金，未中标在拍卖结束后自动解冻；每次出价至少加一档，结束前 5 分钟内有新出价将顺延 5 分钟；中标后保证金抵扣货款，需在 48 小时内完成支付，逾期将按规则扣罚。")}
                    </Text>
                </View>

                {detail.productDesc ? (
                    <View className="auction-detail-section">
                        <Text className="auction-detail-section-title">{t("商品详情")}</Text>
                        <RichText className="v-html" nodes={detail.productDesc} />
                    </View>
                ) : null}

                <View className="auction-detail-spacer" />
                {renderBar()}

                {showDepositSheet && (
                    <View className="auction-sheet-mask" onClick={() => setShowDepositSheet(false)}>
                        <View className="auction-sheet" onClick={(e) => e.stopPropagation()}>
                            <Text className="auction-sheet-title">{t("冻结参拍保证金")}</Text>
                            <View className="auction-sheet-amount">
                                <Text>
                                    {t("保证金金额")} ¥{deposit.toLocaleString()}
                                </Text>
                            </View>
                            <Text className="auction-sheet-text">
                                {t("保证金将从账户余额中冻结：未中标在拍卖结束后自动解冻；中标后抵扣货款；中标后逾期未支付将按规则扣罚。冻结后即可出价，出价被超过时可继续加价。")}
                            </Text>
                            <View className="auction-sheet-actions">
                                <View className="auction-bar-btn" onClick={() => setShowDepositSheet(false)}>
                                    <Text>{t("再想想")}</Text>
                                </View>
                                <View
                                    className={`auction-bar-btn is-primary${busy === "freeze" ? " is-disabled" : ""}`}
                                    onClick={() => {
                                        if (busy !== "freeze") doFreeze();
                                    }}
                                >
                                    <Text>{t("确认冻结")}</Text>
                                </View>
                            </View>
                        </View>
                    </View>
                )}
            </View>
        </TigLayout>
    );
}
