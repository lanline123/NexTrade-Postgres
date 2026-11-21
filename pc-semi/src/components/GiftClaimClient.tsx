'use client';

/**
 * 礼物领取页（我方新增，PC /gift/claim?token=）。
 *
 * 好友打开买家分享的链接：可匿名看礼物概要（商品/赠言，**不显示
 * 任何价格**），领取必须登录并选择自己的收货地址；领取成功后
 * 订单进入正常履约链。与 H5 领取页同一后端接口、同一口径。
 */
import { Suspense, useCallback, useEffect, useState } from 'react';
import { Button, Card, Empty, Radio, RadioGroup, Toast } from '@douyinfe/semi-ui';
import { useSearchParams } from 'next/navigation';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import { giftClaim, giftClaimPreview, type GiftClaimPreviewVO } from '@/lib/gift-api';

interface Address {
  addressId: number;
  consignee?: string;
  mobile?: string;
  regionName?: string;
  fullAddress?: string;
  formattedAddress?: string;
  address?: string;
  isSelected?: boolean;
}

function ClaimInner() {
  const { t } = useTranslation();
  const searchParams = useSearchParams();
  const token = (searchParams.get('token') || '').trim();
  const [preview, setPreview] = useState<GiftClaimPreviewVO | null>(null);
  const [error, setError] = useState('');
  const [addresses, setAddresses] = useState<Address[]>([]);
  const [addressId, setAddressId] = useState(0);
  const [claiming, setClaiming] = useState(false);
  const loggedIn = typeof window !== 'undefined' && !!localStorage.getItem('accessToken');

  const load = useCallback(async () => {
    if (!token) {
      setError(t('领取链接无效'));
      return;
    }
    try {
      const data = await giftClaimPreview(token);
      setPreview(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : t('领取链接无效或已失效'));
      return;
    }
    if (loggedIn) {
      const result = await apiGet<{ records?: Address[] }>('user/address/list', { page: 1, size: 99 }).catch(() => ({ records: [] }));
      const list = result.records || [];
      setAddresses(list);
      const preferred = list.find((a) => a.isSelected) || list[0];
      if (preferred) setAddressId(preferred.addressId);
    }
  }, [token, loggedIn, t]);

  useEffect(() => {
    void load();
  }, [load]);

  const doClaim = async () => {
    if (!addressId) {
      Toast.warning({ content: t('请选择收货地址') });
      return;
    }
    setClaiming(true);
    try {
      await giftClaim(token, addressId);
      Toast.success({ content: t('领取成功') });
      await load();
    } catch (e) {
      Toast.error({ content: e instanceof Error ? e.message : t('领取失败') });
      await load();
    } finally {
      setClaiming(false);
    }
  };

  if (error) {
    return (
      <Card>
        <Empty title={error} />
      </Card>
    );
  }
  if (!preview) {
    return (
      <Card>
        <Empty title={t('加载中...')} />
      </Card>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      <Card>
        <div style={{ fontSize: 18, fontWeight: 600 }}>
          {preview.buyerNameMasked} {t('送你一份礼物')}
        </div>
        {preview.giftMessage ? (
          <div style={{ marginTop: 8, color: '#666' }}>「{preview.giftMessage}」</div>
        ) : null}
        <div style={{ marginTop: 8, fontSize: 13, color: '#999' }}>
          {t('状态')}：{preview.giftStatusName}
          {preview.giftStatus === 1 && preview.expireTime
            ? ` · ${t('请在')} ${preview.expireTime} ${t('前领取')}`
            : ''}
        </div>
      </Card>

      <Card title={t('礼物清单')}>
        {preview.items.map((item) => (
          <div key={item.itemId} style={{ display: 'flex', gap: 12, marginBottom: 12 }}>
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={item.picThumb}
              alt={item.productName}
              style={{ width: 64, height: 64, objectFit: 'cover', borderRadius: 8, background: '#f7f8fa' }}
            />
            <div>
              <div>{item.productName}</div>
              <div style={{ color: '#999', fontSize: 13, marginTop: 4 }}>×{item.quantity}</div>
            </div>
          </div>
        ))}
      </Card>

      {preview.giftStatus === 0 && (
        <Card>{t('好友还没完成付款，付款后即可领取')}</Card>
      )}

      {preview.giftStatus === 1 && !loggedIn && (
        <Card>
          <div style={{ marginBottom: 12 }}>{t('领取需要先登录，并填写你的收货地址')}</div>
          <Button theme="solid" onClick={() => { window.location.href = '/login'; }}>
            {t('去登录')}
          </Button>
        </Card>
      )}

      {preview.giftStatus === 1 && loggedIn && (
        <Card title={t('选择收货地址')}>
          <RadioGroup
            value={addressId}
            onChange={(e) => setAddressId(Number(e.target.value))}
            style={{ display: 'flex', flexDirection: 'column', gap: 8 }}
          >
            {addresses.map((addr) => (
              <Radio key={addr.addressId} value={addr.addressId}>
                {addr.consignee} {addr.mobile} · {addr.formattedAddress || addr.fullAddress || addr.address}
              </Radio>
            ))}
          </RadioGroup>
          {addresses.length === 0 && (
            <div style={{ color: '#999', marginBottom: 8 }}>{t('还没有收货地址，先去添加一个')}</div>
          )}
          <div style={{ marginTop: 12, display: 'flex', gap: 12 }}>
            <Button theme="solid" loading={claiming} onClick={doClaim}>
              {t('确认领取')}
            </Button>
            <Button onClick={() => { window.location.href = '/member/address/list'; }}>
              {t('管理收货地址')}
            </Button>
          </div>
        </Card>
      )}

      {preview.giftStatus === 2 && (
        <Card title={t('已领取')}>
          {preview.units.map((unit) => (
            <div key={unit.orderId} style={{ marginBottom: 6, color: '#666' }}>
              {t('订单')} {unit.orderSn} · {unit.orderStatusName}
              {unit.trackingNo ? ` · ${unit.logisticsName} ${unit.trackingNo}` : ''}
            </div>
          ))}
          <Button theme="solid" style={{ marginTop: 8 }} onClick={() => { window.location.href = '/member/gift'; }}>
            {t('查看我收到的礼物')}
          </Button>
        </Card>
      )}

      {(preview.giftStatus === 3 || preview.giftStatus === 4) && (
        <Card>
          {preview.giftStatus === 3
            ? t('这份礼物已关闭，款项已原路退回送礼人')
            : t('这份礼物已被送礼人取消')}
        </Card>
      )}
    </div>
  );
}

export default function GiftClaimClient() {
  return (
    <Suspense fallback={null}>
      <ClaimInner />
    </Suspense>
  );
}
