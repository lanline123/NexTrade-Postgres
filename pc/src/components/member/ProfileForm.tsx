'use client';

import { useEffect, useState } from 'react';
import { Avatar, Button, Card, Input, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';

/** Vue 基准：pages/member/profile/Info.vue；接口 user/user/detail + updateInformation。 */
interface ProfileData {
    userId?: number;
    nickname?: string;
    birthday?: string;
    email?: string;
    mobile?: string;
    avatar?: string;
}

export default function ProfileForm() {
    const { t } = useTranslation();
    const [form, setForm] = useState<ProfileData>({});
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [error, setError] = useState('');

    useEffect(() => {
        apiGet<{ item?: ProfileData }>('user/user/detail')
            .then((result) => setForm(result.item ?? {}))
            .catch((e: Error) => setError(e.message))
            .finally(() => setLoading(false));
    }, []);

    const save = async () => {
        if (!form.nickname?.trim()) {
            Toast.warning(t('请输入昵称'));
            return;
        }
        setSaving(true);
        try {
            await apiPost('user/user/updateInformation', {
                nickname: form.nickname,
                birthday: form.birthday,
                email: form.email,
                mobile: form.mobile
            });
            Toast.success(t('保存成功'));
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setSaving(false);
        }
    };

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;

    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('个人资料')}</Typography.Title>

            <Card>
                <div className="pc-profile-head">
                    <Avatar size="large" src={form.avatar || '/assets/avatar/default.gif'} alt={form.nickname ?? ''} />
                    <div className="pc-muted">{t('头像上传待接入素材中心')}</div>
                </div>

                <div className="pc-form">
                    <label className="pc-form-label">{t('昵称')}</label>
                    <Input value={form.nickname ?? ''} onChange={(v) => setForm({ ...form, nickname: String(v) })} placeholder={t('请输入昵称')} />

                    <label className="pc-form-label">{t('生日')}</label>
                    <Input value={form.birthday ?? ''} onChange={(v) => setForm({ ...form, birthday: String(v) })} placeholder="YYYY-MM-DD" />

                    <label className="pc-form-label">{t('手机')}</label>
                    <Input value={form.mobile ?? ''} disabled placeholder={t('手机号在账号安全中修改')} />

                    <label className="pc-form-label">{t('邮箱')}</label>
                    <Input value={form.email ?? ''} onChange={(v) => setForm({ ...form, email: String(v) })} placeholder={t('请输入邮箱')} />

                    <div>
                        <Button theme="solid" type="primary" loading={saving} onClick={() => void save()}>
                            {t('保存')}
                        </Button>
                    </div>
                </div>
            </Card>

            {error ? <div className="pc-note">{error}</div> : null}
        </div>
    );
}
