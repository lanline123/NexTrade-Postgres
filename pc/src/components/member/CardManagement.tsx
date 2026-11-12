'use client';

import { useCallback, useEffect, useState } from 'react';
import { Button, Empty, Input, Modal, Radio, RadioGroup, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost, type Paged } from '@/lib/client-api';

/**
 * 卡管理（银行卡 / 支付宝 / 微信收款账号）。
 * Vue 基准：pages/member/account/CardManagement.vue + src/InfoCard.vue。
 *
 * 真实接口（`user/withdrawApply/*`，注意不是 account/*）：
 * - GET  user/withdrawApply/list?page&accountType  → records[{ accountId, accountName, identity, accountNo, bankName, accountType }]
 * - POST user/withdrawApply/createAccount
 * - POST user/withdrawApply/updateAccount
 * - POST user/withdrawApply/delAccount { accountId }
 * accountType：1 银行卡 / 2 支付宝 / 3 微信。
 * 列表里账号做脱敏（前 2 后 4），脱敏规则照 Vue 的 maskString。
 */
interface AccountRow {
    accountId?: number;
    accountName?: string;
    identity?: string;
    accountNo?: string;
    bankName?: string;
    accountType?: number;
}

interface FormState {
    accountId?: number;
    accountType: number;
    accountName: string;
    identity: string;
    accountNo: string;
    bankName: string;
}

const EMPTY: FormState = { accountType: 1, accountName: '', identity: '', accountNo: '', bankName: '' };

function maskString(value?: string): string {
    if (!value) return '-';
    if (value.length < 7) return value;
    return `${value.slice(0, 2)}${'*'.repeat(value.length - 6)}${value.slice(-4)}`;
}

