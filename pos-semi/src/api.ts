const API_BASE = '/adminapi';

export class ApiError extends Error {
  readonly code: number;

  constructor(code: number, message: string) {
    super(message);
    this.name = 'ApiError';
    this.code = code;
  }
}

type Query = Record<string, string | number | undefined>;
type JsonBody = { code?: number; data?: unknown; message?: string; msg?: string };

function buildQuery(query?: Query): string {
  if (!query) return '';
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query)) {
    if (value !== undefined && value !== '') params.set(key, String(value));
  }
  const text = params.toString();
  return text ? `?${text}` : '';
}

function requestHeaders(): Record<string, string> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    'X-ADMIN-TYPE': 'pos',
    'X-CLIENT-TYPE': 'admin',
  };
  const token = localStorage.getItem('accessToken');
  const shopId = localStorage.getItem('shopId');
  if (token) headers.Authorization = `Bearer ${token}`;
  if (shopId) headers['X-SHOP-ID'] = shopId;
  return headers;
}

async function request<T>(method: 'GET' | 'POST', path: string, body?: unknown, query?: Query): Promise<T> {
  const res = await fetch(`${API_BASE}${path}${buildQuery(query)}`, {
    method,
    headers: requestHeaders(),
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await res.text();
  let json: JsonBody = {};
  try {
    json = text ? (JSON.parse(text) as JsonBody) : {};
  } catch {
    throw new ApiError(res.status, text || `HTTP ${res.status}`);
  }
  if (!res.ok) throw new ApiError(res.status, json.message || json.msg || `HTTP ${res.status}`);
  if (json.code !== 0) throw new ApiError(json.code ?? -1, json.message || json.msg || '请求失败');
  return json.data as T;
}

export const api = {
  get: <T>(path: string, query?: Query) => request<T>('GET', path, undefined, query),
  post: <T>(path: string, body?: unknown) => request<T>('POST', path, body),
};
