import { useState } from "react";
import { Input, Text, View } from "@tarojs/components";
import Taro, { useDidShow } from "@tarojs/taro";

import TigButton2 from "@/components/tig/tig-button2/TigButton2";
import TigLayout from "@/components/tig/tig-layout/TigLayout";
import {
    createInvoiceTitle,
    deleteInvoiceTitle,
    getInvoiceTitleList,
    setDefaultInvoiceTitle,
    updateInvoiceTitle
} from "@/api/user/invoiceManagemen";
import { t } from "@/utils/i18n";
import { usePageGuards } from "@/utils/pageMixin";

import "./titleManagement.scss";

interface TitleItem {
    id: number;
    titleType: number;
    titleName: string;
    taxNo: string;
    companyAddress: string;
    companyPhone: string;
    bankName: string;
    bankAccount: string;
    isDefault: number;
}

interface TitleForm {
    id?: number;
    titleType: number;
    titleName: string;
    taxNo: string;
    companyAddress: string;
    companyPhone: string;
    bankName: string;
    bankAccount: string;
    isDefault: boolean;
}

const EMPTY_FORM: TitleForm = {
    titleType: 1,
    titleName: "",
    taxNo: "",
    companyAddress: "",
    companyPhone: "",
    bankName: "",
    bankAccount: "",
    isDefault: false
};

const toastError = (error: any, fallback: string) => {
    const message = error?.message || error?.msg || fallback;
    Taro.showToast({ title: String(message), icon: "none" });
};

/**
 * 常用发票抬头管理（v5.8.65 跟进）
 * - 列表 + 新建/编辑 + 删除 + 设默认（同用户最多 10 条）
 * - 企业抬头税号必填，格式校验在后端（统一社会信用代码 18 位 / 15 位老税号）
 */
