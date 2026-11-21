// 我方新增（万象互联三期①）：登录互认页（万象互联 → 登录互认）。
// 中枢侧只读审计：成员站发起的联邦登录授权请求（设计 DESIGN.md §1——
// 同意由会员本人在中枢前台完成，管理端只看审计，不代签、不可撤销已发码）。
import { Tag } from '@douyinfe/semi-ui';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import { unixToDateTime } from '../utils/format';

interface SsoRequestRow {
  id: number;
  linkId: number;
  peerName?: string;
  returnUrl?: string;
  status: number;
  hubUserId?: number;
  globalMemberId?: string;
  codeUsed?: number;
  addTime?: number;
  updateTime?: number;
}

const SSO_STATUS: Record<number, { label: string; color: 'grey' | 'green' | 'red' | 'orange' }> = {
  0: { label: '待同意', color: 'grey' },
  1: { label: '已同意', color: 'green' },
  2: { label: '已拒绝', color: 'red' },
  3: { label: '已过期', color: 'orange' },
};

export default function HubSso() {
  const { t } = useI18n();

  const columns: ColumnConfig<SsoRequestRow>[] = [
    { title: '请求ID', dataIndex: 'id', width: 90 },
    {
      title: '申请方',
      dataIndex: 'peerName',
      width: 180,
      render: (_: any, record: SsoRequestRow) => record.peerName || `连接 #${record.linkId}`,
    },
    { title: '连接ID', dataIndex: 'linkId', width: 90 },
    {
      title: '状态',
      dataIndex: 'status',
      width: 100,
      render: (v: number) => {
        const s = SSO_STATUS[v ?? -1];
        return s ? <Tag color={s.color}>{t(s.label)}</Tag> : <Tag>{`状态 ${v ?? '-'}`}</Tag>;
      },
    },
    { title: '同意会员ID', dataIndex: 'hubUserId', width: 110, render: (v: number) => (v ? v : '-') },
    { title: '全局会员ID', dataIndex: 'globalMemberId', width: 280, render: (v: string) => v || '-' },
    {
      title: '授权码',
      dataIndex: 'codeUsed',
      width: 100,
      render: (v: number, record: SsoRequestRow) =>
        record.status !== 1 ? '-' : v === 1 ? <Tag color="green">{t('已兑换')}</Tag> : <Tag color="orange">{t('未兑换')}</Tag>,
    },
    { title: '回跳地址', dataIndex: 'returnUrl', width: 260, render: (v: string) => v || '-' },
    { title: '发起时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
    { title: '更新时间', dataIndex: 'updateTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  return (
    <ListPage<SsoRequestRow>
      api="/hub/sso/requests"
      rowKey="id"
      pageSize={10}
      filters={[
        { field: 'linkId', label: '连接ID', type: 'number', placeholder: '连接 ID', width: 130 },
        {
          field: 'status',
          label: '状态',
          type: 'select',
          width: 140,
          options: [
            { label: '待同意', value: 0 },
            { label: '已同意', value: 1 },
            { label: '已拒绝', value: 2 },
            { label: '已过期', value: 3 },
          ],
        },
      ]}
      columns={columns}
    />
  );
}
