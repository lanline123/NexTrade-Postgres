'use client';

import { useEffect, useState } from 'react';
import { Button, Card, Input, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';

/**
 * 会员实名认证。
 * Vue 基准：pages/member/userCertification/Info.vue（个人/企业双类型，多步 + 图片上传）。
 * 当前实现：状态查询 + 基础资料提交；证件照/营业执照上传未接（等素材中心），
 * 因此提交时若后端要求图片字段会失败——这一步需要等上传能力补齐。
 */
interface MyApply {
    id?: number;
    status?: number;
    type?: number;
    companyName?: string;
    corporateName?: string;
    documentNumber?: string;
    contactPhone?: string;
}

const STATUS_TEXT: Record<number, string> = {
    0: '待提交',
    1: '审核中',
    2: '已通过',
    3: '已驳回'
};

export default function CertificationPanel() {
    const { t } = useTranslation();
    const [apply, setApply] = useState<MyApply | null>(null);
    const [loading, setLoading] = useState(true);
    const [type, setType] = useState(1);
    const [form, setForm] = useState({ corporateName: '', documentNumber: '', contactPhone: '', residentialAddress: '' });
    const [submitting, setSubmitting] = useState(false);

    useEffect(() => {
        apiGet<MyApply>('user/company/myApply')
            .then((result) => setApply(result && result.id ? result : null))
            .catch(() => setApply(null))
            .finally(() => setLoading(false));
    }, []);

    const submit = async () => {
        if (!form.corporateName.trim() || !form.documentNumber.trim()) {
            Toast.warning(t('请填写姓名/企业名称与证件号'));
            return;
        }
        setSubmitting(true);
        try {
            await apiPost('user/company/apply', {
                type,
                companyData: {
                    corporateName: form.corporateName.trim(),
                    documentNumber: form.documentNumber.trim(),
                    contactPhone: form.contactPhone.trim(),
                    residentialAddress: form.residentialAddress.trim(),
                    documentType: 1,
                    certificateValidityPeriod: 1
                }
            });
            Toast.success(t('已提交，等待审核'));
            const next = await apiGet<MyApply>('user/company/myApply').catch(() => null);
            setApply(next && next.id ? next : null);
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setSubmitting(false);
        }
    };

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;

    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('实名认证')}</Typography.Title>

            {apply ? (
                <Card>
                    <div className="pc-row-head">
                        <span>{apply.type === 2 ? t('企业认证') : t('个人认证')}</span>
                        <Tag size="small" color={apply.status === 2 ? 'green' : apply.status === 3 ? 'red' : 'blue'}>
                            {t(STATUS_TEXT[apply.status ?? 0] ?? '待提交')}
                        </Tag>
                    </div>
                    <div className="pc-detail-grid">
                        <div><span className="pc-muted">{t('名称')}</span> {apply.companyName ?? apply.corporateName ?? '-'}</div>
                        <div><span className="pc-muted">{t('证件号')}</span> {apply.documentNumber ?? '-'}</div>
                        <div><span className="pc-muted">{t('联系电话')}</span> {apply.contactPhone ?? '-'}</div>
                    </div>
                </Card>
            ) : (
                <Card title={t('提交认证资料')}>
                    <div className="pc-form">
                        <label className="pc-form-label">{t('认证类型')}</label>
                        <div className="pc-tabs">
                            <button type="button" className={type === 1 ? 'is-active' : ''} onClick={() => setType(1)}>
                                {t('个人')}
                            </button>
                            <button type="button" className={type === 2 ? 'is-active' : ''} onClick={() => setType(2)}>
                                {t('企业')}
                            </button>
                        </div>

                        <label className="pc-form-label">{type === 2 ? t('企业名称') : t('姓名')}</label>
                        <Input
                            value={form.corporateName}
                            onChange={(v) => setForm({ ...form, corporateName: String(v) })}
                            placeholder={type === 2 ? t('请输入企业名称') : t('请输入姓名')}
                        />

                        <label className="pc-form-label">{t('证件号码')}</label>
                        <Input
                            value={form.documentNumber}
                            onChange={(v) => setForm({ ...form, documentNumber: String(v) })}
                            placeholder={t('请输入证件号码')}
                        />

                        <label className="pc-form-label">{t('联系电话')}</label>
                        <Input
                            value={form.contactPhone}
                            onChange={(v) => setForm({ ...form, contactPhone: String(v) })}
                            placeholder={t('请输入联系电话')}
                        />

                        <label className="pc-form-label">{t('常住地址')}</label>
                        <Input
                            value={form.residentialAddress}
                            onChange={(v) => setForm({ ...form, residentialAddress: String(v) })}
                            placeholder={t('请输入常住地址')}
                        />

                        <div>
                            <Button theme="solid" type="primary" loading={submitting} onClick={() => void submit()}>
                                {t('提交认证')}
                            </Button>
                        </div>

                        <div className="pc-note">
                            {t('证件照/营业执照上传尚未接入，后端若校验图片字段会提示失败；等素材中心上传能力接上后补齐。')}
                        </div>
                    </div>
                </Card>
            )}
        </div>
    );
}
