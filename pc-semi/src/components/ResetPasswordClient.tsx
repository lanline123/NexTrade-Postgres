'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { Toast } from '@douyinfe/semi-ui';

import { useTranslation } from 'react-i18next';
import i18n from '@/i18n';
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
    throw new Error(json.msg || json.message || (i18n.t('请求失败（{code}）', { code: response.status }) as string));
  }
  return json.data ?? json;
}

async function getCsrf(): Promise<string> {
  const response = await fetch('/api/common/csrf/create', { credentials: 'include' });
  const json = await response.json();
  if (!response.ok || (json.code !== undefined && json.code !== 0)) {
    throw new Error(json.msg || (i18n.t('获取 CSRF Token 失败') as string));
  }
  return typeof json.data === 'string' ? json.data : json.data?.token || '';
}

export default function ResetPasswordClient() {
    const { t } = useTranslation();
  const [step, setStep] = useState(1);
  const [mobile, setMobile] = useState('');
  const [code, setCode] = useState('');
  const [mobileKey, setMobileKey] = useState('');
  const [password, setPassword] = useState('');
  const [password2, setPassword2] = useState('');
  const [countdown, setCountdown] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (countdown <= 0) return;
    const timer = window.setTimeout(() => setCountdown((n) => n - 1), 1000);
    return () => window.clearTimeout(timer);
  }, [countdown]);

  const [sending, setSending] = useState(false);

  const sendCode = async () => {
    // 此前只判非空：格式错误也发请求、且无防重复提交（倒计时在请求成功后
    // 才开始，狂点可并发多次发码请求；后端有 60 秒冷却兜底，前端仍应先拦）。
    if (countdown > 0 || sending) return;
    if (!/^1\d{10}$/.test(mobile.trim())) {
      setError('请输入正确的手机号');
      return;
    }
    setSending(true);
    setError('');
    try {
      const csrf = await getCsrf();
      await postJson('user/login/sendMobileCode', { mobile: mobile.trim(), event: 'forgetPassword' }, { 'X-CSRF-Token': csrf });
      Toast.success({ content: '验证码已发送' });
      setCountdown(60);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '验证码发送失败');
    } finally {
      setSending(false);
    }
  };

  const verify = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!mobile.trim() || !code.trim()) {
      setError('请输入手机号和验证码');
      return;
    }
    setLoading(true);
    setError('');
    try {
      const csrf = await getCsrf();
      const result: any = await postJson('user/login/checkMobile', { mobile: mobile.trim(), code: code.trim() }, { 'X-CSRF-Token': csrf });
      const key = typeof result === 'string' ? result : result?.mobileKey;
      if (!key) throw new Error('验证失败');
      setMobileKey(key);
      setStep(2);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '验证失败');
    } finally {
      setLoading(false);
    }
  };

  const reset = async (event: React.FormEvent) => {
    event.preventDefault();
    // 与后端 validate_password_strength 逐条同规（forget_password 服务端调用）：
    // 6–20 位、不含空白、不能全为数字/字母/符号、不能全为相同字符。
    if (!password || password.length < 6 || password.length > 20) {
      setError('密码应为6-20位字符');
      return;
    }
    if (/\s/.test(password)) {
      setError('密码中不允许有空格');
      return;
    }
    if (/^\d+$/.test(password) || /^[a-zA-Z]+$/.test(password) || /^[^0-9a-zA-Z_]+$/.test(password) || /^(.)\1+$/.test(password)) {
      setError('密码不能全为数字、全为字母、全为符号或全为相同字符');
      return;
    }
    if (password !== password2) {
      setError('两次密码输入不一致');
      return;
    }
    setLoading(true);
    setError('');
    try {
      const csrf = await getCsrf();
      await postJson('user/login/forgetPassword', { mobileKey, password }, { 'X-CSRF-Token': csrf });
      window.location.href = '/login?reset=1';
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '重置密码失败');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="pg-auth-reset">
      {/* 源码 pages/member/reset/index.vue：.full-height-div > .register-info > .title + StepOne/StepTow 的 .content */}
      <div className="full-height-div">
        <div className="register-info">
          <div className="title">{t('找回密码')}</div>

          <div className="content">
            <div className="form-box">
              {step === 1 ? (
                <>
                  <div className="form-item">
                    <div className="form-item-label">{t('手机号码')}</div>
                    <div className="form-item-input area-code-input">
                      <div className="el-input input-border-style">
                        <input
                          id="reset-mobile"
                          className="el-input__inner"
                          value={mobile}
                          onChange={(event) => setMobile(event.target.value)}
                          placeholder={t('手机号码')}
                          autoComplete="tel"
                        />
                      </div>
                      <div className="error-notice" />
                    </div>
                  </div>
                  <div className="form-item">
                    <div className="form-item-label">{t('验证码')}</div>
                    <div className="form-item-input">
                      <div className="el-input input-border-style">
                        <input
                          id="reset-code"
                          className="el-input__inner"
                          value={code}
                          onChange={(event) => setCode(event.target.value)}
                          placeholder={t('验证码')}
                        />
                        <span className="el-input__suffix mobile-code-text">
                          <button className="el-button" type="button" disabled={countdown > 0 || sending} onClick={() => void sendCode()}>
                            {countdown > 0 ? `${countdown}s` : t('获取验证码')}
                          </button>
                        </span>
                      </div>
                      <div className="error-notice" />
                    </div>
                  </div>
                </>
              ) : (
                <>
                  <div className="form-item">
                    <div className="form-item-label">{t('新密码')}</div>
                    <div className="form-item-input area-code-input">
                      <div className="el-input input-border-style">
                        <input
                          id="reset-password"
                          className="el-input__inner"
                          type="password"
                          value={password}
                          onChange={(event) => setPassword(event.target.value)}
                          placeholder={t('新密码')}
                          autoComplete="new-password"
                        />
                      </div>
                      <div className="error-notice" />
                    </div>
                  </div>
                  <div className="form-item">
                    <div className="form-item-label">{t('确认新密码')}</div>
                    <div className="form-item-input area-code-input">
                      <div className="el-input input-border-style">
                        <input
                          id="reset-password2"
                          className="el-input__inner"
                          type="password"
                          value={password2}
                          onChange={(event) => setPassword2(event.target.value)}
                          placeholder={t('确认新密码')}
                          autoComplete="new-password"
                        />
                      </div>
                      <div className="error-notice" />
                    </div>
                  </div>
                </>
              )}
            </div>

            {error ? <div className="error-notice">{error}</div> : null}

            <div className="btn-box">
              <button
                className="el-button submit-button"
                type="button"
                disabled={loading}
                style={{ width: '100%' }}
                onClick={(event) => (step === 1 ? void verify(event as unknown as React.FormEvent) : void reset(event as unknown as React.FormEvent))}
              >
                {step === 1 ? (loading ? t('验证中...') : t('下 一 步')) : loading ? t('提交中...') : t('确 认')}
              </button>
            </div>
          </div>

          <div className="login-box">
            <Link className="font-color" href="/login">
              {t('返回登录')}
            </Link>
          </div>
        </div>
      </div>
    </div>
  );
}

