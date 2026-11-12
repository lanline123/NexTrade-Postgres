/**
 * IM 客服数据层。
 * Vue 基准：api/im/im.ts + utils/websocket.ts + pages/chat/index.vue
 *
 * **口径注意**：IM 接口不在 `/api` 下，Vue 的 `asyncRequest({ prefix: "/im/" })` 会把 baseURL
 * 换成 `/im/`（见 utils/request.ts 第 127 行），所以路径是 `/im/conversation/...`。
 * Next 侧通过 `next.config.mjs` 的 rewrites 把 `/im/*` 转发到后端。
 *
 * 真实接口（2026-11-13 实测，测试账号有 1 个会话 / 15 条消息）：
 * - GET  im/conversation/conversation/list?page&size
 *        → records[{ conversationId, shopId, unreadMessageCount, lastServantId, lastUpdateTime,
 *                    user{userId,username,nickname,avatar}, lastMessage[Message] }]
 * - GET  im/conversation/message/list?shopId&conversationId&firstId&sortOrder
 *        → records[Message]（sortOrder=desc，firstId=-1 表示最新一页）
 * - POST im/conversation/message/send { content:{messageType,content,pic}, shopId, conversationId }
 * - POST im/conversation/message/setRead { shopId, servantId }
 *
 * Message：{ id, conversationId, type(1=客服 2=用户), messageType(text|image|product|order|video),
 *           messageTypeText, content:{ content, pic, product, order, videoUrl }, sendTime, isRead,
 *           shopId, servantId, user{ avatar, nickname, username } }
 *
 * WebSocket：`<ws|wss>://host/ws?token=<token>&platform=pc`，心跳 15s（`{type:"heartBeat"}`），
 * 下行消息 `{type:"message", data:[Message]}`、已读回执 `{type:"read"}`。
 * **部署要求**：nginx 需要把 `/ws` 升级转发到 IM 服务；否则用 `PC_IM_WS` 指定完整地址。
 */
import { ApiError } from '@/lib/client-api';

const IM_TIMEOUT = 20000;

function authHeaders(): Record<string, string> {
    if (typeof window === 'undefined') return {};
    const token = localStorage.getItem('accessToken') || localStorage.getItem('token');
    return token ? { Authorization: `Bearer ${token}` } : {};
}

async function readIm<T>(response: Response): Promise<T> {
    const text = await response.text();
    let json: unknown;
    try {
        json = text ? JSON.parse(text) : {};
    } catch {
        throw new ApiError(`IM 返回格式异常（HTTP ${response.status}）`, response.status);
    }
    const envelope = json as { code?: number; msg?: string; message?: string; data?: T };
    if (!response.ok || (envelope.code !== undefined && envelope.code !== 0)) {
        throw new ApiError(envelope.msg || envelope.message || `IM 请求失败（HTTP ${response.status}）`, envelope.code);
    }
    return (envelope.data ?? json) as T;
}

function withTimeout(path: string, init: RequestInit): Promise<Response> {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), IM_TIMEOUT);
    return fetch(`/im/${path}`, { ...init, signal: controller.signal }).finally(() => clearTimeout(timer));
}

export function imGet<T>(path: string, params: Record<string, unknown> = {}): Promise<T> {
    const query = new URLSearchParams();
    Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined && value !== null && value !== '') query.set(key, String(value));
    });
    const suffix = query.toString() ? `?${query.toString()}` : '';
    return withTimeout(`${path}${suffix}`, { credentials: 'include', headers: authHeaders() }).then(readIm<T>);
}

export function imPost<T>(path: string, body?: unknown): Promise<T> {
    return withTimeout(path, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json', ...authHeaders() },
        body: body === undefined ? undefined : JSON.stringify(body),
    }).then(readIm<T>);
}

export interface ImUser {
    userId?: number;
    username?: string;
    nickname?: string;
    avatar?: string;
}

export interface ImContent {
    messageType?: string;
    content?: string | null;
    pic?: string | null;
    videoUrl?: string | null;
    product?: {
        productId?: number;
        productName?: string;
        picUrl?: string;
        productPrice?: string | number;
        productSn?: string;
    } | null;
    order?: {
        orderId?: number;
        orderSn?: string;
        totalAmount?: string | number;
        orderStatusName?: string;
        productName?: string;
        picUrl?: string;
        quantity?: number;
    } | null;
}

