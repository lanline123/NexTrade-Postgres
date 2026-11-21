// 我方新增（站点搬家 P1）：设置 → 商城设置 → 站点搬家。
// 一页两半：上半「我要分享」生成搬家密钥（口令只显示一次、落库只存哈希），
// 下半「我要导入」贴对方口令拉清单、勾选导入；同页含导入记录（可撤销）
// 与导入前快照（一键回滚）。凭据类设置不在分享范围内（后端白名单导出）。
import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Card,
  Checkbox,
  Input,
  InputNumber,
  Modal,
  Popconfirm,
  Select,
  Space,
  Spin,
  Switch,
  Table,
  Tag,
  TextArea,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { unixToDateTime } from '../utils/format';

const { Text, Title, Paragraph } = Typography;

interface PageResult<T> {
  list?: T[];
  records?: T[];
  total?: number;
}

function pageList<T>(res: PageResult<T>): T[] {
  return res.list ?? res.records ?? [];
}

interface CloneKeyRow {
  keyId: number;
  keyCode: string;
  keyName: string;
  domains: string[];
  sourceShopId: number;
  expireTime: number;
  maxCalls: number;
  usedCalls: number;
  status: number;
  lastUsedTime: number;
  createdByName: string;
  addTime: number;
}

interface KeySecretResult {
  keyId: number;
  keyCode: string;
  secret: string;
  token: string;
}

interface CallLogRow {
  logId: number;
  keyCode: string;
  endpoint: string;
  domain: string;
  httpStatus: number;
  resultCount: number;
  clientIp: string;
  durationMs: number;
  addTime: number;
}

interface SettingsGroupInfo {
  key: string;
  label: string;
  risk: boolean;
  fieldCount: number;
}

interface PreviewData {
  sourceLabel: string;
  sourceOrigin: string;
  exporterVersion: number;
  keyDomains: string[];
  sourceShopId: number;
  productTotal: number;
  productAlreadyImported: number;
  categoryTotal: number;
  brandTotal: number;
  shippingTplTotal: number;
  settingsGroups: SettingsGroupInfo[];
  settingsBlocks: string[];
  warnings: string[];
}

interface JobRow {
  jobId: number;
  sourceLabel: string;
  sourceOrigin: string;
  targetShopId: number;
  domains: string;
  status: string;
  snapshotId: number;
  counts: string;
  failures: string;
  operatorName: string;
  undone: number;
  addTime: number;
  finishTime: number;
}

interface SnapshotRow {
  snapshotId: number;
  jobId: number;
  operatorName: string;
  sourceLabel: string;
  groupKeys: string;
  status: number;
  rollbackTime: number;
  addTime: number;
}

interface JobFailure {
  domain: string;
  sourceId: number;
  name: string;
  error: string;
}

const DOMAIN_LABEL: Record<string, string> = { products: '商品', settings: '设置' };
const KEY_STATUS: Record<number, { text: string; color: string }> = {
  1: { text: '启用', color: 'green' },
  0: { text: '停用', color: 'grey' },
  '-1': { text: '已吊销', color: 'red' },
};
const JOB_STATUS: Record<string, { text: string; color: string }> = {
  running: { text: '执行中', color: 'blue' },
  success: { text: '成功', color: 'green' },
  partial: { text: '部分成功', color: 'orange' },
  failed: { text: '失败', color: 'red' },
};

function parseJson<T>(raw: string, fallback: T): T {
  try {
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    Toast.success('已复制');
  } catch {
    Toast.warning('复制失败，请手动选中复制');
  }
}

