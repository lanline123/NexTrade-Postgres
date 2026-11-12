'use client';

import { useCallback, useEffect, useState } from 'react';
import { Button, Empty, Input, Radio, RadioGroup, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';
import { formatPrice } from '@/lib/format';

/**
 * 会员余额提现。
 * Vue 基准：pages/member/account/Raplytocard.vue（531 行）
 *
 * 真实接口：
 * - GET  user/withdrawApply/list?accountType   → 已绑定账号（accountId / accountNo / accountName / bankName）
 * - GET  user/user/memberCenter               → balance（可用余额）
 * - GET  common/config/initConfigSettings     → memberWithdrawalEnabled（开关）、
 *                                               memberWithdrawalReceiptMethod（1银行卡/2支付宝/3微信）、
 *                                               memberWithdrawalDescription（提现说明 HTML）
 * - POST user/withdrawApply/apply { accountData:{accountType,accountNo,accountName,bankName}, amount }
 *
 * 校验照 Vue：金额最多两位小数且不得高于可用余额；开关关闭时提示并引导回账户明细。
 * 提现说明是后台富文本，这里只去标签取纯文本（不渲染 HTML，避免注入）。
 */
interface AccountRow {
    accountId?: number;
    accountNo?: string;
    accountName?: string;
    bankName?: string;
    accountType?: number;
}

interface Config {
    memberWithdrawalEnabled?: number;
    memberWithdrawalReceiptMethod?: number[] | null;
    memberWithdrawalDescription?: string;
}

const METHODS = [
    { type: 1, label: '银行卡' },
    { type: 2, label: '支付宝' },
    { type: 3, label: '微信' },
];

const stripHtml = (html?: string) => (html ?? '').replace(/<[^>]*>/g, '').replace(/&nbsp;/gi, ' ').trim();

export default function WithdrawPage() {
    const { t } = useTranslation();
    const [config, setConfig] = useState<Config | null>(null);
    const [balance, setBalance] = useState(0);
    const [accountType, setAccountType] = useState(1);
    const [accounts, setAccounts] = useState<Record<number, AccountRow[]>>({});
    const [accountNo, setAccountNo] = useState('');
    const [amount, setAmount] = useState('');
    const [loading, setLoading] = useState(true);
    const [submitting, setSubmitting] = useState(false);

    const methods = (() => {
        const enabled = config?.memberWithdrawalReceiptMethod;
        if (!Array.isArray(enabled)) return METHODS;
        return METHODS.filter((item) => enabled.includes(item.type));
    })();

    const loadAccounts = useCallback(async (type: number) => {
        try {
            const result = await apiGet<{ records?: AccountRow[] }>('user/withdrawApply/list', { accountType: type });
            setAccounts((prev) => ({ ...prev, [type]: result.records ?? [] }));
        } catch {
            setAccounts((prev) => ({ ...prev, [type]: [] }));
        }
    }, []);

    useEffect(() => {
        (async () => {
            try {
                const [cfg, center] = await Promise.all([
                    apiGet<Config>('common/config/initConfigSettings').catch(() => null),
                    apiGet<{ balance?: string | number }>('user/user/memberCenter').catch(() => null),
                ]);
                setConfig(cfg ?? {});
                setBalance(Number(center?.balance ?? 0) || 0);
            } finally {
                setLoading(false);
            }
        })();
    }, []);

    const enabled = config?.memberWithdrawalEnabled;
    const list = accounts[accountType] ?? [];

    useEffect(() => {
        if (enabled !== 1) return;
        const types = methods.length ? methods.map((item) => item.type) : [1];
        setAccountType(types[0]);
        types.forEach((type) => void loadAccounts(type));
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [enabled, config?.memberWithdrawalReceiptMethod]);

    const current = list.find((item) => item.accountNo === accountNo);

    const submit = async () => {
        if (!amount.trim()) return Toast.warning(t('请输入提现金额'));
        if (!/^(0|\d{0,9})(\.\d{1,2})?$/.test(amount.trim())) {
            return Toast.warning(t('提现金额必须大于0，且最多保留两位小数'));
        }
        if (Number(amount) > balance) return Toast.warning(t('提现金额不能高于可用余额'));
        if (!current) return Toast.warning(t('请选择提现账号'));

        setSubmitting(true);
        try {
            await apiPost('user/withdrawApply/apply', {
                accountData: {
                    accountType,
                    accountNo: current.accountNo ?? '',
                    accountName: current.accountName ?? '',
                    bankName: current.bankName ?? '',
                },
                amount: amount.trim(),
            });
            Toast.success(t('提交成功'));
            setAmount('');
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setSubmitting(false);
        }
    };

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;

    if (enabled !== 1) {
        return (
            <div className="pc-member-panel">
                <Empty
                    title={t('会员余额提现功能已关闭')}
                    description={<a href="/member/account/detail">{t('去账户明细')}</a>}
                />
            </div>
        );
    }

    return (
        <div className="pc-member-panel">
            <div className="pc-panel-head">
                <Typography.Title heading={4}>{t('余额提现')}</Typography.Title>
                <span className="pc-muted">
                    {t('可用余额')} {formatPrice(balance)}
                </span>
            </div>

            {methods.length === 0 ? (
                <Empty title={t('暂无可用的提现方式')} description={<a href="/member/account/cardmanagement">{t('去卡管理')}</a>} />
            ) : (
                <div className="pc-form">
                    <div>
                        <span className="pc-form-label">{t('提现方式')}</span>
                        <RadioGroup
                            value={accountType}
                            onChange={(e) => {
                                setAccountType(Number(e.target.value));
                                setAccountNo('');
                            }}
                        >
                            {methods.map((item) => (
                                <Radio key={item.type} value={item.type}>
                                    {t(item.label)}
                                </Radio>
                            ))}
                        </RadioGroup>
                    </div>

                    <div>
                        <span className="pc-form-label">{t('提现至')}</span>
                        {list.length === 0 ? (
                            <div className="pc-muted">
                                {t('还没有绑定账号')}
                                <a href="/member/account/cardmanagement">{t('去卡管理添加')}</a>
                            </div>
                        ) : (
                            <div className="pc-filter-values">
                                {list.map((item) => (
                                    <span
                                        key={item.accountId ?? item.accountNo}
                                        className={`pc-filter-chip${accountNo === item.accountNo ? ' is-active' : ''}`}
                                        onClick={() => setAccountNo(item.accountNo ?? '')}
                                    >
                                        {item.accountNo}
                                    </span>
                                ))}
                            </div>
                        )}
                    </div>

                    {current ? (
                        <>
                            <div className="pc-muted">
                                {t('提现姓名')}：{current.accountName || '-'}
                            </div>
                            {accountType === 1 && current.bankName ? (
                                <div className="pc-muted">
                                    {t('银行卡详情')}：{current.bankName}
                                </div>
                            ) : null}
                        </>
                    ) : null}

                    <div>
                        <span className="pc-form-label">{t('提现金额')}</span>
                        <Input
                            value={amount}
                            placeholder={t('提现金额')}
                            onChange={(v) => setAmount(String(v).replace(/[^\d.]/g, ''))}
                        />
                    </div>

                    {stripHtml(config?.memberWithdrawalDescription) ? (
                        <div className="pc-note">{stripHtml(config?.memberWithdrawalDescription)}</div>
                    ) : null}

                    <div className="pc-form-actions">
                        <Button
                            theme="solid"
                            type="primary"
                            loading={submitting}
                            disabled={list.length === 0}
                            onClick={() => void submit()}
                        >
                            {t('提交')}
                        </Button>
                    </div>
                </div>
            )}
        </div>
    );
}
