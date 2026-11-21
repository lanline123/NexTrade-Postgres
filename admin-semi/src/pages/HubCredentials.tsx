// 我方新增（万象互联 P1）：API 凭证页（万象互联 → API 凭证）。
import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Checkbox,
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
import { HUB_SCOPES } from '../federation';

const { Text } = Typography;

interface CredentialRow {
  credentialId: number;
  connType?: number;
  direction?: number;
  peerBaseUrl?: string;
  credName?: string;
  accessKey?: string;
  secretMasked?: string;
  scopes?: number;
  scopeNames?: string[];
  scopeAllShops?: boolean;
  shopIds?: number[];
  scopeMode?: number;
  includeInquiry?: boolean;
  priceRuleType?: string;
  priceRuleValue?: number | string;
  status?: number;
  expireTime?: number;
  rateLimitPerMin?: number;
  dailyQuota?: number;
  ipWhitelist?: string;
  productCount?: number;
  callCount?: number;
  lastUsedTime?: number;
  addTime?: number;
  updateTime?: number;
}

interface CredentialDetail extends CredentialRow {
  productIds?: number[];
  categoryIds?: number[];
  priceOverrides?: Array<{ productId: number; ruleType: string; ruleValue: number | string }>;
}

interface SecretResult {
  credentialId: number;
  accessKey: string;
  secret: string;
}

interface CallLogRow {
  logId: number;
  endpoint?: string;
  scope?: string;
  direction?: number;
  httpStatus?: number;
  resultCount?: number;
  clientIp?: string;
  durationMs?: number;
  addTime?: number;
}

const CONN_TYPE_LABEL: Record<number, string> = { 1: '单向供货', 2: '单向接入', 3: '全联邦' };
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

interface CredentialForm {
  credName: string;
  connType: number;
  direction: number;
  peerBaseUrl: string;
  peerAccessKey: string;
  peerSecret: string;
  scopes: number;
  scopeAllShops: boolean;
  shopIdsText: string;
  scopeMode: number;
  productIdsText: string;
  categoryIdsText: string;
  includeInquiry: boolean;
  priceRuleType: string;
  priceRuleValue: string;
  status: number;
  expireTimeText: string;
  rateLimitPerMin: string;
  dailyQuota: string;
  ipWhitelist: string;
}

const EMPTY_FORM: CredentialForm = {
  credName: '',
  connType: 1,
  direction: 1,
  peerBaseUrl: '',
  peerAccessKey: '',
  peerSecret: '',
  scopes: 0,
  scopeAllShops: true,
  shopIdsText: '',
  scopeMode: 1,
  productIdsText: '',
  categoryIdsText: '',
  includeInquiry: false,
  priceRuleType: 'none',
  priceRuleValue: '0',
  status: 1,
  expireTimeText: '0',
  rateLimitPerMin: '0',
  dailyQuota: '0',
  ipWhitelist: '',
};

