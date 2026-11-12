'use client';

import { useState } from 'react';
import { Button, Card, Input, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiPost } from '@/lib/client-api';

/** Vue 基准：pages/member/giftCard/exchange.vue；接口 promotion/giftCardGroups/exchange。 */
export default function GiftCardExchange() {
    const { t } = useTranslation();
    const [cardNumber, setCardNumber] = useState('');
    const [cardPassword, setCardPassword] = useState('');
    const [submitting, setSubmitting] = useState(false);

    const submit = async () => {
        if (!cardNumber.trim() || !cardPassword.trim()) {
            Toast.warning(t('请输入卡号和卡密'));
            return;
        }
        setSubmitting(true);
        try {
            const result = await apiPost<{ type?: number; productId?: number }>('promotion/giftCardGroups/exchange', {
                cardNumber: cardNumber.trim(),
                cardPassword: cardPassword.trim()
            });
            Toast.success(t('兑换成功'));
            setCardNumber('');
            setCardPassword('');
            if (result?.productId) {
                window.location.href = `/product?id=${result.productId}`;
            }
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setSubmitting(false);
        }
    };

    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('礼品卡兑换')}</Typography.Title>

            <Card>
                <div className="pc-form">
                    <label className="pc-form-label">{t('卡号')}</label>
                    <Input value={cardNumber} onChange={(v) => setCardNumber(String(v))} placeholder={t('请输入卡号')} />
                    <label className="pc-form-label">{t('卡密')}</label>
                    <Input value={cardPassword} onChange={(v) => setCardPassword(String(v))} placeholder={t('请输入卡密')} />
                    <div>
                        <Button theme="solid" type="primary" loading={submitting} onClick={() => void submit()}>
                            {t('立即兑换')}
                        </Button>
                    </div>
                </div>
            </Card>

            <div className="pc-note">
                <strong>{t('兑换规则')}</strong>
                <br />1. {t('储值卡输入卡密进行兑换将自动充值到余额')}。
                <br />2. {t('兑换卡输入卡密进行兑换后将跳转至该商品详情页进行兑换')}。
            </div>
        </div>
    );
}
