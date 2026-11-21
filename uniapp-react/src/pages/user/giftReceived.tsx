import { useState } from "react";
import { View, Text, Image } from "@tarojs/components";
import Taro, { useDidShow } from "@tarojs/taro";
import TigLayout from "@/components/tig/tig-layout/TigLayout";
import { getReceivedGifts, type GiftReceivedVO } from "@/api/order/gift";
import { t } from "@/utils/i18n";

/**
 * 我收到的礼物（我方新增，论坛帖 197742）。
 *
 * 领取人视角：只展示礼物与履约进度，**不显示任何价格**。
 * 领取后售后由领取人（收货人）发起，退款仍退回送礼人账户。
 */
export default function GiftReceivedPage() {
    const [list, setList] = useState<GiftReceivedVO[]>([]);
    const [loaded, setLoaded] = useState(false);

    useDidShow(() => {
        (async () => {
            try {
                const data = await getReceivedGifts();
                setList(data || []);
            } catch {
                setList([]);
            } finally {
                setLoaded(true);
            }
        })();
    });

    const cardStyle = {
        background: "#fff",
        borderRadius: "18rpx",
        margin: "20rpx 24rpx 0",
        padding: "28rpx"
    };

    const goAftersale = (orderId: number, itemId: number) => {
        Taro.navigateTo({
            url: `/pages/user/afterSale/edit?itemId=${itemId}&orderId=${orderId}`
        });
    };

    return (
        <TigLayout title={t("我收到的礼物")}>
            {loaded && list.length === 0 && (
                <View style={cardStyle}>
                    <Text style={{ fontSize: "28rpx", color: "#999" }}>{t("还没有收到礼物")}</Text>
                </View>
            )}
            {list.map((gift) => (
                <View key={gift.orderId} style={cardStyle}>
                    <Text style={{ fontSize: "30rpx", fontWeight: 600, color: "#1a1a1a" }}>
                        {gift.buyerNameMasked} {t("送的礼物")}
                    </Text>
                    {gift.giftMessage ? (
                        <Text style={{ display: "block", marginTop: "10rpx", fontSize: "26rpx", color: "#666" }}>
                            「{gift.giftMessage}」
                        </Text>
                    ) : null}
                    <Text style={{ display: "block", marginTop: "8rpx", fontSize: "24rpx", color: "#999" }}>
                        {gift.giftStatusName}
                        {gift.claimTime ? ` · ${t("领取于")} ${gift.claimTime}` : ""}
                    </Text>
                    {gift.units.map((unit) => (
                        <View key={unit.orderId} style={{ marginTop: "18rpx" }}>
                            <Text style={{ fontSize: "26rpx", color: "#666" }}>
                                {t("订单")} {unit.orderSn} · {unit.orderStatusName}
                                {unit.trackingNo ? ` · ${unit.logisticsName} ${unit.trackingNo}` : ""}
                            </Text>
                            {unit.items.map((item) => (
                                <View
                                    key={item.itemId}
                                    style={{ display: "flex", flexDirection: "row", marginTop: "12rpx", alignItems: "center" }}
                                >
                                    <Image
                                        src={item.picThumb}
                                        style={{ width: "100rpx", height: "100rpx", borderRadius: "10rpx", background: "#f7f8fa" }}
                                    />
                                    <View style={{ flex: 1, marginLeft: "18rpx" }}>
                                        <Text style={{ fontSize: "27rpx", color: "#1a1a1a" }}>{item.productName}</Text>
                                        <Text style={{ display: "block", fontSize: "24rpx", color: "#999", marginTop: "6rpx" }}>
                                            ×{item.quantity}
                                        </Text>
                                    </View>
                                    {(unit.orderStatus === 1 || unit.orderStatus === 2 || unit.orderStatus === 5) && (
                                        <View
                                            style={{
                                                border: "2rpx solid #65c4aa",
                                                color: "#65c4aa",
                                                borderRadius: "10rpx",
                                                padding: "8rpx 18rpx",
                                                fontSize: "24rpx"
                                            }}
                                            onClick={() => goAftersale(unit.orderId, item.itemId)}
                                        >
                                            {t("申请售后")}
                                        </View>
                                    )}
                                </View>
                            ))}
                        </View>
                    ))}
                    <Text style={{ display: "block", marginTop: "16rpx", fontSize: "22rpx", color: "#bbb" }}>
                        {t("售后退款将原路退回送礼人的支付账户")}
                    </Text>
                </View>
            ))}
        </TigLayout>
    );
}
