'use client';

/**
 * 我的礼品卡详情（P1 新增）。
 *
 * 数据：`GET promotion/giftCardGroups/myCardDetail?giftCardsId=`
 * （归属校验在后端；含消费流水与兑换订单号，P1 流水为空属正常）。
 */
import { Suspense, useEffect, useState } from 'react';
import { Button, Card, Empty, Space, Table, Tag, Toast } from '@douyinfe/semi-ui';
import { useSearchParams } from 'next/navigation';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';

interface MyGiftCard {
  giftCardsId: number;
  cardGroupName: string;
  imageUrl?: string;
  groupType: number;
  cardType: number;
  cardNumber: string;
  giftCardSn?: string;
  faceAmount?: string;
  balance?: string;
  stateText: string;
  isValid: boolean;
  sourceText: string;
  expiredTime: number;
  usedTime?: number;
  addTime?: number;
  /** 适用范围（我方新增，后端下发）：0 全部商品 / 1 指定商品 / 2 指定品类 */
  scopeType?: number;
  /** 适用范围文案：全场通用 / 仅限指定商品 / 仅限指定品类 */
  scopeDesc?: string;
}

type DetailData = MyGiftCard & {
  /** 兑换模式（兑换卡）：1 固定组合 / 2 任选；后端下发后才有，宽松读取 */
  exchangeMode?: number;
  usageLogs: Array<{
    id: number;
    typeName: string;
    amount: string;
    balanceAfter?: string;
    orderId: number;
    addTime: number;
  }>;
  exchangeOrderIds: number[];
};

function DetailInner() {
  const { t } = useTranslation();
  const searchParams = useSearchParams();
  const giftCardsId = searchParams.get('giftCardsId');
  const [data, setData] = useState<DetailData | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!giftCardsId) return;
    setLoading(true);
    apiGet<DetailData>('promotion/giftCardGroups/myCardDetail', { giftCardsId: Number(giftCardsId) })
      .then(setData)
      .catch((e) => Toast.error({ content: e instanceof Error ? e.message : t('加载失败') }))
      .finally(() => setLoading(false));
  }, [giftCardsId, t]);

  const fmtTime = (ts?: number) => (ts ? new Date(ts * 1000).toLocaleString() : '-');

  /**
   * 「去使用」分流（原版口径）：
   * - 储值卡（groupType=1）→ /search 去购物、结算时抵扣卡内余额；
   * - 兑换卡任选（exchangeMode=2）→ 选品页 /giftcard/select；
   * - 兑换卡固定组合 → 先 useCard 核销，再跳结算 flowType=12（带 giftCardSn 若有）。
   */
  const goUse = async () => {
    if (!data) return;
    if (data.groupType === 1) {
      window.location.href = '/search';
      return;
    }
    if (Number(data.exchangeMode || 0) === 2) {
      const params = new URLSearchParams({ giftCardsId: String(data.giftCardsId) });
      if (data.giftCardSn) params.set('giftCardSn', data.giftCardSn);
      window.location.href = `/giftcard/select?${params.toString()}`;
      return;
    }
    try {
      await apiPost('promotion/giftCardGroups/useCard', { giftCardsId: data.giftCardsId });
      window.location.href = `/order/check?flowType=12${data.giftCardSn ? `&giftCardSn=${encodeURIComponent(data.giftCardSn)}` : ''}`;
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('兑换失败') });
    }
  };

  const card = data ?? undefined;
  const usageLogs = data?.usageLogs ?? [];
  const exchangeOrderIds = data?.exchangeOrderIds ?? [];

  return (
    <Card className="member-card" loading={loading}>
      <h2>{t('礼品卡详情')}</h2>
      {!card && !loading ? (
        <Empty description={t('礼品卡不存在')} />
      ) : card ? (
        <div style={{ display: 'grid', gap: 16 }}>
          <div style={{ display: 'flex', gap: 16, alignItems: 'center' }}>
            {card.imageUrl ? (
              // eslint-disable-next-line @next/next/no-img-element
              <img src={card.imageUrl} alt={card.cardGroupName} style={{ width: 160, height: 96, objectFit: 'cover', borderRadius: 6 }} />
            ) : null}
            <div style={{ display: 'grid', gap: 6 }}>
              <div style={{ fontSize: 16, fontWeight: 600 }}>
                {card.cardGroupName} <Tag color={card.isValid ? 'green' : 'grey'}>{card.stateText}</Tag>
              </div>
              <div>{t('卡号')}：{card.cardNumber}</div>
              {card.groupType === 1 ? (
                <div>
                  {t('面额')} ¥{card.faceAmount ?? '-'} · {t('当前余额')} ¥{card.balance ?? '-'}
                </div>
              ) : (
                <div>{t('兑换卡')}</div>
              )}
              <div style={{ color: '#667085', fontSize: 13 }}>
                {t('来源')}：{card.sourceText} · {card.expiredTime ? `${t('有效期至')} ${fmtTime(card.expiredTime)}` : t('永久有效')}
                {card.usedTime ? ` · ${t('使用时间')} ${fmtTime(card.usedTime)}` : ''}
              </div>
              {card.groupType === 1 && card.scopeDesc ? (
                <div style={{ color: '#667085', fontSize: 13 }}>
                  {t('适用范围')}：{t(card.scopeDesc)}
                </div>
              ) : null}
            </div>
          </div>
          {card.isValid ? (
            <Space>
              <Button type="primary" theme="solid" onClick={() => void goUse()}>
                {t('去使用')}
              </Button>
              {card.giftCardSn ? (
                <Button onClick={() => { window.location.href = `/giftcard/transfer/${card.giftCardSn}`; }}>
                  {t('转赠好友')}
                </Button>
              ) : null}
            </Space>
          ) : null}
          {exchangeOrderIds.length > 0 ? (
            <div style={{ fontSize: 13, color: '#667085' }}>
              {t('兑换订单')}：{exchangeOrderIds.map((id) => `#${id}`).join('、')}
            </div>
          ) : null}
          <div>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>{t('消费记录')}</div>
            {usageLogs.length ? (
              <Table
                pagination={false}
                rowKey="id"
                dataSource={usageLogs}
                columns={[
                  { title: t('类型'), dataIndex: 'typeName', width: 120 },
                  { title: t('金额'), dataIndex: 'amount', width: 120 },
                  { title: t('变动后余额'), dataIndex: 'balanceAfter', width: 140, render: (v: string) => v ?? '-' },
                  { title: t('订单号'), dataIndex: 'orderId', width: 120, render: (v: number) => (v ? `#${v}` : '-') },
                  { title: t('时间'), width: 180, render: (_: unknown, r: DetailData['usageLogs'][number]) => fmtTime(r.addTime) },
                ]}
              />
            ) : (
              <Empty description={t('暂无消费记录')} />
            )}
          </div>
        </div>
      ) : null}
    </Card>
  );
}

export default function MyGiftCardDetailClient() {
  return (
    <Suspense fallback={null}>
      <DetailInner />
    </Suspense>
  );
}
