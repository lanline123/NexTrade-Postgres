'use client';

/**
 * 价格门控前端态（询价线第 2 期，我方新增）。
 *
 * 后端谓词（services::system::price_gate）：isIdentity 开启时，仅
 * 已登录且 `is_company_auth = 1` 的会员能看真实价格；公开读接口在
 * 无看价权时把价格置空并在载荷上打 `priceVisible=false` +
 * `priceGate=login|certification`。前端据此渲染四态文案：
 *
 *   正常价 / 询价（询价商品全员一致，不走本模块）
 *   登录后查看价格        → /login
 *   认证后查看价格        → /member/usercertification
 *   认证审核中            → /member/usercertification（子态）
 *   认证未通过，查看原因  → /member/usercertification（子态）
 *
 * 载荷标记是权威信号（列表/搜索卡直接读 priceVisible/priceGate）；
 * 本 hook 只在详情页这类需要子态文案（审核中/驳回）时补查
 * memberCenter（isCompanyAuth）与企业认证申请状态（company/detail：
 * status 0 待审核 / 1 通过 / 2 驳回，与 CertificationClient 同口径）。
 */
import { useEffect, useState } from 'react';
import { apiGet } from '@/lib/client-api';
import { useAuthGate } from '@/lib/auth-gate';

export type PriceAccessState =
  | 'visible'
  | 'needLogin'
  | 'needAuth'
  | 'pending'
  | 'rejected';

export interface PriceGateCopy {
  text: string;
  href: string;
}

export const PRICE_GATE_COPY: Record<
  Exclude<PriceAccessState, 'visible'>,
  PriceGateCopy
> = {
  needLogin: { text: '登录后查看价格', href: '/login' },
  needAuth: { text: '认证后查看价格', href: '/member/usercertification' },
  pending: { text: '认证审核中', href: '/member/usercertification' },
  rejected: { text: '认证未通过，查看原因', href: '/member/usercertification' },
};

/** 载荷 priceGate（login|certification）→ 卡片级文案（卡片不查子态）。 */
export function gateCopyForPayload(priceGate?: string | null): PriceGateCopy {
  return priceGate === 'login'
    ? PRICE_GATE_COPY.needLogin
    : PRICE_GATE_COPY.needAuth;
}

export function hasAccessToken(): boolean {
  return (
    typeof window !== 'undefined' &&
    Boolean(localStorage.getItem('accessToken'))
  );
}

let cachedLoggedIn: PriceAccessState | null = null;
let inflight: Promise<PriceAccessState> | null = null;

/** 已登录用户的门控子态解析（模块级缓存，同页多组件共用）。 */
export function resolveLoggedInPriceAccess(): Promise<PriceAccessState> {
  if (cachedLoggedIn) return Promise.resolve(cachedLoggedIn);
  if (!inflight) {
    inflight = (async (): Promise<PriceAccessState> => {
      try {
        const center = await apiGet<Record<string, any>>(
          'user/user/memberCenter',
        );
        if (Number(center?.isCompanyAuth) === 1) return 'visible';
      } catch {
        /* 继续查认证申请状态 */
      }
      try {
        const company = await apiGet<Record<string, any>>(
          'user/company/detail',
        );
        if (company && company.status !== undefined && company.status !== null) {
          const status = Number(company.status);
          if (status === 0) return 'pending';
          if (status === 2) return 'rejected';
          if (status === 1) return 'visible';
        }
      } catch {
        /* 无申请记录 → 未认证 */
      }
      return 'needAuth';
    })().then((state) => {
      cachedLoggedIn = state;
      return state;
    });
  }
  return inflight;
}

/**
 * 当前访客的价格门控态：门控关 → visible；未登录 → needLogin；
 * 已登录 → 子态解析（visible / needAuth / pending / rejected）。
 */
export function usePriceAccess(): PriceAccessState {
  const gate = useAuthGate();
  const [state, setState] = useState<PriceAccessState>('visible');
  useEffect(() => {
    if (gate.isIdentity !== 1) {
      setState('visible');
      return undefined;
    }
    if (!hasAccessToken()) {
      setState('needLogin');
      return undefined;
    }
    let alive = true;
    void resolveLoggedInPriceAccess().then((resolved) => {
      if (alive) setState(resolved);
    });
    return () => {
      alive = false;
    };
  }, [gate.isIdentity]);
  return state;
}
