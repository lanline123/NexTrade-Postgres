'use client';

import { useCallback, useEffect, useState } from 'react';
import { Button, Card, Empty, Modal, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost, type Paged } from '@/lib/client-api';
import AddressForm from '@/components/member/AddressForm';
import { regionLabel } from '@/lib/region';

/**
 * Vue 基准：pages/member/address/List.vue + EditRegion.vue + components/region/src/ModalRegion.vue
 *
 * 真实接口（实测）：
 * - GET  user/address/list          → records[{addressId, consignee, mobile, mobileAreaCode,
 *                                       regionName, regionNames[], regionIds[], address, isDefault, isSelected}]
 * - POST user/address/setSelected {id}
 * - POST user/address/del {id, addressId}
 * - 新增/修改见 AddressForm（create / update，注意不是 add）
 */
interface AddressRow {
    addressId: number;
    consignee?: string;
    mobile?: string;
    mobileAreaCode?: string;
    telephone?: string;
    address?: string;
    regionName?: string;
    regionNames?: string[];
    isDefault?: number;
    isSelected?: number;
}

export default function AddressList() {
    const { t } = useTranslation();
    const [rows, setRows] = useState<AddressRow[]>([]);
    const [loading, setLoading] = useState(true);
    const [deleting, setDeleting] = useState<AddressRow | null>(null);
    const [editing, setEditing] = useState<{ addressId?: number } | null>(null);

    const load = useCallback(async () => {
        setLoading(true);
        try {
            const result = await apiGet<Paged<AddressRow>>('user/address/list', { page: 1, size: 50 });
            setRows(result.records ?? []);
        } catch (e) {
            Toast.error((e as Error).message);
            setRows([]);
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        void load();
    }, [load]);

    const remove = async () => {
        if (!deleting) return;
        try {
            await apiPost('user/address/del', { id: deleting.addressId, addressId: deleting.addressId });
            Toast.success(t('操作成功'));
            setDeleting(null);
            await load();
        } catch (e) {
            Toast.error((e as Error).message);
        }
    };

    const setDefault = async (row: AddressRow) => {
        try {
            await apiPost('user/address/setSelected', { id: row.addressId });
            Toast.success(t('操作成功'));
            await load();
        } catch (e) {
            Toast.error((e as Error).message);
        }
    };

    return (
        <div className="pc-member-panel">
            <div className="pc-panel-head">
                <Typography.Title heading={4}>{t('收货地址')}</Typography.Title>
                <Button theme="solid" type="primary" size="small" onClick={() => setEditing({})}>
                    {t('添加新地址')}
                </Button>
            </div>

            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : rows.length === 0 ? (
                <Empty title={t('暂无地址')} />
            ) : (
                <div className="pc-card-grid">
                    {rows.map((row) => (
                        <Card key={row.addressId} className="pc-order-card pc-address-card">
                            <div className="pc-addr-name">
                                <span>{row.consignee || '-'}</span>
                                {row.isDefault ? (
                                    <Tag size="small" color="green">
                                        {t('默认')}
                                    </Tag>
                                ) : null}
                            </div>
                            <div className="pc-muted">
                                +{row.mobileAreaCode || '86'} {row.mobile || row.telephone || '-'}
                            </div>
                            <div className="pc-muted">{regionLabel(row.regionNames, row.regionName)}</div>
                            <div className="pc-addr-detail">{row.address}</div>
                            <div className="pc-order-actions">
                                <Button
                                    size="small"
                                    theme="light"
                                    onClick={() => setEditing({ addressId: row.addressId })}
                                >
                                    {t('编辑')}
                                </Button>
                                <Button
                                    size="small"
                                    theme="light"
                                    disabled={!!row.isDefault}
                                    onClick={() => void setDefault(row)}
                                >
                                    {t('设为默认')}
                                </Button>
                                <Button size="small" theme="light" type="danger" onClick={() => setDeleting(row)}>
                                    {t('删除')}
                                </Button>
                            </div>
                        </Card>
                    ))}
                </div>
            )}

            <Modal
                visible={!!deleting}
                title={t('确认操作')}
                onOk={() => void remove()}
                onCancel={() => setDeleting(null)}
            >
                {t('确定要删除该地址吗？')}
            </Modal>

            <Modal
                visible={!!editing}
                title={editing?.addressId ? t('编辑收货地址') : t('添加新地址')}
                footer={null}
                onCancel={() => setEditing(null)}
                width={640}
            >
                {editing ? (
                    <AddressForm
                        addressId={editing.addressId}
                        onCancel={() => setEditing(null)}
                        onSaved={() => {
                            setEditing(null);
                            void load();
                        }}
                    />
                ) : null}
            </Modal>
        </div>
    );
}
