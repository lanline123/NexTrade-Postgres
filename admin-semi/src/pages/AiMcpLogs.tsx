/**
 * AI 运营 MCP 操作日志（三级菜单）。
 * 记录每次 tools/call 的工具名、参数、状态与耗时。
 */
import { useCallback, useEffect, useState } from 'react';
import { Button, DatePicker, InputNumber, Modal, Radio, RadioGroup, Spin, Toast } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';

interface LogRow {
  logId: number;
  tool: string;
  args?: string;
  status: string;
  error?: string;
  durationMs: number;
  addTime: number;
}

function formatTime(sec?: number): string {
  if (!sec) return '-';
  return new Date(sec * 1000).toLocaleString('zh-CN', { hour12: false });
}

export default function AiMcpLogs() {
  const [rows, setRows] = useState<LogRow[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(true);
  const size = 20;

  const load = useCallback(
    async (nextPage = page) => {
      setLoading(true);
      try {
        const res = await apiGet<{ records: LogRow[]; total: number }>('/ai/mcp/logs', {
          page: nextPage,
          size,
        });
        setRows(res.records || []);
        setTotal(res.total || 0);
        setPage(nextPage);
      } catch (e) {
        Toast.error({ content: (e as Error).message || '加载日志失败' });
      } finally {
        setLoading(false);
      }
    },
    [page],
  );

  useEffect(() => {
    void load(1);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const pages = Math.max(1, Math.ceil(total / size));

  // ——— 清理日志：按截止日期删除，或只保留最近 N 天 ———
  const [cleanupVisible, setCleanupVisible] = useState(false);
  const [cleanupMode, setCleanupMode] = useState<'before' | 'retain'>('retain');
  const [beforeDate, setBeforeDate] = useState<Date>(() => new Date());
  const [retainDays, setRetainDays] = useState<number>(30);
  const [cleaning, setCleaning] = useState(false);

  const cleanupHint =
    cleanupMode === 'before'
      ? `将删除 ${beforeDate.toLocaleDateString('zh-CN')} 00:00 之前的全部日志`
      : `将只保留最近 ${retainDays} 天的日志，更早的全部删除`;

  const doCleanup = async () => {
    setCleaning(true);
    try {
      const payload =
        cleanupMode === 'before'
          ? { before: Math.floor(new Date(beforeDate.getFullYear(), beforeDate.getMonth(), beforeDate.getDate()).getTime() / 1000) }
          : { retainDays };
      const res = await apiPost<{ deleted: number }>('/ai/mcp/logs/cleanup', payload);
      Toast.success({ content: `已清理 ${res.deleted ?? 0} 条日志` });
      setCleanupVisible(false);
      void load(1);
    } catch (e) {
      Toast.error({ content: (e as Error).message || '清理失败' });
    } finally {
      setCleaning(false);
    }
  };

  return (
    <div style={{ background: '#fff', borderRadius: 8, padding: '16px 20px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
        <h3 style={{ margin: 0 }}>操作日志（{total}）</h3>
        <div style={{ display: 'flex', gap: 8 }}>
          <Button type="danger" theme="light" onClick={() => setCleanupVisible(true)}>
            清理日志
          </Button>
          <Button onClick={() => void load(page)}>刷新</Button>
        </div>
      </div>
      <Modal
        title="清理调用日志"
        visible={cleanupVisible}
        onCancel={() => setCleanupVisible(false)}
        onOk={() => void doCleanup()}
        okText="确认清理"
        cancelText="取消"
        confirmLoading={cleaning}
        okButtonProps={{ type: 'danger' }}
      >
        <RadioGroup
          type="button"
          value={cleanupMode}
          onChange={(e) => setCleanupMode(e.target.value as 'before' | 'retain')}
          style={{ marginBottom: 16 }}
        >
          <Radio value="retain">按保留天数</Radio>
          <Radio value="before">按截止日期</Radio>
        </RadioGroup>
        {cleanupMode === 'before' ? (
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span>删除此日期之前的日志：</span>
            <DatePicker
              type="date"
              value={beforeDate}
              onChange={(d) => d instanceof Date && setBeforeDate(d)}
            />
          </div>
        ) : (
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span>保留最近</span>
            <InputNumber
              min={0}
              max={3650}
              value={retainDays}
              onChange={(v) => setRetainDays(typeof v === 'number' ? v : 30)}
              style={{ width: 110 }}
            />
            <span>天的日志</span>
          </div>
        )}
        <p style={{ color: '#d93026', marginTop: 16, marginBottom: 0 }}>
          {cleanupHint}，删除后不可恢复。
        </p>
      </Modal>
      {loading ? (
        <div style={{ padding: 24, textAlign: 'center' }}>
          <Spin />
        </div>
      ) : rows.length === 0 ? (
        <p style={{ color: '#8a8f99', padding: '24px 0', textAlign: 'center' }}>暂无调用记录</p>
      ) : (
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
          <thead>
            <tr style={{ background: '#f7f8fa', textAlign: 'left' }}>
              <th style={{ padding: 10 }}>ID</th>
              <th style={{ padding: 10 }}>工具</th>
              <th style={{ padding: 10 }}>状态</th>
              <th style={{ padding: 10 }}>耗时</th>
              <th style={{ padding: 10 }}>参数 / 错误</th>
              <th style={{ padding: 10 }}>时间</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.logId} style={{ borderTop: '1px solid #f0f1f2' }}>
                <td style={{ padding: 10 }}>{row.logId}</td>
                <td style={{ padding: 10 }}>{row.tool}</td>
                <td style={{ padding: 10, color: row.status === 'ok' ? '#1f9d55' : '#d93026' }}>
                  {row.status}
                </td>
                <td style={{ padding: 10 }}>{row.durationMs} ms</td>
                <td style={{ padding: 10, maxWidth: 420, wordBreak: 'break-all', color: '#666' }}>
                  {row.error || row.args || '-'}
                </td>
                <td style={{ padding: 10 }}>{formatTime(row.addTime)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <div style={{ marginTop: 12, display: 'flex', gap: 8, alignItems: 'center' }}>
        <Button size="small" disabled={page <= 1} onClick={() => void load(page - 1)}>
          上一页
        </Button>
        <span style={{ color: '#8a8f99', fontSize: 12 }}>
          {page} / {pages}
        </span>
        <Button size="small" disabled={page >= pages} onClick={() => void load(page + 1)}>
          下一页
        </Button>
      </div>
    </div>
  );
}
