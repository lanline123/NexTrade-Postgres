'use client';

/**
 * PC 注册（与登录页同构的淘宝式双栏卡片改版）
 *
 * 布局与 LoginClient 完全同款：浅灰页面底 + 悬浮顶栏（logo + 欢迎语 +
 * 「已有账号？去登录」）+ 居中白色圆角卡（阴影），卡内左右双栏——
 * 左栏「手机扫码」（二维码指向移动端 H5 /mobile），右栏「手机注册 /
 * 邮箱注册」双页签表单；底部一行左侧微信入口 + 第三方注册。
 * 配色用全站主题色（--general），样式复用 app/auth-modern.scss 的
 * .pg-login-card / .login-card__* 段，注册页特有差异在 .pg-register-card 段。
 *
 * 源码对照：java-b2b2c-overseas-v5.8.61/view/pc/pages/member/Register.vue
 * - 手机号注册 / 邮箱注册切换
 * - 短信 / 邮箱验证码（60s 倒计时，源码 MobileCode）
 * - 注册协议与隐私政策（旧版无勾选框，视为已同意）
 * - 邀请码选填
 * 接口：POST /api/user/login/sendMobileCode
 *       POST /api/user/login/sendEmailCode
 *       POST /api/user/regist/registAct
 *       GET  /api/common/csrf/create
 */
import { useEffect, useRef, useState } from 'react';
import Link from 'next/link';
import { Toast } from '@douyinfe/semi-ui';
import type { PcLoginDecorate } from '@/lib/api';
import { imageFormat } from '@/lib/image';

import { useTranslation } from 'react-i18next';
import i18n from '@/i18n';
import { useOauthLogin, OauthButtons } from '@/components/oauth/useOauthLogin';
import VerifySlide from '@/components/VerifySlide';
async function getCsrf(): Promise<string> {
  const response = await fetch('/api/common/csrf/create', { credentials: 'include' });
  const json = await response.json();
  if (!response.ok || (json.code !== undefined && json.code !== 0)) {
    throw new Error(json.msg || (i18n.t('获取 CSRF Token 失败') as string));
  }
  return typeof json.data === 'string' ? json.data : json.data?.token || '';
}

type Mode = 'mobile' | 'email' | 'wechat';

/** 密码可见切换的眼睛图标（纯展示，本地 state 控制） */
function EyeIcon({ off = false }: { off?: boolean }) {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7Z" />
      <circle cx="12" cy="12" r="3" />
      {off ? <path d="M3 3l18 18" /> : null}
    </svg>
  );
}

