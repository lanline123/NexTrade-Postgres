'use client';

import { useRouter } from 'next/navigation';
import { Card, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import AddressForm from '@/components/member/AddressForm';

/** Vue 基准：pages/member/address/EditRegion.vue（?act=add|edit&addressId=x） */
export default function AddressEditPage({ addressId, act }: { addressId: number; act?: string }) {
    const { t } = useTranslation();
    const router = useRouter();
    const isEdit = addressId > 0 || act === 'edit';

    return (
        <div className="pc-member-panel">
            <div className="pc-panel-head">
                <Typography.Title heading={4}>{isEdit ? t('编辑收货地址') : t('添加新地址')}</Typography.Title>
            </div>
            <Card className="pc-form-card">
                <AddressForm
                    addressId={isEdit ? addressId : undefined}
                    onCancel={() => router.push('/member/address/list')}
                    onSaved={() => router.replace('/member/address/list')}
                    submitText={t('保存')}
                />
            </Card>
        </div>
    );
}
