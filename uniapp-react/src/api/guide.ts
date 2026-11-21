import request, { baseUrl, requestHeader } from "@/utils/request";

/** 登录态被后端拒绝（401）时的统一错误文案（导购页据此识别并切登录引导）。 */
export const GUIDE_LOGIN_EXPIRED_MSG = "登录已过期，请重新登录";

/**
 * 买家侧 AI 导购（我方新增）—— 后端 /api/ai/guide/*（仅登录，平台维度会话）。
 * 普通接口走统一 request；流式对话走 fetch + ReadableStream 读 SSE
 * （H5 目标；小程序端无 ReadableStream 时给出明确错误，不静默失败）。
 */

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

/** 统一响应信封（request 的泛型口径，与其它 api 模块一致） */
interface GuideEnvelope<T> {
    data: T;
    code: number;
    message: string;
}

export const createGuideConversation = () => {
    return request<GuideEnvelope<{ convId: number }>>({
        url: "ai/guide/conversations",
        method: "post",
        data: {}
    });
};

export const getGuideConversations = () => {
    return request<GuideEnvelope<{ list: GuideConversation[] }>>({
        url: "ai/guide/conversations",
        method: "get"
    });
};

export const getGuideMessages = (convId: number) => {
    return request<GuideEnvelope<{ list: GuideMessage[] }>>({
        url: `ai/guide/conversations/${convId}/messages`,
        method: "get"
    });
};

// 注：后端有 DELETE /ai/guide/conversations/{id}，但共享 request 只支持
// GET/POST，H5 MVP 不做删除入口（新会话 + 自动续接最近会话已覆盖主路径）。

export const stopGuideChat = (convId: number) => {
    return request<GuideEnvelope<{ stopped: boolean }>>({
        url: "ai/guide/chat/stop",
        method: "post",
        data: { convId }
    });
};

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
 */
export function streamGuideChat(
    body: { convId?: number; content: string },
    handlers: GuideStreamHandlers
): { abort: () => void } {
    const controller = new AbortController();
    const prefix = process.env.TARO_APP_API_PREFIX || "/api/";
    const url = `${baseUrl}${prefix}ai/guide/chat/stream`;

    const run = async () => {
        if (typeof fetch === "undefined" || typeof ReadableStream === "undefined") {
            handlers.onError?.("当前环境不支持流式对话");
            handlers.onDone?.(false);
            return;
        }
        let resp: Response;
        try {
            resp = await fetch(url, {
                method: "POST",
                headers: {
                    ...requestHeader(),
                    "Content-Type": "application/json",
                    Accept: "text/event-stream"
                },
                body: JSON.stringify(body),
                signal: controller.signal
            });
        } catch (e) {
            if (!controller.signal.aborted) {
                handlers.onError?.("网络异常，请稍后再试");
                handlers.onDone?.(false);
            }
            return;
        }
        if (!resp.ok || !resp.body) {
            handlers.onError?.(resp.status === 401 ? GUIDE_LOGIN_EXPIRED_MSG : "导购服务暂不可用");
            handlers.onDone?.(false);
            return;
        }
        // 后端鉴权/业务拒绝走 HTTP 200 + JSON 信封（不是 SSE）：必须按信封解析报错，
        // 否则信封会被当作 SSE 静默吞掉，页面只剩一个空气泡（PC 端 v63 已修同病，
        // 本次补齐 H5：401 信封 → 过期文案，页面据此切带登录入口的引导态）。
        const contentType = resp.headers.get("content-type") || "";
        if (!contentType.includes("text/event-stream")) {
            let msg = "导购服务暂不可用";
            try {
                const data = JSON.parse(await resp.text()) as { code?: number; msg?: string; message?: string };
                if (data?.code === 401) msg = GUIDE_LOGIN_EXPIRED_MSG;
                else if (data?.msg) msg = String(data.msg);
                else if (data?.message) msg = String(data.message);
            } catch {
                /* 非 JSON 响应：用默认文案 */
            }
            handlers.onError?.(msg);
            handlers.onDone?.(false);
            return;
        }
        const reader = resp.body.getReader();
        const decoder = new TextDecoder("utf-8");
        let buffer = "";
        let finished = false;
        const dispatch = (block: string) => {
            let event = "";
            const dataLines: string[] = [];
            for (const line of block.split("\n")) {
                if (line.startsWith("event:")) event = line.slice(6).trim();
                else if (line.startsWith("data:")) dataLines.push(line.slice(5).trim());
            }
            if (!event || dataLines.length === 0) return;
            let payload: any = {};
            try {
                payload = JSON.parse(dataLines.join("\n"));
            } catch {
                return;
            }
            switch (event) {
                case "assistant":
                    handlers.onAssistant?.(payload.msgId, payload.convId);
                    break;
                case "delta":
                    handlers.onDelta?.(payload.content || "");
                    break;
                case "tool":
                    handlers.onTool?.(payload.name || "", payload.status || "", payload.cards || []);
                    break;
                case "error":
                    handlers.onError?.(payload.message || "导购服务异常");
                    break;
                case "done":
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
                let idx = buffer.indexOf("\n\n");
                while (idx >= 0) {
                    dispatch(buffer.slice(0, idx));
                    buffer = buffer.slice(idx + 2);
                    idx = buffer.indexOf("\n\n");
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
        abort: () => controller.abort()
    };
}
