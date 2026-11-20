import { useState } from "react";
import { Text, View } from "@tarojs/components";
import Taro, { useDidShow, usePullDownRefresh } from "@tarojs/taro";

import { riderGrab, riderGrabList, type DeliveryOrderRow } from "@/api/rider/rider";
import { t } from "@/utils/i18n";

import { currentCoords, fmtTime } from "./location";

import "./hall.scss";

/**
 * 接单大厅（我方新增）：仅显示本人所属店铺、同类型的待抢派送单
 * （作用域在后端按人员绑定强制过滤）；下线状态后端不下发可抢标记并拒绝抢单。
 */
export default function RiderHall() {
    const [rows, setRows] = useState<DeliveryOrderRow[]>([]);
    const [canTake, setCanTake] = useState(true);
    const [loaded, setLoaded] = useState(false);
    const [grabbing, setGrabbing] = useState(0);

    const load = async () => {
        try {
            const res = await riderGrabList({ page: 1, size: 50 });
            setRows(res?.records || []);
            setCanTake(res?.canTakeOrders !== false);
        } catch {
            setRows([]);
        } finally {
            setLoaded(true);
            Taro.stopPullDownRefresh();
        }
    };

    useDidShow(() => {
        void load();
    });
    usePullDownRefresh(() => {
        void load();
    });

    const grab = async (row: DeliveryOrderRow) => {
        if (grabbing) return;
        setGrabbing(row.deliveryOrderId);
        try {
            const coords = await currentCoords();
            await riderGrab({ deliveryOrderId: row.deliveryOrderId, ...coords });
            Taro.showToast({ title: t("抢单成功"), icon: "success" });
            Taro.navigateTo({ url: `/pages/rider/detail?id=${row.deliveryOrderId}` });
        } catch (e: any) {
            Taro.showToast({ title: e?.message || t("抢单失败"), icon: "none" });
            void load();
        } finally {
            setGrabbing(0);
        }
    };

    return (
        <View className="rider-page">
            {!canTake && (
                <View className="rider-card" style={{ borderColor: "#f0c1c1" }}>
                    <Text style={{ color: "#e93d3d", fontSize: "27rpx" }}>{t("当前已下线，上线后才能抢单")}</Text>
                </View>
            )}
            {loaded && rows.length === 0 && <View className="rider-empty">{t("暂无待抢订单")}</View>}
            {rows.map((row) => (
                <View key={row.deliveryOrderId} className="rider-card">
                    <View className="rider-row">
                        <Text className="rider-tag">{row.type === 2 ? t("上门服务") : t("同城配送")}</Text>
                        <Text className="rider-amount">¥{Number(row.orderAmount || 0).toFixed(2)}</Text>
                    </View>
                    <View style={{ marginTop: "18rpx" }}>
                        <View className="rider-addr-label">{row.type === 2 ? t("出发（店铺）") : t("取货（店铺）")}</View>
                        <View className="rider-addr">{(row.pickupRegionNames || "") + (row.pickupAddress || "")}</View>
                    </View>
                    <View style={{ marginTop: "14rpx" }}>
                        <View className="rider-addr-label">{row.type === 2 ? t("服务地址") : t("送货地址")}</View>
                        <View className="rider-addr">{(row.regionNames || "") + (row.address || "")}</View>
                        <View className="rider-sub">
                            {row.consignee} · {row.mobile}
                            {row.serviceTime ? ` · ${t("预约")} ${fmtTime(row.serviceTime)}` : ""}
                        </View>
                        {row.buyerNote ? <View className="rider-sub">{t("备注")}：{row.buyerNote}</View> : null}
                    </View>
                    <View
                        className={canTake ? "rider-btn" : "rider-btn rider-btn--plain"}
                        style={{ marginTop: "20rpx" }}
                        onClick={() => canTake && void grab(row)}
                    >
                        {grabbing === row.deliveryOrderId ? t("抢单中…") : t("抢单")}
                    </View>
                </View>
            ))}
        </View>
    );
}
