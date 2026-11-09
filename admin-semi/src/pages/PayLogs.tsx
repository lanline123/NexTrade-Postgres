import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';

interface PayLogRow {
  paylogId: number;
  orderSn?: string;
  consignee?: string;
  payName?: string;
  payAmount?: number | string;
  refundAmount?: number | string;
  addTime?: string;
  payStatus?: number;
  payStatusName?: string;
}

const columns: ColumnConfig<PayLogRow>[] = [
  { title: '订单编号', dataIndex: 'orderSn', width: 220 },
  { title: '订单收件人', dataIndex: 'consignee', width: 150, render: (value: string) => value || '--' },
  { title: '支付方式', dataIndex: 'payName', width: 150 },
  { title: '支付金额', dataIndex: 'payAmount', width: 130 },
  { title: '已退款金额', dataIndex: 'refundAmount', width: 130, render: (value: number | string) => value ?? '-' },
  { title: '支付日期', dataIndex: 'addTime', width: 180 },
  {
    title: '支付状态',
    dataIndex: 'payStatusName',
    width: 120,
    render: (value: string, record) => (
      <span style={{ color: record.payStatus === 2 ? 'green' : 'red' }}>{value}</span>
    ),
  },
];

export default function PayLogs() {
  return (
    <ListPage<PayLogRow>
      api="/finance/payLog/list"
      rowKey="paylogId"
      pageSize={10}
      actionWidth={0}
      filters={[
        {
          field: 'payStatus',
          label: '支付状态',
          type: 'select',
          width: 130,
          options: [
            { label: '未支付', value: 0 },
            { label: '已支付', value: 2 },
          ],
        },
        {
          field: 'keyword',
          label: '订单号',
          type: 'input',
          placeholder: '输入订单号',
          width: 240,
        },
      ]}
      columns={columns}
    />
  );
}