export default function CardManagement() {
    const { t } = useTranslation();
    const [accountType, setAccountType] = useState(1);
    const [rows, setRows] = useState<AccountRow[]>([]);
    const [loading, setLoading] = useState(true);
    const [form, setForm] = useState<FormState | null>(null);
    const [saving, setSaving] = useState(false);
    const [deleting, setDeleting] = useState<AccountRow | null>(null);

    const load = useCallback(async (nextType: number) => {
        setLoading(true);
        try {
            const result = await apiGet<Paged<AccountRow>>('user/withdrawApply/list', { page: 1, accountType: nextType });
            setRows(result.records ?? []);
        } catch (e) {
            Toast.error((e as Error).message);
            setRows([]);
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        void load(accountType);
    }, [load, accountType]);

    const patch = (next: Partial<FormState>) => setForm((prev) => (prev ? { ...prev, ...next } : prev));

    const save = async () => {
        if (!form) return;
        if (!form.accountName.trim()) return Toast.warning(t('请输入真实姓名'));
        if (!form.identity.trim()) return Toast.warning(t('请输入身份证号'));
        if (!form.accountNo.trim()) return Toast.warning(t('请输入账号'));

        setSaving(true);
        try {
            const payload = {
                accountType: form.accountType,
                accountName: form.accountName.trim(),
                identity: form.identity.trim(),
                accountNo: form.accountNo.trim(),
                bankName: form.accountType === 1 ? form.bankName.trim() : '',
            };
            if (form.accountId) {
                await apiPost('user/withdrawApply/updateAccount', { ...payload, accountId: form.accountId });
            } else {
                await apiPost('user/withdrawApply/createAccount', payload);
            }
            Toast.success(t('操作成功'));
            setForm(null);
            await load(accountType);
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setSaving(false);
        }
    };

    const remove = async () => {
        if (!deleting?.accountId) return;
        try {
            await apiPost('user/withdrawApply/delAccount', { accountId: deleting.accountId });
            Toast.success(t('操作成功'));
            setDeleting(null);
            await load(accountType);
        } catch (e) {
            Toast.error((e as Error).message);
        }
    };

    return (
        <div className="pc-member-panel">
            <div className="pc-panel-head">
                <Typography.Title heading={4}>{t('卡管理')}</Typography.Title>
                <Button
                    theme="solid"
                    type="primary"
                    size="small"
                    onClick={() => setForm({ ...EMPTY, accountType })}
                >
                    {t('添加新卡')}
                </Button>
            </div>

            <RadioGroup value={accountType} onChange={(e) => setAccountType(Number(e.target.value))}>
                <Radio value={1}>{t('银行卡')}</Radio>
                <Radio value={2}>{t('支付宝')}</Radio>
                <Radio value={3}>{t('微信')}</Radio>
            </RadioGroup>

            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : rows.length === 0 ? (
                <Empty title={t('暂无卡片')} />
            ) : (
                <div className="pc-table">
                    <div className="pc-table-head">
                        <span>{t('真实姓名')}</span>
                        <span>{t('身份证号码')}</span>
                        <span>{t('账户号码')}</span>
                        {accountType === 1 ? <span>{t('银行卡详情')}</span> : null}
                        <span>{t('操作')}</span>
                    </div>
                    {rows.map((row) => (
                        <div className="pc-table-row" key={row.accountId}>
                            <span>{row.accountName ?? '-'}</span>
                            <span>{row.identity ?? '-'}</span>
                            <span>{maskString(row.accountNo)}</span>
                            {accountType === 1 ? <span>{row.bankName || '--'}</span> : null}
                            <span className="pc-table-actions">
                                <button
                                    type="button"
                                    className="pc-link-btn"
                                    onClick={() =>
                                        setForm({
                                            accountId: row.accountId,
                                            accountType: row.accountType ?? accountType,
                                            accountName: row.accountName ?? '',
                                            identity: row.identity ?? '',
                                            accountNo: row.accountNo ?? '',
                                            bankName: row.bankName ?? '',
                                        })
                                    }
                                >
                                    {t('编辑')}
                                </button>
                                <i>|</i>
                                <button type="button" className="pc-link-btn" onClick={() => setDeleting(row)}>
                                    {t('删除')}
                                </button>
                            </span>
                        </div>
                    ))}
                </div>
            )}

            <Modal
                visible={!!form}
                title={form?.accountId ? t('编辑卡片') : t('新增卡片')}
                onOk={() => void save()}
                confirmLoading={saving}
                onCancel={() => setForm(null)}
            >
                {form ? (
                    <div className="pc-form">
                        <div>
                            <span className="pc-form-label">{t('卡类型')}</span>
                            <RadioGroup
                                value={form.accountType}
                                onChange={(e) => patch({ accountType: Number(e.target.value) })}
                            >
                                <Radio value={1}>{t('银行卡')}</Radio>
                                <Radio value={2}>{t('支付宝')}</Radio>
                                <Radio value={3}>{t('微信')}</Radio>
                            </RadioGroup>
                        </div>
                        <Input
                            value={form.accountName}
                            placeholder={t('请输入真实姓名')}
                            onChange={(v) => patch({ accountName: String(v) })}
                        />
                        <Input
                            value={form.identity}
                            maxLength={18}
                            placeholder={t('请输入身份证号')}
                            onChange={(v) => patch({ identity: String(v) })}
                        />
                        <Input
                            value={form.accountNo}
                            placeholder={t('请输入账号')}
                            onChange={(v) => patch({ accountNo: String(v) })}
                        />
                        {form.accountType === 1 ? (
                            <Input
                                value={form.bankName}
                                placeholder={t('请输入银行卡详情')}
                                onChange={(v) => patch({ bankName: String(v) })}
                            />
                        ) : null}
                    </div>
                ) : null}
            </Modal>

            <Modal
                visible={!!deleting}
                title={t('确认操作')}
                onOk={() => void remove()}
                onCancel={() => setDeleting(null)}
            >
                {t('确定要删除该卡片吗？')}
            </Modal>
        </div>
    );
}
