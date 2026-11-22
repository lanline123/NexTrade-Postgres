'use client';

/**
 * 联邦登录授权页（我方新增，万象互联三期① 前台接缝）。
 *
 * 路由约定：中枢后端在成员站发起时回的 authorizeUrl =
 * `{中枢 instanceBaseUrl}/federation/authorize?requestId=<id>`，会员浏览器
 * 跳到本页完成同意/拒绝（设计 ~/workspace/federation-phase3/DESIGN.md §1.2
 * 第 2 步）。同意必须由会员本人持本站 JWT 操作，联邦钥永不代签：
 * - 未登录 → 跳本站登录页，登录后按 redirect 参数回到本页；
 * - `GET /api/federation/sso/requestInfo?requestId=`（会员 JWT）取申请方信息；
 * - `POST /api/federation/sso/consent {requestId, agree}`（会员 JWT），
 *   成功回 redirectUrl（成员站回调地址，同意带 code+state、拒绝带
 *   federationSso=denied），浏览器整页跳回成员站；
 * - 当前账号手机号未验证时后端明确拒绝（「当前账号手机号未验证，不能
 *   用于联邦登录」），页面如实展示并引导先去会员中心完成手机验证，
 *   不伪造同意结果。
 */
import { useEffect, useState } from 'react';

import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';
import { formatDateTime } from '@/lib/datetime';

interface SsoRequestInfo {
  requestId: number;
  peerName: string;
  peerBaseUrl: string;
  /** 0 待同意 / 1 已同意 / 2 已拒绝 / 3 已过期 */
  status: number;
  addTime: number;
}

const FINAL_STATUS_TEXT: Record<number, string> = {
  1: '该授权请求已同意，无需重复操作',
  2: '该授权请求已被拒绝',
  3: '该授权请求已过期，请回到对方站点重新发起',
};

export default function FederationAuthorizeClient() {
  const { t } = useTranslation();
  const [info, setInfo] = useState<SsoRequestInfo | null>(null);
  const [error, setError] = useState('');
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const requestId = Number(params.get('requestId') || 0);
    if (!requestId) {
      setError('授权请求参数缺失，请回到对方站点重新发起联邦登录');
      return;
    }
    // 同意面必须会员本人登录：无令牌先去登录，登录后按 redirect 回本页。
    if (!localStorage.getItem('accessToken')) {
      const back = `/federation/authorize?requestId=${requestId}`;
      window.location.replace(`/login?redirect=${encodeURIComponent(back)}`);
      return;
    }
    let alive = true;
    apiGet<SsoRequestInfo>('federation/sso/requestInfo', { requestId })
      .then((data) => {
        if (alive) setInfo(data);
      })
      .catch((reason) => {
        if (alive) setError(reason instanceof Error ? reason.message : '授权请求加载失败');
      });
    return () => {
      alive = false;
    };
  }, []);

  const consent = async (agree: boolean) => {
    if (!info || submitting) return;
    setSubmitting(true);
    setError('');
    try {
      const data = await apiPost<{ redirectUrl: string }>('federation/sso/consent', {
        requestId: info.requestId,
        agree,
      });
      if (!data?.redirectUrl) throw new Error('授权处理未返回回跳地址');
      window.location.href = data.redirectUrl;
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '授权处理失败');
      setSubmitting(false);
    }
  };

  if (error && !info) {
    return (
      <>
        <b>{error}</b>
        <p>
          <a href="/">{t('返回首页')}</a>
        </p>
      </>
    );
  }
  if (!info) {
    return <b>{t('正在加载授权请求...')}</b>;
  }

  const pending = info.status === 0;
  return (
    <>
      <b>{t('联邦登录授权')}</b>
      <p>
        {t('站点')}
        <strong>{info.peerName || info.peerBaseUrl}</strong>
        {t('正在请求使用你的平台账号登录。同意后，该站点将获得你的全局会员身份（手机号、昵称、头像），不包含密码等任何凭据。')}
      </p>
      <p>
        {t('申请方地址')}：{info.peerBaseUrl}
        <br />
        {t('发起时间')}：{formatDateTime(info.addTime)}
      </p>
      {error ? (
        <p style={{ color: '#d92d20' }}>
          {error}
          {error.includes('手机号未验证') ? (
            <>
              {' '}
              <a href="/member">{t('去会员中心完成手机验证')}</a>
            </>
          ) : null}
        </p>
      ) : null}
      {pending ? (
        <p style={{ display: 'flex', gap: 12 }}>
          <button
            type="button"
            disabled={submitting}
            onClick={() => void consent(true)}
            style={{
              padding: '8px 28px',
              border: 'none',
              borderRadius: 4,
              background: 'var(--main-bg, #0d6efd)',
              color: '#fff',
              cursor: 'pointer',
            }}
          >
            {submitting ? t('处理中...') : t('同意授权')}
          </button>
          <button
            type="button"
            disabled={submitting}
            onClick={() => void consent(false)}
            style={{
              padding: '8px 28px',
              border: '1px solid #d0d5dd',
              borderRadius: 4,
              background: '#fff',
              color: '#344054',
              cursor: 'pointer',
            }}
          >
            {t('拒绝')}
          </button>
        </p>
      ) : (
        <p>{t(FINAL_STATUS_TEXT[info.status] || '该授权请求已处理')}</p>
      )}
    </>
  );
}
