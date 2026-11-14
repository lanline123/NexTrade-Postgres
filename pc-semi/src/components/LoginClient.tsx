'use client';

/**
 * PC 登录（淘宝式双栏卡片改版）
 *
 * 布局：浅灰页面底 + 居中白色圆角卡（阴影），卡内左右双栏——
 * 左栏「手机扫码」：二维码指向移动端 H5（/mobile），扫码打开手机版商城；
 * 右栏：密码登录 / 短信登录双页签 + 协议勾选 + 全宽主题色登录按钮，
 * 底部一行左侧第三方登录、右侧忘记密码 / 免费注册。
 * 配色用全站主题色（--main-bg），不照搬淘宝橙。样式在
 * app/auth-modern.scss 的 .pg-login-card / .login-card__* 段（手工维护）。
 * 登录页装修配置的 backgroundPic / backgroundColor 继续作为页面底图/底色。
 * 旧毛玻璃版样式（.pg-login-glass）保留在同一文件里，注册页等仍可参照。
 *
 * 逻辑与旧版对应：
 * - 密码登录：提交前先过滑块行为验证（VerifySlide），带 verifyToken 提交；
 *   接口仍返回 code=1002 时再次弹滑块重试；
 * - 短信登录：获取验证码前先过滑块，verifyToken 随发码请求提交（后端带了必校验）；
 * - 微信扫码：GET /api/user/login/getWxLoginUrl + wechatEvent 轮询（右侧社交区入口）；
 * - 第三方 OAuth（Google/Facebook/Twitter-X）按钮组。
 */
import { useEffect, useRef, useState } from 'react';
import Link from 'next/link';
import { Toast } from '@douyinfe/semi-ui';
import type { PcLoginDecorate } from '@/lib/api';
import { imageFormat } from '@/lib/image';

import { useTranslation } from 'react-i18next';
import i18n from '@/i18n';
import VerifySlide from '@/components/VerifySlide';
import { useOauthLogin, OauthButtons } from '@/components/oauth/useOauthLogin';
async function getCsrf(): Promise<string> {
  const response = await fetch('/api/common/csrf/create', { credentials: 'include' });
  const json = await response.json();
  if (!response.ok || (json.code !== undefined && json.code !== 0)) {
    throw new Error(json.msg || (i18n.t('获取 CSRF Token 失败') as string));
  }
  return typeof json.data === 'string' ? json.data : json.data?.token || '';
}

/**
 * 登录接口错误（保留业务 code：code=1002 时需弹滑块验证后带 verifyToken 重试）。
 */
class ApiError extends Error {
  code?: number;

  constructor(message: string, code?: number) {
    super(message);
    this.name = 'ApiError';
    this.code = code;
  }
}

async function postJson(path: string, body: unknown, headers: Record<string, string> = {}) {
  const response = await fetch(`/api/${path}`, {
    method: 'POST',
    credentials: 'include',
    headers: { 'Content-Type': 'application/json', ...headers },
    body: JSON.stringify(body),
  });
  const text = await response.text();
  const json = text ? JSON.parse(text) : {};
  if (!response.ok || (json.code !== undefined && json.code !== 0)) {
    const code = typeof json.code === 'number' ? json.code : undefined;
    throw new ApiError(
      json.msg || json.message || (i18n.t('请求失败（{code}）', { code: response.status }) as string),
      code,
    );
  }
  return json.data ?? json;
}

function redirectAfterLogin() {
  const params = new URLSearchParams(window.location.search);
  const target = params.get('redirect') || params.get('returnUrl') || '/member';
  window.location.replace(target);
}

