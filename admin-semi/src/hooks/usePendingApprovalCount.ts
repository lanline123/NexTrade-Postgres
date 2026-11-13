import { useEffect, useState } from 'react';
import { apiGet } from '../api';

interface PendingApprovalsResponse {
  records?: unknown[];
  total?: number;
}

/**
 * AI 审批中心待审批计数（供左侧菜单角标 / 顶栏消息红点 / 消息中心入口条共用）。
 *
 * 计数口径与审批中心页面（pages/AiMcpApprovals）完全一致：同一个接口
 * `GET /ai/mcp/approvals`、同一个 `status=pending` 过滤；后端返回的 `total`
 * 就是该状态下的总数，这里只取 size=1，避免把整页审批参数明细拉回来。
 *
 * 行为：挂载立即拉一次，之后每 60 秒轮询；refreshKey 变化（如路由切换）时立即再拉一次。
 * 拉取失败静默处理——保留上一次的计数（首次失败即 0，角标不显示），
 * 绝不向上抛错影响后台主流程。本 hook 只在登录后的 AdminLayout 内使用，登录页不会请求。
 */
export function usePendingApprovalCount(refreshKey?: unknown): number {
  const [count, setCount] = useState(0);

  useEffect(() => {
    let alive = true;
    const refresh = () => {
      void apiGet<PendingApprovalsResponse>('/ai/mcp/approvals', { status: 'pending', size: 1 })
        .then((res) => {
          if (!alive) return;
          const total =
            typeof res?.total === 'number'
              ? res.total
              : Array.isArray(res?.records)
                ? res.records.length
                : 0;
          setCount(Math.max(0, total));
        })
        .catch(() => {
          // 静默：审批接口异常时不动角标
        });
    };
    refresh();
    const timer = window.setInterval(refresh, 60000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [refreshKey]);

  return count;
}
