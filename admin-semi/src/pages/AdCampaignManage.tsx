/**
 * 平台端「投流管理」（组织 > 店铺管理）：投放审核 + 置顶管理 + 强制下架。
 *
 * 后端契约（/adminapi/merchant/adCampaign/*，平台账号）：
 * - GET  auditList  全部投放（可按 auditStatus/status/position 过滤）
 * - POST audit      { id, status: 1 通过 / 2 驳回, auditRemark }（仅待审单可审）
 * - POST pin        { shopId, pinSort, days? } 平台自建置顶单（免费、免审、即时生效）
 * - POST takeDown   { id } 强制下架（未开始/已开始的费用处理一期人工）
 */
import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Card,
  Empty,
  Input,
  InputNumber,
  Modal,
  Popconfirm,
  Radio,
  Select,
  Space,
  Table,
  Tabs,
  TabPane,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import type { ColumnProps } from '@douyinfe/semi-ui/lib/es/table';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import type { PageResult } from '../types';
import {
  AUDIT_META,
  LIFECYCLE_META,
  fmtMoney,
  fmtPeriod,
  lifecycleOf,
  positionLabel,
  targetLabel,
  type AdCampaign,
} from './adCampaignShared';

const { Text, Title } = Typography;

interface ShopOption {
  value: number;
  label: string;
}