export default function SiteClone() {
  // —— 分享端状态 ——
  const [keys, setKeys] = useState<CloneKeyRow[]>([]);
  const [keysLoading, setKeysLoading] = useState(false);
  const [logs, setLogs] = useState<CallLogRow[]>([]);
  const [createVisible, setCreateVisible] = useState(false);
  const [editRow, setEditRow] = useState<CloneKeyRow | null>(null);
  const [secretResult, setSecretResult] = useState<KeySecretResult | null>(null);
  const [formName, setFormName] = useState('');
  const [formDomains, setFormDomains] = useState<string[]>(['products', 'settings']);
  const [formShopId, setFormShopId] = useState<number>(0);
  const [formExpireDays, setFormExpireDays] = useState<number>(7);
  const [formMaxCalls, setFormMaxCalls] = useState<number>(0);
  const [formOrigin, setFormOrigin] = useState<string>(
    typeof window !== 'undefined' ? window.location.origin : '',
  );
  const [formStatus, setFormStatus] = useState(true);
  const [saving, setSaving] = useState(false);

  // —— 导入端状态 ——
  const [token, setToken] = useState('');
  const [manualOrigin, setManualOrigin] = useState('');
  const [manualKeyId, setManualKeyId] = useState('');
  const [manualSecret, setManualSecret] = useState('');
  const [targetShopId, setTargetShopId] = useState<number>(0);
  const [preview, setPreview] = useState<PreviewData | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [selectedGroups, setSelectedGroups] = useState<string[]>([]);
  const [includeRisk, setIncludeRisk] = useState(false);
  const [importProducts, setImportProducts] = useState(true);
  const [importSettings, setImportSettings] = useState(true);
  const [keepStatus, setKeepStatus] = useState(false);
  const [forceDuplicate, setForceDuplicate] = useState(false);
  const [executing, setExecuting] = useState(false);
  const [lastJob, setLastJob] = useState<JobRow | null>(null);

  // —— 记录状态 ——
  const [jobs, setJobs] = useState<JobRow[]>([]);
  const [snapshots, setSnapshots] = useState<SnapshotRow[]>([]);
  const [detailJob, setDetailJob] = useState<JobRow | null>(null);

  const loadKeys = useCallback(async () => {
    setKeysLoading(true);
    try {
      const res = await apiGet<PageResult<CloneKeyRow>>('/siteClone/keys/list', { page: 1, pageSize: 50 });
      setKeys(pageList(res));
    } catch (e) {
      Toast.error(e instanceof Error ? e.message : '加载密钥列表失败');
    } finally {
      setKeysLoading(false);
    }
  }, []);

  const loadLogs = useCallback(async () => {
    try {
      const res = await apiGet<PageResult<CallLogRow>>('/siteClone/callLogs/list', { page: 1, pageSize: 20 });
      setLogs(pageList(res));
    } catch {
      // 审计列表失败不阻塞主流程
    }
  }, []);

  const loadJobs = useCallback(async () => {
    try {
      const res = await apiGet<PageResult<JobRow>>('/siteClone/jobs/list', { page: 1, pageSize: 20 });
      setJobs(pageList(res));
    } catch {
      // 记录列表失败不阻塞主流程
    }
  }, []);

  const loadSnapshots = useCallback(async () => {
    try {
      const res = await apiGet<PageResult<SnapshotRow>>('/siteClone/snapshots/list', { page: 1, pageSize: 10 });
      setSnapshots(pageList(res));
    } catch {
      // 快照列表失败不阻塞主流程
    }
  }, []);

  useEffect(() => {
    void loadKeys();
    void loadLogs();
    void loadJobs();
    void loadSnapshots();
  }, [loadKeys, loadLogs, loadJobs, loadSnapshots]);

  const sourcePayload = () => {
    if (token.trim()) return { token: token.trim() };
    return { origin: manualOrigin.trim(), keyId: manualKeyId.trim(), secret: manualSecret };
  };

  const handleCreate = async () => {
    if (!formName.trim()) {
      Toast.warning('请填写密钥名称');
      return;
    }
    if (formDomains.length === 0) {
      Toast.warning('至少勾选一个分享域');
      return;
    }
    setSaving(true);
    try {
      const res = await apiPost<KeySecretResult>('/siteClone/keys/create', {
        keyName: formName.trim(),
        domains: formDomains,
        sourceShopId: formShopId,
        expireDays: formExpireDays,
        maxCalls: formMaxCalls,
        origin: formOrigin.trim(),
      });
      setCreateVisible(false);
      setSecretResult(res);
      void loadKeys();
    } catch (e) {
      Toast.error(e instanceof Error ? e.message : '生成失败');
    } finally {
      setSaving(false);
    }
  };

  const handleUpdate = async () => {
    if (!editRow) return;
    setSaving(true);
    try {
      await apiPost('/siteClone/keys/update', {
        keyId: editRow.keyId,
        keyName: formName.trim() || editRow.keyName,
        domains: formDomains,
        sourceShopId: formShopId,
        status: formStatus ? 1 : 0,
      });
      Toast.success('已保存');
      setEditRow(null);
      void loadKeys();
    } catch (e) {
      Toast.error(e instanceof Error ? e.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const handleRevoke = async (row: CloneKeyRow) => {
    try {
      await apiPost('/siteClone/keys/revoke', { keyId: row.keyId });
      Toast.success('已吊销，该密钥立即失效');
      void loadKeys();
    } catch (e) {
      Toast.error(e instanceof Error ? e.message : '吊销失败');
    }
  };

  const handleReset = async (row: CloneKeyRow) => {
    try {
      const res = await apiPost<KeySecretResult>('/siteClone/keys/resetSecret', {
        keyId: row.keyId,
        origin: formOrigin.trim() || window.location.origin,
      });
      setSecretResult(res);
      void loadKeys();
    } catch (e) {
      Toast.error(e instanceof Error ? e.message : '重置失败');
    }
  };

  const handlePreview = async () => {
    setPreviewLoading(true);
    setLastJob(null);
    try {
      const res = await apiPost<PreviewData>('/siteClone/import/preview', { source: sourcePayload() });
      setPreview(res);
      setSelectedGroups(res.settingsGroups.filter((g) => !g.risk).map((g) => g.key));
      setIncludeRisk(false);
    } catch (e) {
      Toast.error(e instanceof Error ? e.message : '获取分享内容失败');
    } finally {
      setPreviewLoading(false);
    }
  };

  const handleExecute = async () => {
    setExecuting(true);
    try {
      const job = await apiPost<JobRow>('/siteClone/import/execute', {
        source: sourcePayload(),
        targetShopId,
        importProducts,
        importSettings,
        settingsGroups: selectedGroups,
        includeRiskGroups: includeRisk,
        keepProductStatus: keepStatus,
        forceDuplicate,
      });
      setLastJob(job);
      if (job.status === 'success') Toast.success('导入完成');
      else if (job.status === 'partial') Toast.warning('导入部分成功，请看失败明细');
      else Toast.error('导入失败，请看失败明细');
      void loadJobs();
      void loadSnapshots();
      void loadLogs();
    } catch (e) {
      Toast.error(e instanceof Error ? e.message : '导入执行失败');
    } finally {
      setExecuting(false);
    }
  };

  const handleUndo = async (row: JobRow) => {
    try {
      const res = await apiPost<{ deletedProducts: number }>('/siteClone/jobs/undo', { jobId: row.jobId });
      Toast.success(`已撤销：删除新建商品 ${res.deletedProducts} 件`);
      void loadJobs();
    } catch (e) {
      Toast.error(e instanceof Error ? e.message : '撤销失败');
    }
  };

  const handleRollback = async (row: SnapshotRow) => {
    try {
      await apiPost('/siteClone/snapshots/rollback', { snapshotId: row.snapshotId });
      Toast.success('已回滚到导入前设置');
      void loadSnapshots();
    } catch (e) {
      Toast.error(e instanceof Error ? e.message : '回滚失败');
    }
  };

  const openEdit = (row: CloneKeyRow) => {
    setEditRow(row);
    setFormName(row.keyName);
    setFormDomains(row.domains);
    setFormShopId(row.sourceShopId);
    setFormStatus(row.status === 1);
  };

  const openCreate = () => {
    setFormName('');
    setFormDomains(['products', 'settings']);
    setFormShopId(0);
    setFormExpireDays(7);
    setFormMaxCalls(0);
    setFormOrigin(window.location.origin);
    setCreateVisible(true);
  };

  const keyColumns = [
    { title: '密钥 ID', dataIndex: 'keyCode', width: 190 },
    { title: '名称', dataIndex: 'keyName', width: 140 },
    {
      title: '分享域',
      dataIndex: 'domains',
      width: 130,
      render: (domains: string[]) => (
        <Space>
          {domains.map((d) => (
            <Tag key={d} color="blue">{DOMAIN_LABEL[d] ?? d}</Tag>
          ))}
        </Space>
      ),
    },
    {
      title: '源店铺',
      dataIndex: 'sourceShopId',
      width: 90,
      render: (v: number) => (v === 0 ? '平台自营' : `店铺 ${v}`),
    },
    {
      title: '拉取次数',
      width: 110,
      render: (_: unknown, row: CloneKeyRow) => `${row.usedCalls} / ${row.maxCalls === 0 ? '不限' : row.maxCalls}`,
    },
    {
      title: '有效期',
      dataIndex: 'expireTime',
      width: 160,
      render: (v: number) => (v === 0 ? '长期' : unixToDateTime(v)),
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 80,
      render: (v: number) => {
        const s = KEY_STATUS[v] ?? KEY_STATUS[0];
        return <Tag color={s.color as never}>{s.text}</Tag>;
      },
    },
    {
      title: '最近拉取',
      dataIndex: 'lastUsedTime',
      width: 160,
      render: (v: number) => (v === 0 ? '—' : unixToDateTime(v)),
    },
    {
      title: '操作',
      width: 210,
      render: (_: unknown, row: CloneKeyRow) => (
        <Space>
          <Button size="small" onClick={() => openEdit(row)}>编辑</Button>
          <Button size="small" onClick={() => void handleReset(row)}>重置口令</Button>
          {row.status !== -1 && (
            <Popconfirm title="吊销后该密钥立即失效，确定吊销？" onConfirm={() => void handleRevoke(row)}>
              <Button size="small" type="danger">吊销</Button>
            </Popconfirm>
          )}
        </Space>
      ),
    },
  ];

  const logColumns = [
    { title: '时间', dataIndex: 'addTime', width: 160, render: (v: number) => unixToDateTime(v) },
    { title: '密钥', dataIndex: 'keyCode', width: 190 },
    { title: '端点', dataIndex: 'endpoint', width: 210 },
    { title: '域', dataIndex: 'domain', width: 90 },
    { title: '状态', dataIndex: 'httpStatus', width: 70 },
    { title: '条数', dataIndex: 'resultCount', width: 70 },
    { title: '来源 IP', dataIndex: 'clientIp', width: 130 },
    { title: '耗时(ms)', dataIndex: 'durationMs', width: 90 },
  ];

  const jobColumns = [
    { title: '时间', dataIndex: 'addTime', width: 160, render: (v: number) => unixToDateTime(v) },
    { title: '来源站', dataIndex: 'sourceLabel', width: 150, render: (v: string, row: JobRow) => v || row.sourceOrigin },
    {
      title: '承接店铺',
      dataIndex: 'targetShopId',
      width: 90,
      render: (v: number) => (v === 0 ? '平台自营' : `店铺 ${v}`),
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 90,
      render: (v: string, row: JobRow) => {
        const s = JOB_STATUS[v] ?? JOB_STATUS.failed;
        return (
          <Space>
            <Tag color={s.color as never}>{s.text}</Tag>
            {row.undone === 1 && <Tag color="grey">已撤销</Tag>}
          </Space>
        );
      },
    },
    {
      title: '结果',
      width: 260,
      render: (_: unknown, row: JobRow) => {
        const counts = parseJson<Record<string, number>>(row.counts, {});
        const parts = Object.entries(counts).map(([k, v]) => `${k}: ${v}`);
        return <Text ellipsis={{ showTooltip: true }} style={{ maxWidth: 250 }}>{parts.join('，') || '—'}</Text>;
      },
    },
    { title: '操作人', dataIndex: 'operatorName', width: 100 },
    {
      title: '操作',
      width: 170,
      render: (_: unknown, row: JobRow) => (
        <Space>
          <Button size="small" onClick={() => setDetailJob(row)}>明细</Button>
          {row.undone !== 1 && (
            <Popconfirm
              title="撤销将删除本次导入新建的全部商品（分类/设置不回退，设置请用快照回滚），确定？"
              onConfirm={() => void handleUndo(row)}
            >
              <Button size="small" type="danger">撤销导入</Button>
            </Popconfirm>
          )}
        </Space>
      ),
    },
  ];

  const snapshotColumns = [
    { title: '时间', dataIndex: 'addTime', width: 160, render: (v: number) => unixToDateTime(v) },
    { title: '来源站', dataIndex: 'sourceLabel', width: 150 },
    { title: '覆盖设置组', dataIndex: 'groupKeys', width: 260 },
    {
      title: '状态',
      dataIndex: 'status',
      width: 90,
      render: (v: number) => (v === 2 ? <Tag color="grey">已回滚</Tag> : <Tag color="green">有效</Tag>),
    },
    { title: '操作人', dataIndex: 'operatorName', width: 100 },
    {
      title: '操作',
      width: 110,
      render: (_: unknown, row: SnapshotRow) =>
        row.status !== 2 ? (
          <Popconfirm title="回滚将把设置还原到这次导入之前，确定？" onConfirm={() => void handleRollback(row)}>
            <Button size="small">一键回滚</Button>
          </Popconfirm>
        ) : (
          <Text type="tertiary">{row.rollbackTime ? unixToDateTime(row.rollbackTime) : ''}</Text>
        ),
    },
  ];

  const detailFailures: JobFailure[] = detailJob ? parseJson<JobFailure[]>(detailJob.failures, []) : [];
  const lastCounts: Record<string, number> = lastJob ? parseJson<Record<string, number>>(lastJob.counts, {}) : {};
  const lastFailures: JobFailure[] = lastJob ? parseJson<JobFailure[]>(lastJob.failures, []) : [];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      <Card>
        <Title heading={4} style={{ margin: 0 }}>站点搬家</Title>
        <Paragraph type="tertiary" style={{ marginTop: 8, marginBottom: 0 }}>
          把本站的商品与设置分享给另一个 NexTrade 站点（生成搬家口令），或把别站分享的内容一键拉到本站，实现快速开店。
          支付渠道、各类 API 密钥、站点身份与会员/订单/财务数据不在分享范围内，不会被复制；导入设置前会自动留存快照，可一键回滚。
        </Paragraph>
      </Card>

      <Card title="我要分享">
        <Space style={{ marginBottom: 12 }}>
          <Button theme="solid" onClick={openCreate}>生成搬家密钥</Button>
          <Button onClick={() => { void loadKeys(); void loadLogs(); }}>刷新</Button>
        </Space>
        <Table columns={keyColumns} dataSource={keys} loading={keysLoading} pagination={false} rowKey="keyId" />
        <Title heading={6} style={{ marginTop: 20 }}>拉取审计（最近 20 条）</Title>
        <Table columns={logColumns} dataSource={logs} pagination={false} rowKey="logId" size="small" />
      </Card>

      <Card title="我要导入">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10, maxWidth: 760 }}>
          <Text strong>搬家口令</Text>
          <TextArea
            value={token}
            onChange={setToken}
            placeholder="粘贴对方给的搬家口令（MV1. 开头一整段）"
            rows={3}
          />
          <Text type="tertiary">或手动填写（与口令二选一）：</Text>
          <Space>
            <Input value={manualOrigin} onChange={setManualOrigin} placeholder="来源站地址，如 https://example.com" style={{ width: 280 }} />
            <Input value={manualKeyId} onChange={setManualKeyId} placeholder="密钥 ID（mv_ 开头）" style={{ width: 200 }} />
            <Input value={manualSecret} onChange={setManualSecret} placeholder="密钥 Secret" mode="password" style={{ width: 200 }} />
          </Space>
          <Space align="center">
            <Text>承接店铺：</Text>
            <InputNumber value={targetShopId} onChange={(v) => setTargetShopId(Number(v ?? 0))} min={0} style={{ width: 140 }} />
            <Text type="tertiary">0 = 平台自营店</Text>
            <Button theme="solid" loading={previewLoading} onClick={() => void handlePreview()}>获取分享内容</Button>
          </Space>
        </div>

        {preview && (
          <div style={{ marginTop: 16, borderTop: '1px solid var(--semi-color-border)', paddingTop: 16 }}>
            <Space style={{ marginBottom: 8 }}>
              <Text strong>来源站：{preview.sourceLabel || preview.sourceOrigin}</Text>
              <Tag color="blue">导出版本 {preview.exporterVersion}</Tag>
              {preview.keyDomains.map((d) => (
                <Tag key={d} color="green">{DOMAIN_LABEL[d] ?? d}</Tag>
              ))}
            </Space>
            {preview.warnings.map((w) => (
              <Paragraph key={w} type="warning" style={{ margin: '2px 0' }}>⚠ {w}</Paragraph>
            ))}
            <Paragraph style={{ margin: '8px 0' }}>
              商品共 {preview.productTotal} 件（分类 {preview.categoryTotal} / 品牌 {preview.brandTotal} / 运费模板 {preview.shippingTplTotal}），
              其中已导入过 {preview.productAlreadyImported} 件（默认跳过，不会翻倍）。
            </Paragraph>
            <div style={{ display: 'flex', gap: 32, flexWrap: 'wrap' }}>
              <div>
                <Space align="center">
                  <Switch checked={importProducts} onChange={setImportProducts} />
                  <Text strong>导入商品</Text>
                </Space>
                <div style={{ marginTop: 8, display: 'flex', flexDirection: 'column', gap: 6 }}>
                  <Space align="center">
                    <Switch checked={keepStatus} onChange={setKeepStatus} size="small" />
                    <Text>保持源站上下架状态（默认关 = 导入后一律下架）</Text>
                  </Space>
                  <Space align="center">
                    <Switch checked={forceDuplicate} onChange={setForceDuplicate} size="small" />
                    <Text>已导入商品仍再新增一份</Text>
                  </Space>
                </div>
              </div>
              <div>
                <Space align="center">
                  <Switch checked={importSettings} onChange={setImportSettings} />
                  <Text strong>导入设置（按组覆盖，导入前自动快照）</Text>
                </Space>
                <div style={{ marginTop: 8, display: 'flex', flexDirection: 'column', gap: 4, maxHeight: 220, overflowY: 'auto', paddingRight: 8 }}>
                  {preview.settingsGroups.map((g) => (
                    <Checkbox
                      key={g.key}
                      checked={selectedGroups.includes(g.key)}
                      onChange={(e) => {
                        const checked = e.target.checked ?? false;
                        setSelectedGroups((prev) =>
                          checked ? [...prev, g.key] : prev.filter((k) => k !== g.key),
                        );
                        if (g.risk && checked) setIncludeRisk(true);
                      }}
                    >
                      {g.label}（{g.fieldCount} 项）{g.risk ? ' ⚠ 费率类' : ''}
                    </Checkbox>
                  ))}
                  {preview.settingsBlocks.length > 0 && (
                    <Text type="tertiary">整块覆盖：{preview.settingsBlocks.join('、')}</Text>
                  )}
                </div>
              </div>
            </div>
            <Space style={{ marginTop: 14 }}>
              <Button theme="solid" type="primary" loading={executing} onClick={() => void handleExecute()}>
                开始导入
              </Button>
              {executing && (
                <Space>
                  <Spin size="small" />
                  <Text type="tertiary">导入执行中（商品逐条创建，可能需要数分钟），请不要关闭页面</Text>
                </Space>
              )}
            </Space>
            {lastJob && (
              <div style={{ marginTop: 12 }}>
                <Paragraph style={{ margin: 0 }}>
                  导入结果：<Tag color={(JOB_STATUS[lastJob.status]?.color ?? 'grey') as never}>{JOB_STATUS[lastJob.status]?.text ?? lastJob.status}</Tag>{' '}
                  {Object.entries(lastCounts).map(([k, v]) => `${k}: ${v}`).join('，')}
                </Paragraph>
                {lastFailures.length > 0 && (
                  <div style={{ marginTop: 6 }}>
                    {lastFailures.map((f, i) => (
                      <Paragraph key={i} type="danger" style={{ margin: '2px 0' }}>
                        [{f.domain}] {f.name || f.sourceId}：{f.error}
                      </Paragraph>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </Card>

      <Card title="导入记录">
        <Table columns={jobColumns} dataSource={jobs} pagination={false} rowKey="jobId" />
      </Card>

      <Card title="导入前快照（保留最近 10 份）">
        <Table columns={snapshotColumns} dataSource={snapshots} pagination={false} rowKey="snapshotId" />
      </Card>

      <Modal
        title={editRow ? '编辑搬家密钥' : '生成搬家密钥'}
        visible={createVisible || !!editRow}
        onCancel={() => { setCreateVisible(false); setEditRow(null); }}
        onOk={() => void (editRow ? handleUpdate() : handleCreate())}
        confirmLoading={saving}
        okText={editRow ? '保存' : '生成'}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          <Space align="center">
            <Text style={{ width: 90 }}>名称</Text>
            <Input value={formName} onChange={setFormName} placeholder="如：给新站开店用" style={{ width: 300 }} />
          </Space>
          <Space align="center">
            <Text style={{ width: 90 }}>分享域</Text>
            <Checkbox checked={formDomains.includes('products')} onChange={(e) => setFormDomains((p) => (e.target.checked ? [...p, 'products'] : p.filter((d) => d !== 'products')))}>商品</Checkbox>
            <Checkbox checked={formDomains.includes('settings')} onChange={(e) => setFormDomains((p) => (e.target.checked ? [...p, 'settings'] : p.filter((d) => d !== 'settings')))}>设置</Checkbox>
          </Space>
          <Space align="center">
            <Text style={{ width: 90 }}>源店铺 ID</Text>
            <InputNumber value={formShopId} onChange={(v) => setFormShopId(Number(v ?? 0))} min={0} style={{ width: 160 }} />
            <Text type="tertiary">0 = 平台自营</Text>
          </Space>
          {!editRow && (
            <>
              <Space align="center">
                <Text style={{ width: 90 }}>有效期</Text>
                <Select value={formExpireDays} onChange={(v) => setFormExpireDays(Number(v))} style={{ width: 160 }}>
                  <Select.Option value={1}>1 天</Select.Option>
                  <Select.Option value={7}>7 天</Select.Option>
                  <Select.Option value={30}>30 天</Select.Option>
                  <Select.Option value={0}>长期</Select.Option>
                </Select>
              </Space>
              <Space align="center">
                <Text style={{ width: 90 }}>拉取上限</Text>
                <InputNumber value={formMaxCalls} onChange={(v) => setFormMaxCalls(Number(v ?? 0))} min={0} style={{ width: 160 }} />
                <Text type="tertiary">0 = 不限</Text>
              </Space>
              <Space align="center">
                <Text style={{ width: 90 }}>本站地址</Text>
                <Input value={formOrigin} onChange={setFormOrigin} placeholder="https://你的站点域名" style={{ width: 300 }} />
              </Space>
              <Text type="tertiary">本站地址用于拼搬家口令，对方站点将按此地址来拉取。</Text>
            </>
          )}
          {editRow && (
            <Space align="center">
              <Text style={{ width: 90 }}>状态</Text>
              <Switch checked={formStatus} onChange={setFormStatus} />
              <Text>{formStatus ? '启用' : '停用'}</Text>
            </Space>
          )}
        </div>
      </Modal>

      <Modal
        title="搬家口令（只显示这一次）"
        visible={!!secretResult}
        onCancel={() => setSecretResult(null)}
        footer={<Button theme="solid" onClick={() => setSecretResult(null)}>我已保存</Button>}
      >
        {secretResult && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            <Paragraph type="danger" style={{ margin: 0 }}>
              口令等同钥匙，拿到它的人可以拉走本站分享的商品与设置，请通过可信渠道传递、勿公开发布。关闭后将无法再次查看，只能重置。
            </Paragraph>
            <Text strong>密钥 ID</Text>
            <Space>
              <Input value={secretResult.keyCode} readOnly style={{ width: 320 }} />
              <Button onClick={() => void copyText(secretResult.keyCode)}>复制</Button>
            </Space>
            <Text strong>Secret</Text>
            <Space>
              <Input value={secretResult.secret} readOnly style={{ width: 320 }} />
              <Button onClick={() => void copyText(secretResult.secret)}>复制</Button>
            </Space>
            <Text strong>搬家口令（整段发给对方粘贴导入）</Text>
            <TextArea value={secretResult.token} readOnly rows={4} />
            <Button onClick={() => void copyText(secretResult.token)}>复制口令</Button>
          </div>
        )}
      </Modal>

      <Modal
        title="导入明细"
        visible={!!detailJob}
        onCancel={() => setDetailJob(null)}
        footer={<Button onClick={() => setDetailJob(null)}>关闭</Button>}
        width={720}
      >
        {detailJob && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <Text>来源：{detailJob.sourceLabel || detailJob.sourceOrigin}；任务状态：{detailJob.status}；快照 ID：{detailJob.snapshotId || '无'}</Text>
            <Text>计数：{Object.entries(parseJson<Record<string, number>>(detailJob.counts, {})).map(([k, v]) => `${k}: ${v}`).join('，') || '—'}</Text>
            {detailFailures.length === 0 ? (
              <Text type="tertiary">无失败项</Text>
            ) : (
              detailFailures.map((f, i) => (
                <Paragraph key={i} type="danger" style={{ margin: 0 }}>
                  [{f.domain}] {f.name || f.sourceId}：{f.error}
                </Paragraph>
              ))
            )}
          </div>
        )}
      </Modal>
    </div>
  );
}