export default function RegisterClient({
  shopName = '万物新生',
  shopLogo = '',
}: {
  /** 店铺名称（logo 缺省时展示文字），来自 common/config/base */
  shopName?: string;
  /** 店铺 logo（顶栏展示） */
  shopLogo?: string;
  /** 登录页装修配置（页面保留传参兼容；背景图/背景色不使用，与登录页一致） */
  decorate?: PcLoginDecorate | null;
}) {
    const { t } = useTranslation();
  /** 第三方注册入口（源码 pages/member/Register.vue:192-203） */
  const { switches: oauthSwitches, start: startOauth } = useOauthLogin();
  const [mode, setMode] = useState<Mode>('mobile');
  const [mobile, setMobile] = useState('');
  const [email, setEmail] = useState('');
  const [code, setCode] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [nickname, setNickname] = useState('');
  const [inviteCode, setInviteCode] = useState('');
  // 旧版 register.vue 只有一行「点击注册，表示您同意《注册协议》」文字，没有勾选框，
  // 所以这里直接视为已同意，保留这个 state 只为兼容原有校验逻辑。
  const [agree, setAgree] = useState(true);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [countdown, setCountdown] = useState(0);
  const [sending, setSending] = useState(false);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  /**
   * 滑块行为验证码（与 LoginClient 同款）：发码前主动弹出，verifyToken 随发码请求提交；
   * 后端返回 code=1002 时重弹一次带新 token 重试（codeRetryRef 防死循环）。
   * verifyToken 不入 state（一次性，回调参数直传）。
   */
  const [verifyOpen, setVerifyOpen] = useState(false);
  const codeRetryRef = useRef(false);
  /** 源码 Register.vue：微信扫码注册（getWxLoginUrl + wechatEvent 轮询 2s/60s） */
  const [wechatQr, setWechatQr] = useState('');
  const [wechatExpired, setWechatExpired] = useState(false);
  const wechatTimer = useRef<ReturnType<typeof setInterval> | null>(null);
  const wechatTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);
  /** 左栏手机扫码二维码（指向移动端 H5，与登录页同款生成） */
  const [mobileQr, setMobileQr] = useState('');
  /** 密码可见切换（纯展示，不影响提交逻辑） */
  const [showPassword, setShowPassword] = useState(false);
  const [showConfirm, setShowConfirm] = useState(false);

  useEffect(() => {
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
      if (wechatTimer.current) clearInterval(wechatTimer.current);
      if (wechatTimeout.current) clearTimeout(wechatTimeout.current);
    };
  }, []);

  /** 左栏扫码二维码：指向移动端 H5 入口，扫码打开手机版商城（与登录页一致） */
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const QRCode = (await import('qrcode')).default;
        const url = await QRCode.toDataURL(`${window.location.origin}/mobile`, { width: 196, margin: 1 });
        if (!cancelled) setMobileQr(url);
      } catch {
        /* 二维码生成失败时左栏显示占位文案 */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const startCountdown = (seconds = 60) => {
    setCountdown(seconds);
    if (timerRef.current) clearInterval(timerRef.current);
    timerRef.current = setInterval(() => {
      setCountdown((value) => {
        if (value <= 1) {
          if (timerRef.current) clearInterval(timerRef.current);
          return 0;
        }
        return value - 1;
      });
    }, 1000);
  };

  /** 源码 Register.vue showWechatLogin：拉取微信登录地址并生成二维码，轮询扫码结果 */
  const stopWechat = () => {
    if (wechatTimer.current) { clearInterval(wechatTimer.current); wechatTimer.current = null; }
    if (wechatTimeout.current) { clearTimeout(wechatTimeout.current); wechatTimeout.current = null; }
  };

  const showWechatLogin = async () => {
    stopWechat();
    setWechatExpired(false);
    try {
      const response = await fetch('/api/user/login/getWxLoginUrl', {
        credentials: 'include',
        headers: { 'X-Client-Type': 'pc' },
      });
      const json = await response.json();
      const data = json?.data ?? {};
      const url = typeof data === 'string' ? data : String(data.url || data.qrcodeUrl || '');
      const ticket = typeof data === 'object' ? String(data.ticket || '') : '';
      if (!url || !ticket) {
        setWechatExpired(true);
        return;
      }
      const QRCode = (await import('qrcode')).default;
      setWechatQr(await QRCode.toDataURL(url, { width: 220, margin: 1 }));
      // 轮询扫码事件（POST {key: ticket}，type=1 登录 / type=2 先绑手机）
      wechatTimer.current = setInterval(async () => {
        try {
          const event = await fetch('/api/user/login/wechatEvent', {
            method: 'POST',
            credentials: 'include',
            headers: { 'Content-Type': 'application/json', 'X-Client-Type': 'pc' },
            body: JSON.stringify({ key: ticket }),
          }).then((r) => r.json());
          // 未扫码时后端回业务错误（1001 未登录），属轮询常态，继续等
          if (event?.code !== undefined && event.code !== 0) return;
          const data = event?.data || event;
          if (data?.type === 1 && data?.token) {
            stopWechat();
            localStorage.setItem('accessToken', data.token);
            document.cookie = `accessToken=${encodeURIComponent(data.token)}; path=/; max-age=604800; SameSite=Lax`;
            Toast.success({ content: '微信授权成功' });
            window.location.replace('/member');
          } else if (data?.type === 2) {
            // 首次微信登录：先绑手机号（后端 bindTicket 一次性凭证在 openData 内）
            stopWechat();
            sessionStorage.setItem('wechatBindOpenData', JSON.stringify(data.openData ?? {}));
            window.location.replace('/member/bindmobilephone');
          }
        } catch {
          /* 轮询失败忽略，等待超时 */
        }
      }, 2000);
      wechatTimeout.current = setTimeout(() => {
        stopWechat();
        setWechatExpired(true);
      }, 60000);
    } catch (error) {
      setWechatExpired(true);
      Toast.error({ content: error instanceof Error ? error.message : '微信二维码加载失败' });
    }
  };

  /** 切换到微信模式时自动加载二维码 */
  useEffect(() => {
    if (mode === 'wechat') {
      void showWechatLogin();
    } else {
      stopWechat();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode]);

  /** 点击获取验证码：先校验格式，再过滑块，成功后才真正发码（与登录页一致，防脚本刷码） */
  const sendCode = () => {
    if (countdown > 0 || sending) return;
    if (mode === 'mobile' && !/^1\d{10}$/.test(mobile.trim())) {
      setError('请输入正确的手机号');
      return;
    }
    if (mode === 'email' && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.trim())) {
      setError('请输入正确的邮箱');
      return;
    }
    setError('');
    codeRetryRef.current = false;
    setVerifyOpen(true);
  };

  /** 源码 MobileCode：sendMobileCode / sendEmailCode（verifyToken 随请求提交，后端带了必校验） */
  const doSendCode = async (verifyToken: string) => {
    setSending(true);
    setError('');
    try {
      const url = mode === 'mobile' ? '/api/user/login/sendMobileCode' : '/api/user/login/sendEmailCode';
      const payload = mode === 'mobile' ? { mobile: mobile.trim(), verifyToken } : { email: email.trim(), verifyToken };
      const response = await fetch(url, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      const json = await response.json().catch(() => ({}));
      // code=1002 → 后端要求行为验证：重弹滑块，带新 token 重试一次
      if (json.code === 1002 && !codeRetryRef.current) {
        codeRetryRef.current = true;
        setVerifyOpen(true);
        return;
      }
      if (!response.ok || (json.code !== undefined && json.code !== 0)) {
        throw new Error(json.msg || json.message || t('验证码发送失败'));
      }
      Toast.success({ content: '验证码已发送' });
      startCountdown(60);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '验证码发送失败');
    } finally {
      setSending(false);
    }
  };

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (mode === 'mobile' && !/^1\d{10}$/.test(mobile.trim())) return setError('请输入正确的手机号');
    if (mode === 'email' && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.trim())) return setError('请输入正确的邮箱');
    if (!code.trim()) return setError('请输入验证码');
    if (password.length < 6) return setError('密码至少 6 位');
    if (password !== confirm) return setError('两次输入的密码不一致');
    if (!agree) return setError('请先阅读并同意注册协议与隐私政策');

    setLoading(true);
    setError('');
    try {
      const csrf = await getCsrf();
      const body: Record<string, unknown> = {
        password,
        nickname: nickname.trim(),
        mobileCode: mode === 'mobile' ? code.trim() : '',
        emailCode: mode === 'email' ? code.trim() : '',
      };
      if (mode === 'mobile') body.mobile = mobile.trim();
      else body.email = email.trim();
      if (inviteCode.trim()) body.inviteCode = inviteCode.trim();

      const response = await fetch('/api/user/regist/registAct', {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf },
        body: JSON.stringify(body),
      });
      const json = await response.json().catch(() => ({}));
      if (!response.ok || (json.code !== undefined && json.code !== 0)) {
        throw new Error(json.msg || json.message || t('注册失败'));
      }
      Toast.success({ content: '注册成功，请登录' });
      setTimeout(() => window.location.replace('/login'), 600);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '注册失败');
      setLoading(false);
    }
  };

  /**
   * 密码强度（纯展示）：仅由密码长度与字符种类派生，
   * 不参与校验、不拦截提交（校验规则仍是原有的「至少 6 位」）。
   */
  const pwdKindCount = [/[a-z]/, /[A-Z]/, /\d/, /[^A-Za-z0-9]/].filter((re) => re.test(password)).length;
  const pwdStrength = !password
    ? 0
    : password.length < 6 || pwdKindCount <= 1
      ? 1
      : password.length >= 10 && pwdKindCount >= 3
        ? 3
        : 2;

  return (
    <div className="pg-login-card pg-register-card">
      <header className="login-card__topbar">
        <Link href="/" className="login-card__logo">
          {shopLogo ? (
            <img src={imageFormat(shopLogo)} alt={shopName} />
          ) : (
            <span className="login-card__logo-text">{shopName}</span>
          )}
        </Link>
        <div className="login-card__topbar-info">
          <span className="login-card__welcome">{t('您好，欢迎光临')}{shopName}{t('！')}</span>
          <Link className="login-card__switch" href="/login">{t('已有账号？去登录！')}</Link>
        </div>
      </header>

      <div className="login-card__stage">
        <div className="login-card">
          {/* 左栏：手机扫码（二维码指向移动端 H5） */}
          <aside className="login-card__qr">
            <h3 className="login-card__qr-title">{t('手机扫码')}</h3>
            <p className="login-card__qr-sub">{t('打开手机版商城')}</p>
            <div className="login-card__qr-box">
              {mobileQr ? (
                <img src={mobileQr} alt={t('手机版商城二维码')} />
              ) : (
                <div className="login-card__qr-empty">{t('加载中...')}</div>
              )}
            </div>
            <p className="login-card__qr-tip">{t('使用手机浏览器或微信扫一扫')}</p>
          </aside>

          {/* 右栏：注册表单 / 微信扫码面板 */}
          <div className="login-card__main">
            {mode === 'wechat' ? (
              <div className="login-card__wechat-panel">
                <h2 className="login-card__title login-card__title--center">{t('请使用微信扫码安全登录')}</h2>
                {wechatQr && !wechatExpired ? (
                  <>
                    {/* eslint-disable-next-line @next/next/no-img-element */}
                    <img className="login-card__wxqr" src={wechatQr} alt={t('微信二维码')} />
                  </>
                ) : (
                  <div className="login-card__wxqr login-card__wxqr--empty">{wechatExpired ? t('二维码已失效') : t('加载中...')}</div>
                )}
                <p className="register-card__wechat-tip">{t('扫码后自动完成注册并登录')}</p>
                <div className="login-card__wechat-actions">
                  <button type="button" className="login-card__btn-secondary" onClick={() => void showWechatLogin()}>
                    {t('刷新二维码')}
                  </button>
                  <button type="button" className="login-card__btn-secondary" onClick={() => setMode('mobile')}>
                    {t('返回')}
                  </button>
                </div>
              </div>
            ) : (
              <>
                <div className="login-card__tabs">
                  <button
                    type="button"
                    className={`login-card__tab${mode === 'mobile' ? ' is-active' : ''}`}
                    onClick={() => {
                      setMode('mobile');
                      setError('');
                    }}
                  >
                    {t('手机注册')}
                  </button>
                  <span className="login-card__tab-sep" />
                  <button
                    type="button"
                    className={`login-card__tab${mode === 'email' ? ' is-active' : ''}`}
                    onClick={() => {
                      setMode('email');
                      setError('');
                    }}
                  >
                    {t('邮箱注册')}
                  </button>
                </div>

                <form className="login-card__form" onSubmit={submit}>
                  {mode === 'mobile' ? (
                    <label className="login-card__field">
                      <input
                        className="login-card__input"
                        value={mobile}
                        onChange={(event) => setMobile(event.target.value)}
                        placeholder={t('手机号码')}
                        aria-label={t('手机号码')}
                        maxLength={11}
                        autoComplete="tel"
                      />
                    </label>
                  ) : (
                    <label className="login-card__field">
                      <input
                        className="login-card__input"
                        value={email}
                        onChange={(event) => setEmail(event.target.value)}
                        placeholder={t('邮箱')}
                        aria-label={t('邮箱')}
                        autoComplete="email"
                      />
                    </label>
                  )}

                  <label className="login-card__field login-card__field--code">
                    <input
                      className="login-card__input"
                      value={code}
                      onChange={(event) => setCode(event.target.value)}
                      placeholder={t('验证码')}
                      aria-label={t('验证码')}
                      maxLength={8}
                      autoComplete="one-time-code"
                    />
                    <button className="login-card__code-btn" type="button" disabled={countdown > 0 || sending} onClick={sendCode}>
                      {countdown > 0 ? `${countdown}s` : t('获取验证码')}
                    </button>
                  </label>

                  <label className="login-card__field">
                    <input
                      className="login-card__input"
                      value={nickname}
                      onChange={(event) => setNickname(event.target.value)}
                      placeholder={t('昵称（选填）')}
                      aria-label={t('昵称（选填）')}
                      autoComplete="nickname"
                    />
                  </label>

                  <div className="login-card__field">
                    <div className="register-card__pwd-wrap">
                      <input
                        className="login-card__input register-card__input--pwd"
                        type={showPassword ? 'text' : 'password'}
                        value={password}
                        onChange={(event) => setPassword(event.target.value)}
                        placeholder={t('设置密码')}
                        aria-label={t('设置密码')}
                        autoComplete="new-password"
                      />
                      <button
                        type="button"
                        className="register-card__eye"
                        onClick={() => setShowPassword((value) => !value)}
                        aria-label={showPassword ? t('隐藏密码') : t('显示密码')}
                      >
                        <EyeIcon off={!showPassword} />
                      </button>
                    </div>
                    {password ? (
                      <div className={`register-card__strength register-card__strength--${pwdStrength}`}>
                        <span className="register-card__strength-bar" />
                        <span className="register-card__strength-bar" />
                        <span className="register-card__strength-bar" />
                        <span className="register-card__strength-text">
                          {pwdStrength === 1 ? t('弱') : pwdStrength === 2 ? t('中') : t('强')}
                        </span>
                      </div>
                    ) : null}
                  </div>

                  <div className="login-card__field">
                    <div className="register-card__pwd-wrap">
                      <input
                        className="login-card__input register-card__input--pwd"
                        type={showConfirm ? 'text' : 'password'}
                        value={confirm}
                        onChange={(event) => setConfirm(event.target.value)}
                        placeholder={t('确认密码')}
                        aria-label={t('确认密码')}
                        autoComplete="new-password"
                      />
                      <button
                        type="button"
                        className="register-card__eye"
                        onClick={() => setShowConfirm((value) => !value)}
                        aria-label={showConfirm ? t('隐藏密码') : t('显示密码')}
                      >
                        <EyeIcon off={!showConfirm} />
                      </button>
                    </div>
                  </div>

                  <label className="login-card__field">
                    <input
                      className="login-card__input"
                      value={inviteCode}
                      onChange={(event) => setInviteCode(event.target.value)}
                      placeholder={t('邀请码（选填）')}
                      aria-label={t('邀请码（选填）')}
                    />
                  </label>

                  {error ? <div className="login-card__error">{error}</div> : null}

                  {/* 旧版无勾选框、视为已同意：仅一行提示文字，链接沿用 yhxy / yszc */}
                  <div className="login-card__agree">
                    <span>{t('点击注册，表示您同意')}</span>
                    <Link className="login-card__agree-link" href="/article/issue?categoryId=yhxy" target="_blank">
                      {t('《用户协议》')}
                    </Link>
                    <span>{t('和')}</span>
                    <Link className="login-card__agree-link" href="/article/issue?categoryId=yszc" target="_blank">
                      {t('《隐私政策》')}
                    </Link>
                  </div>

                  <button className="login-card__submit" type="submit" disabled={loading}>
                    {loading ? t('注册中...') : t('同意协议并注册')}
                  </button>
                </form>

                <div className="login-card__foot">
                  <div className="login-card__social">
                    <button className="login-card__wechat" type="button" onClick={() => setMode('wechat')} title={t('微信')}>
                      {/* eslint-disable-next-line @next/next/no-img-element */}
                      <img src="/assets/images/user/login/ico_wx.png" width={22} height={22} alt={t('微信')} />
                    </button>
                    {/* 第三方注册（源码 pages/member/Register.vue:192-203 Google / Facebook / Twitter-X） */}
                    <OauthButtons switches={oauthSwitches} onStart={startOauth} />
                  </div>
                </div>
              </>
            )}
          </div>
        </div>
      </div>

      {/* 滑块行为验证码：发码前主动弹出；后端返回 1002 时重弹一次 */}
      <VerifySlide
        open={verifyOpen}
        onClose={() => setVerifyOpen(false)}
        onSuccess={(token) => {
          // token 一次性，回调里直接用参数发码，不写 state
          setVerifyOpen(false);
          void doSendCode(token);
        }}
      />
    </div>
  );
}
