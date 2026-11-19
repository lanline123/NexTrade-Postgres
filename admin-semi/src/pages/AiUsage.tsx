/**
 * AI 用量记录页（我方新增）。P1 只记不扣：自有密钥流量 credits 恒 0；
 * 商家只能看本店（服务端强制），平台可按店铺筛选看全平台。
 */
import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Card,
  Input,
  Select,
  Table,
  Tag,
  Toast,
} from '@douyinfe/semi-ui';
import { apiGet } from '../api';

interface UsageRecord {
  usageId: number;
  kind: number;
  modelCode: string;
  promptTokens: number;
  completionTokens: number;
  creditsCharged: number;
  selfKey: boolean;
  status: number;
  createTime: number;
}

interface UsageData {
  records: UsageRecord[];
  total: number;
  summary: {
    calls: number;
    promptTokens: number;
    completionTokens: number;
    credits: number;
  };
}

const STATUS_LABEL: Record<number, { text: string; color: 'green' | 'red' | 'amber' }> = {
  1: { text: '成功', color: 'green' },
  2: { text: '失败', color: 'red' },
  3: { text: '部分完成', color: 'amber' },
};

const isPlatform = () => {
  const t = localStorage.getItem('adminType') || '';
  return t === '' || t === 'admin';
};

export default function AiUsage() {
  const [data, setData] = useState<UsageData | null>(null);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(1);
  const [model, setModel] = useState('');
  const [status, setStatus] = useState<number | undefined>(undefined);
  const [shopId, setShopId] = useState('');
  const pageSize = 20;

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await apiGet<UsageData>('/ai/usage', {
        page,
        size: pageSize,
        modelCode: model || undefined,
        status,
        shopId: isPlatform() && shopId ? Number(shopId) : undefined,
      });
      setData(res);
    } catch (e) {
      Toast.error({ content: (e as Error).message || '加载失败' });
    } finally {
      setLoading(false);
    }
  }, [page, model, status, shopId]);

  useEffect(() => {
    void load();
  }, [load]);

  const columns = [
    {
      title: '时间',
      dataIndex: 'createTime',
      render: (v: number) => new Date(v * 1000).toLocaleString(),
    },
    { title: '类型', dataIndex: 'kind', render: (v: number) => (v === 1 ? '对话' : '生图') },
    { title: '模型', dataIndex: 'modelCode' },
    { title: '输入 tokens', dataIndex: 'promptTokens' },
    { title: '输出 tokens', dataIndex: 'completionTokens' },
    {
      title: '来源',
      dataIndex: 'selfKey',
      render: (v: boolean) => (v ? '自有密钥' : '官方渠道'),
    },
    {
      title: '消耗积分',
      dataIndex: 'creditsCharged',
      render: (v: number) => (v > 0 ? v : '—'),
    },
    {
      title: '状态',
      dataIndex: 'status',
      render: (v: number) => {
        const s = STATUS_LABEL[v];
        return s ? <Tag color={s.color}>{s.text}</Tag> : v;
      },
    },
  ];

  return (
    <Card title="AI 用量记录">
      <div style={{ display: 'flex', gap: 8, marginBottom: 12, flexWrap: 'wrap' }}>
        <Input
          placeholder="按模型编码筛选"
          value={model}
          onChange={(v) => {
            setModel(v);
            setPage(1);
          }}
          style={{ width: 200 }}
        />
        <Select
          placeholder="状态"
          value={status}
          onChange={(v) => {
            setStatus(v === undefined ? undefined : Number(v));
            setPage(1);
          }}
          style={{ width: 140 }}
        >
          <Select.Option value={1}>成功</Select.Option>
          <Select.Option value={2}>失败</Select.Option>
          <Select.Option value={3}>部分完成</Select.Option>
        </Select>
        {isPlatform() && (
          <Input
            placeholder="店铺 ID（空=全平台）"
            value={shopId}
            onChange={(v) => {
              setShopId(v.replace(/[^0-9]/g, ''));
              setPage(1);
            }}
            style={{ width: 170 }}
          />
        )}
        <Button onClick={() => void load()}>刷新</Button>
      </div>
      {data && (
        <div style={{ display: 'flex', gap: 16, marginBottom: 12, fontSize: 13 }}>
          <span>
            调用次数 <strong>{data.summary.calls}</strong>
          </span>
          <span>
            输入 tokens <strong>{data.summary.promptTokens.toLocaleString()}</strong>
          </span>
          <span>
            输出 tokens <strong>{data.summary.completionTokens.toLocaleString()}</strong>
          </span>
          <span>
            消耗积分 <strong>{data.summary.credits}</strong>
            <span style={{ color: 'var(--semi-color-text-2)' }}>（P1 只记录不扣减）</span>
          </span>
        </div>
      )}
      <Table
        columns={columns}
        dataSource={data?.records || []}
        rowKey="usageId"
        loading={loading}
        pagination={{
          currentPage: page,
          pageSize,
          total: data?.total || 0,
          onPageChange: (p: number) => setPage(p),
        }}
      />
    </Card>
  );
}
