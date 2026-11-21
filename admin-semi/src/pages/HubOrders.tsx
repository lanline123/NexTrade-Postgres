// 我方新增（万象互联二期）：订单上行页（万象互联 → 订单上行）。
// 中枢侧只读：成员实例上报的订单头级汇总（设计 §7.1——不含地址/收件人/行项目，
// 中枢对订单只读，履约售后仍在来源实例）。
import { Tag } from '@douyinfe/semi-ui';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import { unixToDateTime } from '../utils/format';

interface OrderSummaryRow {
  id: number;
  linkId: number;
  remoteInstanceUuid?: string;
  remoteOrderSn: string;
  remoteShopId?: number;
  remoteShopName?: string;
  remoteUserId?: number;
  globalMemberId?: string;
  orderAmount?: number | string;
  paidAmount?: number | string;
  orderStatus?: number;
  payStatus?: number;
  itemCount?: number;
  orderTime?: number;
  payTime?: number;
  updateTime?: number;
}

const ORDER_STATUS_LABEL: Record<number, string> = {
  0: '待付款',
  1: '待发货',
  2: '待收货',
  3: '已完成',
  4: '已取消',
  5: '已关闭',
};

export default function HubOrders() {
  const { t } = useI18n();

  const columns: ColumnConfig<OrderSummaryRow>[] = [
    { title: '远端订单号', dataIndex: 'remoteOrderSn', width: 200, render: (v: string) => v || '-' },
    {
      title: '来源店铺',
      dataIndex: 'remoteShopName',
      width: 160,
      render: (_: any, record: OrderSummaryRow) =>
        record.remoteShopName || (record.remoteShopId ? `#${record.remoteShopId}` : '-'),
    },
    { title: '来源实例', dataIndex: 'remoteInstanceUuid', width: 200, render: (v: string) => v || '-' },
    { title: '连接ID', dataIndex: 'linkId', width: 90 },
    { title: '买家全局ID', dataIndex: 'globalMemberId', width: 280, render: (v: string) => v || '-' },
    { title: '订单金额', dataIndex: 'orderAmount', width: 110, render: (v: any) => (v === undefined || v === null ? '-' : String(v)) },
    { title: '实付金额', dataIndex: 'paidAmount', width: 110, render: (v: any) => (v === undefined || v === null ? '-' : String(v)) },
    {
      title: '订单状态',
      dataIndex: 'orderStatus',
      width: 110,
      render: (v: number) => <Tag color="blue">{t(ORDER_STATUS_LABEL[v ?? -1] || `状态 ${v ?? '-'}`)}</Tag>,
    },
    {
      title: '支付状态',
      dataIndex: 'payStatus',
      width: 100,
      render: (v: number) => (v === 1 ? <Tag color="green">{t('已支付')}</Tag> : <Tag color="grey">{t('未支付')}</Tag>),
    },
    { title: '件数', dataIndex: 'itemCount', width: 70, render: (v: number) => v ?? 0 },
    { title: '下单时间', dataIndex: 'orderTime', width: 170, render: (v: number) => unixToDateTime(v) },
    { title: '支付时间', dataIndex: 'payTime', width: 170, render: (v: number) => unixToDateTime(v) },
    { title: '最近上报', dataIndex: 'updateTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  return (
    <ListPage<OrderSummaryRow>
      api="/hub/orders/list"
      rowKey="id"
      pageSize={10}
      filters={[
        { field: 'keyword', label: '订单号', type: 'input', placeholder: '远端订单号', width: 240 },
        { field: 'linkId', label: '连接ID', type: 'number', placeholder: '连接 ID', width: 130 },
      ]}
      columns={columns}
    />
  );
}
