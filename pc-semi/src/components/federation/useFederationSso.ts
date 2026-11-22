'use client';

/**
 * 联邦登录（我方新增，万象互联三期① 前台接缝；成员站登录页入口）。
 *
 * 流程（后端三期①已实现，本文件只做前台接线）：
 * 1. 显隐探针：GET /api/federation/sso/options（匿名、只读无副作用）
 *    → { available, peerName }。本站未启用联邦 / 会员模块未开 / 无可用
 *    中枢连接时 available=false，入口不渲染；探针失败同样隐藏
 *    （fail-soft，绝不影响本站账号登录主链）。
 * 2. 发起：POST /api/federation/sso/start { callbackUrl }（不传 linkId，
 *    后端解析本站唯一可用成员连接）→ { authorizeUrl } → 整页跳中枢
 *    授权页（本站 /federation/authorize 对侧页）。
 * 3. 中枢同意后浏览器回跳本站 /federation/callback?code=&state=
 *    （拒绝时回带 federationSso=denied），落地页调 callback 换本站令牌。
 */
import { useCallback, useEffect, useState } from 'react';
import { Toast } from '@douyinfe/semi-ui';

import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';

interface SsoOptions {
  available: boolean;
  peerName: string;
}

export function useFederationSso(): {
  available: boolean;
  peerName: string;
  starting: boolean;
  start: () => void;
} {
  const { t } = useTranslation();
  const [options, setOptions] = useState<SsoOptions>({ available: false, peerName: '' });
  const [starting, setStarting] = useState(false);

  useEffect(() => {
    let alive = true;
    apiGet<SsoOptions>('federation/sso/options')
      .then((data) => {
        if (alive && data) {
          setOptions({ available: !!data.available, peerName: data.peerName || '' });
        }
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, []);

  const start = useCallback(() => {
    if (starting) return;
    setStarting(true);
    (async () => {
      try {
        const callbackUrl = `${window.location.origin}/federation/callback`;
        const data = await apiPost<{ authorizeUrl: string }>('federation/sso/start', { callbackUrl });
        if (!data?.authorizeUrl) throw new Error(t('联邦登录暂不可用'));
        window.location.href = data.authorizeUrl;
      } catch (error) {
        Toast.error({ content: error instanceof Error ? error.message : t('联邦登录暂不可用') });
        setStarting(false);
      }
    })();
  }, [starting, t]);

  return { available: options.available, peerName: options.peerName, starting, start };
}

/**
 * 联邦登录入口按钮（登录页社交登录行）。纯文字按钮（联邦无品牌图标
 * 资产），样式与同排第三方图标按钮同口径（auth-modern.scss
 * .login-card__federation）；中枢名进 title 悬停提示。
 */
export function FederationLoginButton({
  peerName,
  starting,
  onStart,
}: {
  peerName: string;
  starting: boolean;
  onStart: () => void;
}) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      className="login-card__federation"
      disabled={starting}
      onClick={onStart}
      title={peerName ? t('使用{peerName}账号登录', { peerName }) : t('联邦登录')}
    >
      {starting ? t('正在跳转...') : t('联邦登录')}
    </button>
  );
}
