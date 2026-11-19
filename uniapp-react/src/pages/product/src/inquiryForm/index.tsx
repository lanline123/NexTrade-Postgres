import { useEffect, useState } from "react";
import { Input, Text, Textarea, View } from "@tarojs/components";
import { Popup } from "@nutui/nutui-react-taro";
import Taro from "@tarojs/taro";

import { submitPriceInquiry } from "@/api/product/product";
import { useUserStore } from "@/store/user";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    /** 对应 v-model 显隐 */
    visible?: boolean;
    onClose?: () => void;
    /** 目标商品 */
    productId: number;
    productName?: string;
    /** 目标型号（可空：未选规格时只带商品级） */
    skuId?: number;
    skuValue?: string;
}

const MOBILE_RE = /^1[3-9]\d{9}$/;

/**
 * 询价留资表单（询价线第 3 期，我方新增；原版 H5 无询价实现）。
 *
 * 与 PC 端 InquiryFormModal 同一后端契约（POST product/product/priceInquiry）：
 * 手机号 + 需求必填，公司 / 职务可选；匿名可提交（后端限频 + 300 秒去重），
 * 登录时自动带入账号手机号。提交成功后关闭并提示。
 */
export default function InquiryForm({ visible = false, onClose, productId, productName, skuId, skuValue }: Props) {
    const userInfo = useUserStore((s) => s.userInfo);
    const [mobile, setMobile] = useState("");
    const [companyName, setCompanyName] = useState("");
    const [jobTitle, setJobTitle] = useState("");
    const [content, setContent] = useState("");
    const [submitting, setSubmitting] = useState(false);

    // 打开时带入登录账号手机号（可改）
    useEffect(() => {
        if (visible && !mobile) {
            const known = String((userInfo as any)?.mobile ?? "");
            if (known) setMobile(known);
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [visible]);

    const submit = async () => {
        if (submitting) return;
        if (!MOBILE_RE.test(mobile.trim())) {
            Taro.showToast({ title: t("请输入正确的手机号"), icon: "none" });
            return;
        }
        if (!content.trim()) {
            Taro.showToast({ title: t("请填写您的需求"), icon: "none" });
            return;
        }
        setSubmitting(true);
        try {
            const payload: Parameters<typeof submitPriceInquiry>[0] = {
                productId,
                mobile: mobile.trim(),
                content: content.trim()
            };
            if (companyName.trim()) payload.companyName = companyName.trim();
            if (jobTitle.trim()) payload.jobTitle = jobTitle.trim();
            if (skuId) payload.skuId = skuId;
            if (skuValue) payload.skuValue = skuValue;
            await submitPriceInquiry(payload);
            Taro.showToast({ title: t("提交成功，我们会尽快与您联系"), icon: "success" });
            setContent("");
            onClose?.();
        } catch (error: any) {
            Taro.showToast({ title: error?.message || t("提交失败，请稍后重试"), icon: "none" });
        } finally {
            setSubmitting(false);
        }
    };

    return (
        <Popup visible={visible} position="bottom" round closeable onClose={() => onClose?.()}>
            <View className="inquiry-form">
                <View className="inquiry-form__title">{t("立即询价")}</View>
                {!!productName && <View className="inquiry-form__product line1">{productName}</View>}
                {!!skuValue && (
                    <View className="inquiry-form__sku">
                        {t("型号")}：{skuValue}
                    </View>
                )}
                <View className="inquiry-form__field">
                    <Text className="inquiry-form__label">{t("手机号")} *</Text>
                    <Input
                        className="inquiry-form__input"
                        type="number"
                        maxlength={11}
                        placeholder={t("请输入手机号")}
                        value={mobile}
                        onInput={(e) => setMobile(e.detail.value)}
                    />
                </View>
                <View className="inquiry-form__field">
                    <Text className="inquiry-form__label">{t("公司名称")}</Text>
                    <Input
                        className="inquiry-form__input"
                        placeholder={t("选填")}
                        value={companyName}
                        onInput={(e) => setCompanyName(e.detail.value)}
                    />
                </View>
                <View className="inquiry-form__field">
                    <Text className="inquiry-form__label">{t("职务")}</Text>
                    <Input
                        className="inquiry-form__input"
                        placeholder={t("选填")}
                        value={jobTitle}
                        onInput={(e) => setJobTitle(e.detail.value)}
                    />
                </View>
                <View className="inquiry-form__field">
                    <Text className="inquiry-form__label">{t("需求描述")} *</Text>
                    <Textarea
                        className="inquiry-form__textarea"
                        placeholder={t("请填写您的需求，如数量、交付周期等")}
                        maxlength={500}
                        value={content}
                        onInput={(e) => setContent(e.detail.value)}
                    />
                </View>
                <View className={`inquiry-form__submit${submitting ? " inquiry-form__submit--disabled" : ""}`} onClick={submit}>
                    {submitting ? t("提交中…") : t("提交询价")}
                </View>
            </View>
        </Popup>
    );
}
