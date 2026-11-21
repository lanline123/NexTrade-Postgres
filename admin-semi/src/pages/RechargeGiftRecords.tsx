import { useState } from 'react';
import { Button, Modal, Space, Tag, Toast } from '@douyinfe/semi-ui';
import { apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';

interface RechargeGiftRecordRow {
  recordId: number;
  rechargeOrderId: number;
  userId: number;
  giftName?: string;
  giftId: number;
  quantity: number;
  status: number;
  statusText?: string;
  orderId: number;
  consignee?: string;
  mobile?: string;
  addressText?: string;
  adminNote?: string;
  addTime: number;
  claimTime: number;
}

const STATUS_COLORS: Record<number, 'grey' | 'green' | 'red' | 'orange'> = {
  0: 'orange',
  1: 'green',
  2: 'grey',
  3: 'red',
};

function formatTime(seconds?: number) {
  if (!seconds) return '-';
  const date = new Date(seconds * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

/** 充值赠品记录（我方新增，2026-11-21 论坛帖 717721）：
 * 充值到账后生成的待领取赠品在此跟踪；异常（库存不足等）记录可恢复或作废，
 * 恢复前服务端会校验关联充值单仍为已支付。履约订单生成后走商家订单链发货。 */
export default function RechargeGiftRecords() {
  const { t } = useI18n();
  const [reloadKey, setReloadKey] = useState(0);

  const reload = () => setReloadKey((key) => key + 1);

  const act = async (row: RechargeGiftRecordRow, action: 'void' | 'restore') => {
    Modal.confirm({
      title:
        action === 'void'
          ? t('确认作废该赠品记录？作废后会员不可再领取。')
          : t('确认恢复为待领取？恢复后会员可重新领取（领取时再扣库存）。'),
      onOk: async () => {
        try {
          await apiPost(`/promotion/rechargeGift/${action}`, { recordId: row.recordId });
          Toast.success({ content: t('操作成功') });
          reload();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
        }
      },
    });
  };

  const columns: ColumnConfig<RechargeGiftRecordRow>[] = [
    { title: '记录ID', dataIndex: 'recordId', width: 90 },
    {
      title: '赠品',
      dataIndex: 'giftName',
      width: 220,
      render: (value: string, row: RechargeGiftRecordRow) => (
        <span>{`${value || `赠品 ${row.giftId}`} × ${row.quantity}`}</span>
      ),
    },
    { title: '会员ID', dataIndex: 'userId', width: 90 },
    { title: '充值单ID', dataIndex: 'rechargeOrderId', width: 110 },
    {
      title: '状态',
      dataIndex: 'status',
      width: 100,
      render: (value: number, row: RechargeGiftRecordRow) => (
        <Tag color={STATUS_COLORS[value] || 'grey'}>{row.statusText || '-'}</Tag>
      ),
    },
    {
      title: '履约订单',
      dataIndex: 'orderId',
      width: 110,
      render: (value: number) => <span>{value ? value : '-'}</span>,
    },
    {
      title: '收货信息',
      dataIndex: 'consignee',
      width: 240,
      render: (_value: unknown, row: RechargeGiftRecordRow) =>
        row.consignee ? (
          <span>{`${row.consignee} ${row.mobile || ''} ${row.addressText || ''}`}</span>
        ) : (
          <span>-</span>
        ),
    },
    {
      title: '备注',
      dataIndex: 'adminNote',
      width: 200,
      render: (value: string) => <span>{value || '-'}</span>,
    },
    {
      title: '生成时间',
      dataIndex: 'addTime',
      width: 150,
      render: (value: number) => <span>{formatTime(value)}</span>,
    },
    {
      title: '领取时间',
      dataIndex: 'claimTime',
      width: 150,
      render: (value: number) => <span>{formatTime(value)}</span>,
    },
  ];

  return (
    <ListPage<RechargeGiftRecordRow>
      key={reloadKey}
      api="/promotion/rechargeGift/list"
      rowKey="recordId"
      pageSize={10}
      actionWidth={150}
      filters={[
        {
          field: 'status',
          label: '状态',
          type: 'select',
          placeholder: '全部状态',
          width: 160,
          options: [
            { label: '待领取', value: 0 },
            { label: '已领取', value: 1 },
            { label: '已作废', value: 2 },
            { label: '异常', value: 3 },
          ],
        },
        { field: 'userId', label: '会员ID', type: 'number', placeholder: '输入会员ID', width: 160 },
      ]}
      columns={columns}
      rowActions={(row) => (
        <Space spacing={2}>
          {(row.status === 0 || row.status === 3) && (
            <Button theme="borderless" size="small" type="danger" onClick={() => act(row, 'void')}>
              {t('作废')}
            </Button>
          )}
          {(row.status === 3 || row.status === 2) && (
            <Button theme="borderless" size="small" onClick={() => act(row, 'restore')}>
              {t('恢复待领取')}
            </Button>
          )}
        </Space>
      )}
    />
  );
}
