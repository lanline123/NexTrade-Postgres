/**
 * 浏览器侧 API 客户端。
 * 口径来自 Vue 版 utils/request.ts：GET/POST 走 /api/<path>，Bearer token 取 localStorage，
 * 响应是 { code, data, msg } 信封；code=0 才算成功。SSR 不出请求，页面由客户端按需取数。
 */
type Envelope<T> = { code?: number; msg?: string; message?: string; data?: T };

function authHeaders(): Record<string, string> {
    if (typeof window === 'undefined') return {};
    const token = localStorage.getItem('accessToken') || localStorage.getItem('token');
    return token ? { Authorization: `Bearer ${token}` } : {};
}

async function read<T>(response: Response): Promise<T> {
    const text = await response.text();
    let json: unknown;
    try {
        json = text ? JSON.parse(text) : {};
    } catch {
        throw new Error(`接口返回格式异常（HTTP ${response.status}）`);
    }
    const envelope = json as Envelope<T>;
    if (!response.ok || (envelope.code !== undefined && envelope.code !== 0)) {
        throw new Error(envelope.msg || envelope.message || `请求失败（HTTP ${response.status}）`);
    }
    return (envelope.data ?? json) as T;
}

export function apiGet<T>(path: string, params: Record<string, unknown> = {}): Promise<T> {
    const query = new URLSearchParams();
    for (const [key, value] of Object.entries(params)) {
        if (value !== undefined && value !== null && value !== '') query.set(key, String(value));
    }
    const suffix = query.toString() ? `?${query.toString()}` : '';
    return fetch(`/api/${path}${suffix}`, { credentials: 'include', headers: authHeaders() }).then(read<T>);
}

export function apiPost<T>(path: string, body?: unknown): Promise<T> {
    return fetch(`/api/${path}`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json', ...authHeaders() },
        body: body === undefined ? undefined : JSON.stringify(body)
    }).then(read<T>);
}

/** 列表接口统一形状（Vue 版 BaseResponseListWrap）。 */
export interface Paged<T> {
    records?: T[];
    total?: number;
}
