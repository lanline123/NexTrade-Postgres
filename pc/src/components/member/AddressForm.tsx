'use client';

import { useCallback, useEffect, useState } from 'react';
import { Button, Input, Toast } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';
import { ROOT_REGION_ID } from '@/lib/region';
import RegionPicker from '@/components/region/RegionPicker';

/**
 * Vue 基准：components/member/address/EditRegion.vue
 *
 * 真实接口（实测）：
 * - GET  user/address/detail?id=            → 地址详情（含 regionIds / regionNames）
 * - POST user/address/create {…}             → data = 新 addressId
 * - POST user/address/update {…, addressId}  → 更新
 *
 * 注意：Vue 原版把 add 映射成 operation `create`（不是 `add`，`user/address/add` 会 404）。
 */
export interface AddressFormValue {
    addressId?: number;
    consignee: string;
    regionIds: number[];
    regionNames: string[];
    address: string;
    mobile: string;
    telephone: string;
    email: string;
}

const EMPTY: AddressFormValue = {
    consignee: '',
    regionIds: [],
    regionNames: [],
    address: '',
    mobile: '',
    telephone: '',
    email: '',
};

interface Props {
    addressId?: number;
    onSaved?: (addressId?: number) => void;
    onCancel?: () => void;
    submitText?: string;
}

export default function AddressForm({ addressId, onSaved, onCancel, submitText }: Props) {
    const { t } = useTranslation();
    const [form, setForm] = useState<AddressFormValue>(EMPTY);
    const [loading, setLoading] = useState(!!addressId);
    const [saving, setSaving] = useState(false);

    const load = useCallback(async () => {
        if (!addressId) {
            setForm(EMPTY);
            setLoading(false);
            return;
        }
        setLoading(true);
        try {
            const detail = await apiGet<AddressFormValue>('user/address/detail', { id: addressId });
            setForm({ ...EMPTY, ...detail, regionIds: detail.regionIds ?? [] });
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setLoading(false);
        }
    }, [addressId]);

    useEffect(() => {
        void load();
    }, [load]);

    const patch = (next: Partial<AddressFormValue>) => setForm((prev) => ({ ...prev, ...next }));

    const submit = async () => {
        if (!form.consignee.trim()) return Toast.warning(t('请您填写收货人姓名'));
        if (!form.regionIds.length) return Toast.warning(t('请选择收货人所在地区'));
        if (!form.address.trim()) return Toast.warning(t('请您填写详细地址'));
        if (!form.mobile.trim()) return Toast.warning(t('请您填写收货人手机号码'));

        const payload = {
            consignee: form.consignee.trim(),
            regionIds: form.regionIds.length ? form.regionIds : [ROOT_REGION_ID],
            regionNames: form.regionNames,
            address: form.address.trim(),
            mobile: form.mobile.trim(),
            telephone: form.telephone.trim(),
            email: form.email.trim(),
        };

        setSaving(true);
        try {
            if (addressId) {
                await apiPost('user/address/update', { ...payload, addressId });
                Toast.success(t('操作成功'));
                onSaved?.(addressId);
            } else {
                const newId = await apiPost<number>('user/address/create', payload);
                Toast.success(t('操作成功'));
                onSaved?.(typeof newId === 'number' ? newId : undefined);
            }
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setSaving(false);
        }
    };

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;

    return (
        <div className="pc-form pc-address-form">
            <label className="pc-field">
                <span className="pc-field-label">
                    {t('收货人')}
                    <i className="pc-required">*</i>
                </span>
                <Input
                    value={form.consignee}
                    maxLength={20}
                    placeholder={t('请您填写收货人姓名')}
                    onChange={(v) => patch({ consignee: String(v) })}
                />
            </label>

            <label className="pc-field">
                <span className="pc-field-label">
                    {t('所在地区')}
                    <i className="pc-required">*</i>
                </span>
                <RegionPicker
                    value={form.regionIds}
                    names={form.regionNames}
                    onChange={(regionIds, regionNames) => patch({ regionIds, regionNames })}
                />
            </label>

            <label className="pc-field">
                <span className="pc-field-label">
                    {t('详细地址')}
                    <i className="pc-required">*</i>
                </span>
                <Input
                    value={form.address}
                    maxLength={100}
                    placeholder={t('街道、门牌号等')}
                    onChange={(v) => patch({ address: String(v) })}
                />
            </label>

            <label className="pc-field">
                <span className="pc-field-label">
                    {t('手机号码')}
                    <i className="pc-required">*</i>
                </span>
                <Input
                    value={form.mobile}
                    maxLength={20}
                    placeholder={t('请您填写收货人手机号码')}
                    onChange={(v) => patch({ mobile: String(v) })}
                />
            </label>

            <label className="pc-field">
                <span className="pc-field-label">{t('固定电话')}</span>
                <Input value={form.telephone} maxLength={20} onChange={(v) => patch({ telephone: String(v) })} />
            </label>

            <label className="pc-field">
                <span className="pc-field-label">{t('邮箱地址')}</span>
                <Input value={form.email} maxLength={50} onChange={(v) => patch({ email: String(v) })} />
            </label>

            <div className="pc-form-actions">
                <Button theme="solid" type="primary" loading={saving} onClick={() => void submit()}>
                    {submitText ?? t('保存')}
                </Button>
                {onCancel ? (
                    <Button theme="borderless" onClick={onCancel}>
                        {t('取消')}
                    </Button>
                ) : null}
            </div>
        </div>
    );
}
