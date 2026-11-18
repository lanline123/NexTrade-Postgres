import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  Button, Card, Empty, Input, InputNumber, Modal, Radio, RadioGroup, Select, Space,
  Spin, Switch, Table, Tabs, TabPane, Tag, TextArea, Toast, Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';

const { Text, Title } = Typography;

/**
 * 商品复刻（商品 → 批量操作 → 商品复刻）。
 *
 * 取数只走自建抓取边车（通道 A 数据接口已于 2026-11-18 整体移除）。
 * 流程：粘贴链接 → 后台解析为统一草稿 → 预览修正 → 图片转存 → 落草稿/上架。
 * 支持矩阵与边车健康状态由后端 GET /product/productClone/channels 下发，前端不写死。
 */

interface MatrixRow {
  platform: string;
  label: string;
  selfhost: 'full' | 'partial' | 'none';
}

interface ChannelsInfo {
  matrix: MatrixRow[];
  selfhost: {
    enabled: boolean;
    configured: boolean;
    health: { alive: boolean; platforms: Record<string, { cookie: string }> };
  };
}

interface CloneSpec { name: string; value: string }
interface CloneSku {
  specs: CloneSpec[];
  price: string;
  originalPrice?: string | null;
  stock?: number | null;
  image?: string | null;
}
interface CloneDraft {
  sourcePlatform: string;
  sourceChannel: string;
  sourceUrl: string;
  sourceItemId: string;
  title: string;
  mainImages: string[];
  detailHtml: string;
  price: string;
  originalPrice?: string | null;
  skus: CloneSku[];
  props: Array<{ name: string; value: string }>;
  stock?: number | null;
  shopName?: string | null;
  completeness: { missing: string[] };
}

interface CloneTask {
  id: number;
  platform: string;
  channel: string;
  sourceUrl: string;
  sourceItemId: string;
  status: 'parsing' | 'ready' | 'failed' | 'imported';
  title: string;
  draft?: CloneDraft | null;
  attempts: Array<{ channel: string; result: string; errorMsg?: string | null; costFen: number }>;
  costFen: number;
  productId?: number | null;
  errorMsg: string;
  createdBy: string;
  createTime: number;
}

interface PageResult<T> { list: T[]; total: number }

const PLATFORM_LABEL: Record<string, string> = {
  taobao: '淘宝', tmall: '天猫', jd: '京东', pdd: '拼多多',
  '1688': '1688', amazon: '亚马逊', meituan: '美团',
};

const STATUS_TAG: Record<string, { color: 'blue' | 'green' | 'red' | 'grey'; label: string }> = {
  parsing: { color: 'blue', label: '解析中' },
  ready: { color: 'green', label: '待导入' },
  failed: { color: 'red', label: '失败' },
  imported: { color: 'grey', label: '已导入' },
};

function detectPlatform(url: string): string | null {
  const u = url.toLowerCase();
  if (u.includes('tmall.com')) return 'tmall';
  if (u.includes('taobao.com')) return 'taobao';
  if (u.includes('jd.com')) return 'jd';
  if (u.includes('yangkeduo.com') || u.includes('pinduoduo.com')) return 'pdd';
  if (u.includes('1688.com')) return '1688';
  if (u.includes('amazon.')) return 'amazon';
  if (u.includes('meituan.com')) return 'meituan';
  return null;
}

function fmtTime(epochSec: number): string {
  if (!epochSec) return '-';
  return new Date(epochSec * 1000).toLocaleString('zh-CN', { hour12: false });
}

