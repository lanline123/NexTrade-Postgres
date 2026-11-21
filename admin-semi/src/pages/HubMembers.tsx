// 我方新增（万象互联二期）：会员联邦页（万象互联 → 会员联邦）。
// 中枢侧：全局会员映射列表（手机号脱敏）+ 冲突队列人工裁决。
// 口径（设计 §6.2）：主匹配键 = 已验证手机号；弱信号一律进冲突队列，
// 绝不自动合并——本页就是那个人工裁决入口。
import { useState } from 'react';
import { Button, Radio, RadioGroup, Space, Tag, Toast } from '@douyinfe/semi-ui';
import { apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import { unixToDateTime } from '../utils/format';

interface MemberLinkRow {
  id: number;
  globalMemberId: string;
  linkId: number;
  remoteMemberId: number;
  mobileMasked: string;
  nickname?: string;
  email?: string;
  profileUpdatedAt?: number;
  status?: number;
  addTime?: number;
}

interface ConflictRow {
  id: number;
  linkId: number;
  remoteMemberId: number;
  mobileMasked: string;
  nickname?: string;
  email?: string;
  candidateGlobalMemberId?: string;
  conflictType: string;
  status?: number;
  addTime?: number;
}

const LINK_STATUS: Record<number, { label: string; color: string }> = {
  1: { label: '已关联', color: 'green' },
  3: { label: '已解绑', color: 'grey' },
};

const CONFLICT_TYPE_LABEL: Record<string, string> = {
  mobile_multi_remote: '同号多远端账号',
  mobile_claimed: '手机号已被占用',
  identity_mismatch: '实名不一致',
  unverified_mobile: '手机号未验证',
};

const CONFLICT_STATUS: Record<number, { label: string; color: string }> = {
  0: { label: '待处理', color: 'orange' },
  1: { label: '已关联', color: 'green' },
  2: { label: '独立建号', color: 'blue' },
  3: { label: '已忽略', color: 'grey' },
};

export default function HubMembers() {
  const { t } = useI18n();
  const [tab, setTab] = useState<'links' | 'conflicts'>('links');
  const [listKey, setListKey] = useState(0);
  const refresh = () => setListKey((key) => key + 1);
  const [resolvingId, setResolvingId] = useState(0);

  const resolve = async (record: ConflictRow, action: 'link' | 'separate' | 'ignore') => {
    setResolvingId(record.id);
    try {
      await apiPost('/hub/members/resolveConflict', { conflictId: record.id, action });
      Toast.success(t('裁决已生效'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('裁决失败') });
    } finally {
      setResolvingId(0);
    }
  };

  const linkColumns: ColumnConfig<MemberLinkRow>[] = [
    { title: '全局会员ID', dataIndex: 'globalMemberId', width: 300, render: (v: string) => v || '-' },
    { title: '昵称', dataIndex: 'nickname', width: 140, render: (v: string) => v || '-' },
    { title: '手机号', dataIndex: 'mobileMasked', width: 130, render: (v: string) => v || '****' },
    { title: '邮箱', dataIndex: 'email', width: 180, render: (v: string) => v || '-' },
    { title: '连接ID', dataIndex: 'linkId', width: 90 },
    { title: '远端会员ID', dataIndex: 'remoteMemberId', width: 110 },
    {
      title: '状态',
      dataIndex: 'status',
      width: 100,
      render: (v: number) => {
        const item = LINK_STATUS[v || 0] || { label: '未知', color: 'grey' };
        return <Tag color={item.color as any}>{t(item.label)}</Tag>;
      },
    },
    { title: '资料版本', dataIndex: 'profileUpdatedAt', width: 170, render: (v: number) => unixToDateTime(v) },
    { title: '关联时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  const conflictColumns: ColumnConfig<ConflictRow>[] = [
    { title: '冲突ID', dataIndex: 'id', width: 90 },
    {
      title: '冲突类型',
      dataIndex: 'conflictType',
      width: 150,
      render: (v: string) => <Tag color="orange">{t(CONFLICT_TYPE_LABEL[v] || v)}</Tag>,
    },
    { title: '手机号', dataIndex: 'mobileMasked', width: 130, render: (v: string) => v || '****' },
    { title: '昵称', dataIndex: 'nickname', width: 130, render: (v: string) => v || '-' },
    { title: '候选全局ID', dataIndex: 'candidateGlobalMemberId', width: 280, render: (v: string) => v || '-' },
    { title: '连接ID', dataIndex: 'linkId', width: 90 },
    { title: '远端会员ID', dataIndex: 'remoteMemberId', width: 110 },
    {
      title: '状态',
      dataIndex: 'status',
      width: 100,
      render: (v: number) => {
        const item = CONFLICT_STATUS[v ?? -1] || { label: '未知', color: 'grey' };
        return <Tag color={item.color as any}>{t(item.label)}</Tag>;
      },
    },
    { title: '上报时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  return (
    <div>
      <div style={{ marginBottom: 12 }}>
        <RadioGroup
          type="button"
          value={tab}
          onChange={(e) => setTab(e.target.value as 'links' | 'conflicts')}
        >
          <Radio value="links">{t('会员映射')}</Radio>
          <Radio value="conflicts">{t('冲突队列')}</Radio>
        </RadioGroup>
      </div>
      {tab === 'links' ? (
        <ListPage<MemberLinkRow>
          key={`links-${listKey}`}
          api="/hub/members/list"
          rowKey="id"
          pageSize={10}
          filters={[
            { field: 'keyword', label: '会员', type: 'input', placeholder: '全局 ID / 昵称', width: 260 },
            { field: 'linkId', label: '连接ID', type: 'number', placeholder: '连接 ID', width: 130 },
            {
              field: 'status',
              label: '状态',
              type: 'select',
              placeholder: '全部状态',
              width: 150,
              options: [
                { label: '已关联', value: 1 },
                { label: '已解绑', value: 3 },
              ],
            },
          ]}
          columns={linkColumns}
        />
      ) : (
        <ListPage<ConflictRow>
          key={`conflicts-${listKey}`}
          api="/hub/members/conflicts"
          rowKey="id"
          pageSize={10}
          actionWidth={230}
          filters={[
            { field: 'linkId', label: '连接ID', type: 'number', placeholder: '连接 ID', width: 130 },
            {
              field: 'status',
              label: '状态',
              type: 'select',
              placeholder: '全部状态',
              width: 150,
              options: [
                { label: '待处理', value: 0 },
                { label: '已关联', value: 1 },
                { label: '独立建号', value: 2 },
                { label: '已忽略', value: 3 },
              ],
            },
          ]}
          columns={conflictColumns}
          rowActions={(record) =>
            record.status === 0 ? (
              <Space spacing={2}>
                <Button
                  theme="borderless"
                  size="small"
                  type="primary"
                  loading={resolvingId === record.id}
                  disabled={!record.candidateGlobalMemberId}
                  onClick={() => void resolve(record, 'link')}
                >
                  {t('关联候选')}
                </Button>
                <Button
                  theme="borderless"
                  size="small"
                  loading={resolvingId === record.id}
                  onClick={() => void resolve(record, 'separate')}
                >
                  {t('独立建号')}
                </Button>
                <Button
                  theme="borderless"
                  size="small"
                  loading={resolvingId === record.id}
                  onClick={() => void resolve(record, 'ignore')}
                >
                  {t('忽略')}
                </Button>
              </Space>
            ) : null
          }
        />
      )}
    </div>
  );
}
