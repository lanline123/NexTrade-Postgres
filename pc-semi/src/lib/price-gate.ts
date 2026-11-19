'use client';

/**
 * 价格门控前端态（询价线第 2 期，我方新增）。
 *
 * 后端谓词（services::system::price_gate）：isIdentity 开启时，仅
 * 已登录且 `is_company_auth = 1` 的会员能看真实价格；公开读接口在
 * 无看价权时把价格置空并在载荷上打 `priceVisible=false` +
 * `priceGate=login|certification`。前端据此渲染门控文案：
 *
 *   正常价 / 询价（询价商品全员一致，不走本模块）
 *   登录查看              → /login
 *   实名查看              → /member/usercertification
 *
 * 文案口径（2026-11-20 对齐原版）：原版 tigshop 共享价格组件
 * （view/uniapp format-price.vue、view/pc format/Price.vue）在
 * isIdentity=1 且未认证时把整个价格位换成一句文案——未登录
 * 「登录查看」、已登录「实名查看」，不分审核中/驳回子态。
 * 我方第 2 期的自造文案（登录后查看价格/认证后查看价格/认证审核中/
 * 认证未通过，查看原因）自此收敛：子态一律落「实名查看」，
 * 仅跳转目标保留（均指向认证页）。原版文案本身无点击跳转，
 * 我方保留第 2 期既有链接形态（卡片/详情购买区），属呈现层增强，
 * 不改变门控数据口径（未认证者仍拿不到明文价格）。
 *
 * 载荷标记是权威信号（列表/搜索卡直接读 priceVisible/priceGate）；
 * 本 hook 只在详情页这类需要按访客态细化时补查
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
  needLogin: { text: '登录查看', href: '/login' },
  needAuth: { text: '实名查看', href: '/member/usercertification' },
  pending: { text: '实名查看', href: '/member/usercertification' },
  rejected: { text: '实名查看', href: '/member/usercertification' },
};

/** 载荷 priceGate（login|certification）→ 卡片级文案（卡片不查子态）。 */
export function gateCopyForPayload(priceGate?: string | null): PriceGateCopy {
  return priceGate === 'login'
    ? PRICE_GATE_COPY.needLogin
    : PRICE_GATE_COPY.needAuth;
}

/**
 * 装修卡片门控文案解析（首页各装修模块卡片共用）：
 * - 载荷带 `priceVisible === false`（接口已置空价格）→ 门控文案。文案按当前
 *   访客态细化：装修取数多为匿名请求、priceGate 恒为 login，已登录未认证时
 *   以 usePriceAccess 的子态为准；匿名/未解析时与载荷口径一致；
 * - 无标记的存储型快照（装修内嵌商品快照 price）：价格被递归脱敏置空
 *   （null/undefined/''）且当前访客无看价权 → 同样门控；
 * - 其余（有价可见、门控关、已认证）返回 null，调用方正常渲染价格，
 *   绝不把脱敏价渲染成 ¥0.00 或空白。
 */
export function cardGateCopy(
  item: { priceVisible?: boolean; priceGate?: string | null } | null | undefined,
  priceValue: unknown,
  state: PriceAccessState,
  inquiryAll: boolean,
): PriceGateCopy | null {
  if (inquiryAll) return null;
  if (item?.priceVisible === false) {
    return state !== 'visible' ? PRICE_GATE_COPY[state] : gateCopyForPayload(item.priceGate);
  }
  const missing = priceValue === null || priceValue === undefined || priceValue === '';
  if (missing && state !== 'visible') return PRICE_GATE_COPY[state];
  return null;
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