export interface ImMessage {
    id: number | string;
    conversationId?: number;
    /** 1 = 客服，2 = 用户（自己） */
    type?: number;
    messageType?: string;
    messageTypeText?: string;
    content?: ImContent;
    sendTime?: string;
    isRead?: number;
    shopId?: number;
    servantId?: number;
    user?: ImUser;
    /** 本地乐观插入的标记 */
    pending?: boolean;
    failed?: boolean;
}

export interface ImConversation {
    conversationId: number;
    shopId?: number;
    unreadMessageCount?: number;
    lastServantId?: number;
    lastUpdateTime?: string;
    user?: ImUser;
    lastMessage?: ImMessage[];
    shop?: { shopId?: number; shopTitle?: string; shopLogo?: string };
}

export interface ImPage<T> {
    records?: T[];
    total?: number;
}

export const fetchConversations = (page = 1, size = 30) =>
    imGet<ImPage<ImConversation>>('conversation/conversation/list', { page, size });

export const fetchMessages = (params: {
    shopId: number;
    conversationId?: number;
    firstId?: number;
    sortOrder?: string;
}) => imGet<ImPage<ImMessage>>('conversation/message/list', { firstId: -1, sortOrder: 'desc', ...params });

export const sendImMessage = (data: {
    content: { messageType: string; content?: string; pic?: string };
    shopId: number;
    conversationId?: number;
}) => imPost<ImMessage>('conversation/message/send', data);

export const markConversationRead = (data: { shopId: number; servantId: number }) =>
    imPost('conversation/message/setRead', data);

/** WS 地址：优先 `PC_IM_WS`（构建时注入），否则按当前域名拼 `/ws`。 */
export function resolveSocketUrl(): string {
    const configured = process.env.NEXT_PUBLIC_IM_WS || '';
    if (configured) return configured;
    if (typeof window === 'undefined') return '';
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${protocol}//${window.location.host}/ws`;
}

export interface ImSocketHandlers {
    onMessage?: (messages: ImMessage[]) => void;
    onRead?: () => void;
    onStatus?: (connected: boolean) => void;
}

/**
 * IM WebSocket 客户端：心跳 15s、最多重连 5 次（退避 1s 起）。
 * 与 Vue 的 `utils/websocket.ts` 同口径，另外把连接状态抛给上层——
 * 页面在未连上时用轮询兜底，避免 nginx 没配 `/ws` 时聊天直接不可用。
 */
export class ImSocket {
    private socket: WebSocket | null = null;
    private heartbeat?: ReturnType<typeof setInterval>;
    private reconnectTimer?: ReturnType<typeof setTimeout>;
    private attempts = 0;
    private stopped = false;
    private readonly maxAttempts = 5;

    constructor(
        private readonly url: string,
        private readonly handlers: ImSocketHandlers = {},
    ) {}

    connect() {
        if (typeof window === 'undefined' || !this.url) return;
        if (this.socket && this.socket.readyState <= WebSocket.OPEN) return;
        this.stopped = false;
        try {
            this.socket = new WebSocket(this.url);
        } catch {
            this.handlers.onStatus?.(false);
            return;
        }

        this.socket.onopen = () => {
            this.attempts = 0;
            this.handlers.onStatus?.(true);
            this.heartbeat = setInterval(() => {
                if (this.socket?.readyState === WebSocket.OPEN) this.socket.send(JSON.stringify({ type: 'heartBeat' }));
            }, 15000);
        };

        this.socket.onmessage = (event: MessageEvent) => {
            try {
                const data = JSON.parse(String(event.data)) as { type?: string; data?: ImMessage[] };
                if (data.type === 'message' && Array.isArray(data.data) && data.data.length > 0) {
                    this.handlers.onMessage?.(data.data);
                } else if (data.type === 'read') {
                    this.handlers.onRead?.();
                }
            } catch {
                // 心跳回包等非 JSON 内容，忽略
            }
        };

        this.socket.onclose = () => {
            this.handlers.onStatus?.(false);
            this.clearHeartbeat();
            this.scheduleReconnect();
        };

        this.socket.onerror = () => {
            this.handlers.onStatus?.(false);
        };
    }

    private scheduleReconnect() {
        if (this.stopped || this.attempts >= this.maxAttempts) return;
        const delay = 1000 * 2 ** this.attempts;
        this.attempts += 1;
        this.reconnectTimer = setTimeout(() => this.connect(), delay);
    }

    private clearHeartbeat() {
        if (this.heartbeat) {
            clearInterval(this.heartbeat);
            this.heartbeat = undefined;
        }
    }

    close() {
        this.stopped = true;
        this.clearHeartbeat();
        if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
        this.socket?.close();
        this.socket = null;
    }
}
