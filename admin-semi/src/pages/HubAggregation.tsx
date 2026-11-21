// 我方新增（万象互联 P1）：联邦商品聚合页（万象互联 → 联邦商品聚合）。
import { useState } from 'react';
import { Button, Input, Modal, Select, Space, Tag, Toast } from '@douyinfe/semi-ui';
import { apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import { unixToDateTime } from '../utils/format';

interface AggregationRow {
  mapId: number;
  linkId?: number;
  credentialId?: number;
  remoteInstanceUuid?: string;
  remoteShopId?: number;
  remoteShopName?: string;
  remoteProductId?: number;
  productName?: string;
  picUrl?: string;
  price?: number | string;
  marketPrice?: number | string;
  stock?: number;
  contentHash?: string;
  syncState?: number;
  lastSyncTime?: number;
}

const SYNC_STATE: Record<number, { label: string; color: string }> = {
  1: { label: '正常', color: 'green' },
  2: { label: '源下架', color: 'orange' },
  3: { label: '源不可见', color: 'grey' },
  4: { label: '已断开', color: 'red' },
};

/** 联邦商品聚合：远端商品在本实例的映射列表 + 按连接同步 + 同步状态调整。 */
export default function HubAggregation() {
  const { t } = useI18n();
  const [listKey, setListKey] = useState(0);
  const refresh = () => setListKey((key) => key + 1);
  const [syncLinkIdText, setSyncLinkIdText] = useState('');
  const [syncCredentialIdText, setSyncCredentialIdText] = useState('');
  const [syncing, setSyncing] = useState(false);

  const [stateRow, setStateRow] = useState<AggregationRow | null>(null);
  const [stateValue, setStateValue] = useState(1);
  const [stateSaving, setStateSaving] = useState(false);

  const sync = async () => {
    const linkId = Number(syncLinkIdText) || 0;
    const credentialId = Number(syncCredentialIdText) || 0;
    if (!linkId && !credentialId) {
      Toast.warning(t('请填写连接 ID 或凭证 ID'));
      return;
    }
    setSyncing(true);
    try {
      const result = await apiPost<Record<string, any>>(
        '/hub/aggregation/sync',
        linkId ? { linkId } : { credentialId },
      );
      Toast.success(
        `${t('同步完成')}（${result?.remoteInstanceName || '-'}）：${t('拉取')} ${result?.fetched ?? 0} / ${t('新建')} ${result?.created ?? 0} / ${t('更新')} ${result?.updated ?? 0} / ${t('无变化')} ${result?.unchanged ?? 0}`,
      );
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('同步失败') });
    } finally {
      setSyncing(false);
    }
  };

  const submitState = async () => {
    if (!stateRow) return;
    setStateSaving(true);
    try {
      await apiPost('/hub/aggregation/setState', { mapId: stateRow.mapId, syncState: stateValue });
      Toast.success(t('状态已更新'));
      setStateRow(null);
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('更新失败') });
    } finally {
      setStateSaving(false);
    }
  };

  const columns: ColumnConfig<AggregationRow>[] = [
    { title: '映射ID', dataIndex: 'mapId', width: 90 },
    {
      title: '商品',
      dataIndex: 'productName',
      width: 260,
      render: (_: any, record: AggregationRow) => (
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {record.picUrl ? (
            <img src={record.picUrl} alt="" style={{ width: 36, height: 36, objectFit: 'cover', borderRadius: 4 }} />
          ) : null}
          <span>{record.productName || '-'}</span>
        </div>
      ),
    },
    { title: '来源实例', dataIndex: 'remoteInstanceUuid', width: 200, render: (v: string) => v || '-' },
    {
      title: '来源店铺',
      dataIndex: 'remoteShopName',
      width: 160,
      render: (_: any, record: AggregationRow) => record.remoteShopName || (record.remoteShopId ? `#${record.remoteShopId}` : '-'),
    },
    { title: '远端商品ID', dataIndex: 'remoteProductId', width: 110, render: (v: number) => v || '-' },
    { title: '连接ID', dataIndex: 'linkId', width: 90, render: (v: number) => v || '-' },
    { title: '价格', dataIndex: 'price', width: 110, render: (v: any) => (v === undefined || v === null ? '-' : String(v)) },
    { title: '市场价', dataIndex: 'marketPrice', width: 110, render: (v: any) => (v === undefined || v === null ? '-' : String(v)) },
    { title: '库存', dataIndex: 'stock', width: 80, render: (v: number) => v ?? 0 },
    {
      title: '同步状态',
      dataIndex: 'syncState',
      width: 110,
      render: (v: number) => {
        const item = SYNC_STATE[v || 0] || { label: '未知', color: 'grey' };
        return <Tag color={item.color as any}>{t(item.label)}</Tag>;
      },
    },
    { title: '最后同步', dataIndex: 'lastSyncTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  return (
    <div>
      <ListPage<AggregationRow>
        key={listKey}
        api="/hub/aggregation/list"
        rowKey="mapId"
        pageSize={10}
        actionWidth={120}
        filters={[
          { field: 'keyword', label: '商品', type: 'input', placeholder: '搜索商品名', width: 240 },
          { field: 'linkId', label: '连接ID', type: 'number', placeholder: '连接 ID', width: 130 },
          {
            field: 'syncState',
            label: '同步状态',
            type: 'select',
            placeholder: '全部状态',
            width: 150,
            options: [
              { label: '正常', value: 1 },
              { label: '源下架', value: 2 },
              { label: '源不可见', value: 3 },
              { label: '已断开', value: 4 },
            ],
          },
        ]}
        columns={columns}
        toolbarExtra={
          <Space>
            <Input value={syncLinkIdText} onChange={setSyncLinkIdText} placeholder={t('连接 ID')} style={{ width: 110 }} />
            <Input
              value={syncCredentialIdText}
              onChange={setSyncCredentialIdText}
              placeholder={t('或凭证 ID')}
              style={{ width: 110 }}
            />
            <Button type="primary" theme="solid" loading={syncing} onClick={() => void sync()}>
              {t('同步商品')}
            </Button>
          </Space>
        }
        rowActions={(record) => (
          <Button
            theme="borderless"
            size="small"
            onClick={() => {
              setStateRow(record);
              setStateValue(record.syncState || 1);
            }}
          >
            {t('调整状态')}
          </Button>
        )}
      />

      <Modal
        title={t('调整同步状态')}
        visible={Boolean(stateRow)}
        onCancel={() => setStateRow(null)}
        onOk={() => void submitState()}
        okText={t('保存')}
        confirmLoading={stateSaving}
        width={420}
      >
        <div style={{ marginBottom: 6 }}>{stateRow?.productName || ''}</div>
        <Select
          value={stateValue}
          onChange={(v) => setStateValue(Number(v))}
          style={{ width: '100%' }}
          optionList={[
            { label: '正常', value: 1 },
            { label: '源下架', value: 2 },
            { label: '源不可见', value: 3 },
            { label: '已断开', value: 4 },
          ]}
        />
      </Modal>
    </div>
  );
}
