import { useCallback, useEffect, useState } from "react";
import { Image, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import { getAddressList } from "@/api/user/address";
import { claimRechargeGift, getMyRechargeGifts, type RechargeGiftRecord } from "@/api/user/rechargeGift";
import { t } from "@/utils/i18n";
import { usePageGuards } from "@/utils/pageMixin";

import "./gifts.scss";

interface AddressRow {
    addressId: number;
    consignee?: string;
    mobile?: string;
    regionNames?: string | string[];
    address?: string;
    isDefault?: number;
}

function regionText(addr: AddressRow): string {
    return Array.isArray(addr.regionNames) ? addr.regionNames.join("") : addr.regionNames || "";
}

function unwrapList(result: any): any[] {
    if (Array.isArray(result)) return result;
    return result?.records || result?.list || result?.data || [];
}

/**
 * 我的赠品（我方新增，2026-11-21 论坛帖 717721）
 * 充值档位赠送的实物礼品记录：待领取 → 选收货地址领取 → 生成赠品订单由商家发货（包邮）。
 */
export default function Gifts() {
    usePageGuards();

    const [records, setRecords] = useState<RechargeGiftRecord[]>([]);
    const [loaded, setLoaded] = useState(false);
    // 领取弹层：当前待领取记录 + 地址列表
    const [claiming, setClaiming] = useState<RechargeGiftRecord | null>(null);
    const [addresses, setAddresses] = useState<AddressRow[]>([]);
    const [submitting, setSubmitting] = useState(false);

    const load = useCallback(async () => {
        try {
            const result: any = await getMyRechargeGifts({ page: 1, size: 50 });
            setRecords(unwrapList(result) as RechargeGiftRecord[]);
        } catch (error: any) {
            Taro.showToast({ title: error.message, icon: "none" });
        } finally {
            setLoaded(true);
        }
    }, []);

    useEffect(() => {
        void load();
    }, [load]);

    const openClaim = async (record: RechargeGiftRecord) => {
        try {
            const result: any = await getAddressList({});
            const list = unwrapList(result) as AddressRow[];
            if (list.length === 0) {
                Taro.showModal({
                    title: t("提示"),
                    content: t("请先添加收货地址"),
                    confirmText: t("去添加"),
                    success: (res) => {
                        if (res.confirm) {
                            Taro.navigateTo({ url: "/pages/address/list" });
                        }
                    }
                });
                return;
            }
            setAddresses(list);
            setClaiming(record);
        } catch (error: any) {
            Taro.showToast({ title: error.message, icon: "none" });
        }
    };

    const doClaim = async (address: AddressRow) => {
        if (!claiming || submitting) return;
        setSubmitting(true);
        try {
            await claimRechargeGift({ recordId: claiming.recordId, addressId: address.addressId });
            setClaiming(null);
            Taro.showToast({ title: t("领取成功，商家将尽快发货"), icon: "none", duration: 2000 });
            void load();
        } catch (error: any) {
            Taro.showToast({ title: error.message, icon: "none", duration: 2000 });
            setClaiming(null);
            void load();
        } finally {
            setSubmitting(false);
        }
    };

    return (
        <View className="gifts-page">
            {records.map((record) => (
                <View key={record.recordId} className="gift-card">
                    <View className="gift-main">
                        {record.giftPic ? (
                            <Image className="gift-pic" src={record.giftPic} mode="aspectFill" />
                        ) : (
                            <View className="gift-pic gift-pic-empty" />
                        )}
                        <View className="gift-info">
                            <View className="gift-name">
                                {record.giftName} ×{record.quantity}
                            </View>
                            <View className="gift-meta">
                                {record.status === 1 && record.consignee
                                    ? `${record.consignee} ${record.addressText || ""}`
                                    : t("充值赠品 · 包邮发货")}
                            </View>
                            {record.status === 3 && record.adminNote ? (
                                <View className="gift-note">{record.adminNote}</View>
                            ) : null}
                        </View>
                        <View className={`gift-status status-${record.status}`}>{record.statusText}</View>
                    </View>
                    {record.status === 0 ? (
                        <View className="gift-actions">
                            <View className="gift-claim-btn" onClick={() => void openClaim(record)}>
                                {t("立即领取")}
                            </View>
                        </View>
                    ) : null}
                    {record.status === 1 ? (
                        <View className="gift-actions">
                            <View
                                className="gift-order-link"
                                onClick={() => Taro.navigateTo({ url: "/pages/user/order/list" })}
                            >
                                {t("查看订单")} ›
                            </View>
                        </View>
                    ) : null}
                </View>
            ))}
            {loaded && records.length === 0 ? (
                <View className="gifts-empty">{t("暂无赠品，充值指定档位可获赠实物礼品")}</View>
            ) : null}

            {claiming ? (
                <View className="claim-mask" onClick={() => setClaiming(null)}>
                    <View className="claim-sheet" onClick={(e) => e.stopPropagation()}>
                        <View className="claim-title">{t("选择收货地址")}</View>
                        {addresses.map((addr) => (
                            <View
                                key={addr.addressId}
                                className="claim-addr"
                                onClick={() => void doClaim(addr)}
                            >
                                <View className="claim-addr-name">
                                    {addr.consignee} {addr.mobile}
                                </View>
                                <View className="claim-addr-detail">
                                    {regionText(addr)} {addr.address}
                                </View>
                            </View>
                        ))}
                        <View className="claim-cancel" onClick={() => setClaiming(null)}>
                            {t("取消")}
                        </View>
                    </View>
                </View>
            ) : null}
        </View>
    );
}