export default function TitleManagement() {
    usePageGuards();

    const [titles, setTitles] = useState<TitleItem[]>([]);
    const [form, setForm] = useState<TitleForm | null>(null);
    const [saving, setSaving] = useState(false);

    const load = async () => {
        try {
            const result: any = await getInvoiceTitleList();
            const list = Array.isArray(result) ? result : result?.item ?? [];
            setTitles(Array.isArray(list) ? list : []);
        } catch (error) {
            console.error(error);
        }
    };

    useDidShow(() => {
        void load();
    });

    const save = async () => {
        if (!form) return;
        if (!form.titleName.trim()) {
            Taro.showToast({ title: t("抬头名称不能为空"), icon: "none" });
            return;
        }
        if (form.titleType === 2 && !form.taxNo.trim()) {
            Taro.showToast({ title: t("企业抬头必须填写纳税人识别号"), icon: "none" });
            return;
        }
        setSaving(true);
        try {
            const payload = {
                titleType: form.titleType,
                titleName: form.titleName.trim(),
                taxNo: form.taxNo.trim(),
                companyAddress: form.companyAddress.trim(),
                companyPhone: form.companyPhone.trim(),
                bankName: form.bankName.trim(),
                bankAccount: form.bankAccount.trim(),
                isDefault: form.isDefault ? 1 : 0
            };
            if (form.id) {
                await updateInvoiceTitle({ id: form.id, ...payload });
            } else {
                await createInvoiceTitle(payload);
            }
            Taro.showToast({ title: t("保存成功"), icon: "success" });
            setForm(null);
            void load();
        } catch (error) {
            toastError(error, t("保存失败"));
        } finally {
            setSaving(false);
        }
    };

    const handleDelete = (item: TitleItem) => {
        Taro.showModal({
            title: t("删除该抬头？"),
            content: item.isDefault ? t("删除默认抬头后，系统会自动将最近使用的抬头设为默认") : "",
            success: async (res) => {
                if (!res.confirm) return;
                try {
                    await deleteInvoiceTitle(item.id);
                    void load();
                } catch (error) {
                    toastError(error, t("删除失败"));
                }
            }
        });
    };

    const handleSetDefault = async (item: TitleItem) => {
        try {
            await setDefaultInvoiceTitle(item.id);
            void load();
        } catch (error) {
            toastError(error, t("操作失败"));
        }
    };

    const inputProps = (key: keyof TitleForm) => ({
        value: String(form?.[key] ?? ""),
        placeholderStyle: "color:#c0c4cc;font-size:26rpx",
        onInput: (e: any) => setForm((cur) => (cur ? { ...cur, [key]: e.detail.value } : cur))
    });

    return (
        <TigLayout title={t("发票抬头管理")}>
            <View className="title-management">
                {titles.length === 0 && !form ? <View className="title-empty">{t("暂无常用抬头")}</View> : null}
                {titles.map((item) => (
                    <View key={item.id} className="title-card">
                        <View className="title-card-main">
                            <Text className="title-name">{item.titleName}</Text>
                            <Text className="title-type">{item.titleType === 2 ? t("企业") : t("个人")}</Text>
                            {item.isDefault ? <Text className="title-default">{t("默认")}</Text> : null}
                        </View>
                        {item.taxNo ? <View className="title-sub">{item.taxNo}</View> : null}
                        <View className="title-actions">
                            {!item.isDefault ? (
                                <Text className="title-action" onClick={() => void handleSetDefault(item)}>
                                    {t("设为默认")}
                                </Text>
                            ) : null}
                            <Text
                                className="title-action"
                                onClick={() =>
                                    setForm({
                                        id: item.id,
                                        titleType: item.titleType,
                                        titleName: item.titleName,
                                        taxNo: item.taxNo,
                                        companyAddress: item.companyAddress,
                                        companyPhone: item.companyPhone,
                                        bankName: item.bankName,
                                        bankAccount: item.bankAccount,
                                        isDefault: Boolean(item.isDefault)
                                    })
                                }
                            >
                                {t("编辑")}
                            </Text>
                            <Text className="title-action danger" onClick={() => handleDelete(item)}>
                                {t("删除")}
                            </Text>
                        </View>
                    </View>
                ))}

                {form ? (
                    <View className="title-form">
                        <View className="title-form-row">
                            <Text className="title-form-label">{t("抬头类型")}</Text>
                            <View className="title-type-switch">
                                <Text className={`title-type-item${form.titleType === 1 ? " active" : ""}`} onClick={() => setForm({ ...form, titleType: 1 })}>
                                    {t("个人")}
                                </Text>
                                <Text className={`title-type-item${form.titleType === 2 ? " active" : ""}`} onClick={() => setForm({ ...form, titleType: 2 })}>
                                    {t("企业")}
                                </Text>
                            </View>
                        </View>
                        <View className="title-form-row">
                            <Text className="title-form-label">{form.titleType === 2 ? t("企业名称") : t("个人名称")}</Text>
                            <Input className="title-form-input" placeholder={t("请输入抬头名称")} {...inputProps("titleName")} />
                        </View>
                        {form.titleType === 2 ? (
                            <>
                                <View className="title-form-row">
                                    <Text className="title-form-label">{t("纳税人识别号")}</Text>
                                    <Input className="title-form-input" placeholder={t("请输入纳税人识别号")} {...inputProps("taxNo")} />
                                </View>
                                <View className="title-form-row">
                                    <Text className="title-form-label">{t("单位地址")}</Text>
                                    <Input className="title-form-input" placeholder={t("请输入单位地址")} {...inputProps("companyAddress")} />
                                </View>
                                <View className="title-form-row">
                                    <Text className="title-form-label">{t("单位电话")}</Text>
                                    <Input className="title-form-input" placeholder={t("请输入单位电话")} {...inputProps("companyPhone")} />
                                </View>
                                <View className="title-form-row">
                                    <Text className="title-form-label">{t("开户银行")}</Text>
                                    <Input className="title-form-input" placeholder={t("请输入开户银行")} {...inputProps("bankName")} />
                                </View>
                                <View className="title-form-row">
                                    <Text className="title-form-label">{t("银行账户")}</Text>
                                    <Input className="title-form-input" placeholder={t("请输入银行账户")} {...inputProps("bankAccount")} />
                                </View>
                            </>
                        ) : null}
                        <View className="title-form-row">
                            <Text className="title-form-label">{t("设为默认抬头")}</Text>
                            <Text className={`title-type-item${form.isDefault ? " active" : ""}`} onClick={() => setForm({ ...form, isDefault: !form.isDefault })}>
                                {form.isDefault ? t("是") : t("否")}
                            </Text>
                        </View>
                        <View className="title-form-btns">
                            <TigButton2 className="btn" onClick={() => setForm(null)}>
                                {t("取消")}
                            </TigButton2>
                            <TigButton2 className="btn" theme disabled={saving} onClick={() => void save()}>
                                {t("保存")}
                            </TigButton2>
                        </View>
                    </View>
                ) : (
                    <View className="title-add">
                        <TigButton2 className="btn" theme onClick={() => setForm({ ...EMPTY_FORM })}>
                            {t("新建抬头")}
                        </TigButton2>
                    </View>
                )}
            </View>
        </TigLayout>
    );
}
