'use client';

/**
 * 礼品卡兑换（原版口径，D4：储值卡不入会员余额、只有卡内余额结算抵扣）。
 *
 * - 输入卡号、卡密调 exchange 把卡绑进我的卡包；
 * - 储值卡：提示「已存入我的礼品卡，下单结算时可抵扣卡内余额」→ 我的礼品卡；
 * - 兑换卡任选（exchangeMode=2）：跳选品页 /giftcard/select；
 * - 兑换卡固定组合：先调 useCard 核销，再跳结算 /order/check?flowType=12（带 giftCardSn 若有）。
 * 兑换响应字段宽松读取（groupType/exchangeMode/giftCardsId/giftCardSn 可能在顶层或卡对象内）。
 */
import { useState } from 'react';
import { Button, Card, Toast } from '@douyinfe/semi-ui';
import { apiPost } from '@/lib/client-api';

import { useTranslation } from 'react-i18next';
export default function GiftCardExchangeClient() {
    const { t } = useTranslation();
  const [cardNumber, setCardNumber] = useState('');
  const [cardPassword, setCardPassword] = useState('');
  const [loading, setLoading] = useState(false);

  const submit = async () => {
    if (!cardNumber.trim() || !cardPassword.trim()) {
      Toast.error({ content: '请输入卡号和卡密' });
      return;
    }
    setLoading(true);
    try {
      const data = await apiPost<Record<string, any>>('promotion/giftCardGroups/exchange', {
        cardNumber: cardNumber.trim(),
        cardPassword: cardPassword.trim(),
      });
      const card: Record<string, any> = (data?.card ?? data ?? {}) as Record<string, any>;
      const groupType = Number(card.groupType ?? data?.groupType ?? 0);
      const exchangeMode = Number(card.exchangeMode ?? data?.exchangeMode ?? 0);
      const giftCardsId = Number(card.giftCardsId ?? data?.giftCardsId ?? data?.exchangeProductToCart?.giftCardsId ?? 0);
      const giftCardSn = String(card.giftCardSn ?? data?.giftCardSn ?? '');

      if (groupType === 2) {
        // 兑换卡：任选跳选品页；固定组合先 useCard 核销再进结算（flowType=12）
        if (exchangeMode === 2) {
          const params = new URLSearchParams();
          if (giftCardsId) params.set('giftCardsId', String(giftCardsId));
          if (giftCardSn) params.set('giftCardSn', giftCardSn);
          window.location.href = `/giftcard/select${params.toString() ? `?${params.toString()}` : ''}`;
          return;
        }
        if (giftCardsId) {
          await apiPost('promotion/giftCardGroups/useCard', { giftCardsId });
        }
        Toast.success({ content: '兑换成功' });
        window.location.href = `/order/check?flowType=12${giftCardSn ? `&giftCardSn=${encodeURIComponent(giftCardSn)}` : ''}`;
        return;
      }

      if (groupType === 1) {
        // 储值卡：只入卡包，结算时抵扣卡内余额（不入会员余额）
        Toast.success({ content: '兑换成功，已存入我的礼品卡，下单结算时可抵扣卡内余额' });
        window.location.href = '/member/giftcard/list';
        return;
      }

      // 响应未带类型字段时的兼容：旧 exchangeProductToCart.flowType 直达结算，否则按储值卡入卡包口径
      const flowType = data?.exchangeProductToCart?.flowType;
      if (flowType) {
        Toast.success({ content: '兑换成功' });
        window.location.href = `/order/check?flowType=${flowType}${giftCardSn ? `&giftCardSn=${encodeURIComponent(giftCardSn)}` : ''}`;
      } else {
        Toast.success({ content: '兑换成功，已存入我的礼品卡，下单结算时可抵扣卡内余额' });
        window.location.href = '/member/giftcard/list';
      }
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : '兑换失败' });
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card className="member-card giftcard-exchange" loading={loading}>
      <h2>{t('礼品卡兑换')}</h2>
      <div className="security-form">
        <label>{t('卡号')}</label>
        <input value={cardNumber} onChange={(event) => setCardNumber(event.target.value)} placeholder={t('请输入卡号')} />
        <label>{t('卡密')}</label>
        <input value={cardPassword} onChange={(event) => setCardPassword(event.target.value)} placeholder={t('请输入卡密')} type="password" />
        <Button theme="solid" type="primary" loading={loading} onClick={() => void submit()}>{t('立即兑换')}</Button>
      </div>
      <div className="giftcard-exchange__rules" style={{ marginTop: 20, paddingTop: 16, borderTop: '1px solid #f2f4f7', color: '#667085', fontSize: 13, lineHeight: 2 }}>
        <div style={{ fontWeight: 600, color: '#344054', marginBottom: 6 }}>{t('兑换说明')}</div>
        <p style={{ margin: 0 }}>{t('1. 储值卡：输入卡密完成兑换，卡片自动存入「我的礼品卡」，下单结算时可直接抵扣卡内余额。')}</p>
        <p style={{ margin: 0 }}>{t('2. 兑换卡：输入卡密完成兑换，卡片自动存入卡包；选择心仪商品下单，即可自动核销使用。')}</p>
      </div>
    </Card>
  );
}
