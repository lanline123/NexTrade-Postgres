import { useState } from "react";
import { Input, Text, View } from "@tarojs/components";
import Taro, { useDidShow } from "@tarojs/taro";

import { riderApply } from "@/api/rider/rider";
import { getShopList } from "@/api/shop/shop";
import { t } from "@/utils/i18n";

import "./apply.scss";

/**
 * 入驻申请（我方新增）：会员选店铺 + 选身份（配送员/服务人员）提交申请，
 * 平台/商家在 admin「入驻审核」通过后生成人员档案即可上线接单。
 */
export default function RiderApply() {
    const [shops, setShops] = useState<{ shopId: number; shopTitle: string }[]>([]);
    const [shopId, setShopId] = useState(0);
    const [type, setType] = useState(1);
    const [name, setName] = useState("");
    const [phone, setPhone] = useState("");
    const [idCard, setIdCard] = useState("");
    const [remark, setRemark] = useState("");
    const [busy, setBusy] = useState(false);

    useDidShow(() => {
        getShopList({ page: 1, size: 100 })
            .then((res: any) => setShops(res?.records || []))
            .catch(() => setShops([]));
    });

    const submit = async () => {
        if (!shopId) {
            Taro.showToast({ title: t("请选择入驻店铺"), icon: "none" });
            return;
        }
        if (!name.trim() || !phone.trim()) {
            Taro.showToast({ title: t("请填写姓名与手机号"), icon: "none" });
            return;
        }
        setBusy(true);
        try {
            await riderApply({ shopId, type, name: name.trim(), phone: phone.trim(), idCard: idCard.trim() || undefined, remark: remark.trim() || undefined });
            Taro.showToast({ title: t("申请已提交，等待审核"), icon: "success" });
            setTimeout(() => Taro.navigateBack(), 800);
        } catch (e: any) {
            Taro.showToast({ title: e?.message || t("提交失败"), icon: "none" });
        } finally {
            setBusy(false);
        }
    };

    return (
        <View className="rider-page">
            <View className="rider-card">
                <View className="rider-field">
                    <View className="rider-field-label">{t("申请身份")}</View>
                    <View className="rider-row" style={{ gap: "16rpx" }}>
                        <View className={type === 1 ? "rider-btn" : "rider-btn rider-btn--plain"} style={{ flex: 1 }} onClick={() => setType(1)}>
                            {t("配送员")}
                        </View>
                        <View className={type === 2 ? "rider-btn" : "rider-btn rider-btn--plain"} style={{ flex: 1 }} onClick={() => setType(2)}>
                            {t("服务人员")}
                        </View>
                    </View>
                </View>
                <View className="rider-field">
                    <View className="rider-field-label">{t("姓名")}</View>
                    <Input className="rider-input" placeholder={t("真实姓名")} value={name} onInput={(e) => setName(e.detail.value)} />
                </View>
                <View className="rider-field">
                    <View className="rider-field-label">{t("手机号")}</View>
                    <Input className="rider-input" type="number" placeholder={t("手机号")} value={phone} onInput={(e) => setPhone(e.detail.value)} />
                </View>
                <View className="rider-field">
                    <View className="rider-field-label">{t("身份证号（选填）")}</View>
                    <Input className="rider-input" placeholder={t("身份证号")} value={idCard} onInput={(e) => setIdCard(e.detail.value)} />
                </View>
                <View className="rider-field">
                    <View className="rider-field-label">{t("备注（选填）")}</View>
                    <Input className="rider-input" placeholder={t("备注")} value={remark} onInput={(e) => setRemark(e.detail.value)} />
                </View>
            </View>

            <View className="rider-card">
                <View className="rider-field-label">{t("选择入驻店铺")}</View>
                {shops.length === 0 && <View className="rider-empty">{t("暂无可选店铺")}</View>}
                {shops.map((s) => (
                    <View key={s.shopId} className="rider-row" style={{ padding: "16rpx 0", borderTop: "1rpx solid #f0f1f3" }} onClick={() => setShopId(s.shopId)}>
                        <Text style={{ fontSize: "28rpx" }}>
                            {shopId === s.shopId ? "● " : "○ "}
                            {s.shopTitle}
                        </Text>
                        <Text className="rider-sub">#{s.shopId}</Text>
                    </View>
                ))}
            </View>

            <View className="rider-btn" onClick={() => void submit()}>
                {busy ? t("提交中…") : t("提交申请")}
            </View>
        </View>
    );
}
