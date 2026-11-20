// 我方新增（平台联邦 P1）：成员连接页（组织 → 组织管理 → 成员连接，我是中枢 role=1）。
import { useMemo, useState } from 'react';
import { Button, Checkbox, Input, Modal, Popconfirm, Select, Space, Tag, Toast } from '@douyinfe/semi-ui';
import { apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import { unixToDateTime } from '../utils/format';
import { HUB_MODULE_BITS, moduleNamesFromBits } from '../federation';

export interface HubLinkRow {
  linkId: number;
  role?: number;
  peerInstanceUuid?: string;
  peerName?: string;
  peerBaseUrl?: string;
  peerVersion?: string;
  status?: number;
  heartbeatState?: string;
  approvedModules?: number;
  upCredentialId?: number;
  downCredentialId?: number;
  partnerUserId?: number;
  settleMode?: string;
  settleCycle?: string;
  fundChannel?: string;
  rateSource?: string;
  rateOverride?: string | number;
  rejectReason?: string;
  lastHeartbeatTime?: number;
  claimExpireTime?: number;
  addTime?: number;
  updateTime?: number;
}

export const LINK_STATUS: Record<number, { label: string; color: string }> = {
  1: { label: '待审批', color: 'orange' },
  2: { label: '已激活', color: 'green' },
  3: { label: '已停用', color: 'grey' },
  4: { label: '已拒绝', color: 'red' },
  5: { label: '已过期', color: 'grey' },
};

export function linkStatusTag(status?: number) {
  const item = LINK_STATUS[status || 0] || { label: '未知', color: 'grey' };
  return <Tag color={item.color as any}>{item.label}</Tag>;
}

export function heartbeatTag(state?: string) {
  if (state === 'online') return <Tag color="green">online</Tag>;
  if (state === 'stale') return <Tag color="orange">stale</Tag>;
  return <Tag color="grey">unknown</Tag>;
}

/** 成员连接（role=1）：待审批连接的批准/拒绝 + 已激活连接的启停/删除/同步商品。 */
export default function HubLinks() {
  const { t } = useI18n();
  const hubQuery = useMemo(() => ({ role: 1 }), []);
  const [listKey, setListKey] = useState(0);
  const refresh = () => setListKey((key) => key + 1);

  const [approveRow, setApproveRow] = useState<HubLinkRow | null>(null);
  const [approveModules, setApproveModules] = useState(1);
  const [partnerUserId, setPartnerUserId] = useState('');
  const [settleMode, setSettleMode] = useState('');
  const [settleCycle, setSettleCycle] = useState('');
  const [fundChannel, setFundChannel] = useState('');
  const [rateSource, setRateSource] = useState('');
  const [rateOverride, setRateOverride] = useState('');
  const [approving, setApproving] = useState(false);

  const [rejectRow, setRejectRow] = useState<HubLinkRow | null>(null);
  const [rejectReason, setRejectReason] = useState('');
  const [rejecting, setRejecting] = useState(false);

  const openApprove = (record: HubLinkRow) => {
    setApproveRow(record);
    setApproveModules(record.approvedModules || 1);
    setPartnerUserId(record.partnerUserId ? String(record.partnerUserId) : '');
    setSettleMode(record.settleMode || '');
    setSettleCycle(record.settleCycle || '');
    setFundChannel(record.fundChannel || '');
    setRateSource(record.rateSource || '');
    setRateOverride(record.rateOverride !== undefined && record.rateOverride !== null ? String(record.rateOverride) : '');
  };

  const submitApprove = async () => {
    if (!approveRow) return;
    setApproving(true);
    try {
      await apiPost('/hub/links/approve', {
        linkId: approveRow.linkId,
        approvedModules: approveModules,
        partnerUserId: partnerUserId ? Number(partnerUserId) : undefined,
        settleMode: settleMode || undefined,
        settleCycle: settleCycle || undefined,
        fundChannel: fundChannel || undefined,
        rateSource: rateSource || undefined,
        rateOverride: rateOverride || undefined,
      });
      Toast.success(t('已批准'));
      setApproveRow(null);
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('批准失败') });
    } finally {
      setApproving(false);
    }
  };

  const submitReject = async () => {
    if (!rejectRow) return;
    setRejecting(true);
    try {
      await apiPost('/hub/links/reject', { linkId: rejectRow.linkId, reason: rejectReason });
      Toast.success(t('已拒绝'));
      setRejectRow(null);
      setRejectReason('');
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('拒绝失败') });
    } finally {
      setRejecting(false);
    }
  };

  const setStatus = async (record: HubLinkRow, status: number) => {
    try {
      await apiPost('/hub/links/setStatus', { id: record.linkId, status });
      Toast.success(t('状态已更新'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    }
  };

  const remove = async (record: HubLinkRow) => {
    try {
      await apiPost('/hub/links/del', { id: record.linkId });
      Toast.success(t('删除成功'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
    }
  };

  const sync = async (record: HubLinkRow) => {
    try {
      const result = await apiPost<Record<string, any>>('/hub/aggregation/sync', { linkId: record.linkId });
      Toast.success(
        `${t('同步完成')}：${t('拉取')} ${result?.fetched ?? 0} / ${t('新建')} ${result?.created ?? 0} / ${t('更新')} ${result?.updated ?? 0} / ${t('无变化')} ${result?.unchanged ?? 0}`,
      );
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('同步失败') });
    }
  };

  const columns: ColumnConfig<HubLinkRow>[] = [
    { title: '连接ID', dataIndex: 'linkId', width: 90 },
    {
      title: '对端实例',
      dataIndex: 'peerName',
      width: 220,
      render: (_: any, record: HubLinkRow) => (
        <div style={{ display: 'grid' }}>
          <span>{record.peerName || '-'}</span>
          <span style={{ color: 'var(--semi-color-text-2)', fontSize: 12 }}>{record.peerInstanceUuid || ''}</span>
        </div>
      ),
    },
    { title: '对端地址', dataIndex: 'peerBaseUrl', width: 200, render: (v: string) => v || '-' },
    { title: '状态', dataIndex: 'status', width: 100, render: (v: number) => linkStatusTag(v) },
    { title: '心跳', dataIndex: 'heartbeatState', width: 100, render: (v: string) => heartbeatTag(v) },
    {
      title: '批准模块',
      dataIndex: 'approvedModules',
      width: 150,
      render: (v: number) => moduleNamesFromBits(v || 0).join(' / ') || '-',
    },
    { title: '伙伴账户', dataIndex: 'partnerUserId', width: 100, render: (v: number) => v || '-' },
    { title: '结算方式', dataIndex: 'settleMode', width: 110, render: (v: string) => v || '-' },
    { title: '最后心跳', dataIndex: 'lastHeartbeatTime', width: 170, render: (v: number) => unixToDateTime(v) },
    { title: '创建时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  return (
    <div>
      <ListPage<HubLinkRow>
        key={listKey}
        api="/hub/links/list"
        rowKey="linkId"
        pageSize={10}
        actionWidth={300}
        query={hubQuery}
        filters={[
          { field: 'keyword', label: '连接', type: 'input', placeholder: '搜索对端名称/UUID', width: 260 },
          {
            field: 'status',
            label: '状态',
            type: 'select',
            placeholder: '全部状态',
            width: 150,
            options: [
              { label: '待审批', value: 1 },
              { label: '已激活', value: 2 },
              { label: '已停用', value: 3 },
              { label: '已拒绝', value: 4 },
              { label: '已过期', value: 5 },
            ],
          },
        ]}
        columns={columns}
        rowActions={(record) => (
          <Space spacing={2}>
            {record.status === 1 && (
              <>
                <Button theme="borderless" size="small" onClick={() => openApprove(record)}>
                  {t('批准')}
                </Button>
                <Button theme="borderless" size="small" type="danger" onClick={() => setRejectRow(record)}>
                  {t('拒绝')}
                </Button>
              </>
            )}
            {record.status === 2 && (
              <Button theme="borderless" size="small" onClick={() => void sync(record)}>
                {t('同步商品')}
              </Button>
            )}
            {record.status === 2 && (
              <Button theme="borderless" size="small" onClick={() => void setStatus(record, 3)}>
                {t('停用')}
              </Button>
            )}
            {record.status === 3 && (
              <Button theme="borderless" size="small" onClick={() => void setStatus(record, 2)}>
                {t('启用')}
              </Button>
            )}
            <Popconfirm title={t('确定删除该连接？')} onConfirm={() => void remove(record)}>
              <Button theme="borderless" type="danger" size="small">
                {t('删除')}
              </Button>
            </Popconfirm>
          </Space>
        )}
      />

      <Modal
        title={t('批准连接')}
        visible={Boolean(approveRow)}
        onCancel={() => setApproveRow(null)}
        onOk={() => void submitApprove()}
        okText={t('批准')}
        confirmLoading={approving}
        width={560}
      >
        <div style={{ display: 'grid', gap: 14 }}>
          <div>
            <div style={{ marginBottom: 6 }}>{t('批准模块')}</div>
            <Space>
              {HUB_MODULE_BITS.map((item) => (
                <Checkbox
                  key={item.bit}
                  checked={(approveModules & item.bit) === item.bit}
                  onChange={(e) =>
                    setApproveModules((current) =>
                      e.target.checked ? current | item.bit : current & ~item.bit,
                    )
                  }
                >
                  {t(item.label)}
                </Checkbox>
              ))}
            </Space>
          </div>
          <div>
            <div style={{ marginBottom: 6 }}>{t('伙伴账户 userId')}</div>
            <Input value={partnerUserId} onChange={setPartnerUserId} placeholder={t('对端在本实例的伙伴账户 ID')} />
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
            <div>
              <div style={{ marginBottom: 6 }}>{t('结算模式')}</div>
              <Select
                value={settleMode || undefined}
                onChange={(v) => setSettleMode(String(v || ''))}
                placeholder={t('不设置')}
                style={{ width: '100%' }}
                optionList={[
                  { label: '预付 prepaid', value: 'prepaid' },
                  { label: '周期结算 periodic', value: 'periodic' },
                  { label: '佣金 commission', value: 'commission' },
                ]}
              />
            </div>
            <div>
              <div style={{ marginBottom: 6 }}>{t('结算周期')}</div>
              <Select
                value={settleCycle || undefined}
                onChange={(v) => setSettleCycle(String(v || ''))}
                placeholder={t('不设置')}
                style={{ width: '100%' }}
                optionList={[
                  { label: 'T+N', value: 't_plus_N' },
                  { label: '周结 weekly', value: 'weekly' },
                  { label: '月结 monthly', value: 'monthly' },
                ]}
              />
            </div>
            <div>
              <div style={{ marginBottom: 6 }}>{t('资金通道')}</div>
              <Select
                value={fundChannel || undefined}
                onChange={(v) => setFundChannel(String(v || ''))}
                placeholder={t('不设置')}
                style={{ width: '100%' }}
                optionList={[
                  { label: '自有 own', value: 'own' },
                  { label: '渠道分账 channel_split', value: 'channel_split' },
                ]}
              />
            </div>
            <div>
              <div style={{ marginBottom: 6 }}>{t('费率来源')}</div>
              <Select
                value={rateSource || undefined}
                onChange={(v) => setRateSource(String(v || ''))}
                placeholder={t('不设置')}
                style={{ width: '100%' }}
                optionList={[
                  { label: '按等级 level', value: 'level' },
                  { label: '连接覆盖 link_override', value: 'link_override' },
                ]}
              />
            </div>
          </div>
          {rateSource === 'link_override' && (
            <div>
              <div style={{ marginBottom: 6 }}>{t('费率覆盖值')}</div>
              <Input value={rateOverride} onChange={setRateOverride} placeholder={t('如 0.05')} />
            </div>
          )}
        </div>
      </Modal>

      <Modal
        title={t('拒绝连接')}
        visible={Boolean(rejectRow)}
        onCancel={() => setRejectRow(null)}
        onOk={() => void submitReject()}
        okText={t('拒绝')}
        confirmLoading={rejecting}
        width={480}
      >
        <div style={{ marginBottom: 6 }}>{t('拒绝原因')}</div>
        <Input value={rejectReason} onChange={setRejectReason} placeholder={t('填写拒绝原因（将同步给对端）')} />
      </Modal>
    </div>
  );
}
