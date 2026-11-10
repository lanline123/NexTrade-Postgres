/**
 * AI 运营 MCP 操作日志（三级菜单）。
 * 记录每次 tools/call 的工具名、参数、状态与耗时。
 */
import { useCallback, useEffect, useState } from 'react';
import { Button, Spin, Toast } from '@douyinfe/semi-ui';
import { apiGet } from '../api';

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

  return (
    <div style={{ background: '#fff', borderRadius: 8, padding: '16px 20px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
        <h3 style={{ margin: 0 }}>操作日志（{total}）</h3>
        <Button onClick={() => void load(page)}>刷新</Button>
      </div>
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
