export interface PosSession {
  token: string;
  adminType: string;
  shopId: string | null;
  username: string;
}

function tokenPayload(token: string): Record<string, unknown> {
  try {
    const raw = token.split('.')[1] || '';
    return JSON.parse(atob(raw.replace(/-/g, '+').replace(/_/g, '/'))) as Record<string, unknown>;
  } catch {
    return {};
  }
}

export function readSession(): PosSession | null {
  const token = localStorage.getItem('accessToken');
  if (!token) return null;
  const payload = tokenPayload(token);
  return {
    token,
    adminType: localStorage.getItem('adminType') || 'pos',
    shopId: localStorage.getItem('shopId'),
    username: String(payload.username || payload.user_name || payload.nickname || 'admin'),
  };
}
