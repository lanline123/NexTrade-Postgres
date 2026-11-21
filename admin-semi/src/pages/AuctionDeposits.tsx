import { Tag } from '@douyinfe/semi-ui';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';

interface DepositRow {
  depositId: number;
  itemTitle?: string;
  username?: string;
  nickname?: string;
  amount?: string | number;
  status?: number;
  statusName?: string;
  orderId?: number | null;
  penaltyAmount?: string | number;
  addTimeText?: string;
  handleTimeText?: string;
  remark?: string;
}

const STATUS_OPTIONS = [
  { label: '全部状态', value: '' },
  { label: '冻结中', value: 1 },
  { label: '已解冻', value: 2 },
  { label: '已抵扣', value: 3 },
  { label: '已扣罚', value: 4 },
  { label: '部分扣罚后退回', value: 5 },
];

const statusColor = (status?: number) => {
  switch (status) {
    case 1:
      return 'orange';
    case 2:
      return 'grey';
    case 3:
      return 'blue';
    case 4:
      return 'red';
    case 5:
      return 'violet';
    default:
      return 'grey';
  }
};

/** 拍卖保证金流水（商品 → 拍卖 → 保证金流水），只读。 */
export default function AuctionDeposits() {
  const { t } = useI18n();

  const columns: ColumnConfig<DepositRow>[] = [
    { title: t('拍品'), dataIndex: 'itemTitle', width: 240, render: (v: string) => v || '-' },
    {
      title: t('会员'),
      dataIndex: 'username',
      width: 150,
      render: (_: string, row: DepositRow) => row.nickname || row.username || '-',
    },
    { title: t('金额'), dataIndex: 'amount', width: 110, render: (v: any) => `¥${v ?? '-'}` },
    {
      title: t('状态'),
      dataIndex: 'statusName',
      width: 130,
      render: (v: string, row: DepositRow) => <Tag color={statusColor(row.status) as any}>{v ? t(v) : '-'}</Tag>,
    },
    {
      title: t('扣罚'),
      dataIndex: 'penaltyAmount',
      width: 100,
      render: (v: any) => (v != null && Number(v) > 0 ? `¥${v}` : '-'),
    },
    {
      title: t('成交订单'),
      dataIndex: 'orderId',
      width: 100,
      render: (v: any) => (v ? `#${v}` : '-'),
    },
    { title: t('冻结时间'), dataIndex: 'addTimeText', width: 160 },
    { title: t('处理时间'), dataIndex: 'handleTimeText', width: 160 },
    { title: t('备注'), dataIndex: 'remark', width: 220, render: (v: string) => v || '-' },
  ];

  return (
    <div style={{ padding: 16 }}>
      <ListPage<DepositRow>
        api="/promotion/auction/deposit/list"
        rowKey="depositId"
        pageSize={10}
        filters={[
          { field: 'userId', label: t('会员ID'), type: 'number', width: 130 },
          { field: 'itemId', label: t('拍品ID'), type: 'number', width: 130 },
          { field: 'status', label: t('状态'), type: 'select', options: STATUS_OPTIONS, width: 150 },
        ]}
        columns={columns}
      />
    </div>
  );
}