export default function AdCampaignManage() {
  const { t } = useI18n();
  const [tab, setTab] = useState<'pending' | 'all'>('pending');
  const [rows, setRows] = useState<AdCampaign[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const pageSize = 15;

  // 审核弹窗
  const [auditTarget, setAuditTarget] = useState<AdCampaign | null>(null);
  const [auditStatus, setAuditStatus] = useState<1 | 2>(1);
  const [auditRemark, setAuditRemark] = useState('');
  const [auditing, setAuditing] = useState(false);

  // 新建置顶
  const [pinVisible, setPinVisible] = useState(false);
  const [pinShopId, setPinShopId] = useState<number | undefined>(undefined);
  const [shopOptions, setShopOptions] = useState<ShopOption[]>([]);
  const [pinSort, setPinSort] = useState(1);
  const [pinDays, setPinDays] = useState<number | undefined>(undefined);
  const [pinning, setPinning] = useState(false);

  const load = useCallback(
    async (p: number, which: 'pending' | 'all') => {
      setLoading(true);
      try {
        const result = await apiGet<PageResult<AdCampaign>>('/merchant/adCampaign/auditList', {
          page: p,
          size: pageSize,
          auditStatus: which === 'pending' ? 0 : undefined,
        });
        setRows(result.records || []);
        setTotal(result.total || 0);
      } catch (e: any) {
        Toast.error({ content: e.message });
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  useEffect(() => {
    setPage(1);
    void load(1, tab);
  }, [load, tab]);

  const searchShops = async (keyword: string) => {
    try {
      const result = await apiGet<PageResult<any>>('/merchant/shop/list', {
        keyword: keyword || undefined,
        page: 1,
        size: 20,
        status: 1,
      });
      setShopOptions(
        (result.records || []).map((r: any) => ({ value: Number(r.shopId), label: String(r.shopTitle || r.shopId) })),
      );
    } catch {
      /* 搜索失败保持现有选项 */
    }
  };

  const submitAudit = async () => {
    if (!auditTarget) return;
    setAuditing(true);
    try {
      await apiPost('/merchant/adCampaign/audit', { id: auditTarget.id, status: auditStatus, auditRemark });
      Toast.success({ content: auditStatus === 1 ? t('已通过') : t('已驳回') });
      setAuditTarget(null);
      setAuditRemark('');
      void load(page, tab);
    } catch (e: any) {
      Toast.error({ content: e.message });
    } finally {
      setAuditing(false);
    }
  };

  const takeDown = async (id: number) => {
    try {
      await apiPost('/merchant/adCampaign/takeDown', { id });
      Toast.success({ content: t('已下架') });
      void load(page, tab);
    } catch (e: any) {
      Toast.error({ content: e.message });
    }
  };

  const submitPin = async () => {
    if (!pinShopId) {
      Toast.warning({ content: t('请选择店铺') });
      return;
    }
    setPinning(true);
    try {
      await apiPost('/merchant/adCampaign/pin', { shopId: pinShopId, pinSort, days: pinDays });
      Toast.success({ content: t('置顶已创建') });
      setPinVisible(false);
      setTab('all');
      void load(1, 'all');
    } catch (e: any) {
      Toast.error({ content: e.message });
    } finally {
      setPinning(false);
    }
  };

  const columns: Array<ColumnProps<AdCampaign>> = [
    { title: 'ID', dataIndex: 'id', width: 70 },
    {
      title: t('店铺'),
      dataIndex: 'shopTitle',
      render: (v: string, r: AdCampaign) => v || `#${r.shopId}`,
    },
    {
      title: t('投放对象'),
      dataIndex: 'targetType',
      render: (_: unknown, r: AdCampaign) => (
        <Space vertical align="start" spacing={2}>
          <Text>{targetLabel(r)}</Text>
          {r.targetType === 2 ? (
            <Text type="tertiary" size="small">
              {r.productName || `${t('商品')} #${r.productId}`}
            </Text>
          ) : r.isPin === 1 ? (
            <Text type="tertiary" size="small">
              {t('置顶顺序')} {r.pinSort}
            </Text>
          ) : null}
        </Space>
      ),
    },
    { title: t('投放位置'), dataIndex: 'position', render: (v: string) => positionLabel(v) },
    { title: t('投放时段'), dataIndex: 'startTime', render: (_: unknown, r: AdCampaign) => fmtPeriod(r) },
    { title: t('金额'), dataIndex: 'amount', render: (v: string | number) => fmtMoney(v) },
    {
      title: t('审核状态'),
      dataIndex: 'auditStatus',
      render: (v: number, r: AdCampaign) =>
        r.isPin === 1 ? (
          <Tag color="violet">{t('平台置顶')}</Tag>
        ) : (
          <Tag color={(AUDIT_META[v]?.color || 'grey') as any}>{t(AUDIT_META[v]?.label || String(v))}</Tag>
        ),
    },
    {
      title: t('状态'),
      dataIndex: 'status',
      render: (_: unknown, r: AdCampaign) => {
        const meta = LIFECYCLE_META[lifecycleOf(r)];
        return <Tag color={meta.color as any}>{t(meta.label)}</Tag>;
      },
    },
    { title: t('审核备注'), dataIndex: 'auditRemark', render: (v: string) => v || '--' },
    {
      title: t('操作'),
      dataIndex: 'op',
      width: 170,
      render: (_: unknown, r: AdCampaign) => {
        const life = lifecycleOf(r);
        const canTakeDown = !['ended', 'rejected', 'takenDown'].includes(life);
        return (
          <Space>
            {r.auditStatus === 0 && r.isPin !== 1 && (
              <Button
                size="small"
                theme="borderless"
                onClick={() => {
                  setAuditTarget(r);
                  setAuditStatus(1);
                  setAuditRemark(r.auditRemark || '');
                }}
              >
                {t('审核')}
              </Button>
            )}
            {canTakeDown && (
              <Popconfirm title={t('确定强制下架该投放？费用处理需人工跟进。')} onConfirm={() => void takeDown(r.id)}>
                <Button size="small" theme="borderless" type="danger">
                  {t('下架')}
                </Button>
              </Popconfirm>
            )}
          </Space>
        );
      },
    },
  ];

  return (
    <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 16 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Title heading={4} style={{ margin: 0 }}>
          {t('投流管理')}
        </Title>
        <Button
          type="primary"
          theme="solid"
          onClick={() => {
            setPinVisible(true);
            void searchShops('');
          }}
        >
          {t('新建置顶')}
        </Button>
      </div>

      <Card>
        <Tabs activeKey={tab} onChange={(k) => setTab(k as 'pending' | 'all')}>
          <TabPane tab={t('待审核')} itemKey="pending" />
          <TabPane tab={t('全部投放')} itemKey="all" />
        </Tabs>
        <Table
          loading={loading}
          dataSource={rows}
          rowKey={(r?: AdCampaign) => String(r?.id ?? Math.random())}
          empty={<Empty description={t('暂无投放')} />}
          pagination={{
            currentPage: page,
            pageSize,
            total,
            onPageChange: (p: number) => {
              setPage(p);
              void load(p, tab);
            },
          }}
          columns={columns}
        />
      </Card>

      <Modal
        title={t('投放审核')}
        visible={!!auditTarget}
        onCancel={() => setAuditTarget(null)}
        onOk={() => void submitAudit()}
        okText={t('提交审核')}
        cancelText={t('取消')}
        confirmLoading={auditing}
        width={440}
      >
        <Space vertical align="start" spacing={14} style={{ width: '100%' }}>
          <Text>
            {auditTarget ? `${t('店铺')} #${auditTarget.shopId} · ${positionLabel(auditTarget.position)} · ${fmtMoney(auditTarget.amount)}` : ''}
          </Text>
          <Radio.Group value={auditStatus} onChange={(e: any) => setAuditStatus(Number(e?.target?.value ?? e) as 1 | 2)}>
            <Radio value={1}>{t('通过')}</Radio>
            <Radio value={2}>{t('驳回')}</Radio>
          </Radio.Group>
          <Input
            placeholder={t('审核备注（驳回时请说明原因）')}
            value={auditRemark}
            onChange={setAuditRemark}
            style={{ width: '100%' }}
          />
        </Space>
      </Modal>

      <Modal
        title={t('新建置顶')}
        visible={pinVisible}
        onCancel={() => setPinVisible(false)}
        onOk={() => void submitPin()}
        okText={t('创建置顶')}
        cancelText={t('取消')}
        confirmLoading={pinning}
        width={440}
      >
        <Space vertical align="start" spacing={14} style={{ width: '100%' }}>
          <Space align="center" spacing={12}>
            <Text strong>{t('选择店铺')}</Text>
            <Select
              style={{ width: 260 }}
              placeholder={t('搜索店铺名称')}
              value={pinShopId}
              onChange={(v) => setPinShopId(Number(v))}
              optionList={shopOptions}
              filter
              remote
              onSearch={(v: string) => void searchShops(v)}
            />
          </Space>
          <Space align="center" spacing={12}>
            <Text strong>{t('置顶顺序')}</Text>
            <InputNumber min={0} max={999} precision={0} style={{ width: 120 }} value={pinSort} onChange={(v) => setPinSort(Number(v ?? 0))} />
            <Text type="tertiary">{t('数字越小越靠前')}</Text>
          </Space>
          <Space align="center" spacing={12}>
            <Text strong>{t('置顶天数')}</Text>
            <InputNumber
              min={1}
              max={3650}
              precision={0}
              style={{ width: 120 }}
              value={pinDays}
              onChange={(v) => setPinDays(v === null || v === undefined || v === '' ? undefined : Number(v))}
              placeholder={t('长期')}
            />
            <Text type="tertiary">{t('不填 = 长期（10 年）')}</Text>
          </Space>
        </Space>
      </Modal>
    </div>
  );
}
