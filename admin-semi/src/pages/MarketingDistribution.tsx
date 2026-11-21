import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Card,
  Input,
  Modal,
  Select,
  Space,
  Table,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';

interface SourceActivity {
  sourceKind: number;
  sourceKindText?: string;
  sourceId: number;
  sourceName: string;
  startTime: number;
  endTime: number;
}

interface TargetShop {
  shopId: number;
  shopName: string;
}

interface DistributionItem {
  itemId: number;
  distributionId: number;
  targetShopId: number;
  targetShopName: string;
  copyId: number;
  status: number;
  statusText?: string;
  message?: string;
}

interface DistributionBatch {
  distributionId: number;
  sourceKind: number;
  sourceKindText?: string;
  sourceId: number;
  sourceName: string;
  targetCount: number;
  successCount: number;
  skippedCount: number;
  failedCount: number;
  revokedCount: number;
  status: number;
  statusText?: string;
  remark?: string;
  addTime: number;
}

interface PageResp<T> {
  records: T[];
  total: number;
  current: number;
  pages: number;
}

const BATCH_COLORS: Record<number, 'green' | 'grey' | 'orange'> = {
  1: 'green',
  2: 'grey',
  3: 'orange',
};

const ITEM_COLORS: Record<number, 'grey' | 'green' | 'orange' | 'red'> = {
  0: 'grey',
  1: 'green',
  2: 'orange',
  3: 'grey',
  4: 'red',
};

