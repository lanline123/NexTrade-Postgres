// 我方新增（万象互联三期②）：配置分发页（万象互联 → 配置分发）。
// 中枢侧：发布带版本配置包（公告/活动/频道/自定义）+ 撤销 + 回执覆盖率；
// 成员侧：按连接拉取（验签+哈希双验后落快照）、手动包逐包确认/拒绝。
// 应用语义如实标注：公告类回写本地平台公告，其余类型只落快照归档。
import { useState } from 'react';
import { Button, Input, InputNumber, Modal, Radio, RadioGroup, Select, Space, Tag, TextArea, Toast } from '@douyinfe/semi-ui';
import { apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import { unixToDateTime } from '../utils/format';

interface PackageRow {
  id: number;
  packageType: string;
  title: string;
  version: number;
  payload?: string;
  applyMode: number;
  effectiveFrom?: number;
  effectiveTo?: number;
  status: number;
  ackCount?: number;
  appliedCount?: number;
  addTime?: number;
}

interface AckRow {
  id: number;
  linkId: number;
  peerName?: string;
  packageId: number;
  version: number;
  ackStatus: number;
  message?: string;
  addTime?: number;
  updateTime?: number;
}

interface ApplyRow {
  id: number;
  linkId: number;
  packageId: number;
  version: number;
  packageType: string;
  title: string;
  payload?: string;
  applyMode: number;
  applyStatus: number;
  applyMessage?: string;
  addTime?: number;
  updateTime?: number;
}

const TYPE_LABEL: Record<string, string> = {
  announcement: '平台公告',
  activity: '活动配置',
  channel: '频道配置',
  custom: '自定义',
};

const ACK_STATUS: Record<number, { label: string; color: 'green' | 'red' | 'orange' | 'grey' }> = {
  1: { label: '已应用', color: 'green' },
  2: { label: '已拒绝', color: 'red' },
  3: { label: '应用失败', color: 'orange' },
};

const APPLY_STATUS: Record<number, { label: string; color: 'green' | 'red' | 'orange' | 'grey' }> = {
  0: { label: '待处理', color: 'orange' },
  1: { label: '已应用', color: 'green' },
  2: { label: '已拒绝', color: 'red' },
  3: { label: '失败', color: 'grey' },
};

export default function HubConfig() {
  const { t } = useI18n();
  const [tab, setTab] = useState<'packages' | 'acks' | 'applies'>('packages');
  const [listKey, setListKey] = useState(0);
  const refresh = () => setListKey((k) => k + 1);
  const [busyId, setBusyId] = useState(0);

  // 发布表单
  const [publishOpen, setPublishOpen] = useState(false);
  const [pType, setPType] = useState('announcement');
  const [pTitle, setPTitle] = useState('');
  const [pPayload, setPPayload] = useState('{\n  "title": "",\n  "content": ""\n}');
  const [pMode, setPMode] = useState(1);
  const [publishing, setPublishing] = useState(false);

  // 成员侧拉取
  const [pullLinkId, setPullLinkId] = useState<number>(0);
  const [pulling, setPulling] = useState(false);

  const publish = async () => {
    let payload: unknown;
    try {
      payload = JSON.parse(pPayload);
    } catch {
      Toast.error(t('内容必须是合法 JSON'));
      return;
    }
    setPublishing(true);
    try {
      await apiPost('/hub/config/publish', {
        packageType: pType,
        title: pTitle,
        payload,
        applyMode: pMode,
      });
      Toast.success(t('配置包已发布'));
      setPublishOpen(false);
      setPTitle('');
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('发布失败') });
    } finally {
      setPublishing(false);
    }
  };

  const revoke = async (record: PackageRow) => {
    setBusyId(record.id);
    try {
      await apiPost('/hub/config/revoke', { id: record.id });
      Toast.success(t('配置包已撤销'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('撤销失败') });
    } finally {
      setBusyId(0);
    }
  };

  const pull = async () => {
    if (!pullLinkId) {
      Toast.warning(t('请先填写连接 ID'));
      return;
    }
    setPulling(true);
    try {
      const res = (await apiPost('/hub/myLinks/pullConfig', { id: pullLinkId })) as {
        fetched?: number;
        applied?: number;
        pending?: number;
        failed?: number;
        rejected?: number;
      };
      Toast.success(
        `${t('拉取完成')}：${t('收到')} ${res.fetched ?? 0} · ${t('已应用')} ${res.applied ?? 0} · ${t('待确认')} ${res.pending ?? 0} · ${t('失败')} ${res.failed ?? 0} · ${t('验签拒收')} ${res.rejected ?? 0}`,
      );
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('拉取失败') });
    } finally {
      setPulling(false);
    }
  };

  const decide = async (record: ApplyRow, agree: boolean) => {
    setBusyId(record.id);
    try {
      await apiPost('/hub/config/decide', { applyId: record.id, agree });
      Toast.success(agree ? t('已确认应用') : t('已拒绝'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('处理失败') });
    } finally {
      setBusyId(0);
    }
  };

  const packageColumns: ColumnConfig<PackageRow>[] = [
    { title: '版本', dataIndex: 'version', width: 70 },
    {
      title: '类型',
      dataIndex: 'packageType',
      width: 110,
      render: (v: string) => <Tag>{t(TYPE_LABEL[v] || v)}</Tag>,
    },
    { title: '标题', dataIndex: 'title', width: 220, render: (v: string) => v || '-' },
    {
      title: '应用方式',
      dataIndex: 'applyMode',
      width: 100,
      render: (v: number) => (v === 2 ? t('自动应用') : t('手动确认')),
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 90,
      render: (v: number) => (v === 1 ? <Tag color="green">{t('生效')}</Tag> : <Tag color="grey">{t('已撤销')}</Tag>),
    },
    {
      title: '回执覆盖',
      dataIndex: 'ackCount',
      width: 130,
      render: (_: any, record: PackageRow) => `${record.appliedCount ?? 0} ${t('已应用')} / ${record.ackCount ?? 0} ${t('已回执')}`,
    },
    { title: '发布时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  const ackColumns: ColumnConfig<AckRow>[] = [
    {
      title: '成员站',
      dataIndex: 'peerName',
      width: 180,
      render: (_: any, record: AckRow) => record.peerName || `连接 #${record.linkId}`,
    },
    { title: '包ID', dataIndex: 'packageId', width: 80 },
    { title: '版本', dataIndex: 'version', width: 70 },
    {
      title: '回执',
      dataIndex: 'ackStatus',
      width: 100,
      render: (v: number) => {
        const s = ACK_STATUS[v ?? -1];
        return s ? <Tag color={s.color}>{t(s.label)}</Tag> : <Tag>{`状态 ${v ?? '-'}`}</Tag>;
      },
    },
    { title: '说明', dataIndex: 'message', width: 320, render: (v: string) => v || '-' },
    { title: '回执时间', dataIndex: 'updateTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  const applyColumns: ColumnConfig<ApplyRow>[] = [
    { title: '版本', dataIndex: 'version', width: 70 },
    {
      title: '类型',
      dataIndex: 'packageType',
      width: 110,
      render: (v: string) => <Tag>{t(TYPE_LABEL[v] || v)}</Tag>,
    },
    { title: '标题', dataIndex: 'title', width: 220, render: (v: string) => v || '-' },
    { title: '来源连接', dataIndex: 'linkId', width: 100 },
    {
      title: '落地状态',
      dataIndex: 'applyStatus',
      width: 100,
      render: (v: number) => {
        const s = APPLY_STATUS[v ?? -1];
        return s ? <Tag color={s.color}>{t(s.label)}</Tag> : <Tag>{`状态 ${v ?? '-'}`}</Tag>;
      },
    },
    { title: '落地说明', dataIndex: 'applyMessage', width: 340, render: (v: string) => v || '-' },
    { title: '拉取时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  return (
    <div>
      <div style={{ marginBottom: 12 }}>
        <RadioGroup type="button" value={tab} onChange={(e) => setTab(e.target.value as typeof tab)}>
          <Radio value="packages">{t('配置包（中枢）')}</Radio>
          <Radio value="acks">{t('回执覆盖')}</Radio>
          <Radio value="applies">{t('本机接收')}</Radio>
        </RadioGroup>
      </div>

      {tab === 'packages' && (
        <ListPage<PackageRow>
          key={`packages-${listKey}`}
          api="/hub/config/packages"
          rowKey="id"
          pageSize={10}
          actionWidth={90}
          filters={[
            {
              field: 'packageType',
              label: '类型',
              type: 'select',
              placeholder: '全部类型',
              width: 150,
              options: Object.entries(TYPE_LABEL).map(([value, label]) => ({ label, value })),
            },
            {
              field: 'status',
              label: '状态',
              type: 'select',
              placeholder: '全部状态',
              width: 140,
              options: [
                { label: '生效', value: 1 },
                { label: '已撤销', value: 0 },
              ],
            },
          ]}
          columns={packageColumns}
          toolbarExtra={
            <Button type="primary" theme="solid" onClick={() => setPublishOpen(true)}>
              {t('发布配置包')}
            </Button>
          }
          rowActions={(record) =>
            record.status === 1 ? (
              <Button theme="borderless" size="small" type="danger" loading={busyId === record.id} onClick={() => void revoke(record)}>
                {t('撤销')}
              </Button>
            ) : null
          }
        />
      )}

      {tab === 'acks' && (
        <ListPage<AckRow>
          key={`acks-${listKey}`}
          api="/hub/config/acks"
          rowKey="id"
          pageSize={10}
          filters={[
            { field: 'packageId', label: '包ID', type: 'number', placeholder: '包 ID', width: 120 },
            { field: 'linkId', label: '连接ID', type: 'number', placeholder: '连接 ID', width: 130 },
          ]}
          columns={ackColumns}
        />
      )}

      {tab === 'applies' && (
        <div>
          <Space style={{ marginBottom: 12 }}>
            <InputNumber placeholder={t('连接 ID')} value={pullLinkId || undefined} onChange={(v) => setPullLinkId(Number(v) || 0)} style={{ width: 140 }} />
            <Button type="primary" theme="solid" loading={pulling} onClick={() => void pull()}>
              {t('拉取配置包')}
            </Button>
          </Space>
          <ListPage<ApplyRow>
            key={`applies-${listKey}`}
            api="/hub/config/applies"
            rowKey="id"
            pageSize={10}
            actionWidth={170}
            filters={[
              { field: 'linkId', label: '连接ID', type: 'number', placeholder: '连接 ID', width: 130 },
              {
                field: 'applyStatus',
                label: '状态',
                type: 'select',
                placeholder: '全部状态',
                width: 140,
                options: [
                  { label: '待处理', value: 0 },
                  { label: '已应用', value: 1 },
                  { label: '已拒绝', value: 2 },
                  { label: '失败', value: 3 },
                ],
              },
            ]}
            columns={applyColumns}
            rowActions={(record) =>
              record.applyStatus === 0 ? (
                <Space spacing={2}>
                  <Button theme="borderless" size="small" type="primary" loading={busyId === record.id} onClick={() => void decide(record, true)}>
                    {t('确认应用')}
                  </Button>
                  <Button theme="borderless" size="small" type="danger" loading={busyId === record.id} onClick={() => void decide(record, false)}>
                    {t('拒绝')}
                  </Button>
                </Space>
              ) : null
            }
          />
        </div>
      )}

      <Modal
        title={t('发布配置包')}
        visible={publishOpen}
        onCancel={() => setPublishOpen(false)}
        footer={
          <Space>
            <Button onClick={() => setPublishOpen(false)}>{t('取消')}</Button>
            <Button type="primary" theme="solid" loading={publishing} onClick={() => void publish()}>
              {t('发布')}
            </Button>
          </Space>
        }
        width={640}
      >
        <div style={{ display: 'grid', gap: 12 }}>
          <Select value={pType} onChange={(v) => setPType(String(v))} style={{ width: '100%' }}>
            {Object.entries(TYPE_LABEL).map(([value, label]) => (
              <Select.Option key={value} value={value}>
                {t(label)}
              </Select.Option>
            ))}
          </Select>
          <Input placeholder={t('标题')} value={pTitle} onChange={setPTitle} />
          <Select value={pMode} onChange={(v) => setPMode(Number(v))} style={{ width: '100%' }}>
            <Select.Option value={1}>{t('成员手动确认后应用')}</Select.Option>
            <Select.Option value={2}>{t('成员拉取后自动应用')}</Select.Option>
          </Select>
          <TextArea placeholder={t('内容 JSON（公告类为 {"title":"…","content":"…"}）')} value={pPayload} onChange={setPPayload} rows={8} />
        </div>
      </Modal>
    </div>
  );
}
