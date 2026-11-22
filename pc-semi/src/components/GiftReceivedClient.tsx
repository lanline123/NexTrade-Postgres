'use client';

/**
 * 我收到的礼物（我方新增，PC /member/gift）。
 *
 * 领取人视角：只展示礼物与履约进度，**不显示任何价格**。
 * 领取后售后由领取人（收货人）发起，退款仍退回送礼人账户。
 */
import { useEffect, useState } from 'react';
import Link from 'next/link';
import { Button, Card, Empty, Toast } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { getReceivedGifts, type GiftReceivedVO } from '@/lib/gift-api';

export default function GiftReceivedClient() {
  const { t } = useTranslation();
  const [list, setList] = useState<GiftReceivedVO[]>([]);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const data = await getReceivedGifts();
        setList(data || []);
      } catch (e) {
        Toast.error({ content: e instanceof Error ? e.message : t('加载失败') });
        setList([]);
      } finally {
        setLoaded(true);
      }
    })();
  }, [t]);

  if (loaded && list.length === 0) {
    return (
      <Card>
        <Empty title={t('还没有收到礼物')} />
      </Card>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      {list.map((gift) => (
        <Card key={gift.orderId}>
          <div style={{ fontSize: 16, fontWeight: 600 }}>
            {gift.buyerNameMasked} {t('送的礼物')}
          </div>
          {gift.giftMessage ? (
            <div style={{ marginTop: 6, color: '#666' }}>「{gift.giftMessage}」</div>
          ) : null}
          <div style={{ marginTop: 6, fontSize: 13, color: '#999' }}>
            {gift.giftStatusName}
            {gift.claimTime ? ` · ${t('领取于')} ${gift.claimTime}` : ''}
          </div>
          {gift.units.map((unit) => (
            <div key={unit.orderId} style={{ marginTop: 14 }}>
              <div style={{ color: '#666', marginBottom: 8 }}>
                {t('订单')} {unit.orderSn} · {unit.orderStatusName}
                {unit.trackingNo ? ` · ${unit.logisticsName} ${unit.trackingNo}` : ''}
              </div>
              {unit.items.map((item) => (
                <div key={item.itemId} style={{ display: 'flex', gap: 12, alignItems: 'center', marginBottom: 10 }}>
                  {/* eslint-disable-next-line @next/next/no-img-element */}
                  <img
                    src={item.picThumb}
                    alt={item.productName}
                    style={{ width: 56, height: 56, objectFit: 'cover', borderRadius: 6, background: '#f7f8fa' }}
                  />
                  <div style={{ flex: 1 }}>
                    <div>{item.productName}</div>
                    <div style={{ color: '#999', fontSize: 13, marginTop: 4 }}>×{item.quantity}</div>
                  </div>
                  {(unit.orderStatus === 1 || unit.orderStatus === 2 || unit.orderStatus === 5) && (
                    <Link href={`/member/aftersales/apply?orderId=${unit.orderId}`}>
                      <Button>{t('申请售后')}</Button>
                    </Link>
                  )}
                </div>
              ))}
            </div>
          ))}
          <div style={{ marginTop: 10, fontSize: 12, color: '#bbb' }}>
            {t('售后退款将原路退回送礼人的支付账户')}
          </div>
        </Card>
      ))}
    </div>
  );
}
