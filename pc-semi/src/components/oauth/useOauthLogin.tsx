'use client';

/**
 * 第三方 OAuth 登录（Google / Facebook / Twitter-X）。
 *
 * 源码对照：
 * - java-b2b2c-overseas-v5.8.61/view/pc/components/member/login/OverseasLogin.vue:117-127
 * - java-b2b2c-overseas-v5.8.61/view/pc/pages/member/Register.vue:192-203
 *
 * 流程（后端已实现，前端此前零调用）：
 * 1. GET /api/user/oauth/render/{source}?redirectUri=...&platform=pc → 授权 URL
 *    （crates/api/public/src/routes/member/oauth.rs；state 前缀编码了 provider）
 * 2. 跳转到第三方授权页
 * 3. 第三方回调 /oauth/callback?code=..&state=.. → app/oauth/callback/page.tsx
 *    POST /api/user/oauth/callback/{source} → token
 *
 * 开关来自后台配置 common/config/base 的 googleLoginOn / facebookLoginOn / twitterLoginOn。
 * 本仓库为跨境版 PC，源码 isOverseas() 恒为 true，故不做海外站判定。
 */
import { useCallback, useEffect, useState } from 'react';
import { Toast } from '@douyinfe/semi-ui';

import { useTranslation } from 'react-i18next';

export type OauthSource = 'google' | 'facebook' | 'twitter';

export interface OauthSwitches {
  google: boolean;
  facebook: boolean;
  twitter: boolean;
}

export function useOauthLogin(): {
  switches: OauthSwitches;
  start: (source: OauthSource) => void;
  anyEnabled: boolean;
} {
    const { t } = useTranslation();
  const [switches, setSwitches] = useState<OauthSwitches>({ google: false, facebook: false, twitter: false });

  useEffect(() => {
    let alive = true;
    fetch('/api/common/config/base', { credentials: 'include' })
      .then((response) => response.json())
      .then((json) => {
        if (!alive) return;
        const data = (json?.data ?? json) as Record<string, unknown>;
        const on = (key: string) => {
          const value = data?.[key];
          return value === 1 || value === '1' || value === true;
        };
        setSwitches({
          google: on('googleLoginOn'),
          facebook: on('facebookLoginOn'),
          twitter: on('twitterLoginOn'),
        });
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, []);

  const start = useCallback(
    (source: OauthSource) => {
      (async () => {
        try {
          const redirectUri = `${window.location.origin}/oauth/callback`;
          // 后端 render 只认 returnUrl（RenderQuery），并把它写进 state 供回调页
          // 回跳。此前前端只传后端不读的 redirectUri、且不透传当前页的 redirect
          // 参数，OAuth 登录成功后恒回首页、丢失来路。只透传站内路径防开放重定向。
          const current = new URLSearchParams(window.location.search);
          const rawReturn = current.get('redirect') || current.get('returnUrl') || '/member';
          const returnUrl =
            rawReturn.startsWith('/') && !rawReturn.startsWith('//') && !rawReturn.includes('\\')
              ? rawReturn
              : '/member';
          const response = await fetch(
            `/api/user/oauth/render/${source}?redirectUri=${encodeURIComponent(redirectUri)}&returnUrl=${encodeURIComponent(returnUrl)}&platform=pc`,
            { credentials: 'include' },
          );
          const json = await response.json();
          if (!response.ok || (json.code !== undefined && json.code !== 0)) {
            throw new Error(json.msg || json.message || t('第三方登录暂不可用'));
          }
          const url = typeof json?.data === 'string' ? json.data : String(json?.data?.url || '');
          if (!url) throw new Error(t('第三方登录暂不可用'));
          window.location.href = url;
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('第三方登录暂不可用') });
        }
      })();
    },
    [t],
  );

  return { switches, start, anyEnabled: switches.google || switches.facebook || switches.twitter };
}

/**
 * 第三方登录按钮组（登录页 / 注册页共用）。
 * 只渲染品牌图标，不显示品牌文字（用户 2026-11-15 指示）；品牌名保留在
 * 按钮的 aria-label / title 里，供无障碍与悬停提示使用。
 */
export function OauthButtons({
  switches,
  onStart,
}: {
  switches: OauthSwitches;
  onStart: (source: OauthSource) => void;
}) {
    const { t } = useTranslation();
  if (!switches.google && !switches.facebook && !switches.twitter) return null;
  return (
    <div className="pdp-login__oauth">
      {switches.google ? (
        <button type="button" className="pdp-login__oauth-item" aria-label="Google" title="Google" onClick={() => onStart('google')}>
          <img className="pdp-login__oauth-logo" src="/assets/images/user/login/google.png" alt="" />
        </button>
      ) : null}
      {switches.facebook ? (
        <button type="button" className="pdp-login__oauth-item" aria-label="Facebook" title="Facebook" onClick={() => onStart('facebook')}>
          <img className="pdp-login__oauth-logo" src="/assets/images/user/login/Facebook.png?v=2" alt="" />
        </button>
      ) : null}
      {switches.twitter ? (
        <button type="button" className="pdp-login__oauth-item" aria-label="Twitter/X" title="Twitter/X" onClick={() => onStart('twitter')}>
          <img className="pdp-login__oauth-logo" src="/assets/images/user/login/twitter.svg" alt="" />
        </button>
      ) : null}
    </div>
  );
}
