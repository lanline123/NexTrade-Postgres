import { useState } from "react";
import { Text, View } from "@tarojs/components";
import Taro, { useDidShow, usePullDownRefresh } from "@tarojs/taro";

import { riderMyList, type DeliveryOrderRow } from "@/api/rider/rider";
import { t } from "@/utils/i18n";

import { DELIVERY_STATUS_TEXT, fmtTime } from "./location";

import "./orders.scss";

const TABS = [
    { key: "pending", label: "待取货" },
    { key: "processing", label: "配送中" },
    { key: "completed", label: "已完成" }
] as const;

/** 我的派送（我方新增）：待取货 / 配送中 / 已完成三页签。 */
export default function RiderOrders() {
    const [tab, setTab] = useState<string>("pending");
    const [rows, setRows] = useState<DeliveryOrderRow[]>([]);
    const [loaded, setLoaded] = useState(false);

    const load = async (which: string) => {
        try {
            const res = await riderMyList({ page: 1, size: 50, tab: which });
            setRows(res?.records || []);
        } catch {
            setRows([]);
        } finally {
            setLoaded(true);
            Taro.stopPullDownRefresh();
        }
    };

    useDidShow(() => {
        void load(tab);
    });
    usePullDownRefresh(() => {
        void load(tab);
    });

    const switchTab = (key: string) => {
        setTab(key);
        setLoaded(false);
        void load(key);
    };

    return (
        <View className="rider-page">
            <View className="rider-tabs">
                {TABS.map((it) => (
                    <View key={it.key} className={tab === it.key ? "rider-tab rider-tab--active" : "rider-tab"} onClick={() => switchTab(it.key)}>
                        {t(it.label)}
                    </View>
                ))}
            </View>
            {loaded && rows.length === 0 && <View className="rider-empty">{t("暂无派送单")}</View>}
            {rows.map((row) => (
                <View key={row.deliveryOrderId} className="rider-card" onClick={() => Taro.navigateTo({ url: `/pages/rider/detail?id=${row.deliveryOrderId}` })}>
                    <View className="rider-row">
                        <Text style={{ fontSize: "26rpx", color: "#5c6167" }}>{row.deliveryOrderSn}</Text>
                        <Text className={row.status === 40 ? "rider-tag rider-tag--green" : "rider-tag"}>
                            {t(DELIVERY_STATUS_TEXT[row.status] || "")}
                        </Text>
                    </View>
                    <View className="rider-addr" style={{ marginTop: "14rpx" }}>
                        {(row.regionNames || "") + (row.address || "")}
                    </View>
                    <View className="rider-row" style={{ marginTop: "10rpx" }}>
                        <Text className="rider-sub">
                            {row.consignee} · {fmtTime(row.createTime)}
                        </Text>
                        <Text className="rider-amount">¥{Number(row.orderAmount || 0).toFixed(2)}</Text>
                    </View>
                    {row.status === 40 && Number(row.serviceFee || 0) > 0 ? (
                        <View className="rider-sub" style={{ marginTop: "6rpx" }}>
                            {t("服务费")} ¥{Number(row.serviceFee).toFixed(2)} {t("已入账")}
                        </View>
                    ) : null}
                </View>
            ))}
        </View>
    );
}
