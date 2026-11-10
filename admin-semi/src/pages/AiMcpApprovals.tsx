/**
 * AI 运营 MCP 审批中心（三级菜单）。
 * 改价超阈值、库存超上限、发布装修等高风险动作会在这里等待通过/驳回。
 */
import { useCallback, useEffect, useState } from 'react';
import { Button, Spin, Toast } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';

interface ApprovalRow {
  approvalId: number;
  tool: string;
  args: string;
  status: string;
  addTime: number;
}

const TOOL_LABEL: Record<string, string> = {
  product_update_price: '修改商品价格',
  product_update_stock: '修改商品库存',
  decorate_publish: '发布装修页',
};

function formatTime(sec?: number): string {
  if (!sec) return '-';
  const d = new Date(sec * 1000);
  return d.toLocaleString('zh-CN', { hour12: false });
}

export default function AiMcpApprovals() {
  const [rows, setRows] = useState<ApprovalRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [acting, setActing] = useState<number | null>(null);

  const load = useCallback(async () => {
    try {
      const res = await apiGet<{ records: ApprovalRow[] }>('/ai/mcp/approvals', {
        status: 'pending',
        size: 50,
      });
      setRows(res.records || []);
    } catch (e) {
      Toast.error({ content: (e as Error).message || '加载审批列表失败' });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const act = async (approvalId: number, action: 'approve' | 'reject') => {
    setActing(approvalId);
    try {
      await apiPost(`/ai/mcp/approvals/${action}`, { approvalId });
      Toast.success({ content: action === 'approve' ? '已通过并执行' : '已驳回' });
      void load();
    } catch (e) {
      Toast.error({ content: (e as Error).message || '操作失败' });
    } finally {
      setActing(null);
    }
  };

  if (loading) {
    return (
      <div style={{ padding: 40, textAlign: 'center' }}>
        <Spin />
      </div>
    );
  }

  return (
    <div style={{ background: '#fff', borderRadius: 8, padding: '16px 20px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
        <h3 style={{ margin: 0 }}>待审批（{rows.length}）</h3>
        <Button onClick={() => void load()}>刷新</Button>
      </div>
      {rows.length === 0 ? (
        <p style={{ color: '#8a8f99', padding: '24px 0', textAlign: 'center' }}>暂无待审批操作</p>
      ) : (
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
          <thead>
            <tr style={{ background: '#f7f8fa', textAlign: 'left' }}>
              <th style={{ padding: 10 }}>ID</th>
              <th style={{ padding: 10 }}>操作</th>
              <th style={{ padding: 10 }}>参数</th>
              <th style={{ padding: 10 }}>提交时间</th>
              <th style={{ padding: 10, width: 160 }}>处理</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.approvalId} style={{ borderTop: '1px solid #f0f1f2' }}>
                <td style={{ padding: 10 }}>{row.approvalId}</td>
                <td style={{ padding: 10 }}>{TOOL_LABEL[row.tool] || row.tool}</td>
                <td style={{ padding: 10, maxWidth: 360, wordBreak: 'break-all', color: '#666' }}>
                  {row.args}
                </td>
                <td style={{ padding: 10 }}>{formatTime(row.addTime)}</td>
                <td style={{ padding: 10 }}>
                  <Button
                    theme="solid"
                    type="primary"
                    size="small"
                    loading={acting === row.approvalId}
                    onClick={() => void act(row.approvalId, 'approve')}
                  >
                    通过
                  </Button>
                  <Button
                    size="small"
                    style={{ marginLeft: 8 }}
                    disabled={acting === row.approvalId}
                    onClick={() => void act(row.approvalId, 'reject')}
                  >
                    驳回
                  </Button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
