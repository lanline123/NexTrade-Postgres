export interface PosSession {
  token: string;
  adminType: string;
  shopId: string | null;
}

export function readSession(): PosSession | null {
  const token = localStorage.getItem('accessToken');
  if (!token) return null;
  return {
    token,
    adminType: localStorage.getItem('adminType') || 'pos',
    shopId: localStorage.getItem('shopId'),
  };
}
