'use client';

/**
 * 兑换卡任选选品页（原版有，PC /giftcard/select）。
 *
 * - 商品池来自我的卡详情的 giftCardProducts（详情未下发时以 giftCardSn 调
 *   promotion/giftCardGroups/detail 兜底，字段宽松读取）；
 * - 按 productId-skuId 加减数量，同款可多件、库存封顶，总件数不超过 selectCount；
 * - 头部「可选 X / 已选 Y」，已选件数 == selectCount 才可提交；
 * - 提交 POST promotion/giftCardGroups/useCard {giftCardsId, items}，成功跳结算 flowType=12。
 */
import { Suspense, useEffect, useMemo, useState } from 'react';
import { Button, Card, Empty, Toast } from '@douyinfe/semi-ui';
import { useSearchParams } from 'next/navigation';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';
import { formatDisplayPriceWithExact } from '@/lib/currency';

interface SelectProduct {
  productId: number;
  skuId: number;
  productName?: string;
  picUrl?: string;
  skuData?: string;
  skuPrice?: string | number;
  productPrice?: string | number;
  stock: number;
}

function readProducts(raw: unknown): SelectProduct[] {
  if (!Array.isArray(raw)) return [];
  return raw
    .map((row: any) => ({
      productId: Number(row?.productId || 0),
      skuId: Number(row?.skuId || 0),
      productName: row?.productName ? String(row.productName) : '',
      picUrl: row?.picUrl ? String(row.picUrl) : '',
      skuData: row?.skuData ? String(row.skuData) : '',
      skuPrice: row?.skuPrice,
      productPrice: row?.productPrice,
      // 库存封顶：与原版 Mobile 口径一致 skuStock ?? productStock，再兜底 stock 字段
      stock: Number(row?.skuStock ?? row?.productStock ?? row?.stock ?? 0),
    }))
    .filter((row) => row.productId > 0);
}

