'use client';

/**
 * 买家侧 AI 导购（我方新增）—— PC 端接口层。
 *
 * 与 H5 导购（uniapp-react/src/api/guide.ts）共用同一套后端接口
 * （仅登录、平台维度会话），PC 侧经同源 /api 代理直达，不新造接口：
 * - POST /api/ai/guide/conversations        新建会话
 * - GET  /api/ai/guide/conversations        我的会话列表
 * - GET  /api/ai/guide/conversations/{id}/messages  会话消息（tool 行带 cards 回显）
 * - POST /api/ai/guide/chat/stop            停止进行中的回复
 * - POST /api/ai/guide/chat/stream          发送消息（SSE 流式）
 *
 * 普通接口走统一 client-api（信封/鉴权头与全站一致）；流式对话走
 * fetch + ReadableStream 读 SSE（解析口径与 H5 端逐项对齐）。
 */
import { apiGet, apiPost } from '@/lib/client-api';

export interface GuideCard {
  productId: number;
  productName: string;
  picUrl?: string;
  picThumb?: string;
  shopId?: number;
  /** 已按当前买家看价权限处理：priceVisible=false 时恒为 null */
  productPrice?: string | number | null;
  marketPrice?: string | number | null;
  priceVisible?: boolean;
  /** none / login / certification */
  priceGate?: string;
  productStock?: number;
  sales?: number;
}

export interface GuideConversation {
  convId: number;
  title: string;
  channelId: number;
  modelCode: string;
  createTime: number;
  updateTime: number;
}

export interface GuideMessage {
  msgId: number;
  /** 1 买家 / 2 助手 / 3 工具 */
  role: number;
  content: string;
  /** 工具行：{ callId, cards }（cards 供回显渲染商品卡） */
  toolCalls?: { callId?: string; cards?: GuideCard[] } | null;
  toolName?: string | null;
  toolStatus: number;
  createTime: number;
}

export const createGuideConversation = () =>
  apiPost<{ convId: number }>('ai/guide/conversations', {});

export const getGuideConversations = () =>
  apiGet<{ list: GuideConversation[] }>('ai/guide/conversations');

export const getGuideMessages = (convId: number) =>
  apiGet<{ list: GuideMessage[] }>(`ai/guide/conversations/${convId}/messages`);

export const stopGuideChat = (convId: number) =>
  apiPost<{ stopped: boolean }>('ai/guide/chat/stop', { convId });

export interface GuideStreamHandlers {
  onAssistant?: (msgId: number, convId: number) => void;
  onDelta?: (content: string) => void;
  onTool?: (name: string, status: string, cards: GuideCard[]) => void;
  onError?: (message: string) => void;
  onDone?: (stopped: boolean) => void;
}

/**
 * 发送一轮导购对话（SSE 流式）。返回中止句柄：abort 只断本地连接，
 * 服务端停止请配合 stopGuideChat(convId)。
 * 错误文案经 handlers.onError 以 i18n key 抛出，由面板 t() 渲染。
 */
export function streamGuideChat(
  body: { convId?: number; content: string },
  handlers: GuideStreamHandlers,
): { abort: () => void } {
  const controller = new AbortController();

  const run = async () => {
    let resp: Response;
    try {
      const token = typeof window !== 'undefined' ? localStorage.getItem('accessToken') : null;
      resp = await fetch('/api/ai/guide/chat/stream', {
        method: 'POST',
        credentials: 'include',
        headers: {
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
          'Content-Type': 'application/json',
          Accept: 'text/event-stream',
        },
        body: JSON.stringify(body),
        signal: controller.signal,
      });
    } catch {
      if (!controller.signal.aborted) {
        handlers.onError?.('网络异常，请稍后再试');
        handlers.onDone?.(false);
      }
      return;
    }
    if (!resp.ok || !resp.body) {
      handlers.onError?.(resp.status === 401 ? '登录已过期，请重新登录' : '导购服务暂不可用');
      handlers.onDone?.(false);
      return;
    }
    const reader = resp.body.getReader();
    const decoder = new TextDecoder('utf-8');
    let buffer = '';
    let finished = false;
    const dispatch = (block: string) => {
      let event = '';
      const dataLines: string[] = [];
      for (const line of block.split('\n')) {
        if (line.startsWith('event:')) event = line.slice(6).trim();
        else if (line.startsWith('data:')) dataLines.push(line.slice(5).trim());
      }
      if (!event || dataLines.length === 0) return;
      let payload: {
        msgId?: number;
        convId?: number;
        content?: string;
        name?: string;
        status?: string;
        cards?: GuideCard[];
        message?: string;
        stopped?: boolean;
      } = {};
      try {
        payload = JSON.parse(dataLines.join('\n'));
      } catch {
        return;
      }
      switch (event) {
        case 'assistant':
          handlers.onAssistant?.(payload.msgId || 0, payload.convId || 0);
          break;
        case 'delta':
          handlers.onDelta?.(payload.content || '');
          break;
        case 'tool':
          handlers.onTool?.(payload.name || '', payload.status || '', payload.cards || []);
          break;
        case 'error':
          handlers.onError?.(payload.message || '导购服务异常');
          break;
        case 'done':
          finished = true;
          handlers.onDone?.(!!payload.stopped);
          break;
        default:
          break;
      }
    };
    try {
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });
        let idx = buffer.indexOf('\n\n');
        while (idx >= 0) {
          dispatch(buffer.slice(0, idx));
          buffer = buffer.slice(idx + 2);
          idx = buffer.indexOf('\n\n');
        }
      }
      if (buffer.trim()) dispatch(buffer);
    } catch {
      // 本地 abort 或连接中断：未收到 done 时按停止收口
    }
    if (!finished && !controller.signal.aborted) {
      handlers.onDone?.(false);
    }
  };
  void run();
  return {
    abort: () => controller.abort(),
  };
}
