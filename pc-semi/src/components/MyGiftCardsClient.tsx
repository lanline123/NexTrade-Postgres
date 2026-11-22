'use client';

/**
 * 我的礼品卡（P1 新增）——会员中心列表页。
 *
 * 数据：`GET promotion/giftCardGroups/myCards`（归属=本人，卡号已脱敏），
 * 支持按卡类型（储值/兑换）与有效状态筛选；点击卡片进详情。
 */
import { useCallback, useEffect, useState } from 'react';
import { Button, Card, Empty, Pagination, Tabs, TabPane, Tag, Toast } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';

interface MyGiftCard {
  giftCardsId: number;
  cardGroupsId: number;
  cardGroupName: string;
  imageUrl?: string;
  groupType: number;
  cardType: number;
  cardNumber: string;
  giftCardSn?: string;
  faceAmount?: string;
  balance?: string;
  status: number;
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

export default function MyGiftCardsClient() {
  const { t } = useTranslation();
  const [type, setType] = useState<string>('0');
  const [state, setState] = useState<string>('valid');
  const [rows, setRows] = useState<MyGiftCard[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);

  const load = useCallback(
    (targetPage: number, targetType: string, targetState: string) => {
      setLoading(true);
      apiGet<{ records: MyGiftCard[]; total: number }>('promotion/giftCardGroups/myCards', {
        page: targetPage,
        size: 10,
        type: targetType === '0' ? undefined : Number(targetType),
        state: targetState,
      })
        .then((res) => {
          setRows(res.records || []);
          setTotal(res.total || 0);
        })
        .catch((e) => Toast.error({ content: e instanceof Error ? e.message : t('加载失败') }))
        .finally(() => setLoading(false));
    },
    [t],
  );

  useEffect(() => {
    load(1, type, state);
  }, [load, type, state]);

  const fmtDate = (ts?: number) => (ts ? new Date(ts * 1000).toLocaleDateString() : '-');

  return (
    <Card className="member-card" loading={loading}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <h2>{t('我的礼品卡')}</h2>
        {/* 原版口径：购卡入口 = 商品搜索过滤 onlyGiftCardProduct=1（无专属频道页） */}
        <Button
          theme="solid"
          type="primary"
          size="small"
          onClick={() => { window.location.href = '/search?onlyGiftCardProduct=1'; }}
        >
          {t('购买礼品卡')}
        </Button>
      </div>
      <Tabs
        activeKey={type}
        onChange={(key) => {
          setType(key);
          setPage(1);
        }}
      >
        <TabPane tab={t('全部')} itemKey="0" />
        <TabPane tab={t('储值卡')} itemKey="1" />
        <TabPane tab={t('兑换卡')} itemKey="2" />
      </Tabs>
      <div style={{ margin: '12px 0' }}>
        <Button
          theme={state === 'valid' ? 'solid' : 'borderless'}
          type="primary"
          size="small"
          onClick={() => { setState('valid'); setPage(1); }}
        >
          {t('有效卡')}
        </Button>
        <Button
          theme={state === 'invalid' ? 'solid' : 'borderless'}
          type="primary"
          size="small"
          onClick={() => { setState('invalid'); setPage(1); }}
        >
          {t('已失效')}
        </Button>
      </div>
      {rows.length === 0 && !loading ? (
        <Empty description={t('暂无礼品卡')} />
      ) : (
        <div style={{ display: 'grid', gap: 12 }}>
          {rows.map((card) => (
            <div
              key={card.giftCardsId}
              onClick={() => { window.location.href = `/member/giftcard/detail?giftCardsId=${card.giftCardsId}`; }}
              style={{
                display: 'flex',
                gap: 16,
                alignItems: 'center',
                border: '1px solid #eaecf0',
                borderRadius: 6,
                padding: 16,
                cursor: 'pointer',
                opacity: card.isValid ? 1 : 0.6,
              }}
            >
              {card.imageUrl ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img src={card.imageUrl} alt={card.cardGroupName} style={{ width: 120, height: 72, objectFit: 'cover', borderRadius: 6 }} />
              ) : null}
              <div style={{ flex: 1, display: 'grid', gap: 4 }}>
                <div style={{ fontWeight: 600 }}>{card.cardGroupName}</div>
                <div style={{ fontSize: 13, color: '#667085' }}>
                  {t('卡号')}：{card.cardNumber}
                </div>
                <div style={{ fontSize: 13, color: '#667085' }}>
                  {card.groupType === 1
                    ? `${t('面额')} ¥${card.faceAmount ?? '-'} · ${t('余额')} ¥${card.balance ?? '-'}`
                    : t('兑换卡')}
                  {' · '}
                  {t('来源')}：{card.sourceText}
                  {' · '}
                  {card.expiredTime ? `${t('有效期至')} ${fmtDate(card.expiredTime)}` : t('永久有效')}
                </div>
                {card.groupType === 1 && card.scopeDesc ? (
                  <div style={{ fontSize: 13, color: '#667085' }}>
                    {t('适用范围')}：{t(card.scopeDesc)}
                  </div>
                ) : null}
              </div>
              <Tag color={card.isValid ? 'green' : 'grey'}>{card.stateText}</Tag>
            </div>
          ))}
        </div>
      )}
      {total > 10 ? (
        <div style={{ marginTop: 16, textAlign: 'right' }}>
          <Pagination
            total={total}
            pageSize={10}
            currentPage={page}
            onPageChange={(p) => { setPage(p); load(p, type, state); }}
          />
        </div>
      ) : null}
    </Card>
  );
}
