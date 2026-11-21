'use client';

import { useCallback, useEffect, useState } from 'react';
import Link from 'next/link';
import { Button, Card, Modal, Toast } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '@/lib/client-api';

import { useTranslation } from 'react-i18next';

/** 充值赠品记录（我方新增，2026-11-21 论坛帖 717721）。 */
interface GiftRecord {
  recordId: number;
  giftName?: string;
  giftPic?: string;
  quantity: number;
  /** 0 待领取 / 1 已领取 / 2 已作废 / 3 异常 */
  status: number;
  statusText?: string;
  orderId: number;
  consignee?: string;
  addressText?: string;
  adminNote?: string;
}

interface Address {
  addressId: number;
  consignee?: string;
  mobile?: string;
  regionNames?: string[];
  address?: string;
  isDefault?: number;
}

const STATUS_COLOR: Record<number, string> = {
  0: 'var(--general)',
  1: '#07c160',
  2: '#999',
  3: '#e64340',
};

export default function RechargeGiftClient() {
  const { t } = useTranslation();
  const [rows, setRows] = useState<GiftRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [needLogin, setNeedLogin] = useState(false);
  const [claiming, setClaiming] = useState<GiftRecord | null>(null);
  const [addresses, setAddresses] = useState<Address[]>([]);
  const [submitting, setSubmitting] = useState(false);

  const load = useCallback(() => {
    setLoading(true);
    apiGet<{ records?: GiftRecord[] }>('user/rechargeGift/list', { page: 1, size: 50 })
      .then((res) => setRows(res.records || []))
      .catch((error) => {
        const message = error instanceof Error ? error.message : '';
        setNeedLogin(message.includes('登录') || message.includes('401'));
      })
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const openClaim = async (record: GiftRecord) => {
    try {
      const res = await apiGet<{ records?: Address[] }>('user/address/list', { page: 1, size: 50 });
      const list = res.records || [];
      if (list.length === 0) {
        Modal.confirm({
          title: t('提示'),
          content: t('请先添加收货地址'),
          okText: t('去添加'),
          onOk: () => {
            window.location.href = '/member/address/list';
          },
        });
        return;
      }
      setAddresses(list);
      setClaiming(record);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载地址失败') });
    }
  };

  const doClaim = async (address: Address) => {
    if (!claiming || submitting) return;
    setSubmitting(true);
    try {
      await apiPost('user/rechargeGift/claim', {
        recordId: claiming.recordId,
        addressId: address.addressId,
      });
      Toast.success({ content: t('领取成功，商家将尽快发货') });
      setClaiming(null);
      load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('领取失败') });
      setClaiming(null);
      load();
    } finally {
      setSubmitting(false);
    }
  };

  if (needLogin) {
    return (
      <Card className="cart-state">
        <h2>{t('登录后查看我的赠品')}</h2>
        <Link href="/login?redirect=/member/rechargegift">
          <Button theme="solid" type="primary" size="large">{t('去登录')}</Button>
        </Link>
      </Card>
    );
  }

  return (
    <div className="member-recharge-gift">
      <Card className="member-card" title={t('我的赠品')}>
        {loading ? (
          <p>{t('加载中...')}</p>
        ) : rows.length ? (
          <div>
            {rows.map((row) => (
              <div
                key={row.recordId}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 16,
                  padding: '14px 0',
                  borderBottom: '1px solid #f0f0f0',
                }}
              >
                {row.giftPic ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img src={row.giftPic} alt="" style={{ width: 64, height: 64, objectFit: 'cover', borderRadius: 6 }} />
                ) : (
                  <div style={{ width: 64, height: 64, background: '#f0f0f0', borderRadius: 6 }} />
                )}
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 15, color: '#333' }}>
                    {row.giftName} ×{row.quantity}
                  </div>
                  <div style={{ fontSize: 12, color: '#999', marginTop: 4 }}>
                    {row.status === 1 && row.consignee
                      ? `${row.consignee} ${row.addressText || ''}`
                      : t('充值赠品 · 包邮发货')}
                  </div>
                  {row.status === 3 && row.adminNote ? (
                    <div style={{ fontSize: 12, color: '#e64340', marginTop: 4 }}>{row.adminNote}</div>
                  ) : null}
                </div>
                <span style={{ color: STATUS_COLOR[row.status] || '#333', fontSize: 14 }}>
                  {row.statusText}
                </span>
                {row.status === 0 ? (
                  <Button theme="solid" type="primary" onClick={() => void openClaim(row)}>
                    {t('立即领取')}
                  </Button>
                ) : null}
                {row.status === 1 ? (
                  <Link href="/member/order/list">
                    <Button theme="borderless">{t('查看订单')}</Button>
                  </Link>
                ) : null}
              </div>
            ))}
          </div>
        ) : (
          <p style={{ color: '#999', padding: '24px 0', textAlign: 'center' }}>
            {t('暂无赠品，充值指定档位可获赠实物礼品')}
          </p>
        )}
      </Card>

      <Modal
        title={t('选择收货地址')}
        visible={!!claiming}
        footer={null}
        onCancel={() => setClaiming(null)}
      >
        <div>
          {addresses.map((addr) => (
            <div
              key={addr.addressId}
              onClick={() => void doClaim(addr)}
              style={{
                padding: '12px 8px',
                borderBottom: '1px solid #f0f0f0',
                cursor: 'pointer',
              }}
            >
              <div style={{ fontSize: 14, color: '#333' }}>
                {addr.consignee} {addr.mobile}
                {addr.isDefault ? <span style={{ color: '#999', marginLeft: 8 }}>{t('默认')}</span> : null}
              </div>
              <div style={{ fontSize: 12, color: '#999', marginTop: 4 }}>
                {(addr.regionNames || []).join('')} {addr.address}
              </div>
            </div>
          ))}
        </div>
      </Modal>
    </div>
  );
}
