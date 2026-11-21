// 我方新增（万象互联三期③）：跨店订单页（万象互联 → 跨店订单）。
// 中枢侧：跨店主单（买家在本机跨成员店铺下单）+ 子单概要 + 路由重试；
// 成员侧：本机路由单（中枢买家落到本机的本地单）+ 逐单同步状态到中枢。
// 资金口径：付款走本站支付通道；分账只记账目（见联邦结算页），渠道未执行。
import { useState } from 'react';
import { Button, Radio, RadioGroup, Space, Tag, Toast } from '@douyinfe/semi-ui';
import { apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import { unixToDateTime } from '../utils/format';

interface XSubRow {
  id: number;
  subSn: string;
  remoteShopName?: string;
  amount?: number | string;
  status: number;
  remoteOrderSn?: string;
  trackingNo?: string;
}

interface XMasterRow {
  id: number;
  xorderSn: string;
  buyerUserId: number;
  totalAmount?: number | string;
  payStatus: number;
  status: number;
  paySn?: string;
  consignee?: string;
  addTime?: number;
  payTime?: number;
  subs?: XSubRow[];
}

interface XRouteRow {
  id: number;
  linkId: number;
  subSn: string;
  localOrderId: number;
  localOrderSn: string;
  buyerGlobalMemberId?: string;
  status: number;
  localOrderStatus?: number;
  localShippingStatus?: number;
  addTime?: number;
  updateTime?: number;
}

const MASTER_STATUS: Record<number, { label: string; color: 'grey' | 'blue' | 'green' | 'red' | 'orange' }> = {
  0: { label: '待付款', color: 'grey' },
  1: { label: '已付待路由', color: 'blue' },
  2: { label: '已全部路由', color: 'blue' },
  3: { label: '已完成', color: 'green' },
  4: { label: '已关闭', color: 'grey' },
  5: { label: '售后中', color: 'orange' },
};

const SUB_STATUS: Record<number, string> = {
  0: '待预占',
  1: '已预占',
  2: '已路由',
  3: '已发货',
  4: '已完成',
  5: '店主拒单',
  6: '预占失败',
  7: '已退款',
  8: '退款申请中',
  9: '已关闭',
};

export default function HubXOrders() {
  const { t } = useI18n();
  const [tab, setTab] = useState<'master' | 'routes'>('master');
  const [listKey, setListKey] = useState(0);
  const refresh = () => setListKey((k) => k + 1);
  const [busyId, setBusyId] = useState(0);

  const retry = async (record: XMasterRow) => {
    setBusyId(record.id);
    try {
      await apiPost('/hub/xorder/routeRetry', { id: record.id });
      Toast.success(t('路由已重跑'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('重试失败') });
    } finally {
      setBusyId(0);
    }
  };

  const sync = async (record: XRouteRow) => {
    setBusyId(record.id);
    try {
      const res = (await apiPost('/hub/xorder/syncRouteStatus', { id: record.id })) as { synced?: boolean };
      Toast.success(res.synced ? t('状态已同步到中枢') : t('状态无变化'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('同步失败') });
    } finally {
      setBusyId(0);
    }
  };

  const masterColumns: ColumnConfig<XMasterRow>[] = [
    { title: '跨店单号', dataIndex: 'xorderSn', width: 210, render: (v: string) => v || '-' },
    { title: '买家ID', dataIndex: 'buyerUserId', width: 90 },
    {
      title: '状态',
      dataIndex: 'status',
      width: 110,
      render: (v: number) => {
        const s = MASTER_STATUS[v ?? -1];
        return s ? <Tag color={s.color}>{t(s.label)}</Tag> : <Tag>{`状态 ${v ?? '-'}`}</Tag>;
      },
    },
    {
      title: '支付',
      dataIndex: 'payStatus',
      width: 90,
      render: (v: number) => (v === 1 ? <Tag color="green">{t('已支付')}</Tag> : v === 2 ? <Tag color="orange">{t('已退款')}</Tag> : <Tag color="grey">{t('未支付')}</Tag>),
    },
    { title: '总额', dataIndex: 'totalAmount', width: 110, render: (v: any) => (v === undefined || v === null ? '-' : String(v)) },
    {
      title: '子单',
      dataIndex: 'subs',
      width: 380,
      render: (_: any, record: XMasterRow) => (
        <div style={{ display: 'grid', gap: 2 }}>
          {(record.subs || []).map((s) => (
            <span key={s.id}>
              {s.remoteShopName || '-'} · {String(s.amount ?? '-')} · {t(SUB_STATUS[s.status] || `状态 ${s.status}`)}
              {s.remoteOrderSn ? ` · ${s.remoteOrderSn}` : ''}
              {s.trackingNo ? ` · ${s.trackingNo}` : ''}
            </span>
          ))}
        </div>
      ),
    },
    { title: '收货人', dataIndex: 'consignee', width: 100, render: (v: string) => v || '-' },
    { title: '下单时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
    { title: '支付时间', dataIndex: 'payTime', width: 170, render: (v: number) => (v ? unixToDateTime(v) : '-') },
  ];

  const routeColumns: ColumnConfig<XRouteRow>[] = [
    { title: '子单号', dataIndex: 'subSn', width: 210, render: (v: string) => v || '-' },
    { title: '本地单号', dataIndex: 'localOrderSn', width: 190, render: (v: string) => v || '-' },
    { title: '买家全局ID', dataIndex: 'buyerGlobalMemberId', width: 260, render: (v: string) => v || '-' },
    {
      title: '镜像状态',
      dataIndex: 'status',
      width: 110,
      render: (v: number) => <Tag color="blue">{t(SUB_STATUS[v] || `状态 ${v ?? '-'}`)}</Tag>,
    },
    {
      title: '本地单状态',
      dataIndex: 'localOrderStatus',
      width: 120,
      render: (v: number, record: XRouteRow) =>
        v === undefined || v < 0 ? '-' : `${v} / ${t('发货')} ${record.localShippingStatus ?? 0}`,
    },
    { title: '落单时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
    { title: '更新时间', dataIndex: 'updateTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  return (
    <div>
      <div style={{ marginBottom: 12 }}>
        <RadioGroup type="button" value={tab} onChange={(e) => setTab(e.target.value as typeof tab)}>
          <Radio value="master">{t('跨店主单（中枢）')}</Radio>
          <Radio value="routes">{t('本机路由单')}</Radio>
        </RadioGroup>
      </div>
      {tab === 'master' ? (
        <ListPage<XMasterRow>
          key={`master-${listKey}`}
          api="/hub/xorder/list"
          rowKey="id"
          pageSize={10}
          actionWidth={100}
          filters={[
            { field: 'keyword', label: '单号', type: 'input', placeholder: '跨店单号', width: 220 },
            {
              field: 'status',
              label: '状态',
              type: 'select',
              placeholder: '全部状态',
              width: 150,
              options: Object.entries(MASTER_STATUS).map(([value, s]) => ({ label: s.label, value: Number(value) })),
            },
          ]}
          columns={masterColumns}
          rowActions={(record) =>
            record.payStatus === 1 && record.status === 1 ? (
              <Button theme="borderless" size="small" type="primary" loading={busyId === record.id} onClick={() => void retry(record)}>
                {t('路由重试')}
              </Button>
            ) : null
          }
        />
      ) : (
        <ListPage<XRouteRow>
          key={`routes-${listKey}`}
          api="/hub/xorder/routes"
          rowKey="id"
          pageSize={10}
          actionWidth={110}
          filters={[
            { field: 'linkId', label: '连接ID', type: 'number', placeholder: '连接 ID', width: 130 },
            {
              field: 'status',
              label: '镜像状态',
              type: 'select',
              placeholder: '全部状态',
              width: 150,
              options: Object.entries(SUB_STATUS).map(([value, label]) => ({ label, value: Number(value) })),
            },
          ]}
          columns={routeColumns}
          rowActions={(record) => (
            <Space spacing={2}>
              <Button theme="borderless" size="small" type="primary" loading={busyId === record.id} onClick={() => void sync(record)}>
                {t('同步状态')}
              </Button>
            </Space>
          )}
        />
      )}
    </div>
  );
}