export default function LoginClient({
  shopName = '万物新生',
  shopLogo = '',
  shopRegClosed = 0,
  decorate,
}: {
  /** 店铺名称（logo 缺省时展示文字），来自 common/config/base */
  shopName?: string;
  /** 店铺 logo（顶栏展示） */
  shopLogo?: string;
  /** 后台注册开关：0 = 允许注册（展示注册入口） */
  shopRegClosed?: number;
  /** 登录页装修配置：backgroundPic / backgroundColor 决定整页背景 */
  decorate?: PcLoginDecorate | null;
}) {
    const { t } = useTranslation();

  const [tab, setTab] = useState<'normal' | 'mobile'>('normal');
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [mobile, setMobile] = useState('');
  const [mobileCode, setMobileCode] = useState('');
  const [areaCode, setAreaCode] = useState('+86');
  const [agreed, setAgreed] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [countdown, setCountdown] = useState(0);
  const [wechatMode, setWechatMode] = useState(false);
  const [wechatQr, setWechatQr] = useState('');
  const [wechatExpired, setWechatExpired] = useState(false);
  const [mobileQr, setMobileQr] = useState('');
  /**
   * 滑块行为验证码：密码登录提交前、短信发码前主动弹出；
   * 登录接口返回 code=1002 时也会弹出（后端失败次数触发的兜底）。
   * verifyToken 不入 state：后端校验成功后 token 即失效（防重放），
   * 且 React 18 批处理 setState，回调里读 state 是旧值——一律走参数传递。
   * sliderPurpose 区分本次滑块的用途（登录提交 / 发送验证码）。
   */
  const [verifyOpen, setVerifyOpen] = useState(false);
  const sliderPurpose = useRef<'login' | 'sms'>('login');
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);
  const poll = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => () => {
    if (timer.current) clearInterval(timer.current);
    if (poll.current) clearInterval(poll.current);
  }, []);

  /** 左栏扫码二维码：指向移动端 H5 入口，扫码打开手机版商城 */
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

  /** 整页背景：装修配置的背景图 cover 铺底 + 背景色兜底 */
  const bgUrl = decorate?.backgroundPic?.picUrl || '';
  const bgStyle: React.CSSProperties = {
    backgroundImage: bgUrl ? `url(${bgUrl})` : undefined,
    backgroundColor: decorate?.backgroundColor || undefined,
  };

  /**
   * 第三方登录开关（源码 components/member/login/OverseasLogin.vue:117-127）。
   * 后台配置项：common/config/base → googleLoginOn / facebookLoginOn / twitterLoginOn。
   */
  const { switches: oauthSwitches, start: startOauth } = useOauthLogin();

  /** 点击获取验证码：先校验手机号，再过滑块，成功后才真正发码 */
  const requestSendCode = () => {
    if (countdown > 0) return;
    if (!/^1\d{10}$/.test(mobile.trim())) return setError('请输入正确的手机号');
    setError('');
    sliderPurpose.current = 'sms';
    setVerifyOpen(true);
  };

  const doSendCode = async (verifyToken: string) => {
    try {
      await postJson('user/login/sendMobileCode', { mobile: mobile.trim(), areaCode, verifyToken });
      Toast.success({ content: '验证码已发送' });
      setCountdown(60);
      if (timer.current) clearInterval(timer.current);
      timer.current = setInterval(() => {
        setCountdown((value) => {
          if (value <= 1) {
            if (timer.current) clearInterval(timer.current);
            return 0;
          }
          return value - 1;
        });
      }, 1000);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '验证码发送失败');
    }
  };

  /**
   * 登录提交。
   * 密码登录且未带 verifyToken 时，先弹滑块验证，成功后由回调带 token 重入；
   * 短信登录本身以验证码为凭据，不前置滑块（1002 兜底仍保留）。
   * @param event 表单事件（可选，滑块验证回调重试时无事件）
   * @param verifyTokenOverride 滑块验证得到的 verifyToken（显式传入，避免读到批处理前的旧 state）
   */
  const submit = async (event?: React.FormEvent, verifyTokenOverride?: string) => {
    event?.preventDefault();
    if (!agreed) return setError('请先阅读并同意服务协议与隐私协议');
    if (tab === 'normal') {
      if (!username.trim() || !password) return setError('请输入账号和密码');
      if (!verifyTokenOverride) {
        setError('');
        sliderPurpose.current = 'login';
        setVerifyOpen(true);
        return;
      }
    } else if (!/^1\d{10}$/.test(mobile.trim()) || !mobileCode.trim()) {
      return setError('请输入手机号和验证码');
    }
    setLoading(true);
    setError('');
    /** 滑块验证得到的 verifyToken（仅重试时传入，首次登录为空）。 */
    const effectiveVerifyToken = verifyTokenOverride ?? '';
    try {
      const csrf = await getCsrf();
      const result: any = await postJson(
        'user/login/signin',
        tab === 'normal'
          ? { loginType: 'password', username: username.trim(), password, mobile: '', mobileCode: '', verifyToken: effectiveVerifyToken }
          : { loginType: 'mobile', username: '', password: '', mobile: mobile.trim(), mobileCode: mobileCode.trim(), areaCode, verifyToken: effectiveVerifyToken },
        { 'X-CSRF-Token': csrf },
      );
      const authToken = result?.token || result?.accessToken;
      if (!authToken) throw new Error('登录接口未返回 token');
      localStorage.setItem('accessToken', authToken);
      document.cookie = `accessToken=${encodeURIComponent(authToken)}; path=/; max-age=604800; SameSite=Lax`;
      Toast.success({ content: '登录成功' });
      redirectAfterLogin();
    } catch (reason) {
      // code=1002 → 弹滑块验证，完成后带 verifyToken 重试
      if (reason instanceof ApiError && reason.code === 1002) {
        sliderPurpose.current = 'login';
        setVerifyOpen(true);
        setLoading(false);
        return;
      }
      setError(reason instanceof Error ? reason.message : '登录失败');
      setLoading(false);
    }
  };
  /** 源码 showWechatLogin：微信扫码登录 */
  const openWechat = async () => {
    setWechatMode(true);
    setWechatExpired(false);
    try {
      const response = await fetch('/api/user/login/getWxLoginUrl', { credentials: 'include' });
      const json = await response.json();
      const url = typeof json?.data === 'string' ? json.data : String(json?.data?.url || json?.data?.qrcodeUrl || '');
      if (!url) {
        setWechatExpired(true);
        return;
      }
      const QRCode = (await import('qrcode')).default;
      setWechatQr(await QRCode.toDataURL(url, { width: 200, margin: 1 }));
      poll.current = setInterval(async () => {
        try {
          const event = await fetch('/api/user/login/wechatEvent', { credentials: 'include' }).then((r) => r.json());
          const data = event?.data || event;
          if (data?.status === 1 || data?.success) {
            if (poll.current) clearInterval(poll.current);
            Toast.success({ content: '微信授权成功' });
            redirectAfterLogin();
          }
        } catch {
          /* 忽略轮询异常 */
        }
      }, 2000);
    } catch {
      setWechatExpired(true);
    }
  };

  return (
    <div className="pg-login-card" style={bgStyle}>
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
          {shopRegClosed === 0 ? (
            <Link className="login-card__switch" href="/register">{t('没有账号？注册新用户！')}</Link>
          ) : null}
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

          {/* 右栏：登录表单 / 微信扫码面板 */}
          <div className="login-card__main">
            {wechatMode ? (
              <div className="login-card__wechat-panel">
                <h2 className="login-card__title login-card__title--center">{t('请使用微信扫码安全登录')}</h2>
                {wechatQr && !wechatExpired ? (
                  <img className="login-card__wxqr" src={wechatQr} alt={t('微信二维码')} />
                ) : (
                  <div className="login-card__wxqr login-card__wxqr--empty">{wechatExpired ? t('二维码已失效') : t('加载中...')}</div>
                )}
                <div className="login-card__wechat-actions">
                  <button type="button" className="login-card__btn-secondary" onClick={() => void openWechat()}>
                    {t('刷新二维码')}
                  </button>
                  <button
                    type="button"
                    className="login-card__btn-secondary"
                    onClick={() => {
                      setWechatMode(false);
                      if (poll.current) clearInterval(poll.current);
                    }}
                  >
                    {t('返回')}
                  </button>
                </div>
              </div>
            ) : (
              <>
                <div className="login-card__tabs">
                  <button
                    type="button"
                    className={`login-card__tab${tab === 'normal' ? ' is-active' : ''}`}
                    onClick={() => {
                      setTab('normal');
                      setError('');
                    }}
                  >
                    {t('密码登录')}
                  </button>
                  <span className="login-card__tab-sep" />
                  <button
                    type="button"
                    className={`login-card__tab${tab === 'mobile' ? ' is-active' : ''}`}
                    onClick={() => {
                      setTab('mobile');
                      setError('');
                    }}
                  >
                    {t('短信登录')}
                  </button>
                </div>

                <form className="login-card__form" onSubmit={submit}>
                  {tab === 'normal' ? (
                    <>
                      <label className="login-card__field">
                        <input
                          className="login-card__input"
                          value={username}
                          onChange={(event) => setUsername(event.target.value)}
                          placeholder={t('用户名/手机/邮箱')}
                          aria-label={t('用户名/手机/邮箱')}
                          autoComplete="username"
                        />
                      </label>
                      <label className="login-card__field">
                        <input
                          className="login-card__input"
                          type="password"
                          value={password}
                          onChange={(event) => setPassword(event.target.value)}
                          placeholder={t('密码')}
                          aria-label={t('密码')}
                          autoComplete="current-password"
                        />
                      </label>
                    </>
                  ) : (
                    <>
                      <div className="login-card__field login-card__field--row">
                        <select
                          className="login-card__select"
                          value={areaCode}
                          onChange={(event) => setAreaCode(event.target.value)}
                          aria-label={t('区号')}
                        >
                          <option value="+86">(+86)</option>
                          <option value="+61">(+61)</option>
                        </select>
                        <input
                          className="login-card__input"
                          value={mobile}
                          onChange={(event) => setMobile(event.target.value)}
                          placeholder={t('您的手机号')}
                          aria-label={t('您的手机号')}
                          maxLength={11}
                          autoComplete="tel"
                        />
                      </div>
                      <label className="login-card__field login-card__field--code">
                        <input
                          className="login-card__input"
                          value={mobileCode}
                          onChange={(event) => setMobileCode(event.target.value)}
                          placeholder={t('请输入验证码')}
                          aria-label={t('请输入验证码')}
                          maxLength={8}
                          autoComplete="one-time-code"
                        />
                        <button className="login-card__code-btn" type="button" disabled={countdown > 0} onClick={requestSendCode}>
                          {countdown > 0 ? `${countdown}s` : t('获取验证码')}
                        </button>
                      </label>
                    </>
                  )}

                  {error ? <div className="login-card__error">{error}</div> : null}

                  <div className="login-card__agree">
                    <span className="login-card__agree-star">*</span>
                    <label className="login-card__checkbox">
                      <input type="checkbox" checked={agreed} onChange={(event) => setAgreed(event.target.checked)} />
                      <span>{t('已阅读并同意以下协议')}</span>
                    </label>
                    <Link className="login-card__agree-link" href="/article/issue/info?articleSn=fwxy" target="_blank">
                      {t('《服务协议》')}
                    </Link>
                    <span>{t('、')}</span>
                    <Link className="login-card__agree-link" href="/article/issue/info?articleSn=ysxy" target="_blank">
                      {t('《隐私协议》')}
                    </Link>
                  </div>

                  <button className="login-card__submit" type="submit" disabled={!agreed || loading}>
                    {loading ? t('登录中...') : t('登录')}
                  </button>
                </form>

                <div className="login-card__foot">
                  <div className="login-card__social">
                    <button className="login-card__wechat" type="button" onClick={() => void openWechat()} title={t('微信扫码登录')}>
                      <img src="/assets/images/user/login/ico_wx.png" width={22} height={22} alt={t('微信扫码登录')} />
                    </button>
                    <OauthButtons switches={oauthSwitches} onStart={startOauth} />
                  </div>
                  <div className="login-card__links">
                    {tab === 'normal' ? <Link href="/member/reset">{t('忘记密码')}</Link> : null}
                    {shopRegClosed === 0 ? <Link href="/register">{t('免费注册')}</Link> : null}
                  </div>
                </div>
              </>
            )}
          </div>
        </div>
      </div>

      {/* 滑块行为验证码：密码登录提交前 / 短信发码前主动弹出；1002 时兜底重弹 */}
      <VerifySlide
        open={verifyOpen}
        onClose={() => setVerifyOpen(false)}
        onSuccess={(token) => {
          // token 一次性，回调里直接用参数分发，不写 state
          setVerifyOpen(false);
          if (sliderPurpose.current === 'sms') {
            void doSendCode(token);
          } else {
            void submit(undefined, token);
          }
        }}
      />
    </div>
  );
}
