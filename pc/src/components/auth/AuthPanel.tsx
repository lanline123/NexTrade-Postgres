'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { Button, Card, Input, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost, errorCode } from '@/lib/client-api';
import SlideCaptcha from '@/components/captcha/SlideCaptcha';

/**
 * 登录 / 注册。
 * Vue 基准：pages/member/Login.vue、Register.vue + components/member/login/BaseLogin.vue。
 * 登录参数照 BaseLogin：{ loginType: 'password'|'mobile', username, password, mobile, mobileCode, verifyToken }，
 * 登录前先打 common/csrf/create。
 * 业务码 1002 = 需要滑动验证码：弹 SlideCaptcha，拿到 verifyToken 后原样重试（Vue 版同逻辑）。
 * 注册参数后端未在 Vue 类型里写死，这里用 mobile/mobileCode/password/username 的常见组合，
 * 联调时若字段不符只需改这一处。
 */
type Mode = 'login' | 'register';
type LoginTab = 'password' | 'mobile';

export default function AuthPanel({ mode: initialMode = 'login' }: { mode?: Mode }) {
    const { t } = useTranslation();
    const router = useRouter();
    const [mode, setMode] = useState<Mode>(initialMode);
    const [tab, setTab] = useState<LoginTab>('password');
    const [username, setUsername] = useState('');
    const [password, setPassword] = useState('');
    const [mobile, setMobile] = useState('');
    const [code, setCode] = useState('');
    const [countdown, setCountdown] = useState(0);
    const [submitting, setSubmitting] = useState(false);
    const [captchaVisible, setCaptchaVisible] = useState(false);
    const [verifyToken, setVerifyToken] = useState('');

    useEffect(() => {
        if (countdown <= 0) return;
        const timer = setTimeout(() => setCountdown(countdown - 1), 1000);
        return () => clearTimeout(timer);
    }, [countdown]);

    const sendCode = async () => {
        if (!/^1[3-9]\d{9}$/.test(mobile.trim())) {
            Toast.warning(t('请输入正确的手机号'));
            return;
        }
        try {
            await apiPost('user/login/sendMobileCode', {
                mobile: mobile.trim(),
                type: mode === 'register' ? 'regist' : 'login'
            });
            Toast.success(t('验证码已发送'));
            setCountdown(60);
        } catch (e) {
            Toast.error((e as Error).message);
        }
    };

    const persistToken = (result: Record<string, unknown>) => {
        const token = String(result.token ?? result.accessToken ?? result.tokenValue ?? '');
        if (token) {
            localStorage.setItem('accessToken', token);
        }
    };

    const submit = async (token?: string) => {
        setSubmitting(true);
        try {
            if (mode === 'register') {
                if (!mobile.trim() || !code.trim() || password.length < 6) {
                    Toast.warning(t('请填写手机号、验证码与至少 6 位密码'));
                    return;
                }
                const result = await apiPost<Record<string, unknown>>('user/regist/registAct', {
                    mobile: mobile.trim(),
                    mobileCode: code.trim(),
                    password,
                    username: mobile.trim(),
                    verifyToken: token ?? verifyToken
                });
                persistToken(result ?? {});
                Toast.success(t('注册成功'));
                router.push('/member');
                return;
            }

            await apiGet('common/csrf/create').catch(() => null);
            const payload =
                tab === 'password'
                    ? {
                          loginType: 'password',
                          username: username.trim(),
                          password,
                          mobile: '',
                          mobileCode: '',
                          verifyToken: token ?? verifyToken
                      }
                    : {
                          loginType: 'mobile',
                          username: '',
                          password: '',
                          mobile: mobile.trim(),
                          mobileCode: code.trim(),
                          verifyToken: token ?? verifyToken
                      };
            const result = await apiPost<Record<string, unknown>>('user/login/signin', payload);
            persistToken(result ?? {});
            Toast.success(t('登录成功'));
            const returnUrl = new URLSearchParams(window.location.search).get('returnUrl');
            router.push(returnUrl || '/member');
        } catch (e) {
            // 1002：本次登录要求图形验证码
            if (errorCode(e) === 1002) {
                setCaptchaVisible(true);
                return;
            }
            Toast.error((e as Error).message);
        } finally {
            setSubmitting(false);
        }
    };

    return (
        <div className="pc-auth">
            <Card>
                <Typography.Title heading={3}>
                    {mode === 'login' ? t('登录') : t('注册')}
                </Typography.Title>

                {mode === 'login' ? (
                    <div className="pc-tabs" style={{ marginTop: 12 }}>
                        <button type="button" className={tab === 'password' ? 'is-active' : ''} onClick={() => setTab('password')}>
                            {t('账号密码')}
                        </button>
                        <button type="button" className={tab === 'mobile' ? 'is-active' : ''} onClick={() => setTab('mobile')}>
                            {t('手机验证码')}
                        </button>
                    </div>
                ) : null}

                <div className="pc-form" style={{ marginTop: 16 }}>
                    {mode === 'login' && tab === 'password' ? (
                        <>
                            <Input value={username} onChange={(v) => setUsername(String(v))} placeholder={t('用户名 / 手机号')} />
                            <Input mode="password" value={password} onChange={(v) => setPassword(String(v))} placeholder={t('密码')} />
                        </>
                    ) : (
                        <>
                            <Input value={mobile} onChange={(v) => setMobile(String(v))} placeholder={t('手机号')} />
                            <div className="pc-code-row">
                                <Input value={code} onChange={(v) => setCode(String(v))} placeholder={t('验证码')} />
                                <Button size="small" disabled={countdown > 0} onClick={() => void sendCode()}>
                                    {countdown > 0 ? `${countdown}s` : t('获取验证码')}
                                </Button>
                            </div>
                            {mode === 'register' ? (
                                <Input mode="password" value={password} onChange={(v) => setPassword(String(v))} placeholder={t('设置密码（至少 6 位）')} />
                            ) : null}
                        </>
                    )}

                    <Button theme="solid" type="primary" loading={submitting} onClick={() => void submit()}>
                        {mode === 'login' ? t('登录') : t('注册')}
                    </Button>

                    <div className="pc-muted">
                        {mode === 'login' ? (
                            <>
                                {t('还没有账号？')}
                                <button type="button" className="pc-link-btn" onClick={() => setMode('register')}>
                                    {t('去注册')}
                                </button>
                            </>
                        ) : (
                            <>
                                {t('已有账号？')}
                                <button type="button" className="pc-link-btn" onClick={() => setMode('login')}>
                                    {t('去登录')}
                                </button>
                            </>
                        )}
                    </div>
                </div>
            </Card>

            <SlideCaptcha
                visible={captchaVisible}
                onClose={() => setCaptchaVisible(false)}
                onSuccess={(token) => {
                    setCaptchaVisible(false);
                    setVerifyToken(token);
                    void submit(token);
                }}
            />
        </div>
    );
}
