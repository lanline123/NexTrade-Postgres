'use client';

/**
 * 联邦登录回调落地页（我方新增，万象互联三期① 前台接缝）。
 *
 * 路由 /federation/callback：成员站发起时登记给中枢的 returnUrl 就是
 * 本页地址。中枢同意后回跳带 ?code=&state=，本页调
 * POST /api/federation/sso/callback 换本站普通会员令牌（落号三级：
 * 映射 → 手机号既有号 → 新建影子账号，后端处理），存令牌口径与
 * 第三方 OAuth 回调页一致（localStorage + cookie），随后进会员中心。
 * 拒绝授权时回跳带 ?federationSso=denied，本页如实提示并给回登录页
 * 入口；code/state 无效等错误展示后端原文案，不伪造登录态。
 */
import { useEffect, useState } from 'react';

import { useTranslation } from 'react-i18next';
import { apiPost } from '@/lib/client-api';

interface SsoCallbackResult {
  token?: string;
  accessToken?: string;
  userId?: number;
  created?: boolean;
}

export default function FederationCallbackPage() {
  const { t } = useTranslation();
  const [message, setMessage] = useState('正在完成联邦登录...');

  useEffect(() => {
    const run = async () => {
      const params = new URLSearchParams(window.location.search);
      if (params.get('federationSso') === 'denied') {
        setMessage('你已拒绝联邦授权，可使用本站账号密码登录');
        return;
      }
      const code = params.get('code') || '';
      const state = params.get('state') || '';
      if (!code || !state) {
        setMessage('联邦登录参数缺失，请重新从登录页发起');
        return;
      }
      try {
        const data = await apiPost<SsoCallbackResult>('federation/sso/callback', { code, state });
        const token = data?.token || data?.accessToken;
        if (!token) throw new Error('联邦登录未返回令牌');
        localStorage.setItem('accessToken', token);
        document.cookie = `accessToken=${encodeURIComponent(token)}; path=/; max-age=604800; SameSite=Lax`;
        window.location.replace('/member');
      } catch (reason) {
        setMessage(reason instanceof Error ? reason.message : '联邦登录失败');
      }
    };
    run();
  }, []);

  return (
    <main className="pc-static-page">
      <div className="pc-container">
        <div className="pc-static-card">
          <b>{message}</b>
          <p>
            <a href="/login">{t('返回登录')}</a>
          </p>
        </div>
      </div>
    </main>
  );
}
