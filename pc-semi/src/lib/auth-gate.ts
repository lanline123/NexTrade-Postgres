'use client';

/**
 * 认证门控配置（GET /api/common/config/authGate）。
 *
 * 后台「会员认证设置」的公众下发口：closeAuth（认证总开关）/ isIdentity
 * （实名后才能查看价格）/ type（1 仅企业 / 0 个人+企业）。
 * 接口未上线或取数失败时一律回落全关（closeAuth=0），前台按「未开启认证」
 * 渲染——认证入口隐藏、价格不门控，与现行行为一致。
 *
 * 模块级缓存：同页多个组件（会员导航、顶栏等）共用一次请求。
 */
import { useEffect, useState } from 'react';
import { apiGet } from '@/lib/client-api';

export interface AuthGateConfig {
  /** 认证总开关：1 开启（会员实名认证入口显示）/ 0 关闭 */
  closeAuth: number;
  /** 是否实名后才能查看价格：1 是 / 0 否 */
  isIdentity: number;
  /** 可申请的认证类型：1 仅企业 / 0 个人+企业 */
  type: number;
}

export const AUTH_GATE_FALLBACK: AuthGateConfig = { closeAuth: 0, isIdentity: 0, type: 0 };

let cached: AuthGateConfig | null = null;
let inflight: Promise<AuthGateConfig> | null = null;

export function fetchAuthGate(): Promise<AuthGateConfig> {
  if (cached) return Promise.resolve(cached);
  if (!inflight) {
    inflight = apiGet<Record<string, any>>('common/config/authGate')
      .then((data) => {
        cached = {
          closeAuth: Number(data?.closeAuth ?? 0) === 1 ? 1 : 0,
          isIdentity: Number(data?.isIdentity ?? 0) === 1 ? 1 : 0,
          type: Number(data?.type ?? 0),
        };
        return cached;
      })
      .catch(() => AUTH_GATE_FALLBACK);
  }
  return inflight;
}

export function useAuthGate(): AuthGateConfig {
  const [gate, setGate] = useState<AuthGateConfig>(() => cached ?? AUTH_GATE_FALLBACK);
  useEffect(() => {
    let alive = true;
    void fetchAuthGate().then((config) => {
      if (alive) setGate(config);
    });
    return () => {
      alive = false;
    };
  }, []);
  return gate;
}
