'use client';

import { useEffect, useState } from 'react';
import { Button, Input, Modal, Radio, RadioGroup, Toast } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiPost } from '@/lib/client-api';

/**
 * Vue 基准：components/invoice/InvoiceDialog.vue（554 行）
 *
 * 三种用法（Vue 用 params.type 区分）：
 * - check：结算页临时选票，只回传 invoiceData，不落库；
 * - apply：会员中心申请开票，落库 user/orderInvoice/create；
 * - info ：只读展示（本组件只覆盖前两种，详情页用 EntityDetail）。
 *
 * 发票类型：1 普通发票 / 2 增值税专用发票（需 initConfigSettings.invoiceAdded === 1 才可选）。
 * 抬头：1 个人 / 2 企业。普通+企业、专用都需要纳税人识别号等 6 项信息。
 */
export interface InvoiceValue {
    invoiceType: number;
    titleType: number;
    companyName: string;
    companyCode: string;
    companyAddress: string;
    companyPhone: string;
    companyBank: string;
    companyAccount: string;
    mobile: string;
    email: string;
    invoiceContent: string;
}

export const EMPTY_INVOICE: InvoiceValue = {
    invoiceType: 0,
    titleType: 1,
    companyName: '',
    companyCode: '',
    companyAddress: '',
    companyPhone: '',
    companyBank: '',
    companyAccount: '',
    mobile: '',
    email: '',
    invoiceContent: '商品明细',
};

interface Props {
    visible: boolean;
    value: InvoiceValue;
    /** 是否允许开增值税专用发票（initConfigSettings.invoiceAdded === 1） */
    invoiceAdded?: boolean;
    mode?: 'check' | 'apply';
    orderId?: number;
    orderSn?: string;
    totalAmount?: number;
    amount?: number;
    onClose: () => void;
    onConfirm: (value: InvoiceValue) => void;
}

