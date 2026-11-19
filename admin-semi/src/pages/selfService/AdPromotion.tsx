/**
 * 商家端「我的推广」（推广中心）：店铺账号自助投流。
 *
 * 后端契约（/adminapi/merchant/adCampaign/*，店铺账号强制本店，前端不传 shopId）：
 * - GET  myList   我的投放（分页，可按 status/position 过滤）
 * - POST create   新建投放 { targetType, productId?, position, startTime?, days }
 *                 计价 = 平台单价快照 × 天数，提交即从店铺余额扣款；
 *                 余额不足后端原样报错（如「店铺余额不足…」），此处原样 Toast 提示且不关弹窗
 * - POST pause / resume { id }（已暂停不退费，恢复续投）
 * 单价/最短天数读前台公开配置 GET /api/common/config/adSettings（平台定价实时生效）。
 */
import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Button,
  Card,
  DatePicker,
  Empty,
  InputNumber,
  Modal,
  Popconfirm,
  Radio,
  Select,
  Space,
  Table,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import type { ColumnProps } from '@douyinfe/semi-ui/lib/es/table';
import { apiGet, apiPost } from '../../api';
import { useI18n } from '../../i18n';
import type { PageResult, ProductRow } from '../../types';
import {
  AD_POSITIONS,
  LIFECYCLE_META,
  fmtMoney,
  fmtPeriod,
  lifecycleOf,
  positionLabel,
  targetLabel,
  type AdCampaign,
  type AdSettings,
} from '../adCampaignShared';

const { Text, Title } = Typography;

/** 后端内置默认（公开配置拉取失败时兜底展示，实际计价以后端为准）。 */
const FALLBACK_SETTINGS: AdSettings = {
  prices: { shop_street: 30, product_search: 20, product_list: 15, recommend_feed: 20 },
  minDays: 3,
  pinLimit: 3,
  promotedPerPage: 2,
  recommendEnabled: true,
  needAudit: true,
};

async function fetchPublicAdSettings(): Promise<AdSettings> {
  try {
    const res = await fetch('/api/common/config/adSettings');
    const json = await res.json();
    if (json && json.code === 0 && json.data && typeof json.data === 'object') {
      return { ...FALLBACK_SETTINGS, ...json.data, prices: { ...FALLBACK_SETTINGS.prices, ...(json.data.prices || {}) } };
    }
  } catch {
    /* 落兜底 */
  }
  return FALLBACK_SETTINGS;
}