function SelectInner() {
  const { t } = useTranslation();
  const searchParams = useSearchParams();
  const giftCardSn = (searchParams.get('giftCardSn') || '').trim();
  const [giftCardsId, setGiftCardsId] = useState(Number(searchParams.get('giftCardsId') || 0));
  const [cardName, setCardName] = useState('');
  const [selectCount, setSelectCount] = useState(1);
  const [products, setProducts] = useState<SelectProduct[]>([]);
  const [qtyMap, setQtyMap] = useState<Record<string, number>>({});
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      try {
        let pool: SelectProduct[] = [];
        let count = 0;
        if (giftCardsId > 0) {
          const detail: any = await apiGet('promotion/giftCardGroups/myCardDetail', { giftCardsId }).catch(() => null);
          if (detail) {
            setCardName(String(detail.cardGroupName || ''));
            count = Number(detail.selectCount || detail.maxSelectCount || 0);
            pool = readProducts(detail.giftCardProducts);
          }
        }
        // 详情未下发商品池时，按原版 detail?giftCardSn 口径兜底取卡组绑品
        if (!pool.length && giftCardSn) {
          const bySn: any = await apiGet('promotion/giftCardGroups/detail', { giftCardSn }).catch(() => null);
          if (bySn) {
            if (!giftCardsId && Number(bySn.giftCardsId || 0) > 0) setGiftCardsId(Number(bySn.giftCardsId));
            if (!count) count = Number(bySn.selectCount || bySn.maxSelectCount || bySn.giftCardGroups?.selectCount || 0);
            if (!cardName) setCardName(String(bySn.cardGroupName || bySn.giftCardGroups?.cardGroupName || ''));
            pool = readProducts(bySn.giftCardProducts || bySn.giftCardGroups?.giftCardProducts);
          }
        }
        if (cancelled) return;
        setProducts(pool);
        setSelectCount(count > 0 ? count : 1);
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [giftCardsId, giftCardSn]);

  const selectedTotal = useMemo(() => Object.values(qtyMap).reduce((sum, n) => sum + n, 0), [qtyMap]);
  const canSubmit = giftCardsId > 0 && selectedTotal === selectCount && !submitting;

  const changeQty = (product: SelectProduct, delta: number) => {
    const key = `${product.productId}-${product.skuId}`;
    setQtyMap((prev) => {
      const current = prev[key] || 0;
      let next = current + delta;
      if (next < 0) next = 0;
      if (next > product.stock) {
        if (delta > 0) Toast.warning({ content: t('商品库存不足') });
        next = product.stock;
      }
      // 总件数不得超过可选件数（原版「最多可选择 X 件」）
      const totalOthers = Object.entries(prev)
        .filter(([k]) => k !== key)
        .reduce((sum, [, n]) => sum + n, 0);
      if (totalOthers + next > selectCount) next = Math.max(0, selectCount - totalOthers);
      return { ...prev, [key]: next };
    });
  };

  const submit = async () => {
    if (!canSubmit) return;
    setSubmitting(true);
    try {
      const items = products
        .map((p) => ({ productId: p.productId, skuId: p.skuId, quantity: qtyMap[`${p.productId}-${p.skuId}`] || 0 }))
        .filter((item) => item.quantity > 0);
      await apiPost('promotion/giftCardGroups/useCard', { giftCardsId, items });
      Toast.success({ content: t('兑换成功') });
      window.location.href = `/order/check?flowType=12${giftCardSn ? `&giftCardSn=${encodeURIComponent(giftCardSn)}` : ''}`;
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('兑换失败') });
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Card className="member-card" loading={loading}>
      <h2>{cardName || t('礼品卡选品')}</h2>
      <div style={{ margin: '8px 0 16px', color: '#667085', fontSize: 14 }}>
        {t('可选')} {selectCount} / {t('已选')} {selectedTotal}
        <span style={{ marginLeft: 8 }}>{t('以下商品任选 {n} 件', { n: selectCount })}</span>
      </div>
      {products.length === 0 && !loading ? (
        <Empty description={t('暂无可兑换商品')} />
      ) : (
        <div style={{ display: 'grid', gap: 12 }}>
          {products.map((product) => {
            const key = `${product.productId}-${product.skuId}`;
            const qty = qtyMap[key] || 0;
            return (
              <div
                key={key}
                style={{ display: 'flex', gap: 16, alignItems: 'center', border: '1px solid #eaecf0', borderRadius: 8, padding: 12 }}
              >
                {product.picUrl ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img src={product.picUrl} alt={product.productName || ''} style={{ width: 72, height: 72, objectFit: 'cover', borderRadius: 6 }} />
                ) : null}
                <div style={{ flex: 1, display: 'grid', gap: 4 }}>
                  <div style={{ fontWeight: 600 }}>{product.productName}</div>
                  {product.skuData ? <div style={{ fontSize: 13, color: '#667085' }}>{product.skuData}</div> : null}
                  <div style={{ fontSize: 13, color: '#667085' }}>
                    {formatDisplayPriceWithExact(product.skuPrice ?? product.productPrice)}
                    {' · '}
                    {product.stock > 0 ? `${t('库存')} ${product.stock}` : t('无库存')}
                  </div>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Button disabled={qty <= 0} onClick={() => changeQty(product, -1)}>-</Button>
                  <span style={{ minWidth: 24, textAlign: 'center' }}>{qty}</span>
                  <Button disabled={product.stock <= 0 || qty >= product.stock} onClick={() => changeQty(product, 1)}>+</Button>
                </div>
              </div>
            );
          })}
        </div>
      )}
      <div style={{ marginTop: 20, textAlign: 'right' }}>
        <Button theme="solid" type="primary" disabled={!canSubmit} loading={submitting} onClick={() => void submit()}>
          {t('确认兑换')}
        </Button>
      </div>
    </Card>
  );
}

export default function GiftCardSelectClient() {
  return (
    <Suspense fallback={null}>
      <SelectInner />
    </Suspense>
  );
}
