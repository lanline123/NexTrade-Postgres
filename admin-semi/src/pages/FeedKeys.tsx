// 我方新增（平台联邦 P1）：对接 API 管理页（组织 → 商品流通 → 对接 API 管理，走既有 /product/feed/* 接口）。
import { useCallback, useState } from 'react';
import {
  Button,
  Input,
  Modal,
  Popconfirm,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import { unixToDateTime } from '../utils/format';

const { Text } = Typography;

interface FeedKeyRow {
  feedKeyId: number;
  keyName?: string;
  accessKey?: string;
  status?: number;
  scopeMode?: number;
  includeInquiry?: boolean;
  priceRuleType?: string;
  priceRuleValue?: number | string;
  expireTime?: number;
  remark?: string;
  productCount?: number;
  callCount?: number;
  lastUsedTime?: number;
  addTime?: number;
  updateTime?: number;
}

interface FeedKeyDetail extends FeedKeyRow {
  productIds?: number[];
  categoryIds?: number[];
  priceOverrides?: Array<{
    productId: number;
    productName?: string;
    basePrice?: number | string;
    ruleType: string;
    ruleValue: number | string;
    finalPrice?: number | string;
  }>;
}

interface FeedSecretResult {
  feedKeyId: number;
  accessKey: string;
  secret: string;
}

interface ScopeProductRow {
  productId: number;
  productName?: string;
  productSn?: string;
  picUrl?: string;
  categoryId?: number;
  basePrice?: number | string;
  finalPrice?: number | string;
  stock?: number;
  inquiry?: boolean;
  hasOverride?: boolean;
}

const SCOPE_MODE_LABEL: Record<number, string> = { 1: '全部商品', 2: '指定商品', 3: '指定分类' };
const PRICE_RULE_OPTIONS = [
  { label: '原价 none', value: 'none' },
  { label: '加价百分比 markup_percent', value: 'markup_percent' },
  { label: '加固定金额 add_fixed', value: 'add_fixed' },
  { label: '固定售价 fixed_price', value: 'fixed_price' },
];

function parseIds(text: string): number[] {
  return text
    .split(/[,，\s]+/)
    .map((item) => Number(item.trim()))
    .filter((item) => Number.isFinite(item) && item > 0);
}

interface FeedKeyForm {
  keyName: string;
  status: number;
  scopeMode: number;
  productIdsText: string;
  categoryIdsText: string;
  includeInquiry: boolean;
  priceRuleType: string;
  priceRuleValue: string;
  expireTimeText: string;
  remark: string;
}

const EMPTY_FORM: FeedKeyForm = {
  keyName: '',
  status: 1,
  scopeMode: 1,
  productIdsText: '',
  categoryIdsText: '',
  includeInquiry: false,
  priceRuleType: 'none',
  priceRuleValue: '0',
  expireTimeText: '0',
  remark: '',
};

/** 对接 API 管理：feed 钥的列表/新建/编辑/重置/删除 + 可见商品预览 + 单品覆盖。 */
export default function FeedKeys() {
  const { t } = useI18n();
  const [listKey, setListKey] = useState(0);
  const refresh = () => setListKey((key) => key + 1);

  const [formVisible, setFormVisible] = useState(false);
  const [editing, setEditing] = useState<FeedKeyRow | null>(null);
  const [form, setForm] = useState<FeedKeyForm>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [secretResult, setSecretResult] = useState<FeedSecretResult | null>(null);

  const [detail, setDetail] = useState<FeedKeyDetail | null>(null);
  const [scopeProducts, setScopeProducts] = useState<ScopeProductRow[]>([]);
  const [overrideProductId, setOverrideProductId] = useState('');
  const [overrideRuleType, setOverrideRuleType] = useState('none');
  const [overrideRuleValue, setOverrideRuleValue] = useState('0');

  const setField = <K extends keyof FeedKeyForm>(key: K, value: FeedKeyForm[K]) =>
    setForm((current) => ({ ...current, [key]: value }));

  const openCreate = () => {
    setEditing(null);
    setForm(EMPTY_FORM);
    setFormVisible(true);
  };

  const openEdit = async (record: FeedKeyRow) => {
    try {
      const data = await apiGet<FeedKeyDetail>('/product/feed/detail', { id: record.feedKeyId });
      const row = data || record;
      setEditing(row);
      setForm({
        keyName: row.keyName || '',
        status: row.status ?? 1,
        scopeMode: row.scopeMode || 1,
        productIdsText: (row.productIds || []).join(','),
        categoryIdsText: (row.categoryIds || []).join(','),
        includeInquiry: Boolean(row.includeInquiry),
        priceRuleType: row.priceRuleType || 'none',
        priceRuleValue: row.priceRuleValue !== undefined ? String(row.priceRuleValue) : '0',
        expireTimeText: row.expireTime !== undefined ? String(row.expireTime) : '0',
        remark: row.remark || '',
      });
      setFormVisible(true);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载密钥详情失败') });
    }
  };

  const submitForm = async () => {
    if (!form.keyName.trim()) {
      Toast.warning(t('请填写密钥名称'));
      return;
    }
    setSaving(true);
    try {
      const payload: Record<string, any> = {
        keyName: form.keyName.trim(),
        scopeMode: form.scopeMode,
        productIds: form.scopeMode === 2 ? parseIds(form.productIdsText) : [],
        categoryIds: form.scopeMode === 3 ? parseIds(form.categoryIdsText) : [],
        includeInquiry: form.includeInquiry,
        priceRuleType: form.priceRuleType,
        priceRuleValue: Number(form.priceRuleValue) || 0,
        expireTime: Number(form.expireTimeText) || 0,
        remark: form.remark,
      };
      if (editing) {
        await apiPost('/product/feed/update', { ...payload, feedKeyId: editing.feedKeyId, status: form.status });
        Toast.success(t('更新成功'));
      } else {
        const result = await apiPost<FeedSecretResult>('/product/feed/create', payload);
        Toast.success(t('创建成功'));
        if (result?.secret) setSecretResult(result);
      }
      setFormVisible(false);
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  const resetSecret = async (record: FeedKeyRow) => {
    try {
      const result = await apiPost<FeedSecretResult>('/product/feed/resetSecret', { feedKeyId: record.feedKeyId });
      if (result?.secret) setSecretResult(result);
      Toast.success(t('secret 已重置'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('重置失败') });
    }
  };

  const remove = async (record: FeedKeyRow) => {
    try {
      await apiPost('/product/feed/del', { feedKeyId: record.feedKeyId });
      Toast.success(t('删除成功'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
    }
  };

  const openDetail = useCallback(
    async (record: FeedKeyRow) => {
      try {
        const data = await apiGet<FeedKeyDetail>('/product/feed/detail', { id: record.feedKeyId });
        setDetail(data || record);
        setOverrideProductId('');
        setOverrideRuleType('none');
        setOverrideRuleValue('0');
      } catch (error) {
        Toast.error({ content: error instanceof Error ? error.message : t('加载密钥详情失败') });
        return;
      }
      try {
        const products = await apiGet<{ records: ScopeProductRow[] }>('/product/feed/scopeProducts', {
          feedKeyId: record.feedKeyId,
          page: 1,
          pageSize: 20,
        });
        setScopeProducts(products?.records || []);
      } catch {
        setScopeProducts([]);
      }
    },
    [t],
  );

  const saveOverride = async () => {
    if (!detail) return;
    const productId = Number(overrideProductId);
    if (!productId) {
      Toast.warning(t('请填写商品 ID'));
      return;
    }
    try {
      await apiPost('/product/feed/priceOverride/save', {
        feedKeyId: detail.feedKeyId,
        productId,
        ruleType: overrideRuleType,
        ruleValue: Number(overrideRuleValue) || 0,
      });
      Toast.success(t('覆盖已保存'));
      await openDetail({ feedKeyId: detail.feedKeyId } as FeedKeyRow);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存覆盖失败') });
    }
  };

  const delOverride = async (productId: number) => {
    if (!detail) return;
    try {
      await apiPost('/product/feed/priceOverride/del', { feedKeyId: detail.feedKeyId, productId });
      Toast.success(t('覆盖已删除'));
      await openDetail({ feedKeyId: detail.feedKeyId } as FeedKeyRow);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('删除覆盖失败') });
    }
  };

  const copyText = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      Toast.success(t('已复制'));
    } catch {
      Toast.warning(t('复制失败，请手动复制'));
    }
  };

  const columns: ColumnConfig<FeedKeyRow>[] = [
    { title: 'ID', dataIndex: 'feedKeyId', width: 70 },
    { title: '名称', dataIndex: 'keyName', width: 180, render: (v: string) => v || '-' },
    { title: 'AccessKey', dataIndex: 'accessKey', width: 180, render: (v: string) => v || '-' },
    {
      title: '范围',
      dataIndex: 'scopeMode',
      width: 110,
      render: (v: number) => SCOPE_MODE_LABEL[v] || '-',
    },
    {
      title: '价格规则',
      dataIndex: 'priceRuleType',
      width: 170,
      render: (_: any, record: FeedKeyRow) =>
        record.priceRuleType === 'none' || !record.priceRuleType
          ? t('原价')
          : `${record.priceRuleType} / ${record.priceRuleValue ?? 0}`,
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 90,
      render: (v: number) => (v === 1 ? <Tag color="green">{t('启用')}</Tag> : <Tag color="grey">{t('停用')}</Tag>),
    },
    { title: '可见商品数', dataIndex: 'productCount', width: 110, render: (v: number) => v ?? 0 },
    { title: '调用次数', dataIndex: 'callCount', width: 100, render: (v: number) => v || 0 },
    { title: '最后使用', dataIndex: 'lastUsedTime', width: 170, render: (v: number) => unixToDateTime(v) },
    { title: '创建时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  const labelStyle = { marginBottom: 6 } as const;

  return (
    <div>
      <ListPage<FeedKeyRow>
        key={listKey}
        api="/product/feed/list"
        rowKey="feedKeyId"
        pageSize={10}
        actionWidth={300}
        filters={[
          { field: 'keyword', label: '密钥', type: 'input', placeholder: '搜索名称/AccessKey', width: 260 },
          {
            field: 'status',
            label: '状态',
            type: 'select',
            placeholder: '全部状态',
            width: 140,
            options: [
              { label: '启用', value: 1 },
              { label: '停用', value: 0 },
            ],
          },
        ]}
        columns={columns}
        toolbarExtra={
          <Button type="primary" theme="solid" onClick={openCreate}>
            {t('新建密钥')}
          </Button>
        }
        rowActions={(record) => (
          <Space spacing={2}>
            <Button theme="borderless" size="small" onClick={() => void openDetail(record)}>
              {t('详情')}
            </Button>
            <Button theme="borderless" size="small" onClick={() => void openEdit(record)}>
              {t('编辑')}
            </Button>
            <Popconfirm title={t('重置后旧 secret 立即失效，确定重置？')} onConfirm={() => void resetSecret(record)}>
              <Button theme="borderless" size="small">
                {t('重置 secret')}
              </Button>
            </Popconfirm>
            <Popconfirm title={t('确定删除该密钥？')} onConfirm={() => void remove(record)}>
              <Button theme="borderless" type="danger" size="small">
                {t('删除')}
              </Button>
            </Popconfirm>
          </Space>
        )}
      />

      <Modal
        title={editing ? t('编辑密钥') : t('新建密钥')}
        visible={formVisible}
        onCancel={() => setFormVisible(false)}
        onOk={() => void submitForm()}
        okText={t('保存')}
        confirmLoading={saving}
        width={640}
      >
        <div style={{ display: 'grid', gap: 14 }}>
          <div>
            <div style={labelStyle}>{t('密钥名称')}</div>
            <Input value={form.keyName} onChange={(v) => setField('keyName', v)} placeholder={t('如：下游渠道A')} />
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
            <div>
              <div style={labelStyle}>{t('授权范围')}</div>
              <Select
                value={form.scopeMode}
                onChange={(v) => setField('scopeMode', Number(v))}
                style={{ width: '100%' }}
                optionList={[
                  { label: '全部商品', value: 1 },
                  { label: '指定商品', value: 2 },
                  { label: '指定分类', value: 3 },
                ]}
              />
            </div>
            {editing && (
              <div>
                <div style={labelStyle}>{t('状态')}</div>
                <Select
                  value={form.status}
                  onChange={(v) => setField('status', Number(v))}
                  style={{ width: '100%' }}
                  optionList={[
                    { label: '启用', value: 1 },
                    { label: '停用', value: 0 },
                  ]}
                />
              </div>
            )}
          </div>
          {form.scopeMode === 2 && (
            <div>
              <div style={labelStyle}>{t('商品 ID（逗号分隔）')}</div>
              <Input value={form.productIdsText} onChange={(v) => setField('productIdsText', v)} placeholder="101, 102, 103" />
            </div>
          )}
          {form.scopeMode === 3 && (
            <div>
              <div style={labelStyle}>{t('分类 ID（逗号分隔，含全部后代）')}</div>
              <Input value={form.categoryIdsText} onChange={(v) => setField('categoryIdsText', v)} placeholder="11, 12" />
            </div>
          )}
          <Space align="center">
            <Switch checked={form.includeInquiry} onChange={(v) => setField('includeInquiry', v)} />
            <span>{t('包含询价商品')}</span>
          </Space>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
            <div>
              <div style={labelStyle}>{t('默认价格规则')}</div>
              <Select
                value={form.priceRuleType}
                onChange={(v) => setField('priceRuleType', String(v))}
                style={{ width: '100%' }}
                optionList={PRICE_RULE_OPTIONS}
              />
            </div>
            <div>
              <div style={labelStyle}>{t('规则值')}</div>
              <Input value={form.priceRuleValue} onChange={(v) => setField('priceRuleValue', v)} placeholder="0" />
            </div>
            <div>
              <div style={labelStyle}>{t('过期时间（秒级时间戳，0=永不过期）')}</div>
              <Input value={form.expireTimeText} onChange={(v) => setField('expireTimeText', v)} />
            </div>
            <div>
              <div style={labelStyle}>{t('备注')}</div>
              <Input value={form.remark} onChange={(v) => setField('remark', v)} />
            </div>
          </div>
        </div>
      </Modal>

      <Modal
        title={t('密钥 Secret（仅展示一次）')}
        visible={Boolean(secretResult)}
        onCancel={() => setSecretResult(null)}
        footer={
          <Space>
            <Button onClick={() => setSecretResult(null)}>{t('关闭')}</Button>
            <Button type="primary" theme="solid" onClick={() => secretResult && void copyText(secretResult.secret)}>
              {t('复制 Secret')}
            </Button>
          </Space>
        }
        width={560}
      >
        <div style={{ display: 'grid', gap: 10 }}>
          <Text type="warning">{t('secret 明文仅此一次可见，请立即复制并安全保存，关闭后无法再次查看。')}</Text>
          <div>
            <Text strong>AccessKey：</Text>
            <Text style={{ fontFamily: 'monospace' }}>{secretResult?.accessKey}</Text>
          </div>
          <div
            style={{
              fontFamily: 'monospace',
              wordBreak: 'break-all',
              background: 'var(--semi-color-fill-0)',
              padding: 12,
              borderRadius: 6,
            }}
          >
            {secretResult?.secret}
          </div>
        </div>
      </Modal>

      <Modal
        title={t('密钥详情')}
        visible={Boolean(detail)}
        onCancel={() => setDetail(null)}
        footer={<Button onClick={() => setDetail(null)}>{t('关闭')}</Button>}
        width={860}
      >
        {detail && (
          <div style={{ display: 'grid', gap: 16, maxHeight: '70vh', overflowY: 'auto', paddingRight: 4 }}>
            <Text type="tertiary">
              {detail.keyName || '-'} · AccessKey：{detail.accessKey || '-'} · {t('范围')}：
              {SCOPE_MODE_LABEL[detail.scopeMode || 0] || '-'} · {t('商品 ID')}：
              {(detail.productIds || []).join(', ') || '-'} · {t('分类 ID')}：
              {(detail.categoryIds || []).join(', ') || '-'}
            </Text>

            <div>
              <Text strong>{t('单品价格覆盖')}</Text>
              <Table
                dataSource={detail.priceOverrides || []}
                rowKey="productId"
                pagination={false}
                style={{ marginTop: 8 }}
                columns={[
                  { title: t('商品 ID'), dataIndex: 'productId', width: 100 },
                  { title: t('商品名'), dataIndex: 'productName', width: 200 },
                  { title: t('原价'), dataIndex: 'basePrice', width: 110 },
                  { title: t('规则类型'), dataIndex: 'ruleType', width: 160 },
                  { title: t('规则值'), dataIndex: 'ruleValue', width: 110 },
                  { title: t('规则后价'), dataIndex: 'finalPrice', width: 110 },
                  {
                    title: t('操作'),
                    width: 90,
                    render: (_: any, record: any) => (
                      <Popconfirm title={t('确定删除该覆盖？')} onConfirm={() => void delOverride(record.productId)}>
                        <Button theme="borderless" type="danger" size="small">
                          {t('删除')}
                        </Button>
                      </Popconfirm>
                    ),
                  },
                ] as any}
              />
              <Space style={{ marginTop: 10 }} wrap>
                <Input value={overrideProductId} onChange={setOverrideProductId} placeholder={t('商品 ID')} style={{ width: 130 }} />
                <Select
                  value={overrideRuleType}
                  onChange={(v) => setOverrideRuleType(String(v))}
                  optionList={PRICE_RULE_OPTIONS}
                  style={{ width: 220 }}
                />
                <Input value={overrideRuleValue} onChange={setOverrideRuleValue} placeholder={t('规则值')} style={{ width: 120 }} />
                <Button type="primary" onClick={() => void saveOverride()}>
                  {t('保存覆盖')}
                </Button>
              </Space>
            </div>

            <div>
              <Text strong>{t('可见商品预览（最近 20 条）')}</Text>
              <Table
                dataSource={scopeProducts}
                rowKey="productId"
                pagination={false}
                style={{ marginTop: 8 }}
                columns={[
                  { title: t('商品 ID'), dataIndex: 'productId', width: 100 },
                  { title: t('商品名'), dataIndex: 'productName', width: 240 },
                  { title: t('货号'), dataIndex: 'productSn', width: 130 },
                  { title: t('原价'), dataIndex: 'basePrice', width: 110 },
                  { title: t('规则后价'), dataIndex: 'finalPrice', width: 110 },
                  { title: t('库存'), dataIndex: 'stock', width: 80 },
                  {
                    title: t('标记'),
                    width: 140,
                    render: (_: any, record: ScopeProductRow) => (
                      <Space>
                        {record.inquiry ? <Tag color="orange">{t('询价')}</Tag> : null}
                        {record.hasOverride ? <Tag color="blue">{t('有覆盖')}</Tag> : null}
                      </Space>
                    ),
                  },
                ] as any}
              />
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}
