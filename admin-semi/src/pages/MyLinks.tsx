// 我方新增（万象互联 P1）：我的连接页（万象互联 → 我的连接，我是成员 role=2）。
import { useMemo, useState } from 'react';
import { Button, Card, Input, Space, Toast } from '@douyinfe/semi-ui';
import { apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import { unixToDateTime } from '../utils/format';
import { heartbeatTag, linkStatusTag, type HubLinkRow } from './HubLinks';
import { moduleNamesFromBits } from '../federation';

/** 我的连接（role=2）：向中枢发起配对 + 轮询审批状态（批准后自动领取激活）+ 发送心跳。 */
export default function MyLinks() {
  const { t } = useI18n();
  const myQuery = useMemo(() => ({ role: 2 }), []);
  const [listKey, setListKey] = useState(0);
  const refresh = () => setListKey((key) => key + 1);
  const [hubBaseUrl, setHubBaseUrl] = useState('');
  const [pairingCode, setPairingCode] = useState('');
  const [initiating, setInitiating] = useState(false);

  const initiate = async () => {
    if (!hubBaseUrl.trim() || !pairingCode.trim()) {
      Toast.warning(t('请填写中枢地址与配对码'));
      return;
    }
    setInitiating(true);
    try {
      await apiPost('/hub/myLinks/initiate', {
        hubBaseUrl: hubBaseUrl.trim(),
        pairingCode: pairingCode.trim(),
      });
      Toast.success(t('配对已发起，等待中枢审批'));
      setPairingCode('');
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('发起配对失败') });
    } finally {
      setInitiating(false);
    }
  };

  const poll = async (record: HubLinkRow) => {
    try {
      const result = await apiPost<HubLinkRow>('/hub/myLinks/poll', { id: record.linkId });
      Toast.success(`${t('当前状态')}：${result?.status ?? record.status}`);
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('轮询失败') });
    }
  };

  const heartbeat = async (record: HubLinkRow) => {
    try {
      await apiPost('/hub/myLinks/heartbeat', { id: record.linkId });
      Toast.success(t('心跳已发送'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('心跳发送失败') });
    }
  };

  const columns: ColumnConfig<HubLinkRow>[] = [
    { title: '连接ID', dataIndex: 'linkId', width: 90 },
    {
      title: '中枢实例',
      dataIndex: 'peerName',
      width: 220,
      render: (_: any, record: HubLinkRow) => (
        <div style={{ display: 'grid' }}>
          <span>{record.peerName || '-'}</span>
          <span style={{ color: 'var(--semi-color-text-2)', fontSize: 12 }}>{record.peerInstanceUuid || ''}</span>
        </div>
      ),
    },
    { title: '中枢地址', dataIndex: 'peerBaseUrl', width: 200, render: (v: string) => v || '-' },
    { title: '状态', dataIndex: 'status', width: 100, render: (v: number) => linkStatusTag(v) },
    { title: '心跳', dataIndex: 'heartbeatState', width: 100, render: (v: string) => heartbeatTag(v) },
    {
      title: '批准模块',
      dataIndex: 'approvedModules',
      width: 150,
      render: (v: number) => moduleNamesFromBits(v || 0).join(' / ') || '-',
    },
    { title: '拒绝原因', dataIndex: 'rejectReason', width: 160, render: (v: string) => v || '-' },
    { title: '最后心跳', dataIndex: 'lastHeartbeatTime', width: 170, render: (v: number) => unixToDateTime(v) },
    { title: '创建时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      <Card title={t('发起配对（接入中枢）')}>
        <Space wrap>
          <Input
            value={hubBaseUrl}
            onChange={setHubBaseUrl}
            placeholder={t('中枢对外地址，如 https://hub.example.com')}
            style={{ width: 360 }}
          />
          <Input
            value={pairingCode}
            onChange={setPairingCode}
            placeholder={t('中枢发放的配对码')}
            style={{ width: 320 }}
          />
          <Button type="primary" theme="solid" loading={initiating} onClick={() => void initiate()}>
            {t('发起配对')}
          </Button>
        </Space>
      </Card>

      <ListPage<HubLinkRow>
        key={listKey}
        api="/hub/links/list"
        rowKey="linkId"
        pageSize={10}
        actionWidth={220}
        query={myQuery}
        filters={[
          { field: 'keyword', label: '连接', type: 'input', placeholder: '搜索中枢名称/UUID', width: 260 },
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
            <Button theme="borderless" size="small" onClick={() => void poll(record)}>
              {t('刷新状态')}
            </Button>
            {record.status === 2 && (
              <Button theme="borderless" size="small" onClick={() => void heartbeat(record)}>
                {t('发送心跳')}
              </Button>
            )}
          </Space>
        )}
      />
    </div>
  );
}
