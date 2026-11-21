import { useState } from "react";
import { View, Text, Image } from "@tarojs/components";
import Taro, { useDidShow, useRouter } from "@tarojs/taro";
import TigLayout from "@/components/tig/tig-layout/TigLayout";
import { giftClaim, giftClaimPreview, type GiftClaimPreviewVO } from "@/api/order/gift";
import { getAddressList } from "@/api/user/address";
import { t } from "@/utils/i18n";

/**
 * 礼物领取页（我方新增，论坛帖 197742）。
 *
 * 买家付款后分享链接，好友打开本页：可匿名看礼物概要（商品/赠言，
 * **不显示任何价格**），领取必须登录并选择自己的收货地址。
 * 领取成功后订单进入正常履约链，商家发货到领取人地址。
 */

interface AddressRecord {
    addressId: number;
    consignee: string;
    mobile: string;
    regionNames?: string | string[];
    address: string;
    isDefault?: number;
}

const regionText = (raw?: string | string[]): string => {
    if (Array.isArray(raw)) return raw.join(" ");
    if (typeof raw === "string") {
        try {
            const parsed = JSON.parse(raw);
            if (Array.isArray(parsed)) return parsed.join(" ");
        } catch {
            return raw;
        }
        return raw;
    }
    return "";
};

export default function GiftClaimPage() {
    const router = useRouter();
    const token = String(router.params.token || "");
    const [preview, setPreview] = useState<GiftClaimPreviewVO | null>(null);
    const [loadError, setLoadError] = useState("");
    const [addresses, setAddresses] = useState<AddressRecord[]>([]);
    const [selectedAddressId, setSelectedAddressId] = useState(0);
    const [claiming, setClaiming] = useState(false);
    const loggedIn = !!Taro.getStorageSync("token");

    const load = async () => {
        if (!token) {
            setLoadError(t("领取链接无效"));
            return;
        }
        try {
            const data = await giftClaimPreview(token);
            setPreview(data);
        } catch (e: any) {
            setLoadError(e?.message || t("领取链接无效或已失效"));
            return;
        }
        if (loggedIn) {
            try {
                const list: any = await getAddressList({});
                const records: AddressRecord[] = list?.records || list || [];
                setAddresses(records);
                const def = records.find((r) => r.isDefault === 1) || records[0];
                if (def) setSelectedAddressId(def.addressId);
            } catch {
                // 地址加载失败不阻断预览，领取时再提示
            }
        }
    };

    useDidShow(() => {
        void load();
    });

    const doClaim = async () => {
        if (!selectedAddressId) {
            Taro.showToast({ title: t("请选择收货地址"), icon: "none" });
            return;
        }
        setClaiming(true);
        try {
            await giftClaim(token, selectedAddressId);
            Taro.showToast({ title: t("领取成功"), icon: "success" });
            await load();
        } catch (e: any) {
            Taro.showToast({ title: e?.message || t("领取失败"), icon: "none" });
            await load();
        } finally {
            setClaiming(false);
        }
    };

    const goLogin = () => {
        Taro.navigateTo({ url: "/pages/login/index" });
    };

    const goAddAddress = () => {
        Taro.navigateTo({ url: "/pages/address/list" });
    };

    const goReceived = () => {
        Taro.redirectTo({ url: "/pages/user/giftReceived" });
    };

    const cardStyle = {
        background: "#fff",
        borderRadius: "18rpx",
        margin: "20rpx 24rpx 0",
        padding: "28rpx"
    };

    return (
        <TigLayout title={t("领取礼物")}>
            {loadError && (
                <View style={cardStyle}>
                    <Text style={{ fontSize: "30rpx", color: "#333" }}>{loadError}</Text>
                </View>
            )}
            {preview && (
                <>
                    <View style={cardStyle}>
                        <Text style={{ fontSize: "32rpx", fontWeight: 600, color: "#1a1a1a" }}>
                            {preview.buyerNameMasked} {t("送你一份礼物")}
                        </Text>
                        {preview.giftMessage ? (
                            <Text style={{ display: "block", marginTop: "12rpx", fontSize: "28rpx", color: "#666" }}>
                                「{preview.giftMessage}」
                            </Text>
                        ) : null}
                        <Text style={{ display: "block", marginTop: "12rpx", fontSize: "24rpx", color: "#999" }}>
                            {t("状态")}：{preview.giftStatusName}
                            {preview.giftStatus === 1 && preview.expireTime
                                ? ` · ${t("请在")} ${preview.expireTime} ${t("前领取")}`
                                : ""}
                        </Text>
                    </View>

                    <View style={cardStyle}>
                        {preview.items.map((item) => (
                            <View
                                key={item.itemId}
                                style={{ display: "flex", flexDirection: "row", marginBottom: "16rpx" }}
                            >
                                <Image
                                    src={item.picThumb}
                                    style={{ width: "120rpx", height: "120rpx", borderRadius: "12rpx", background: "#f7f8fa" }}
                                />
                                <View style={{ flex: 1, marginLeft: "20rpx" }}>
                                    <Text style={{ fontSize: "28rpx", color: "#1a1a1a" }}>{item.productName}</Text>
                                    <Text style={{ display: "block", fontSize: "24rpx", color: "#999", marginTop: "8rpx" }}>
                                        ×{item.quantity}
                                    </Text>
                                </View>
                            </View>
                        ))}
                    </View>

                    {preview.giftStatus === 0 && (
                        <View style={cardStyle}>
                            <Text style={{ fontSize: "28rpx", color: "#666" }}>{t("好友还没完成付款，付款后即可领取")}</Text>
                        </View>
                    )}

                    {preview.giftStatus === 1 && !loggedIn && (
                        <View style={cardStyle}>
                            <Text style={{ fontSize: "28rpx", color: "#666" }}>{t("领取需要先登录，并填写你的收货地址")}</Text>
                            <View
                                style={{
                                    marginTop: "20rpx",
                                    background: "#65c4aa",
                                    color: "#fff",
                                    textAlign: "center",
                                    padding: "20rpx 0",
                                    borderRadius: "12rpx",
                                    fontSize: "30rpx"
                                }}
                                onClick={goLogin}
                            >
                                {t("去登录")}
                            </View>
                        </View>
                    )}

                    {preview.giftStatus === 1 && loggedIn && (
                        <>
                            <View style={cardStyle}>
                                <Text style={{ fontSize: "30rpx", fontWeight: 600 }}>{t("选择收货地址")}</Text>
                                {addresses.map((addr) => (
                                    <View
                                        key={addr.addressId}
                                        style={{
                                            marginTop: "16rpx",
                                            padding: "20rpx",
                                            borderRadius: "12rpx",
                                            border: selectedAddressId === addr.addressId ? "2rpx solid #65c4aa" : "2rpx solid #eee",
                                            background: selectedAddressId === addr.addressId ? "#f2faf7" : "#fff"
                                        }}
                                        onClick={() => setSelectedAddressId(addr.addressId)}
                                    >
                                        <Text style={{ fontSize: "28rpx", color: "#1a1a1a" }}>
                                            {addr.consignee} {addr.mobile}
                                        </Text>
                                        <Text style={{ display: "block", fontSize: "24rpx", color: "#999", marginTop: "6rpx" }}>
                                            {regionText(addr.regionNames)} {addr.address}
                                        </Text>
                                    </View>
                                ))}
                                <View style={{ marginTop: "16rpx" }} onClick={goAddAddress}>
                                    <Text style={{ fontSize: "26rpx", color: "#65c4aa" }}>{t("＋ 管理收货地址")}</Text>
                                </View>
                            </View>
                            <View
                                style={{
                                    margin: "28rpx 24rpx 40rpx",
                                    background: claiming ? "#bfe3d7" : "#65c4aa",
                                    color: "#fff",
                                    textAlign: "center",
                                    padding: "24rpx 0",
                                    borderRadius: "12rpx",
                                    fontSize: "32rpx"
                                }}
                                onClick={() => !claiming && void doClaim()}
                            >
                                {claiming ? t("领取中…") : t("确认领取")}
                            </View>
                        </>
                    )}

                    {preview.giftStatus === 2 && (
                        <View style={cardStyle}>
                            <Text style={{ fontSize: "30rpx", fontWeight: 600, color: "#1a1a1a" }}>{t("已领取")}</Text>
                            {preview.units.map((unit) => (
                                <View key={unit.orderId} style={{ marginTop: "14rpx" }}>
                                    <Text style={{ fontSize: "26rpx", color: "#666" }}>
                                        {t("订单")} {unit.orderSn} · {unit.orderStatusName}
                                        {unit.trackingNo ? ` · ${unit.logisticsName} ${unit.trackingNo}` : ""}
                                    </Text>
                                </View>
                            ))}
                            <View
                                style={{
                                    marginTop: "20rpx",
                                    background: "#65c4aa",
                                    color: "#fff",
                                    textAlign: "center",
                                    padding: "20rpx 0",
                                    borderRadius: "12rpx",
                                    fontSize: "30rpx"
                                }}
                                onClick={goReceived}
                            >
                                {t("查看我收到的礼物")}
                            </View>
                        </View>
                    )}

                    {(preview.giftStatus === 3 || preview.giftStatus === 4) && (
                        <View style={cardStyle}>
                            <Text style={{ fontSize: "28rpx", color: "#666" }}>
                                {preview.giftStatus === 3
                                    ? t("这份礼物已关闭，款项已原路退回送礼人")
                                    : t("这份礼物已被送礼人取消")}
                            </Text>
                        </View>
                    )}
                </>
            )}
        </TigLayout>
    );
}