export default function AdPromotionManage() {
  const { t } = useI18n();
  const [settings, setSettings] = useState<AdSettings>(FALLBACK_SETTINGS);
  const [rows, setRows] = useState<AdCampaign[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const pageSize = 15;

  // 新建投放表单
  const [createVisible, setCreateVisible] = useState(false);
  const [targetType, setTargetType] = useState<1 | 2>(1);
  const [productId, setProductId] = useState<number | undefined>(undefined);
  const [products, setProducts] = useState<ProductRow[]>([]);
  const [position, setPosition] = useState('shop_street');
  const [days, setDays] = useState(3);
  const [startDate, setStartDate] = useState<Date | undefined>(undefined);
  const [submitting, setSubmitting] = useState(false);

  const load = useCallback(
    async (p: number) => {
      setLoading(true);
      try {
        const result = await apiGet<PageResult<AdCampaign>>('/merchant/adCampaign/myList', { page: p, size: pageSize });
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
    void load(1);
    void fetchPublicAdSettings().then(setSettings);
  }, [load]);

  useEffect(() => {
    if (settings.minDays && days < settings.minDays) setDays(settings.minDays);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings.minDays]);

  const unitPrice = Number(settings.prices[position] ?? 0);
  const totalPrice = useMemo(() => unitPrice * (days || 0), [unitPrice, days]);

  const openCreate = async () => {
    setCreateVisible(true);
    if (!products.length) {
      try {
        const result = await apiGet<PageResult<ProductRow>>('/product/product/list', {
          page: 1,
          pageSize: 100,
          productStatus: 1,
          isDelete: 0,
        });
        setProducts(result.records || []);
      } catch {
        /* 商品拉取失败时仍可投店铺，商品选项为空即可 */
      }
    }
  };

  const submitCreate = async () => {
    if (targetType === 2 && !productId) {
      Toast.warning({ content: t('请选择要推广的商品') });
      return;
    }
    setSubmitting(true);
    try {
      await apiPost('/merchant/adCampaign/create', {
        targetType,
        productId: targetType === 2 ? productId : undefined,
        position,
        days,
        startTime: startDate ? Math.floor(startDate.getTime() / 1000) : undefined,
      });
      Toast.success({ content: t('投放已提交，金额已从店铺余额扣除') });
      setCreateVisible(false);
      setPage(1);
      void load(1);
    } catch (e: any) {
      // 余额不足等后端业务错误原样提示，不吞文案
      Toast.error({ content: e.message });
    } finally {
      setSubmitting(false);
    }
  };

  const act = async (path: string, id: number, okText: string) => {
    try {
      await apiPost(path, { id });
      Toast.success({ content: t(okText) });
      void load(page);
    } catch (e: any) {
      Toast.error({ content: e.message });
    }
  };

  const columns: Array<ColumnProps<AdCampaign>> = [
    { title: 'ID', dataIndex: 'id', width: 70 },
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
          ) : null}
        </Space>
      ),
    },
    { title: t('投放位置'), dataIndex: 'position', render: (v: string) => positionLabel(v) },
    { title: t('投放时段'), dataIndex: 'startTime', render: (_: unknown, r: AdCampaign) => fmtPeriod(r) },
    { title: t('天数'), dataIndex: 'days', width: 60 },
    {
      title: t('金额'),
      dataIndex: 'amount',
      render: (v: string | number, r: AdCampaign) => (
        <Space vertical align="start" spacing={2}>
          <Text>{fmtMoney(v)}</Text>
          <Text type="tertiary" size="small">
            {fmtMoney(r.unitPrice)}/{t('天')}
          </Text>
        </Space>
      ),
    },
    {
      title: t('审核状态'),
      dataIndex: 'auditStatus',
      render: (v: number, r: AdCampaign) =>
        r.isPin === 1 ? (
          <Tag color="violet">{t('平台置顶')}</Tag>
        ) : (
          <Tag color={v === 1 ? 'green' : v === 2 ? 'red' : 'amber'}>
            {v === 1 ? t('已通过') : v === 2 ? t('已驳回') : t('待审核')}
          </Tag>
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
    {
      title: t('审核备注'),
      dataIndex: 'auditRemark',
      render: (v: string) => v || '--',
    },
    {
      title: t('操作'),
      dataIndex: 'op',
      width: 150,
      render: (_: unknown, r: AdCampaign) => {
        const life = lifecycleOf(r);
        return (
          <Space>
            {(life === 'active' || life === 'scheduled') && (
              <Popconfirm
                title={t('暂停后不退费，恢复可继续投放，确定暂停？')}
                onConfirm={() => void act('/merchant/adCampaign/pause', r.id, '已暂停')}
              >
                <Button size="small" theme="borderless">
                  {t('暂停')}
                </Button>
              </Popconfirm>
            )}
            {life === 'paused' && (
              <Button size="small" theme="borderless" onClick={() => void act('/merchant/adCampaign/resume', r.id, '已恢复投放')}>
                {t('恢复')}
              </Button>
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
          {t('我的推广')}
        </Title>
        <Button type="primary" theme="solid" onClick={() => void openCreate()}>
          {t('新建投放')}
        </Button>
      </div>

      <Card>
        <Text type="tertiary">
          {t(
            '投放提交后立即从店铺余额扣款；平台审核通过后按所选时段生效。暂停不退费，可随时恢复续投。推广内容在前台会明确标注「推广」。',
          )}
          {settings.needAudit ? t('当前平台设置为投放需审核。') : t('当前平台设置为投放免审核，付款后直接排期。')}
        </Text>
      </Card>

      <Card>
        <Table
          loading={loading}
          dataSource={rows}
          rowKey={(r?: AdCampaign) => String(r?.id ?? Math.random())}
          empty={<Empty description={t('暂无投放，点击右上角新建')} />}
          pagination={{
            currentPage: page,
            pageSize,
            total,
            onPageChange: (p: number) => {
              setPage(p);
              void load(p);
            },
          }}
          columns={columns}
        />
      </Card>

      <Modal
        title={t('新建投放')}
        visible={createVisible}
        onCancel={() => setCreateVisible(false)}
        onOk={() => void submitCreate()}
        okText={t('提交并付款')}
        cancelText={t('取消')}
        confirmLoading={submitting}
        width={520}
      >
        <Space vertical align="start" spacing={16} style={{ width: '100%' }}>
          <Space align="center" spacing={12}>
            <Text strong>{t('投放对象')}</Text>
            <Radio.Group
              value={targetType}
              onChange={(e: any) => {
                const v = Number(e?.target?.value ?? e) as 1 | 2;
                setTargetType(v);
                if (v === 1) setPosition('shop_street');
              }}
            >
              <Radio value={1}>{t('本店')}</Radio>
              <Radio value={2}>{t('本店商品')}</Radio>
            </Radio.Group>
          </Space>

          {targetType === 2 && (
            <Space align="center" spacing={12}>
              <Text strong>{t('选择商品')}</Text>
              <Select
                style={{ width: 320 }}
                placeholder={t('选择本店在售商品')}
                value={productId}
                onChange={(v) => setProductId(Number(v))}
                optionList={products.map((p) => ({ value: p.productId, label: `${p.productName}（${fmtMoney(p.productPrice)}）` }))}
                filter
              />
            </Space>
          )}

          <Space align="center" spacing={12}>
            <Text strong>{t('投放位置')}</Text>
            <Select
              style={{ width: 220 }}
              value={position}
              onChange={(v) => setPosition(String(v))}
              optionList={(targetType === 1 ? AD_POSITIONS.filter((p) => p.value === 'shop_street') : AD_POSITIONS).map((p) => ({
                value: p.value,
                label: `${t(p.label)} · ${fmtMoney(settings.prices[p.value])}/${t('天')}`,
              }))}
            />
            {targetType === 1 && <Text type="tertiary">{t('店铺投放只能投店铺街')}</Text>}
          </Space>

          <Space align="center" spacing={12}>
            <Text strong>{t('购买天数')}</Text>
            <InputNumber
              min={settings.minDays || 1}
              max={365}
              precision={0}
              style={{ width: 120 }}
              value={days}
              onChange={(v) => setDays(Number(v ?? settings.minDays ?? 1))}
            />
            <Text type="tertiary">
              {t('最短')} {settings.minDays} {t('天')}
            </Text>
          </Space>

          <Space align="center" spacing={12}>
            <Text strong>{t('开始时间')}</Text>
            <DatePicker
              type="dateTime"
              placeholder={t('不选则立即开始')}
              value={startDate}
              onChange={(v: any) => setStartDate(v ? new Date(v as any) : undefined)}
            />
          </Space>

          <Card style={{ width: '100%' }}>
            <Space vertical align="start" spacing={4}>
              <Text>
                {t('单价')}：{fmtMoney(unitPrice)}/{t('天')} × {days} {t('天')}
              </Text>
              <Text strong>
                {t('总价')}：{fmtMoney(totalPrice)}
                <Text type="tertiary">（{t('提交后从店铺余额扣除')}）</Text>
              </Text>
            </Space>
          </Card>
        </Space>
      </Modal>
    </div>
  );
}
