import { useState } from "react";
import { Text, View } from "@tarojs/components";
import Taro, { useDidShow } from "@tarojs/taro";

import { riderMyApplies, riderMyInfo, riderStatistics, riderUpdateWorkStatus, type RiderInfo } from "@/api/rider/rider";
import { t } from "@/utils/i18n";

import "./index.scss";

/**
 * 骑手工作台（我方新增）：人员信息 + 在线/离线开关 + 单量统计 + 功能入口。
 * 未绑定服务人员的会员在此看到入驻申请入口与申请进度。
 */
export default function RiderIndex() {
    const [info, setInfo] = useState<RiderInfo | null>(null);
    const [loaded, setLoaded] = useState(false);
    const [stats, setStats] = useState<{ total: number; pending: number; processing: number; completed: number } | null>(null);
    const [applies, setApplies] = useState<any[]>([]);
    const [toggling, setToggling] = useState(false);

    const load = async () => {
        try {
            const me = await riderMyInfo();
            setInfo(me);
            if (me) {
                const s = await riderStatistics().catch(() => null);
                setStats(s);
            } else {
                const a = await riderMyApplies().catch(() => null);
                setApplies(a?.records || []);
            }
        } catch {
            setInfo(null);
        } finally {
            setLoaded(true);
        }
    };

    useDidShow(() => {
        void load();
    });

    const toggleWork = async () => {
        if (!info || toggling) return;
        const next = info.workStatus === 0 ? 1 : 0;
        setToggling(true);
        try {
            await riderUpdateWorkStatus(next);
            setInfo({ ...info, workStatus: next });
            Taro.showToast({ title: next === 1 ? t("已上线，可接单") : t("已下线"), icon: "none" });
        } catch (e: any) {
            Taro.showToast({ title: e?.message || t("切换失败"), icon: "none" });
        } finally {
            setToggling(false);
        }
    };

    if (!loaded) {
        return (
            <View className="rider-page">
                <View className="rider-empty">{t("加载中…")}</View>
            </View>
        );
    }

    if (!info) {
        return (
            <View className="rider-page">
                <View className="rider-card">
                    <View className="rider-title">{t("成为骑手 / 服务人员")}</View>
                    <View className="rider-sub">{t("入驻店铺后可接同城配送与上门服务订单，服务费实时入账钱包")}</View>
                    <View className="rider-btn" style={{ marginTop: "28rpx" }} onClick={() => Taro.navigateTo({ url: "/pages/rider/apply" })}>
                        {t("申请入驻")}
                    </View>
                </View>
                {applies.length > 0 && (
                    <View className="rider-card">
                        <View className="rider-title" style={{ fontSize: "30rpx" }}>{t("我的申请")}</View>
                        {applies.map((a) => (
                            <View key={a.applyId} className="rider-row" style={{ padding: "16rpx 0", borderTop: "1rpx solid #f0f1f3" }}>
                                <View>
                                    <Text>{a.type === 2 ? t("服务员") : t("配送员")} · {t("店铺")} #{a.shopId}</Text>
                                    {a.status === 2 && a.auditRemark ? <View className="rider-sub">{a.auditRemark}</View> : null}
                                </View>
                                <Text className={a.status === 1 ? "rider-tag rider-tag--green" : a.status === 2 ? "rider-tag rider-tag--grey" : "rider-tag"}>
                                    {a.status === 0 ? t("待审核") : a.status === 1 ? t("已通过") : t("已驳回")}
                                </Text>
                            </View>
                        ))}
                    </View>
                )}
            </View>
        );
    }

    const online = info.workStatus !== 0;
    return (
        <View className="rider-page">
            <View className="rider-card">
                <View className="rider-row">
                    <View>
                        <View className="rider-title">{info.name}</View>
                        <View className="rider-sub">
                            {info.type === 2 ? t("服务员") : t("配送员")} · {t("店铺")} #{info.storeId} · {info.phone}
                        </View>
                    </View>
                    <Text className={online ? "rider-tag rider-tag--green" : "rider-tag rider-tag--grey"}>
                        {info.status === 0 ? t("已被禁用") : info.workStatus === 2 ? t("忙碌中") : online ? t("在线") : t("已下线")}
                    </Text>
                </View>
                {info.status === 1 && (
                    <View className={online ? "rider-btn rider-btn--plain" : "rider-btn"} style={{ marginTop: "24rpx" }} onClick={toggleWork}>
                        {toggling ? t("切换中…") : online ? t("下线休息") : t("上线接单")}
                    </View>
                )}
            </View>

            {stats && (
                <View className="rider-card rider-row" style={{ textAlign: "center" }}>
                    <View style={{ flex: 1 }}>
                        <View className="rider-title">{stats.pending}</View>
                        <View className="rider-sub">{t("待取货")}</View>
                    </View>
                    <View style={{ flex: 1 }}>
                        <View className="rider-title">{stats.processing}</View>
                        <View className="rider-sub">{info.type === 2 ? t("服务中") : t("配送中")}</View>
                    </View>
                    <View style={{ flex: 1 }}>
                        <View className="rider-title">{stats.completed}</View>
                        <View className="rider-sub">{t("已完成")}</View>
                    </View>
                    <View style={{ flex: 1 }}>
                        <View className="rider-title">{info.completedOrderCount || 0}</View>
                        <View className="rider-sub">{t("累计单量")}</View>
                    </View>
                </View>
            )}

            <View className="rider-card">
                <View className="rider-row" style={{ padding: "14rpx 0" }} onClick={() => Taro.navigateTo({ url: "/pages/rider/hall" })}>
                    <Text style={{ fontSize: "30rpx" }}>{t("接单大厅")}</Text>
                    <Text className="rider-sub">{t("本店待抢订单")} ›</Text>
                </View>
                <View className="rider-row" style={{ padding: "14rpx 0", borderTop: "1rpx solid #f0f1f3" }} onClick={() => Taro.navigateTo({ url: "/pages/rider/orders" })}>
                    <Text style={{ fontSize: "30rpx" }}>{t("我的派送")}</Text>
                    <Text className="rider-sub">{t("待取货 / 配送中 / 已完成")} ›</Text>
                </View>
                <View className="rider-row" style={{ padding: "14rpx 0", borderTop: "1rpx solid #f0f1f3" }} onClick={() => Taro.navigateTo({ url: "/pages/rider/wallet" })}>
                    <Text style={{ fontSize: "30rpx" }}>{t("我的钱包")}</Text>
                    <Text className="rider-sub">{t("余额 / 明细 / 提现")} ›</Text>
                </View>
            </View>
        </View>
    );
}
