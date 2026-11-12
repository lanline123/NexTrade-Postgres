/**
 * 服务端取数（RSC / route handler 用）。
 *
 * 与 `lib/client-api.ts` 的区别：
 * - client-api 依赖 `localStorage` 里的 token，**只能在浏览器跑**；
 * - 这里的接口都是公开的（首页装修、商品详情等），服务端直连 `PC_API_BASE`，不带任何凭证，
 *   拿到数据后作为 props 交给客户端组件，首屏 HTML 里就是真内容（真正的 SSR）。
 *
 * 失败一律返回 null，由调用方决定兜底（通常是客户端再取一次），不让首页白屏。
 */
const API_BASE = (process.env.PC_API_BASE || 'https://battery.recy.cc').replace(/\/$/, '');

export async function serverGet<T>(path: string, params: Record<string, string | number | undefined> = {}): Promise<T | null> {
    const query = new URLSearchParams();
    Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined && value !== null && value !== '') query.set(key, String(value));
    });
    const suffix = query.toString() ? `?${query.toString()}` : '';
    try {
        const response = await fetch(`${API_BASE}/api/${path}${suffix}`, {
            headers: { Accept: 'application/json' },
            // 首页装修会经常改，但也没必要每请求都打后端；交给页面级 revalidate 控制。
            next: { revalidate: 60 }
        });
        if (!response.ok) return null;
        const json = (await response.json()) as { code?: number; data?: T };
        if (typeof json?.code === 'number' && json.code !== 0) return null;
        return (json?.data ?? (json as unknown as T)) as T;
    } catch {
        return null;
    }
}

/** 服务端 POST（公开接口用，例如商品详情若将来改成 POST）。 */
export async function serverPost<T>(path: string, body: unknown): Promise<T | null> {
    try {
        const response = await fetch(`${API_BASE}/api/${path}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
            body: JSON.stringify(body ?? {}),
            next: { revalidate: 60 }
        });
        if (!response.ok) return null;
        const json = (await response.json()) as { code?: number; data?: T };
        if (typeof json?.code === 'number' && json.code !== 0) return null;
        return (json?.data ?? (json as unknown as T)) as T;
    } catch {
        return null;
    }
}
