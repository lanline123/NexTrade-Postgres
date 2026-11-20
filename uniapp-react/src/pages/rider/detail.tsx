import { useState } from "react";
import { Image, Input, Text, View } from "@tarojs/components";
import Taro, { useDidShow, useRouter } from "@tarojs/taro";

import {
    riderCancel,
    riderComplete,
    riderConfirmPickup,
    riderDetail,
    riderHomeConfig,
    riderLocalConfig,
    type DeliveryOrderRow
} from "@/api/rider/rider";
import { uploadImage } from "@/utils/upload";
import { t } from "@/utils/i18n";

import { callPhone, currentCoords, DELIVERY_STATUS_TEXT, fmtTime, navigateToPlace } from "./location";

import "./detail.scss";

/**
 * 派送详情（我方新增）：地址/电话/导航唤起 + 取货/送达动作。
 * 拍照凭证按店铺配置执行：配置要求拍照时未传照片不允许提交（后端同口径校验）；
 * 每次状态动作附带当前定位（动作点打点，失败静默降级）。
 */
export default function RiderDetail() {
    const router = useRouter();
    const id = Number(router.params.id || 0);
    const [row, setRow] = useState<DeliveryOrderRow | null>(null);
    const [photos, setPhotos] = useState<string[]>([]);
    const [photoRequired, setPhotoRequired] = useState(false);
    const [cancelReason, setCancelReason] = useState("");
    const [showCancel, setShowCancel] = useState(false);
    const [busy, setBusy] = useState(false);

    const load = async () => {
        try {
            const d = await riderDetail(id);
            setRow(d);
            setPhotos([]);
            if (d) {
                const cfg = d.type === 2 ? await riderHomeConfig(d.shopId).catch(() => null) : await riderLocalConfig(d.shopId).catch(() => null);
                if (cfg) {
                    if (d.status === 20) {
                        setPhotoRequired(Number(d.type === 2 ? cfg.serviceStartPhotoEnabled : cfg.pickupPhotoEnabled) === 1);
                    } else if (d.status === 30) {
                        setPhotoRequired(Number(d.type === 2 ? cfg.serviceCompletePhotoEnabled : cfg.deliveryPhotoEnabled) === 1);
                    } else {
                        setPhotoRequired(false);
                    }
                }
            }
        } catch {
            setRow(null);
        }
    };

    useDidShow(() => {
        void load();
    });

    const addPhoto = async () => {
        try {
            const picked = await Taro.chooseImage({ count: 3 - photos.length, sizeType: ["compressed"] });
            const paths = picked.tempFilePaths || [];
            const urls: string[] = [];
            for (const p of paths) {
                urls.push(await uploadImage(p));
            }
            setPhotos([...photos, ...urls]);
        } catch {
            // 用户取消选图：静默
        }
    };

    const act = async (kind: "pickup" | "complete" | "cancel") => {
        if (!row || busy) return;
        if (kind !== "cancel" && photoRequired && photos.length === 0) {
            Taro.showToast({ title: t("请先拍照上传凭证"), icon: "none" });
            return;
        }
        if (kind === "cancel" && !cancelReason.trim()) {
            Taro.showToast({ title: t("请填写取消原因"), icon: "none" });
            return;
        }
        setBusy(true);
        try {
            const coords = await currentCoords();
            if (kind === "pickup") {
                await riderConfirmPickup({ deliveryOrderId: row.deliveryOrderId, photo: photos, ...coords });
                Taro.showToast({ title: t("已确认"), icon: "success" });
            } else if (kind === "complete") {
                const res = await riderComplete({ deliveryOrderId: row.deliveryOrderId, photo: photos, ...coords });
                const fee = Number((res as any)?.serviceFee || 0);
                Taro.showToast({ title: fee > 0 ? `${t("已完成，服务费")} ¥${fee.toFixed(2)}` : t("已完成"), icon: "success" });
            } else {
                await riderCancel({ deliveryOrderId: row.deliveryOrderId, cancelReason: cancelReason.trim() });
                Taro.showToast({ title: t("已取消"), icon: "success" });
            }
            setShowCancel(false);
            await load();
        } catch (e: any) {
            Taro.showToast({ title: e?.message || t("操作失败"), icon: "none" });
        } finally {
            setBusy(false);
        }
    };

    if (!row) {
        return (
            <View className="rider-page">
                <View className="rider-empty">{t("加载中…")}</View>
            </View>
        );
    }

    const isService = row.type === 2;
    return (
        <View className="rider-page">
            <View className="rider-card">
                <View className="rider-row">
                    <Text style={{ fontSize: "26rpx", color: "#5c6167" }}>{row.deliveryOrderSn}</Text>
                    <Text className={row.status === 40 ? "rider-tag rider-tag--green" : row.status === 50 ? "rider-tag rider-tag--grey" : "rider-tag"}>
                        {t(DELIVERY_STATUS_TEXT[row.status] || "")}
                    </Text>
                </View>
                <View className="rider-row" style={{ marginTop: "14rpx" }}>
                    <Text className="rider-sub">{t("订单")} {row.orderSn}</Text>
                    <Text className="rider-amount">¥{Number(row.orderAmount || 0).toFixed(2)}</Text>
                </View>
                {row.serviceTime ? <View className="rider-sub" style={{ marginTop: "6rpx" }}>{t("预约时间")}：{fmtTime(row.serviceTime)}</View> : null}
                {row.buyerNote ? <View className="rider-sub" style={{ marginTop: "6rpx" }}>{t("买家备注")}：{row.buyerNote}</View> : null}
            </View>

            <View className="rider-card">
                <View className="rider-addr-label">{isService ? t("出发（店铺）") : t("取货（店铺）")}</View>
                <View className="rider-addr">{(row.pickupRegionNames || "") + (row.pickupAddress || "")}</View>
                <View className="rider-row" style={{ marginTop: "16rpx" }}>
                    <View className="rider-btn rider-btn--plain" onClick={() => callPhone(row.pickupPhone)}>{t("联系店铺")}</View>
                    <View className="rider-btn rider-btn--plain" onClick={() => navigateToPlace(t("店铺"), (row.pickupRegionNames || "") + (row.pickupAddress || ""), null, null)}>
                        {t("复制地址")}
                    </View>
                </View>
            </View>

            <View className="rider-card">
                <View className="rider-addr-label">{isService ? t("服务地址") : t("送货地址")}</View>
                <View className="rider-addr">{(row.regionNames || "") + (row.address || "")}</View>
                <View className="rider-sub" style={{ marginTop: "6rpx" }}>{row.consignee} · {row.mobile}</View>
                <View className="rider-row" style={{ marginTop: "16rpx" }}>
                    <View className="rider-btn rider-btn--plain" onClick={() => callPhone(row.mobile)}>{t("联系买家")}</View>
                    <View
                        className="rider-btn rider-btn--plain"
                        onClick={() => navigateToPlace(row.consignee, (row.regionNames || "") + (row.address || ""), row.longitude, row.latitude)}
                    >
                        {t("导航")}
                    </View>
                </View>
            </View>

            {(row.status === 20 || row.status === 30) && (
                <View className="rider-card">
                    <View className="rider-field-label">
                        {row.status === 20 ? (isService ? t("开始服务凭证") : t("取货凭证")) : isService ? t("服务完成凭证") : t("送达凭证")}
                        {photoRequired ? `（${t("必传")}）` : `（${t("选传")}）`}
                    </View>
                    <View style={{ display: "flex", gap: "16rpx", flexWrap: "wrap" }}>
                        {photos.map((p) => (
                            <Image key={p} src={p} style={{ width: "150rpx", height: "150rpx" }} mode="aspectFill" />
                        ))}
                        {photos.length < 3 && (
                            <View
                                style={{ width: "150rpx", height: "150rpx", border: "1rpx dashed #c3c7cd", textAlign: "center", lineHeight: "150rpx", color: "#8a8f98", fontSize: "44rpx" }}
                                onClick={() => void addPhoto()}
                            >
                                +
                            </View>
                        )}
                    </View>
                    <View className="rider-btn" style={{ marginTop: "24rpx" }} onClick={() => void act(row.status === 20 ? "pickup" : "complete")}>
                        {busy ? t("提交中…") : row.status === 20 ? (isService ? t("确认开始服务") : t("确认取货")) : isService ? t("确认服务完成") : t("确认送达")}
                    </View>
                    {!showCancel ? (
                        <View className="rider-btn rider-btn--danger" style={{ marginTop: "16rpx" }} onClick={() => setShowCancel(true)}>
                            {t("取消派送")}
                        </View>
                    ) : (
                        <View style={{ marginTop: "20rpx" }}>
                            <Input className="rider-input" placeholder={t("取消原因")} value={cancelReason} onInput={(e) => setCancelReason(e.detail.value)} />
                            <View className="rider-btn rider-btn--danger" style={{ marginTop: "14rpx" }} onClick={() => void act("cancel")}>
                                {t("确认取消")}
                            </View>
                        </View>
                    )}
                </View>
            )}

            {row.status === 40 && (
                <View className="rider-card">
                    <View className="rider-sub">
                        {t("完成时间")}：{fmtTime(row.completeTime)}
                        {Number(row.serviceFee || 0) > 0 ? ` · ${t("服务费")} ¥${Number(row.serviceFee).toFixed(2)} ${t("已入账钱包")}` : ""}
                    </View>
                </View>
            )}
            {row.status === 50 && (
                <View className="rider-card">
                    <View className="rider-sub">{t("取消原因")}：{row.cancelReason || "-"}</View>
                </View>
            )}
        </View>
    );
}
