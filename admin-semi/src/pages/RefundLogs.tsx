import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';

interface RefundLogRow {
  refundLogId: number;
  aftersalesSn?: string;
  refundTypeText?: string;
  refundAmount?: number | string;
  addTime?: string;
  refund?: { aftersales?: { aftersalesSn?: string } };
}

const columns: ColumnConfig<RefundLogRow>[] = [
  {
    title: '退款信息',
    dataIndex: 'aftersalesSn',
    width: 260,
    render: (_value: string, record) =>
      record.aftersalesSn || record.refund?.aftersales?.aftersalesSn || '--',
  },
  { title: '退款方式', dataIndex: 'refundTypeText', width: 160 },
  { title: '退款金额', dataIndex: 'refundAmount', width: 140 },
  { title: '退款时间', dataIndex: 'addTime', width: 180 },
];

export default function RefundLogs() {
  return (
    <ListPage<RefundLogRow>
      api="/finance/refundLog/list"
      rowKey="refundLogId"
      pageSize={10}
      filters={[
        {
          field: 'type',
          label: '退款方式',
          type: 'select',
          width: 150,
          options: [
            { label: '原路返回', value: 1 },
            { label: '余额退回', value: 2 },
            { label: '线下退回', value: 3 },
          ],
        },
        {
          field: 'keyword',
          label: '售后编号',
          type: 'input',
          placeholder: '输入售后编号',
          width: 240,
        },
      ]}
      columns={columns}
    />
  );
}