export default function InvoiceDialog({
    visible,
    value,
    invoiceAdded = false,
    mode = 'check',
    orderId,
    orderSn,
    totalAmount,
    amount,
    onClose,
    onConfirm,
}: Props) {
    const { t } = useTranslation();
    const [form, setForm] = useState<InvoiceValue>(value ?? EMPTY_INVOICE);
    const [saving, setSaving] = useState(false);

    useEffect(() => {
        if (visible) setForm(value ?? EMPTY_INVOICE);
    }, [visible, value]);

    const patch = (next: Partial<InvoiceValue>) => setForm((prev) => ({ ...prev, ...next }));

    const needCompanyFields = form.invoiceType === 2 || (form.invoiceType === 1 && form.titleType === 2);

    const submit = async () => {
        if (form.invoiceType > 0 && !form.companyName.trim()) {
            Toast.warning(form.titleType === 1 ? t('个人名称不能为空') : t('企业名称不能为空'));
            return;
        }
        if (needCompanyFields && !form.companyCode.trim()) {
            Toast.warning(t('纳税人识号不能为空'));
            return;
        }
        if (form.email && !/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/.test(form.email)) {
            Toast.warning(t('请输入正确的邮箱格式'));
            return;
        }

        if (mode === 'check') {
            onConfirm(form);
            onClose();
            return;
        }

        setSaving(true);
        try {
            await apiPost('user/orderInvoice/create', {
                id: orderId,
                orderSn,
                totalAmount,
                amount,
                invoiceType: form.invoiceType,
                titleType: form.titleType,
                companyName: form.companyName,
                companyCode: needCompanyFields ? form.companyCode : '',
                companyAddress: needCompanyFields ? form.companyAddress : '',
                companyPhone: needCompanyFields ? form.companyPhone : '',
                companyBank: needCompanyFields ? form.companyBank : '',
                companyAccount: needCompanyFields ? form.companyAccount : '',
                mobile: form.mobile,
                email: form.email,
                invoiceContent: form.invoiceContent,
            });
            Toast.success(t('操作成功'));
            onConfirm(form);
            onClose();
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setSaving(false);
        }
    };

    return (
        <Modal
            visible={visible}
            title={t('发票申请')}
            width={720}
            onCancel={onClose}
            footer={
                <div className="pc-form-actions" style={{ justifyContent: 'flex-end' }}>
                    <Button theme="borderless" onClick={onClose}>
                        {t('取消')}
                    </Button>
                    <Button theme="solid" type="primary" loading={saving} onClick={() => void submit()}>
                        {t('确定')}
                    </Button>
                </div>
            }
        >
            <div className="pc-invoice">
                <div className="pc-invoice-tabs">
                    {[
                        { type: 0, label: t('不开发票') },
                        { type: 1, label: t('普通发票') },
                        ...(invoiceAdded ? [{ type: 2, label: t('增值税专用发票') }] : []),
                    ].map((item) => (
                        <span
                            key={item.type}
                            className={`pc-invoice-tab${form.invoiceType === item.type ? ' is-active' : ''}`}
                            onClick={() => patch({ invoiceType: item.type })}
                        >
                            {item.label}
                        </span>
                    ))}
                </div>

                {form.invoiceType === 0 ? (
                    <div className="pc-muted">{t('当前订单不开发票')}</div>
                ) : (
                    <>
                        {form.invoiceType === 1 ? (
                            <div className="pc-invoice-row">
                                <span className="pc-invoice-label">{t('发票抬头')}</span>
                                <RadioGroup
                                    value={form.titleType}
                                    onChange={(e) => patch({ titleType: Number(e.target.value) })}
                                >
                                    <Radio value={1}>{t('个人')}</Radio>
                                    <Radio value={2}>{t('企业')}</Radio>
                                </RadioGroup>
                            </div>
                        ) : (
                            <div className="pc-invoice-note">
                                {t('所有发票由商品供货商开具')}；{t('您还未通过增票资质申请，暂时无法开具增值税专用发票')}
                            </div>
                        )}

                        <div className="pc-invoice-row">
                            <span className="pc-invoice-label">
                                {needCompanyFields ? t('企业名称') : t('个人名称')}
                            </span>
                            <Input
                                value={form.companyName}
                                onChange={(v) => patch({ companyName: String(v) })}
                                placeholder={needCompanyFields ? t('请输入企业名称') : t('请输入个人名称')}
                            />
                        </div>

                        {needCompanyFields ? (
                            <>
                                <div className="pc-invoice-row">
                                    <span className="pc-invoice-label">{t('纳税人识号')}</span>
                                    <Input
                                        value={form.companyCode}
                                        onChange={(v) => patch({ companyCode: String(v) })}
                                        placeholder={t('请输入纳税人识号')}
                                    />
                                </div>
                                <div className="pc-invoice-row">
                                    <span className="pc-invoice-label">{t('单位地址')}</span>
                                    <Input
                                        value={form.companyAddress}
                                        onChange={(v) => patch({ companyAddress: String(v) })}
                                        placeholder={t('请输入单位地址')}
                                    />
                                </div>
                                <div className="pc-invoice-row">
                                    <span className="pc-invoice-label">{t('单位电话')}</span>
                                    <Input
                                        value={form.companyPhone}
                                        onChange={(v) => patch({ companyPhone: String(v) })}
                                        placeholder={t('请输入单位电话')}
                                    />
                                </div>
                                <div className="pc-invoice-row">
                                    <span className="pc-invoice-label">{t('开户银行')}</span>
                                    <Input
                                        value={form.companyBank}
                                        onChange={(v) => patch({ companyBank: String(v) })}
                                        placeholder={t('请输入开户银行')}
                                    />
                                </div>
                                <div className="pc-invoice-row">
                                    <span className="pc-invoice-label">{t('银行账户')}</span>
                                    <Input
                                        value={form.companyAccount}
                                        onChange={(v) => patch({ companyAccount: String(v) })}
                                        placeholder={t('请输入银行账户')}
                                    />
                                </div>
                            </>
                        ) : null}

                        <div className="pc-invoice-row">
                            <span className="pc-invoice-label">{t('收票人手机')}</span>
                            <Input
                                value={form.mobile}
                                onChange={(v) => patch({ mobile: String(v) })}
                                placeholder={t('请输入收票人手机')}
                            />
                        </div>
                        <div className="pc-invoice-row">
                            <span className="pc-invoice-label">{t('收票人邮箱')}</span>
                            <Input
                                value={form.email}
                                onChange={(v) => patch({ email: String(v) })}
                                placeholder={t('请输入收票人邮箱')}
                            />
                        </div>

                        <div className="pc-invoice-note">
                            {t('发票内容将显示详细商品名称与价格信息')}；{t('使用礼品卡支付部分的金额，不支持开发票')}
                        </div>
                    </>
                )}
            </div>
        </Modal>
    );
}