export default function ProductClone() {
  const [channels, setChannels] = useState<ChannelsInfo | null>(null);
  const [urlsText, setUrlsText] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [tasks, setTasks] = useState<CloneTask[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [statusFilter, setStatusFilter] = useState<string>('');
  const [platformFilter, setPlatformFilter] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [editTask, setEditTask] = useState<CloneTask | null>(null);
  const pollRef = useRef<number | null>(null);

  const loadChannels = useCallback(async () => {
    try {
      const info = await apiGet<ChannelsInfo>('/product/productClone/channels');
      setChannels(info);
    } catch {
      /* 接口未部署时页面仍可展示空态 */
    }
  }, []);

  const loadTasks = useCallback(async () => {
    setLoading(true);
    try {
      const res = await apiGet<PageResult<CloneTask>>('/product/productClone/tasks', {
        page, pageSize: 20,
        ...(statusFilter ? { status: statusFilter } : {}),
        ...(platformFilter ? { platform: platformFilter } : {}),
      });
      setTasks(res.list || []);
      setTotal(res.total || 0);
    } catch (e) {
      Toast.error(`加载复刻记录失败：${(e as Error).message}`);
    } finally {
      setLoading(false);
    }
  }, [page, statusFilter, platformFilter]);

  useEffect(() => { loadChannels(); }, [loadChannels]);
  useEffect(() => { loadTasks(); }, [loadTasks]);

  // 有解析中任务时轮询
  useEffect(() => {
    if (pollRef.current) window.clearInterval(pollRef.current);
    if (tasks.some((t) => t.status === 'parsing')) {
      pollRef.current = window.setInterval(loadTasks, 3000);
    }
    return () => { if (pollRef.current) window.clearInterval(pollRef.current); };
  }, [tasks, loadTasks]);

  const urlLines = useMemo(
    () => urlsText.split('\n').map((x) => x.trim()).filter(Boolean),
    [urlsText],
  );

  const matrixByPlatform = useMemo(() => {
    const m = new Map<string, MatrixRow>();
    (channels?.matrix || []).forEach((r) => m.set(r.platform, r));
    return m;
  }, [channels]);

  /** 待提交链接里边车不支持的条目（提交前标红，不建任务）。 */
  const unsupportedLines = useMemo(() => {
    const bad: Array<{ url: string; reason: string }> = [];
    for (const url of urlLines) {
      const p = detectPlatform(url);
      if (!p) { bad.push({ url, reason: '无法识别平台' }); continue; }
      const row = matrixByPlatform.get(p);
      if (row && row.selfhost === 'none') {
        bad.push({ url, reason: `${row.label} 暂不支持复刻（边车一期仅淘宝/天猫）` });
      }
    }
    return bad;
  }, [urlLines, matrixByPlatform]);

  const submitFetch = async () => {
    if (urlLines.length === 0) { Toast.warning('请先粘贴商品链接，一行一条'); return; }
    if (urlLines.length > 20) { Toast.warning('一次最多 20 条链接'); return; }
    setSubmitting(true);
    try {
      const results = await apiPost<Array<{ taskId?: number; url?: string; error?: string; duplicate?: { taskId: number; status: string; productId?: number | null } }>>(
        '/product/productClone/fetch',
        { urls: urlLines },
      );
      const errors = results.filter((r) => r.error);
      const dup = results.filter((r) => r.duplicate);
      if (errors.length) Toast.warning(`${errors.length} 条链接无法识别，已跳过`);
      if (dup.length) Toast.info(`${dup.length} 条链接此前已复刻过（记录中可查），本次仍会重新解析`);
      if (!errors.length && !dup.length) Toast.success('已开始解析，任务在下方列表中');
      setUrlsText('');
      setPage(1);
      await loadTasks();
    } catch (e) {
      Toast.error(`提交失败：${(e as Error).message}`);
    } finally {
      setSubmitting(false);
    }
  };

  const retry = async (task: CloneTask) => {
    try {
      await apiPost('/product/productClone/retry', { taskId: task.id });
      Toast.success('已重新解析');
      await loadTasks();
    } catch (e) {
      Toast.error(`重试失败：${(e as Error).message}`);
    }
  };

  const openEdit = async (task: CloneTask) => {
    try {
      const detail = await apiGet<CloneTask>('/product/productClone/taskDetail', { id: task.id });
      setEditTask(detail);
    } catch (e) {
      Toast.error(`加载草稿失败：${(e as Error).message}`);
    }
  };

  const gotoProduct = (productId: number) => {
    window.dispatchEvent(new CustomEvent('admin-navigate', {
      detail: { page: 'products', params: { productIds: String(productId) } },
    }));
  };

  const columns = [
    {
      title: '商品', dataIndex: 'title', width: 320,
      render: (_: unknown, t: CloneTask) => (
        <div>
          <div style={{ fontWeight: 500 }}>{t.title || '（解析中/失败）'}</div>
          <a href={t.sourceUrl} target="_blank" rel="noreferrer" style={{ fontSize: 12, color: '#888' }}>
            {t.sourceUrl.length > 60 ? `${t.sourceUrl.slice(0, 60)}…` : t.sourceUrl}
          </a>
        </div>
      ),
    },
    {
      title: '平台/通道', width: 130,
      render: (_: unknown, t: CloneTask) => (
        <Space vertical spacing={2} align="start">
          <Tag>{PLATFORM_LABEL[t.platform] || t.platform}</Tag>
          <Tag color={t.channel === 'api' ? 'violet' : 'teal'}>
            {t.channel === 'api' ? '数据接口' : '自建抓取'}
          </Tag>
        </Space>
      ),
    },
    {
      title: '状态', dataIndex: 'status', width: 100,
      render: (s: string) => <Tag color={STATUS_TAG[s]?.color || 'grey'}>{STATUS_TAG[s]?.label || s}</Tag>,
    },
    {
      title: '费用', dataIndex: 'costFen', width: 90,
      render: (fen: number) => (fen > 0 ? <Text>¥{(fen / 100).toFixed(2)}</Text> : <Text type="tertiary">-</Text>),
    },
    {
      title: '时间', dataIndex: 'createTime', width: 160,
      render: (v: number) => fmtTime(v),
    },
    {
      title: '结果/错误', width: 220,
      render: (_: unknown, t: CloneTask) => {
        if (t.status === 'failed') return <Text type="danger" style={{ fontSize: 12 }}>{t.errorMsg}</Text>;
        if (t.status === 'imported') return <Text type="success">已生成商品 #{t.productId}</Text>;
        return <Text type="tertiary">{t.createdBy}</Text>;
      },
    },
    {
      title: '操作', width: 240,
      render: (_: unknown, t: CloneTask) => (
        <Space>
          {t.status === 'ready' && <Button size="small" theme="solid" onClick={() => openEdit(t)}>预览修正</Button>}
          {t.status === 'failed' && (
            <Button size="small" onClick={() => retry(t)}>
              重新解析
            </Button>
          )}
          {t.status === 'imported' && t.productId && (
            <Button size="small" onClick={() => gotoProduct(t.productId!)}>查看商品</Button>
          )}
        </Space>
      ),
    },
  ];

  const taskTable = (
    <>
      <Space style={{ marginBottom: 12 }}>
        <Select placeholder="全部状态" value={statusFilter} onChange={(v) => { setStatusFilter(String(v || '')); setPage(1); }} style={{ width: 140 }}>
          <Select.Option value="">全部状态</Select.Option>
          <Select.Option value="parsing">解析中</Select.Option>
          <Select.Option value="ready">待导入</Select.Option>
          <Select.Option value="failed">失败</Select.Option>
          <Select.Option value="imported">已导入</Select.Option>
        </Select>
        <Select placeholder="全部平台" value={platformFilter} onChange={(v) => { setPlatformFilter(String(v || '')); setPage(1); }} style={{ width: 140 }}>
          <Select.Option value="">全部平台</Select.Option>
          {Object.entries(PLATFORM_LABEL).map(([k, label]) => (
            <Select.Option key={k} value={k}>{label}</Select.Option>
          ))}
        </Select>
        <Button onClick={loadTasks}>刷新</Button>
        <Text type="tertiary">共 {total} 条</Text>
      </Space>
      <Table
        columns={columns}
        dataSource={tasks}
        loading={loading}
        rowKey="id"
        pagination={{ currentPage: page, pageSize: 20, total, onPageChange: setPage }}
        empty={<Empty description="暂无复刻记录" />}
      />
    </>
  );

  return (
    <div style={{ padding: 16 }}>
      <Title heading={4} style={{ marginBottom: 4 }}>商品复刻</Title>
      <Text type="tertiary">
        把淘宝、京东、拼多多、1688、亚马逊的商品链接粘贴进来，抓取主图、SKU、价格与图文详情，修正后导入本站。
        请仅复刻自有或已授权的商品；导入默认下架（草稿），核对后再上架。
      </Text>
      <Tabs type="line" style={{ marginTop: 12 }}>
        <TabPane tab="新建复刻" itemKey="new">
          <Card style={{ marginBottom: 16 }}>
            <div style={{ marginBottom: 12 }}>
              <Text strong style={{ marginRight: 12 }}>数据通道</Text>
              <Text>自建抓取（边车）</Text>
              <Text type="tertiary" style={{ marginLeft: 12, fontSize: 12 }}>
                {!channels
                  ? '边车状态加载中…'
                  : !channels.selfhost.enabled
                    ? '⚪ 边车未启用（请到 设置 → 商品复刻设置 开启）'
                    : channels.selfhost.health?.alive
                      ? '🟢 边车在线'
                      : '🔴 边车离线'}
              </Text>
            </div>
            <div style={{ marginBottom: 8 }}>
              <Space wrap>
                {(channels?.matrix || []).map((r) => {
                  const lv = r.selfhost;
                  return (
                    <Tag key={r.platform} color={lv === 'none' ? 'grey' : lv === 'partial' ? 'orange' : 'green'}>
                      {r.label}{lv === 'none' ? '（不支持）' : lv === 'partial' ? '（实验性）' : ''}
                    </Tag>
                  );
                })}
              </Space>
            </div>
            <TextArea
              placeholder={'粘贴商品链接，一行一条，最多 20 条\n例如：https://item.taobao.com/item.htm?id=...'}
              value={urlsText}
              onChange={setUrlsText}
              rows={5}
            />
            {unsupportedLines.length > 0 && (
              <div style={{ marginTop: 8 }}>
                {unsupportedLines.map((b, i) => (
                  <div key={i}><Text type="danger" style={{ fontSize: 12 }}>{b.reason}：{b.url}</Text></div>
                ))}
              </div>
            )}
            <div style={{ marginTop: 12 }}>
              <Button theme="solid" type="primary" loading={submitting} onClick={submitFetch}>
                开始解析（{urlLines.length} 条）
              </Button>
              <Text type="tertiary" style={{ marginLeft: 12 }}>
                解析在后台进行，可离开页面。
              </Text>
            </div>
          </Card>
          {taskTable}
        </TabPane>
        <TabPane tab="复刻记录" itemKey="records">
          {taskTable}
        </TabPane>
      </Tabs>

      {editTask && (
        <CloneEditModal
          task={editTask}
          onClose={() => setEditTask(null)}
          onImported={() => { setEditTask(null); loadTasks(); }}
        />
      )}
    </div>
  );
}

/** 预览修正 + 导入设置弹窗。 */
function CloneEditModal({ task, onClose, onImported }: {
  task: CloneTask;
  onClose: () => void;
  onImported: () => void;
}) {
  const [draft, setDraft] = useState<CloneDraft>(() => JSON.parse(JSON.stringify(task.draft)) as CloneDraft);
  const [keepImages, setKeepImages] = useState<boolean[]>(() => (task.draft?.mainImages || []).map(() => true));
  const [categories, setCategories] = useState<Array<{ categoryId: number; categoryName: string }>>([]);
  const [brands, setBrands] = useState<Array<{ brandId: number; brandName: string }>>([]);
  const [categoryId, setCategoryId] = useState<number | undefined>(undefined);
  const [brandId, setBrandId] = useState<number | undefined>(undefined);
  const [priceMode, setPriceMode] = useState<'keep' | 'coefficient' | 'add'>('keep');
  const [priceValue, setPriceValue] = useState<string>('');
  const [targetStatus, setTargetStatus] = useState(0);
  const [transferImages, setTransferImages] = useState(true);
  const [force, setForce] = useState(false);
  const [importing, setImporting] = useState(false);

  useEffect(() => {
    apiGet<{ catList: Array<{ categoryId: number; categoryName: string }>; brandList: Array<{ brandId: number; brandName: string }> }>(
      '/product/product/catAndBrandList',
    ).then((r) => {
      setCategories(r.catList || []);
      setBrands(r.brandList || []);
    }).catch(() => undefined);
  }, []);

  const setSku = (idx: number, patch: Partial<CloneSku>) => {
    setDraft((d) => {
      const skus = d.skus.map((s, i) => (i === idx ? { ...s, ...patch } : s));
      const minPrice = skus.reduce((m, s) => {
        const v = parseFloat(s.price);
        return Number.isFinite(v) && (m === null || v < m) ? v : m;
      }, null as number | null);
      return { ...d, skus, price: minPrice !== null ? minPrice.toFixed(2) : d.price };
    });
  };

  const moveImage = (idx: number, dir: -1 | 1) => {
    const j = idx + dir;
    if (j < 0 || j >= draft.mainImages.length) return;
    const images = [...draft.mainImages];
    const keeps = [...keepImages];
    [images[idx], images[j]] = [images[j], images[idx]];
    [keeps[idx], keeps[j]] = [keeps[j], keeps[idx]];
    setDraft({ ...draft, mainImages: images });
    setKeepImages(keeps);
  };

  const doImport = async () => {
    if (!categoryId) { Toast.warning('请选择商品类目'); return; }
    const finalDraft: CloneDraft = {
      ...draft,
      mainImages: draft.mainImages.filter((_, i) => keepImages[i]),
    };
    if (finalDraft.mainImages.length === 0) { Toast.warning('至少保留一张主图'); return; }
    setImporting(true);
    try {
      const res = await apiPost<{ productId: number; productSn: string; imageTransferFailed: number; warnings: string[] }>(
        '/product/productClone/import',
        {
          taskId: task.id,
          draft: finalDraft,
          categoryId,
          brandId,
          priceRule: priceMode === 'keep' ? { mode: 'keep' } : { mode: priceMode, value: priceValue },
          targetStatus,
          transferImages,
          force,
        },
      );
      Toast.success(`导入成功，商品 SN：${res.productSn}`);
      (res.warnings || []).forEach((w) => Toast.warning(w));
      onImported();
    } catch (e) {
      Toast.error(`导入失败：${(e as Error).message}`);
    } finally {
      setImporting(false);
    }
  };

  const missingLabel: Record<string, string> = { mainImages: '主图', detail: '图文详情', skus: 'SKU' };

  return (
    <Modal
      title={`预览修正 · ${PLATFORM_LABEL[task.platform] || task.platform} · ${task.sourceItemId}`}
      visible
      onCancel={onClose}
      width={1080}
      footer={
        <Space>
          <Button onClick={onClose}>取消</Button>
          <Button theme="solid" type="primary" loading={importing} onClick={doImport}>确认导入</Button>
        </Space>
      }
    >
      <div style={{ display: 'flex', gap: 16, maxHeight: '62vh', overflow: 'auto' }}>
        <div style={{ width: 300, flexShrink: 0 }}>
          <Text strong>主图（第一张为封面，可调序/取消保留）</Text>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, marginTop: 8 }}>
            {draft.mainImages.map((u, i) => (
              <div key={i} style={{ width: 88, opacity: keepImages[i] ? 1 : 0.35 }}>
                <img src={u} alt="" style={{ width: 88, height: 88, objectFit: 'cover', borderRadius: 4, border: i === 0 ? '2px solid #1664ff' : '1px solid #eee' }} />
                <Space spacing={2} style={{ marginTop: 2 }}>
                  <Button size="small" theme="borderless" onClick={() => moveImage(i, -1)}>←</Button>
                  <Button size="small" theme="borderless" onClick={() => moveImage(i, 1)}>→</Button>
                  <Button size="small" theme="borderless" type={keepImages[i] ? 'danger' : 'primary'} onClick={() => {
                    const k = [...keepImages]; k[i] = !k[i]; setKeepImages(k);
                  }}>{keepImages[i] ? '弃' : '留'}</Button>
                </Space>
                {i === 0 && <Tag size="small" color="blue">封面</Tag>}
              </div>
            ))}
          </div>
          <Text strong style={{ display: 'block', marginTop: 16 }}>图文详情预览</Text>
          {draft.completeness?.missing?.includes('detail') ? (
            <Text type="warning">未抓到图文详情，可重新解析或导入后在商品编辑中补</Text>
          ) : (
            <div
              style={{ border: '1px solid #eee', borderRadius: 4, padding: 8, marginTop: 8, overflow: 'auto', maxHeight: 300 }}
              dangerouslySetInnerHTML={{ __html: draft.detailHtml || '' }}
            />
          )}
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
          {(draft.completeness?.missing || []).length > 0 && (
            <div style={{ marginBottom: 8 }}>
              <Text type="warning">
                以下内容未抓到：{draft.completeness.missing.map((m) => missingLabel[m] || m).join('、')}（可手动补或重新解析）
              </Text>
            </div>
          )}
          <Space vertical align="start" style={{ width: '100%' }} spacing={10}>
            <div style={{ width: '100%' }}>
              <Text strong>商品名称</Text>
              <Input value={draft.title} onChange={(v) => setDraft({ ...draft, title: v })} style={{ marginTop: 4 }} />
            </div>
            <Space>
              <div>
                <Text strong>售价（元）</Text>
                <Input value={draft.price} onChange={(v) => setDraft({ ...draft, price: v })} style={{ marginTop: 4, width: 140 }} />
              </div>
              <div>
                <Text strong>市场价（元）</Text>
                <Input value={draft.originalPrice || ''} onChange={(v) => setDraft({ ...draft, originalPrice: v || null })} style={{ marginTop: 4, width: 140 }} />
              </div>
              {draft.shopName && <Text type="tertiary">来源店铺：{draft.shopName}</Text>}
            </Space>
            <div style={{ width: '100%' }}>
              <Text strong>SKU 明细</Text>
              <Table
                size="small"
                style={{ marginTop: 6 }}
                pagination={false}
                rowKey={(record?: CloneSku) => String(record ? draft.skus.indexOf(record) : 0)}
                dataSource={draft.skus}
                columns={[
                  {
                    title: '规格', width: 200,
                    render: (_: unknown, s: CloneSku) => (s.specs.length
                      ? s.specs.map((sp) => `${sp.name}:${sp.value}`).join(' / ')
                      : '默认规格'),
                  },
                  {
                    title: '图', width: 48,
                    render: (_: unknown, s: CloneSku) => (s.image ? <img src={s.image} alt="" style={{ width: 32, height: 32, objectFit: 'cover' }} /> : '-'),
                  },
                  {
                    title: '售价', width: 110,
                    render: (_: unknown, s: CloneSku, i: number) => (
                      <Input size="small" value={s.price} onChange={(v) => setSku(i, { price: v })} />
                    ),
                  },
                  {
                    title: '市场价', width: 110,
                    render: (_: unknown, s: CloneSku, i: number) => (
                      <Input size="small" value={s.originalPrice || ''} onChange={(v) => setSku(i, { originalPrice: v || null })} />
                    ),
                  },
                  {
                    title: '库存', width: 100,
                    render: (_: unknown, s: CloneSku, i: number) => (
                      <InputNumber size="small" value={s.stock ?? undefined} onChange={(v) => setSku(i, { stock: typeof v === 'number' ? v : null })} style={{ width: 90 }} />
                    ),
                  },
                ]}
              />
            </div>
            {draft.props.length > 0 && (
              <div>
                <Text strong>商品参数（将带入为商品属性）</Text>
                <div style={{ marginTop: 6 }}>
                  <Space wrap>
                    {draft.props.map((p, i) => <Tag key={i}>{p.name}：{p.value}</Tag>)}
                  </Space>
                </div>
              </div>
            )}
            <Card style={{ width: '100%' }} bodyStyle={{ padding: 12 }}>
              <Text strong>导入设置</Text>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: 16, marginTop: 10, alignItems: 'flex-end' }}>
                <div>
                  <Text>商品类目（必选）</Text>
                  <Select placeholder="选择类目" value={categoryId} onChange={(v) => setCategoryId(v as number)} style={{ width: 200, display: 'block', marginTop: 4 }} filter>
                    {categories.map((c) => (
                      <Select.Option key={c.categoryId} value={c.categoryId}>{c.categoryName}</Select.Option>
                    ))}
                  </Select>
                </div>
                <div>
                  <Text>品牌</Text>
                  <Select placeholder="可选" value={brandId} onChange={(v) => setBrandId(v as number)} style={{ width: 160, display: 'block', marginTop: 4 }} filter showClear>
                    {brands.map((b) => (
                      <Select.Option key={b.brandId} value={b.brandId}>{b.brandName}</Select.Option>
                    ))}
                  </Select>
                </div>
                <div>
                  <Text>价格规则</Text>
                  <Space style={{ marginTop: 4 }}>
                    <Select value={priceMode} onChange={(v) => setPriceMode(v as typeof priceMode)} style={{ width: 130 }}>
                      <Select.Option value="keep">原价直用</Select.Option>
                      <Select.Option value="coefficient">统一系数</Select.Option>
                      <Select.Option value="add">统一加减价</Select.Option>
                    </Select>
                    {priceMode !== 'keep' && (
                      <Input value={priceValue} onChange={setPriceValue} placeholder={priceMode === 'coefficient' ? '如 1.2' : '如 10 / -5'} style={{ width: 100 }} />
                    )}
                  </Space>
                </div>
                <div>
                  <Text>落地状态</Text>
                  <RadioGroup value={targetStatus} onChange={(e) => setTargetStatus(e.target.value as number)} style={{ marginTop: 6, display: 'block' }}>
                    <Radio value={0}>下架（草稿）</Radio>
                    <Radio value={1}>直接上架</Radio>
                  </RadioGroup>
                </div>
                <Space>
                  <Switch checked={transferImages} onChange={setTransferImages} />
                  <Text>图片转存到本站</Text>
                </Space>
                <Space>
                  <Switch checked={force} onChange={setForce} />
                  <Text>重复时再建一份</Text>
                </Space>
              </div>
            </Card>
          </Space>
        </div>
      </div>
      <Spin spinning={false} />
    </Modal>
  );
}
