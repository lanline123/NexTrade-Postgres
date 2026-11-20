import { useState } from "react";
import { Input, Text, View } from "@tarojs/components";
import Taro, { useDidShow } from "@tarojs/taro";

import {
    riderAccounts,
    riderApplyWithdraw,
    riderSaveAccount,
    riderWallet,
    riderWalletLogs,
    riderWithdrawConfig,
    riderWithdrawList
} from "@/api/rider/rider";
import { t } from "@/utils/i18n";

import { fmtTime } from "./location";

import "./wallet.scss";

const WITHDRAW_STATUS: Record<number, string> = { 0: "待审核", 1: "已通过", 2: "已驳回", 3: "已打款" };

/**
 * 我的钱包（我方新增前端；钱包/提现接口为原版有·已移植）：
 * 余额 + 收支明细 + 收款账号 + 申请提现 + 提现记录，一页收口。
 */
export default function RiderWallet() {
    const [wallet, setWallet] = useState<any>(null);
    const [logs, setLogs] = useState<any[]>([]);
    const [accounts, setAccounts] = useState<any[]>([]);
    const [withdraws, setWithdraws] = useState<any[]>([]);
    const [cfg, setCfg] = useState<any>(null);
    const [amount, setAmount] = useState("");
    const [accountId, setAccountId] = useState(0);
    const [showAccountForm, setShowAccountForm] = useState(false);
    const [accName, setAccName] = useState("");
    const [accNo, setAccNo] = useState("");
    const [accBank, setAccBank] = useState("");
    const [busy, setBusy] = useState(false);

    const load = async () => {
        const [w, l, a, wd, c] = await Promise.all([
            riderWallet().catch(() => null),
            riderWalletLogs({ page: 1, size: 30 }).catch(() => null),
            riderAccounts().catch(() => null),
            riderWithdrawList({ page: 1, size: 20 }).catch(() => null),
            riderWithdrawConfig().catch(() => null)
        ]);
        setWallet(w);
        setLogs(l?.records || []);
        const accs = Array.isArray(a) ? a : [];
        setAccounts(accs);
        const def = accs.find((x: any) => Number(x.isDefault) === 1) || accs[0];
        if (def) setAccountId(Number(def.accountId));
        setWithdraws(wd?.records || []);
        setCfg(c);
    };

    useDidShow(() => {
        void load();
    });

    const saveAccount = async () => {
        if (!accName.trim() || !accNo.trim()) {
            Taro.showToast({ title: t("请填写收款人姓名与账号"), icon: "none" });
            return;
        }
        setBusy(true);
        try {
            await riderSaveAccount({ accountType: 3, accountName: accName.trim(), accountNo: accNo.trim(), bankName: accBank.trim() });
            Taro.showToast({ title: t("账号已保存"), icon: "success" });
            setShowAccountForm(false);
            setAccName("");
            setAccNo("");
            setAccBank("");
            await load();
        } catch (e: any) {
            Taro.showToast({ title: e?.message || t("保存失败"), icon: "none" });
        } finally {
            setBusy(false);
        }
    };

    const applyWithdraw = async () => {
        const amt = Number(amount);
        if (!amt || amt <= 0) {
            Taro.showToast({ title: t("请输入提现金额"), icon: "none" });
            return;
        }
        if (!accountId) {
            Taro.showToast({ title: t("请先添加收款账号"), icon: "none" });
            return;
        }
        setBusy(true);
        try {
            await riderApplyWithdraw({ amount: amt, accountId });
            Taro.showToast({ title: t("提现申请已提交"), icon: "success" });
            setAmount("");
            await load();
        } catch (e: any) {
            Taro.showToast({ title: e?.message || t("提现失败"), icon: "none" });
        } finally {
            setBusy(false);
        }
    };

    return (
        <View className="rider-page">
            <View className="rider-card">
                <View className="rider-addr-label">{t("钱包余额（元）")}</View>
                <View style={{ fontSize: "56rpx", fontWeight: 700 }}>¥{Number(wallet?.balance || 0).toFixed(2)}</View>
                <View className="rider-sub">
                    {t("累计收入")} ¥{Number(wallet?.totalIncome || 0).toFixed(2)}
                    {Number(wallet?.frozenBalance || 0) > 0 ? ` · ${t("冻结")} ¥${Number(wallet?.frozenBalance).toFixed(2)}` : ""}
                </View>
            </View>

            <View className="rider-card">
                <View className="rider-title" style={{ fontSize: "30rpx" }}>{t("申请提现")}</View>
                {cfg ? (
                    <View className="rider-sub" style={{ margin: "8rpx 0 16rpx" }}>
                        {t("最低")} ¥{Number(cfg.minWithdrawAmount || 0).toFixed(2)}
                        {Number(cfg.withdrawFeeRate || 0) > 0 ? ` · ${t("手续费")} ${cfg.withdrawFeeRate}%` : ""}
                    </View>
                ) : null}
                <View className="rider-field">
                    <Input className="rider-input" type="digit" placeholder={t("提现金额")} value={amount} onInput={(e) => setAmount(e.detail.value)} />
                </View>
                {accounts.length === 0 ? (
                    <View className="rider-sub">{t("暂无收款账号，请先添加")}</View>
                ) : (
                    accounts.map((a) => (
                        <View
                            key={a.accountId}
                            className="rider-row"
                            style={{ padding: "14rpx 0", borderTop: "1rpx solid #f0f1f3" }}
                            onClick={() => setAccountId(Number(a.accountId))}
                        >
                            <Text style={{ fontSize: "27rpx" }}>
                                {Number(a.accountId) === accountId ? "● " : "○ "}
                                {a.accountName} · {a.accountNo}
                                {a.bankName ? ` · ${a.bankName}` : ""}
                            </Text>
                        </View>
                    ))
                )}
                <View className="rider-row" style={{ marginTop: "18rpx", gap: "16rpx" }}>
                    <View className="rider-btn rider-btn--plain" style={{ flex: 1 }} onClick={() => setShowAccountForm(!showAccountForm)}>
                        {t("添加收款账号")}
                    </View>
                    <View className="rider-btn" style={{ flex: 1 }} onClick={() => void applyWithdraw()}>
                        {busy ? t("提交中…") : t("提交提现")}
                    </View>
                </View>
                {showAccountForm && (
                    <View style={{ marginTop: "20rpx" }}>
                        <View className="rider-field">
                            <Input className="rider-input" placeholder={t("收款人姓名")} value={accName} onInput={(e) => setAccName(e.detail.value)} />
                        </View>
                        <View className="rider-field">
                            <Input className="rider-input" placeholder={t("银行卡号")} value={accNo} onInput={(e) => setAccNo(e.detail.value)} />
                        </View>
                        <View className="rider-field">
                            <Input className="rider-input" placeholder={t("开户行（选填）")} value={accBank} onInput={(e) => setAccBank(e.detail.value)} />
                        </View>
                        <View className="rider-btn" onClick={() => void saveAccount()}>{t("保存账号")}</View>
                    </View>
                )}
            </View>

            {withdraws.length > 0 && (
                <View className="rider-card">
                    <View className="rider-title" style={{ fontSize: "30rpx" }}>{t("提现记录")}</View>
                    {withdraws.map((w) => (
                        <View key={w.withdrawId} className="rider-row" style={{ padding: "14rpx 0", borderTop: "1rpx solid #f0f1f3" }}>
                            <View>
                                <Text style={{ fontSize: "28rpx" }}>¥{Number(w.amount || 0).toFixed(2)}</Text>
                                <View className="rider-sub">{fmtTime(w.applyTime)}</View>
                            </View>
                            <Text className="rider-tag rider-tag--grey">{t(WITHDRAW_STATUS[Number(w.status)] || "")}</Text>
                        </View>
                    ))}
                </View>
            )}

            <View className="rider-card">
                <View className="rider-title" style={{ fontSize: "30rpx" }}>{t("收支明细")}</View>
                {logs.length === 0 && <View className="rider-empty">{t("暂无明细")}</View>}
                {logs.map((l) => (
                    <View key={l.logId} className="rider-row" style={{ padding: "14rpx 0", borderTop: "1rpx solid #f0f1f3" }}>
                        <View>
                            <Text style={{ fontSize: "27rpx" }}>{l.remark || (Number(l.type) === 1 ? t("服务费收入") : t("提现"))}</Text>
                            <View className="rider-sub">{l.transactionSn || ""} · {fmtTime(l.createTime)}</View>
                        </View>
                        <Text style={{ fontSize: "29rpx", color: Number(l.type) === 1 ? "#12a150" : "#e93d3d" }}>
                            {Number(l.type) === 1 ? "+" : "-"}¥{Number(l.amount || 0).toFixed(2)}
                        </Text>
                    </View>
                ))}
            </View>
        </View>
    );
}
