'use client';

import { useCallback, useEffect, useState } from 'react';
import { Button, Card, Input, Modal, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';

/**
 * 账号安全。
 * Vue 基准：pages/member/security/Info.vue + src/VerificationSecurity.vue。
 * 三个流程的参数严格照 Vue 的调用：modifyMobile({mobile,code})、modifyEmail({email,code})、
 * checkModifyPasswordMobileCode({password,code})；发码接口只收 {mobile} 或 {email}。
 */
type Mode = 'mobile' | 'email' | 'password';

interface UserInfo {
    mobile?: string;
    email?: string;
}

const maskMobile = (mobile?: string) => (mobile ? mobile.replace(/^(\d{3})\d{4}(\d{4})$/, '$1****$2') : '');

export default function SecurityPanel() {
    const { t } = useTranslation();
    const [info, setInfo] = useState<UserInfo>({});
    const [mode, setMode] = useState<Mode | null>(null);
    const [mobileValue, setMobileValue] = useState('');
    const [emailValue, setEmailValue] = useState('');
    const [code, setCode] = useState('');
    const [password, setPassword] = useState('');
    const [confirm, setConfirm] = useState('');
    const [countdown, setCountdown] = useState(0);
    const [submitting, setSubmitting] = useState(false);

    const loadInfo = useCallback(() => {
        // 实测：user/user/detail 直接返回用户对象，没有 item 包裹
        apiGet<UserInfo>('user/user/detail')
            .then((result) => setInfo(result ?? {}))
            .catch(() => setInfo({}));
    }, []);

    useEffect(loadInfo, [loadInfo]);

    useEffect(() => {
        if (countdown <= 0) return;
        const timer = setTimeout(() => setCountdown(countdown - 1), 1000);
        return () => clearTimeout(timer);
    }, [countdown]);

    const open = (next: Mode) => {
        setMode(next);
        setCode('');
        setPassword('');
        setConfirm('');
        setMobileValue('');
        setEmailValue('');
    };

    const close = () => {
        setMode(null);
        setCountdown(0);
    };

    const sendCode = async () => {
        if (!mode) return;
        try {
            if (mode === 'password') {
                await apiPost('user/user/sendMobileCodeByModifyPassword', { mobile: info.mobile });
            } else if (mode === 'mobile') {
                if (!mobileValue.trim()) return Toast.warning(t('请输入手机号'));
                await apiPost('user/user/sendMobileCodeByModifyMobile', { mobile: mobileValue.trim() });
            } else {
                if (!emailValue.trim()) return Toast.warning(t('请输入邮箱'));
                await apiPost('user/user/sendEmailCodeByModifyEmail', { email: emailValue.trim() });
            }
            Toast.success(t('验证码已发送'));
            setCountdown(60);
        } catch (e) {
            Toast.error((e as Error).message);
        }
    };

    const submit = async () => {
        if (!mode) return;
        if (!code.trim()) return Toast.warning(t('请输入验证码'));
        setSubmitting(true);
        try {
            if (mode === 'password') {
                if (password.length < 6) return Toast.warning(t('密码至少 6 位'));
                if (password !== confirm) return Toast.warning(t('两次密码不一致'));
                await apiPost('user/user/checkModifyPasswordMobileCode', { password, code: code.trim() });
                Toast.success(t('修改成功，请重新登录'));
            } else if (mode === 'mobile') {
                await apiPost('user/user/modifyMobile', { mobile: mobileValue.trim(), code: code.trim() });
                Toast.success(t('修改成功'));
            } else {
                await apiPost('user/user/modifyEmail', { email: emailValue.trim(), code: code.trim() });
                Toast.success(t('修改成功'));
            }
            close();
            loadInfo();
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setSubmitting(false);
        }
    };

    const title = mode === 'password' ? t('修改密码') : mode === 'mobile' ? t('换绑手机') : t('换绑邮箱');

    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('账号安全')}</Typography.Title>

            <Card>
                <div className="pc-security-row">
                    <div>
                        <div>{t('手机')}</div>
                        <div className="pc-muted">{maskMobile(info.mobile) || t('未绑定')}</div>
                    </div>
                    <Button size="small" theme="light" onClick={() => open('mobile')}>
                        {info.mobile ? t('换绑') : t('绑定')}
                    </Button>
                </div>

                <div className="pc-security-row">
                    <div>
                        <div>{t('邮箱')}</div>
                        <div className="pc-muted">{info.email || t('未绑定')}</div>
                    </div>
                    <Button size="small" theme="light" onClick={() => open('email')}>
                        {info.email ? t('换绑') : t('绑定')}
                    </Button>
                </div>

                <div className="pc-security-row">
                    <div>
                        <div>{t('登录密码')}</div>
                        <div className="pc-muted">{t('已设置')}</div>
                    </div>
                    <Button size="small" theme="light" onClick={() => open('password')}>
                        {t('修改')}
                    </Button>
                </div>
            </Card>

            <Modal visible={!!mode} title={title} onOk={() => void submit()} onCancel={close} okText={t('提交')} cancelText={t('取消')}>
                <div className="pc-form">
                    {mode === 'mobile' ? (
                        <Input value={mobileValue} onChange={(v) => setMobileValue(String(v))} placeholder={t('请输入新手机号')} />
                    ) : null}
                    {mode === 'email' ? (
                        <Input value={emailValue} onChange={(v) => setEmailValue(String(v))} placeholder={t('请输入新邮箱')} />
                    ) : null}

                    <div className="pc-code-row">
                        <Input value={code} onChange={(v) => setCode(String(v))} placeholder={t('请输入验证码')} />
                        <Button size="small" disabled={countdown > 0} onClick={() => void sendCode()}>
                            {countdown > 0 ? `${countdown}s` : t('获取验证码')}
                        </Button>
                    </div>

                    {mode === 'password' ? (
                        <>
                            <Input
                                mode="password"
                                value={password}
                                onChange={(v) => setPassword(String(v))}
                                placeholder={t('请输入新密码')}
                            />
                            <Input
                                mode="password"
                                value={confirm}
                                onChange={(v) => setConfirm(String(v))}
                                placeholder={t('请再次输入新密码')}
                            />
                        </>
                    ) : null}
                    <div className="pc-muted">
                        {mode === 'password'
                            ? t('验证码发送到当前手机 {mobile}', { mobile: maskMobile(info.mobile) || '-' })
                            : t('验证码发送到新填写的手机/邮箱')}
                    </div>
                </div>
            </Modal>
        </div>
    );
}