function formatTime(seconds?: number) {
  if (!seconds) return '-';
  const date = new Date(seconds * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

/** 营销活动下发（我方新增，2026-11-21 论坛帖 177866）：
 * 平台级优惠券 / 满减活动一键下发到目标店铺，逐店生成店铺级活动副本。
 * 副本生成后即为店铺自己的活动（店铺可在本店优惠券 / 满减页改本地参数），
 * 平台改来源不回同步；撤销只停未开始的副本，已开始 / 已被领取的保持不动。 */
export default function MarketingDistribution() {
  const { t } = useI18n();
  const [kind, setKind] = useState<number>(1);
  const [sources, setSources] = useState<SourceActivity[]>([]);
  const [shops, setShops] = useState<TargetShop[]>([]);
  const [sourceId, setSourceId] = useState<number | undefined>(undefined);
  const [targetIds, setTargetIds] = useState<number[]>([]);
  const [remark, setRemark] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const [records, setRecords] = useState<DistributionBatch[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);

  const [detail, setDetail] = useState<{ batch: DistributionBatch; items: DistributionItem[] } | null>(null);
  const [detailVisible, setDetailVisible] = useState(false);

  const loadSources = useCallback(
    async (k: number) => {
      try {
        const data = await apiGet<SourceActivity[]>('/promotion/distribution/sources', { kind: k });
        setSources(data || []);
      } catch (error) {
        Toast.error({ content: error instanceof Error ? error.message : t('加载活动失败') });
      }
    },
    [t],
  );

  const loadShops = useCallback(async () => {
    try {
      const data = await apiGet<TargetShop[]>('/promotion/distribution/shops');
      setShops(data || []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载店铺失败') });
    }
  }, [t]);

  const loadRecords = useCallback(
    async (p: number) => {
      setLoading(true);
      try {
        const data = await apiGet<PageResp<DistributionBatch>>('/promotion/distribution/list', {
          page: p,
          size: 10,
        });
        setRecords(data?.records || []);
        setTotal(data?.total || 0);
      } catch (error) {
        Toast.error({ content: error instanceof Error ? error.message : t('加载下发记录失败') });
      } finally {
        setLoading(false);
      }
    },
    [t],
  );

  useEffect(() => {
    void loadSources(kind);
  }, [kind, loadSources]);

  useEffect(() => {
    void loadShops();
    void loadRecords(1);
  }, [loadShops, loadRecords]);

  const openDetail = async (batchId: number) => {
    try {
      const data = await apiGet<{ batch: DistributionBatch; items: DistributionItem[] }>(
        '/promotion/distribution/detail',
        { id: batchId },
      );
      setDetail(data);
      setDetailVisible(true);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载详情失败') });
    }
  };

  const refreshDetail = async (batchId: number) => {
    const data = await apiGet<{ batch: DistributionBatch; items: DistributionItem[] }>(
      '/promotion/distribution/detail',
      { id: batchId },
    );
    setDetail(data);
  };

  const doDistribute = async () => {
    if (!sourceId) {
      Toast.warning({ content: t('请选择要下发的平台活动') });
      return;
    }
    if (targetIds.length === 0) {
      Toast.warning({ content: t('请选择至少一个目标店铺') });
      return;
    }
    Modal.confirm({
      title: t('确认下发'),
      content: t('将为每个目标店铺生成一份独立的店铺级活动副本，副本生成后由店铺自行管理。'),
      onOk: async () => {
        setSubmitting(true);
        try {
          const result = await apiPost<{ distributionId: number; items: DistributionItem[] }>(
            '/promotion/distribution/distribute',
            { sourceKind: kind, sourceId, targetShopIds: targetIds, remark },
          );
          const created = (result.items || []).filter((i) => i.status === 1).length;
          const skipped = (result.items || []).filter((i) => i.status === 2).length;
          Toast.success({
            content: `${t('下发完成')}：${t('成功')} ${created} / ${t('跳过')} ${skipped}`,
          });
          setTargetIds([]);
          setRemark('');
          setPage(1);
          await loadRecords(1);
          await openDetail(result.distributionId);
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('下发失败') });
        } finally {
          setSubmitting(false);
        }
      },
    });
  };

  const doRevokeBatch = (batch: DistributionBatch) => {
    Modal.confirm({
      title: t('确认整批撤销'),
      content: t('仅会停用尚未开始的副本；已开始或已被领取的副本保持不动，原因会记在明细里。'),
      onOk: async () => {
        try {
          await apiPost('/promotion/distribution/revoke', { distributionId: batch.distributionId });
          Toast.success({ content: t('操作成功') });
          await loadRecords(page);
          if (detailVisible && detail?.batch.distributionId === batch.distributionId) {
            await refreshDetail(batch.distributionId);
          }
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('撤销失败') });
        }
      },
    });
  };

  const doRevokeItem = (item: DistributionItem) => {
    Modal.confirm({
      title: t('确认撤销该店铺的副本'),
      content: t('仅未开始的副本可撤销；已开始或已被领取的将保持不动。'),
      onOk: async () => {
        try {
          await apiPost('/promotion/distribution/revokeItem', { itemId: item.itemId });
          Toast.success({ content: t('操作成功') });
          await loadRecords(page);
          if (detail) await refreshDetail(detail.batch.distributionId);
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('撤销失败') });
        }
      },
    });
  };

  const columns = [
    { title: t('批次'), dataIndex: 'distributionId', width: 80 },
    {
      title: t('来源活动'),
      dataIndex: 'sourceName',
      width: 260,
      render: (_: unknown, row: DistributionBatch) => (
        <span>{`[${row.sourceKindText || ''}] ${row.sourceName}（#${row.sourceId}）`}</span>
      ),
    },
    {
      title: t('下发结果'),
      dataIndex: 'targetCount',
      width: 220,
      render: (_: unknown, row: DistributionBatch) => (
        <span>
          {`${t('目标')} ${row.targetCount} · ${t('成功')} ${row.successCount} · ${t('跳过')} ${row.skippedCount} · ${t('失败')} ${row.failedCount} · ${t('已撤销')} ${row.revokedCount}`}
        </span>
      ),
    },
    {
      title: t('状态'),
      dataIndex: 'status',
      width: 100,
      render: (value: number, row: DistributionBatch) => (
        <Tag color={BATCH_COLORS[value] || 'grey'}>{row.statusText || '-'}</Tag>
      ),
    },
    {
      title: t('备注'),
      dataIndex: 'remark',
      width: 160,
      render: (value: string) => <span>{value || '-'}</span>,
    },
    {
      title: t('下发时间'),
      dataIndex: 'addTime',
      width: 150,
      render: (value: number) => <span>{formatTime(value)}</span>,
    },
    {
      title: t('操作'),
      dataIndex: 'actions',
      width: 170,
      render: (_: unknown, row: DistributionBatch) => (
        <Space spacing={2}>
          <Button theme="borderless" size="small" onClick={() => void openDetail(row.distributionId)}>
            {t('明细')}
          </Button>
          {row.status !== 2 && (
            <Button theme="borderless" size="small" type="danger" onClick={() => doRevokeBatch(row)}>
              {t('整批撤销')}
            </Button>
          )}
        </Space>
      ),
    },
  ];

  const itemColumns = [
    {
      title: t('目标店铺'),
      dataIndex: 'targetShopName',
      width: 200,
      render: (value: string, row: DistributionItem) => (
        <span>{`${value || '-'}（#${row.targetShopId}）`}</span>
      ),
    },
    {
      title: t('副本ID'),
      dataIndex: 'copyId',
      width: 100,
      render: (value: number) => <span>{value || '-'}</span>,
    },
    {
      title: t('状态'),
      dataIndex: 'status',
      width: 90,
      render: (value: number, row: DistributionItem) => (
        <Tag color={ITEM_COLORS[value] || 'grey'}>{row.statusText || '-'}</Tag>
      ),
    },
    {
      title: t('说明'),
      dataIndex: 'message',
      width: 260,
      render: (value: string) => <span>{value || '-'}</span>,
    },
    {
      title: t('操作'),
      dataIndex: 'actions',
      width: 90,
      render: (_: unknown, row: DistributionItem) =>
        row.status === 1 ? (
          <Button theme="borderless" size="small" type="danger" onClick={() => doRevokeItem(row)}>
            {t('撤销')}
          </Button>
        ) : (
          <span>-</span>
        ),
    },
  ];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      <Card title={t('发起下发')}>
        <Space vertical align="start" spacing={12} style={{ width: '100%' }}>
          <Typography.Text type="tertiary">
            {t('选择平台级优惠券或满减活动与目标店铺，一键生成店铺级活动副本。副本生成后归店铺管理，平台改来源不会同步到副本；同一活动对同一店铺已有有效副本时自动跳过。')}
          </Typography.Text>
          <Space wrap spacing={12}>
            <Select
              value={kind}
              style={{ width: 140 }}
              onChange={(v) => {
                setKind(v as number);
                setSourceId(undefined);
              }}
            >
              <Select.Option value={1}>{t('优惠券')}</Select.Option>
              <Select.Option value={2}>{t('满减')}</Select.Option>
            </Select>
            <Select
              value={sourceId}
              placeholder={t('选择平台活动')}
              style={{ width: 320 }}
              filter
              onChange={(v) => setSourceId(v as number)}
            >
              {sources.map((s) => (
                <Select.Option key={`${s.sourceKind}-${s.sourceId}`} value={s.sourceId}>
                  {`${s.sourceName}（#${s.sourceId}）`}
                </Select.Option>
              ))}
            </Select>
            <Select
              multiple
              value={targetIds}
              placeholder={t('选择目标店铺（可多选）')}
              style={{ width: 380 }}
              filter
              maxTagCount={3}
              onChange={(v) => setTargetIds((v as number[]) || [])}
            >
              {shops.map((s) => (
                <Select.Option key={s.shopId} value={s.shopId}>
                  {`${s.shopName || t('未命名店铺')}（#${s.shopId}）`}
                </Select.Option>
              ))}
            </Select>
            <Input
              value={remark}
              placeholder={t('备注（可选）')}
              style={{ width: 200 }}
              onChange={(v) => setRemark(v)}
            />
            <Button theme="solid" loading={submitting} onClick={() => void doDistribute()}>
              {t('一键下发')}
            </Button>
          </Space>
        </Space>
      </Card>

      <Card title={t('下发记录')}>
        <Table
          columns={columns}
          dataSource={records}
          rowKey="distributionId"
          loading={loading}
          pagination={{
            currentPage: page,
            pageSize: 10,
            total,
            onPageChange: (p: number) => {
              setPage(p);
              void loadRecords(p);
            },
          }}
        />
      </Card>

      <Modal
        visible={detailVisible}
        title={detail ? `${t('下发明细')} #${detail.batch.distributionId}` : t('下发明细')}
        width={860}
        footer={null}
        onCancel={() => setDetailVisible(false)}
      >
        {detail && (
          <Space vertical align="start" spacing={12} style={{ width: '100%' }}>
            <Typography.Text>
              {`[${detail.batch.sourceKindText || ''}] ${detail.batch.sourceName} · ${detail.batch.statusText || ''}`}
            </Typography.Text>
            <Table
              columns={itemColumns}
              dataSource={detail.items}
              rowKey="itemId"
              pagination={false}
            />
          </Space>
        )}
      </Modal>
    </div>
  );
}
