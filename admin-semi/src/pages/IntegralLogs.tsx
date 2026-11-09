import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';

interface IntegralLogRow {
  id: number;
  changeTime?: string;
  username?: string;
  changeDesc?: string;
  changeType?: number;
  points?: number | string;
}

const columns: ColumnConfig<IntegralLogRow>[] = [
  { title: '操作时间', dataIndex: 'changeTime', width: 180 },
  { title: '会员名称', dataIndex: 'username', width: 180 },
  { title: '操作备注', dataIndex: 'changeDesc', width: 240, render: (value: string) => value || '-' },
  {
    title: '积分',
    dataIndex: 'points',
    width: 140,
    render: (value: number | string, record) => (
      <span style={{ color: record.changeType === 1 ? '#1677ff' : '#f5222d' }}>
        {record.changeType === 1 ? '＋' : '－'}
        {value ?? 0}
      </span>
    ),
  },
];

export default function IntegralLogs() {
  return (
    <ListPage<IntegralLogRow>
      api="/user/userPointsLog/list"
      rowKey="id"
      pageSize={10}
      filters={[
        {
          field: 'keyword',
          label: '会员',
          type: 'input',
          placeholder: '输入会员名称/手机号/邮箱',
          width: 280,
        },
      ]}
      columns={columns}
    />
  );
}
