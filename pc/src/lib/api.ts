// API 地址由 next.config.mjs 的 env 注入，服务端与浏览器同一套口径。
const API_URL = (process.env.PC_API_BASE || '').replace(/\/$/, '');
const API_PREFIX = process.env.PC_API_PREFIX || '/api';

export function apiPath(path: string): string {
    const normalized = path.startsWith('/') ? path : `/${path}`;
    return `${API_URL}${API_PREFIX}${normalized}`;
}

/** 服务端组件里可直接 await；客户端组件里也可调用。 */
export async function request<T>(path: string, init?: RequestInit): Promise<T> {
    const response = await fetch(apiPath(path), {
        ...init,
        headers: {
            'Content-Type': 'application/json',
            ...(init?.headers ?? {})
        }
    });
    if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    return response.json() as Promise<T>;
}