/** API 凭证：入站（我方签发）/出站（对端签发）钥管理 + 单品覆盖 + 调用日志。 */
export default function HubCredentials() {
  const { t } = useI18n();
  const [listKey, setListKey] = useState(0);
  const refresh = () => setListKey((key) => key + 1);

  const [formVisible, setFormVisible] = useState(false);
  const [editing, setEditing] = useState<CredentialRow | null>(null);
  const [form, setForm] = useState<CredentialForm>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [secretResult, setSecretResult] = useState<SecretResult | null>(null);

  const [feedKeyIdText, setFeedKeyIdText] = useState('');
  const [fromFeedLoading, setFromFeedLoading] = useState(false);

  const [detail, setDetail] = useState<CredentialDetail | null>(null);
  const [callLogs, setCallLogs] = useState<CallLogRow[]>([]);
  const [overrideProductId, setOverrideProductId] = useState('');
  const [overrideRuleType, setOverrideRuleType] = useState('none');
  const [overrideRuleValue, setOverrideRuleValue] = useState('0');

  const setField = <K extends keyof CredentialForm>(key: K, value: CredentialForm[K]) =>
    setForm((current) => ({ ...current, [key]: value }));

  const openCreate = () => {
    setEditing(null);
    setForm(EMPTY_FORM);
    setFormVisible(true);
  };

  const openEdit = async (record: CredentialRow) => {
    try {
      const data = await apiGet<CredentialDetail>('/hub/credentials/detail', { id: record.credentialId });
      const row = data || record;
      setEditing(row);
      setForm({
        credName: row.credName || '',
        connType: row.connType || 1,
        direction: row.direction || 1,
        peerBaseUrl: row.peerBaseUrl || '',
        peerAccessKey: row.accessKey || '',
        peerSecret: '',
        scopes: row.scopes || 0,
        scopeAllShops: row.scopeAllShops !== false,
        shopIdsText: (row.shopIds || []).join(','),
        scopeMode: row.scopeMode || 1,
        productIdsText: (row.productIds || []).join(','),
        categoryIdsText: (row.categoryIds || []).join(','),
        includeInquiry: Boolean(row.includeInquiry),
        priceRuleType: row.priceRuleType || 'none',
        priceRuleValue: row.priceRuleValue !== undefined ? String(row.priceRuleValue) : '0',
        status: row.status ?? 1,
        expireTimeText: row.expireTime !== undefined ? String(row.expireTime) : '0',
        rateLimitPerMin: row.rateLimitPerMin !== undefined ? String(row.rateLimitPerMin) : '0',
        dailyQuota: row.dailyQuota !== undefined ? String(row.dailyQuota) : '0',
        ipWhitelist: row.ipWhitelist || '',
      });
      setFormVisible(true);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载凭证详情失败') });
    }
  };

  const submitForm = async () => {
    if (!form.credName.trim()) {
      Toast.warning(t('请填写凭证名称'));
      return;
    }
    setSaving(true);
    try {
      const payload: Record<string, any> = {
        credName: form.credName.trim(),
        connType: form.connType,
        direction: form.direction,
        peerBaseUrl: form.direction === 2 ? form.peerBaseUrl : undefined,
        peerAccessKey: form.direction === 2 ? form.peerAccessKey : undefined,
        peerSecret: form.direction === 2 && form.peerSecret ? form.peerSecret : undefined,
        scopes: form.scopes,
        scopeAllShops: form.scopeAllShops,
        shopIds: form.scopeAllShops ? [] : parseIds(form.shopIdsText),
        scopeMode: form.scopeMode,
        productIds: form.scopeMode === 2 ? parseIds(form.productIdsText) : [],
        categoryIds: form.scopeMode === 3 ? parseIds(form.categoryIdsText) : [],
        includeInquiry: form.includeInquiry,
        priceRuleType: form.priceRuleType,
        priceRuleValue: Number(form.priceRuleValue) || 0,
        status: form.status,
        expireTime: Number(form.expireTimeText) || 0,
        rateLimitPerMin: Number(form.rateLimitPerMin) || 0,
        dailyQuota: Number(form.dailyQuota) || 0,
        ipWhitelist: form.ipWhitelist,
      };
      if (editing) {
        await apiPost('/hub/credentials/update', { ...payload, credentialId: editing.credentialId });
        Toast.success(t('更新成功'));
      } else {
        const result = await apiPost<SecretResult>('/hub/credentials/create', payload);
        Toast.success(t('创建成功'));
        // 入站钥 secret 明文仅此一次，出站钥无 secret 时不弹
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

  const resetSecret = async (record: CredentialRow) => {
    try {
      const result = await apiPost<SecretResult>('/hub/credentials/resetSecret', { id: record.credentialId });
      if (result?.secret) setSecretResult(result);
      Toast.success(t('secret 已重置'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('重置失败') });
    }
  };

  const remove = async (record: CredentialRow) => {
    try {
      await apiPost('/hub/credentials/del', { id: record.credentialId });
      Toast.success(t('删除成功'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
    }
  };

  const fromFeedKey = async () => {
    const feedKeyId = Number(feedKeyIdText);
    if (!feedKeyId) {
      Toast.warning(t('请填写 feed 钥 ID'));
      return;
    }
    setFromFeedLoading(true);
    try {
      const result = await apiPost<SecretResult>('/hub/credentials/fromFeedKey', { feedKeyId });
      if (result?.secret) setSecretResult(result);
      Toast.success(t('已从 feed 钥生成凭证'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('生成失败') });
    } finally {
      setFromFeedLoading(false);
    }
  };

  const openDetail = useCallback(
    async (record: CredentialRow) => {
      try {
        const data = await apiGet<CredentialDetail>('/hub/credentials/detail', { id: record.credentialId });
        setDetail(data || record);
      } catch (error) {
        Toast.error({ content: error instanceof Error ? error.message : t('加载凭证详情失败') });
        return;
      }
      try {
        const logs = await apiGet<{ records: CallLogRow[] }>('/hub/callLogs/list', {
          credentialId: record.credentialId,
          page: 1,
          pageSize: 20,
        });
        setCallLogs(logs?.records || []);
      } catch {
        setCallLogs([]);
      }
    },
    [t],
  );

  useEffect(() => {
    setOverrideProductId('');
    setOverrideRuleType('none');
    setOverrideRuleValue('0');
  }, [detail?.credentialId]);

  const saveOverride = async () => {
    if (!detail) return;
    const productId = Number(overrideProductId);
    if (!productId) {
      Toast.warning(t('请填写商品 ID'));
      return;
    }
    try {
      await apiPost('/hub/credentials/priceOverride/save', {
        credentialId: detail.credentialId,
        productId,
        ruleType: overrideRuleType,
        ruleValue: Number(overrideRuleValue) || 0,
      });
      Toast.success(t('覆盖已保存'));
      const row = { credentialId: detail.credentialId } as CredentialRow;
      await openDetail(row);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存覆盖失败') });
    }
  };

  const delOverride = async (productId: number) => {
    if (!detail) return;
    try {
      await apiPost('/hub/credentials/priceOverride/del', {
        credentialId: detail.credentialId,
        productId,
      });
      Toast.success(t('覆盖已删除'));
      await openDetail({ credentialId: detail.credentialId } as CredentialRow);
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

  const columns: ColumnConfig<CredentialRow>[] = [
    { title: 'ID', dataIndex: 'credentialId', width: 70 },
    { title: '名称', dataIndex: 'credName', width: 170, render: (v: string) => v || '-' },
    {
      title: '方向',
      dataIndex: 'direction',
      width: 130,
      render: (v: number) => (v === 2 ? <Tag color="violet">{t('出站·对端签发')}</Tag> : <Tag color="blue">{t('入站·我方签发')}</Tag>),
    },
    {
      title: '类型',
      dataIndex: 'connType',
      width: 100,
      render: (v: number) => CONN_TYPE_LABEL[v] || '-',
    },
    { title: 'AccessKey', dataIndex: 'accessKey', width: 170, render: (v: string) => v || '-' },
    {
      title: '权限',
      dataIndex: 'scopeNames',
      width: 220,
      render: (v: string[]) => (v && v.length ? v.join(', ') : '-'),
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 90,
      render: (v: number) => (v === 1 ? <Tag color="green">{t('启用')}</Tag> : <Tag color="grey">{t('停用')}</Tag>),
    },
    { title: '可见商品数', dataIndex: 'productCount', width: 110, render: (v: number) => (v === -1 ? '-' : v ?? 0) },
    { title: '调用次数', dataIndex: 'callCount', width: 100, render: (v: number) => v || 0 },
    { title: '最后使用', dataIndex: 'lastUsedTime', width: 170, render: (v: number) => unixToDateTime(v) },
    { title: '创建时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  const labelStyle = { marginBottom: 6 } as const;

  return (
    <div>
      <ListPage<CredentialRow>
        key={listKey}
        api="/hub/credentials/list"
        rowKey="credentialId"
        pageSize={10}
        actionWidth={300}
        filters={[
          { field: 'keyword', label: '凭证', type: 'input', placeholder: '搜索名称/AccessKey', width: 260 },
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
          {
            field: 'connType',
            label: '类型',
            type: 'select',
            placeholder: '全部类型',
            width: 150,
            options: [
              { label: '单向供货', value: 1 },
              { label: '单向接入', value: 2 },
              { label: '全联邦', value: 3 },
            ],
          },
        ]}
        columns={columns}
        toolbarExtra={
          <Space>
            <Input
              value={feedKeyIdText}
              onChange={setFeedKeyIdText}
              placeholder={t('feed 钥 ID')}
              style={{ width: 120 }}
            />
            <Button loading={fromFeedLoading} onClick={() => void fromFeedKey()}>
              {t('从 feed 钥生成')}
            </Button>
            <Button type="primary" theme="solid" onClick={openCreate}>
              {t('新建凭证')}
            </Button>
          </Space>
        }
        rowActions={(record) => (
          <Space spacing={2}>
            <Button theme="borderless" size="small" onClick={() => void openDetail(record)}>
              {t('详情')}
            </Button>
            <Button theme="borderless" size="small" onClick={() => void openEdit(record)}>
              {t('编辑')}
            </Button>
            {record.direction !== 2 && (
              <Popconfirm title={t('重置后旧 secret 立即失效，确定重置？')} onConfirm={() => void resetSecret(record)}>
                <Button theme="borderless" size="small">
                  {t('重置 secret')}
                </Button>
              </Popconfirm>
            )}
            <Popconfirm title={t('确定删除该凭证？')} onConfirm={() => void remove(record)}>
              <Button theme="borderless" type="danger" size="small">
                {t('删除')}
              </Button>
            </Popconfirm>
          </Space>
        )}
      />

      <Modal
        title={editing ? t('编辑凭证') : t('新建凭证')}
        visible={formVisible}
        onCancel={() => setFormVisible(false)}
        onOk={() => void submitForm()}
        okText={t('保存')}
        confirmLoading={saving}
        width={720}
      >
        <div style={{ display: 'grid', gap: 14, maxHeight: '65vh', overflowY: 'auto', paddingRight: 4 }}>
          <div>
            <div style={labelStyle}>{t('凭证名称')}</div>
            <Input value={form.credName} onChange={(v) => setField('credName', v)} placeholder={t('如：华东下游A')} />
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
            <div>
              <div style={labelStyle}>{t('连接类型')}</div>
              <Select
                value={form.connType}
                onChange={(v) => setField('connType', Number(v))}
                style={{ width: '100%' }}
                optionList={[
                  { label: '单向供货', value: 1 },
                  { label: '单向接入', value: 2 },
                  { label: '全联邦', value: 3 },
                ]}
              />
            </div>
            <div>
              <div style={labelStyle}>{t('方向')}</div>
              <Select
                value={form.direction}
                onChange={(v) => setField('direction', Number(v))}
                style={{ width: '100%' }}
                optionList={[
                  { label: '入站（我方签发）', value: 1 },
                  { label: '出站（对端签发）', value: 2 },
                ]}
              />
            </div>
          </div>
          {form.direction === 2 && (
            <div style={{ display: 'grid', gap: 12 }}>
              <div>
                <div style={labelStyle}>{t('对端地址 peerBaseUrl')}</div>
                <Input value={form.peerBaseUrl} onChange={(v) => setField('peerBaseUrl', v)} placeholder="https://peer.example.com" />
              </div>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
                <div>
                  <div style={labelStyle}>{t('对端 AccessKey')}</div>
                  <Input value={form.peerAccessKey} onChange={(v) => setField('peerAccessKey', v)} />
                </div>
                <div>
                  <div style={labelStyle}>{t('对端 Secret（编辑时留空表示不改）')}</div>
                  <Input mode="password" value={form.peerSecret} onChange={(v) => setField('peerSecret', v)} />
                </div>
              </div>
            </div>
          )}
          <div>
            <div style={labelStyle}>{t('权限范围 scopes')}</div>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6 }}>
              {HUB_SCOPES.map((scope) => (
                <Checkbox
                  key={scope.bit}
                  checked={(form.scopes & scope.bit) === scope.bit}
                  onChange={(e) =>
                    setField('scopes', e.target.checked ? form.scopes | scope.bit : form.scopes & ~scope.bit)
                  }
                >
                  {scope.label}
                </Checkbox>
              ))}
            </div>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
            <div>
              <div style={labelStyle}>{t('商品范围')}</div>
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
          <div style={{ display: 'flex', gap: 24, alignItems: 'center' }}>
            <Space align="center">
              <Switch checked={form.scopeAllShops} onChange={(v) => setField('scopeAllShops', v)} />
              <span>{t('全部店铺')}</span>
            </Space>
            <Space align="center">
              <Switch checked={form.includeInquiry} onChange={(v) => setField('includeInquiry', v)} />
              <span>{t('包含询价商品')}</span>
            </Space>
          </div>
          {!form.scopeAllShops && (
            <div>
              <div style={labelStyle}>{t('店铺 ID（逗号分隔）')}</div>
              <Input value={form.shopIdsText} onChange={(v) => setField('shopIdsText', v)} placeholder="1, 2" />
            </div>
          )}
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
              <div style={labelStyle}>{t('每分钟限流（0=不限）')}</div>
              <Input value={form.rateLimitPerMin} onChange={(v) => setField('rateLimitPerMin', v)} />
            </div>
            <div>
              <div style={labelStyle}>{t('每日配额（0=不限）')}</div>
              <Input value={form.dailyQuota} onChange={(v) => setField('dailyQuota', v)} />
            </div>
            <div>
              <div style={labelStyle}>{t('IP 白名单')}</div>
              <Input value={form.ipWhitelist} onChange={(v) => setField('ipWhitelist', v)} placeholder={t('逗号分隔，留空不限')} />
            </div>
          </div>
          {editing && editing.direction !== 2 && editing.secretMasked !== undefined && (
            <Text type="tertiary">{t('出站 secret 脱敏展示')}：{editing.secretMasked || '-'}</Text>
          )}
        </div>
      </Modal>

      <Modal
        title={t('凭证 Secret（仅展示一次）')}
        visible={Boolean(secretResult)}
        onCancel={() => setSecretResult(null)}
        footer={
          <Space>
            <Button onClick={() => setSecretResult(null)}>{t('关闭')}</Button>
            <Button
              type="primary"
              theme="solid"
              onClick={() => secretResult && void copyText(secretResult.secret)}
            >
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
        title={t('凭证详情')}
        visible={Boolean(detail)}
        onCancel={() => setDetail(null)}
        footer={
          <Button onClick={() => setDetail(null)}>{t('关闭')}</Button>
        }
        width={860}
      >
        {detail && (
          <div style={{ display: 'grid', gap: 16, maxHeight: '70vh', overflowY: 'auto', paddingRight: 4 }}>
            <div style={{ display: 'grid', gap: 6 }}>
              <Text>
                <Text strong>{detail.credName || '-'}</Text> · {t('方向')}：
                {detail.direction === 2 ? t('出站·对端签发') : t('入站·我方签发')} · {t('类型')}：
                {CONN_TYPE_LABEL[detail.connType || 0] || '-'}
              </Text>
              <Text type="tertiary">
                AccessKey：{detail.accessKey || '-'}
                {detail.secretMasked ? ` · Secret：${detail.secretMasked}` : ''} · {t('可见商品数')}：
                {detail.productCount === -1 ? '-' : detail.productCount ?? 0} · {t('调用次数')}：{detail.callCount || 0}
              </Text>
              <Text type="tertiary">
                {t('商品 ID')}：{(detail.productIds || []).join(', ') || '-'} · {t('分类 ID')}：
                {(detail.categoryIds || []).join(', ') || '-'}
              </Text>
            </div>

            <div>
              <Text strong>{t('单品价格覆盖')}</Text>
              <Table
                dataSource={detail.priceOverrides || []}
                rowKey="productId"
                pagination={false}
                style={{ marginTop: 8 }}
                columns={[
                  { title: t('商品 ID'), dataIndex: 'productId', width: 120 },
                  { title: t('规则类型'), dataIndex: 'ruleType', width: 180 },
                  { title: t('规则值'), dataIndex: 'ruleValue', width: 140 },
                  {
                    title: t('操作'),
                    width: 100,
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
                <Input
                  value={overrideProductId}
                  onChange={setOverrideProductId}
                  placeholder={t('商品 ID')}
                  style={{ width: 130 }}
                />
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
              <Text strong>{t('调用日志（最近 20 条）')}</Text>
              <Table
                dataSource={callLogs}
                rowKey="logId"
                pagination={false}
                style={{ marginTop: 8 }}
                columns={[
                  { title: t('接口'), dataIndex: 'endpoint', width: 220 },
                  { title: t('权限'), dataIndex: 'scope', width: 130 },
                  { title: 'HTTP', dataIndex: 'httpStatus', width: 70 },
                  { title: t('结果数'), dataIndex: 'resultCount', width: 80 },
                  { title: 'IP', dataIndex: 'clientIp', width: 130 },
                  { title: t('耗时(ms)'), dataIndex: 'durationMs', width: 90 },
                  { title: t('时间'), dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
                ] as any}
              />
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}
