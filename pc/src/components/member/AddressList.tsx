'use client';

import { useCallback, useEffect, useState } from 'react';
import { Button, Card, Empty, Input, Modal, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost, type Paged } from '@/lib/client-api';

/** Vue 基准：pages/member/address/List.vue、EditRegion.vue。 */
interface AddressRow {
    addressId?: number;
    id?: number;
    consignee?: string;
    mobile?: string;
    telephone?: string;
    address?: string;
    regionName?: string;
    isDefault?: number;
}

export default function AddressList() {
    const { t } = useTranslation();
    const [rows, setRows] = useState<AddressRow[]>([]);
    const [loading, setLoading] = useState(true);
    const [deleting, setDeleting] = useState<AddressRow | null>(null);
    const [adding, setAdding] = useState(false);
    const [form, setForm] = useState({ consignee: '', mobile: '', address: '' });

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
        const id = deleting.addressId ?? deleting.id;
        try {
            await apiPost('user/address/del', { id, addressId: id });
            Toast.success(t('操作成功'));
            setDeleting(null);
            await load();
        } catch (e) {
            Toast.error((e as Error).message);
        }
    };

    const create = async () => {
        try {
            await apiPost('user/address/add', form);
            Toast.success(t('操作成功'));
            setAdding(false);
            setForm({ consignee: '', mobile: '', address: '' });
            await load();
        } catch (e) {
            Toast.error((e as Error).message);
        }
    };

    return (
        <div className="pc-member-panel">
            <div className="pc-panel-head">
                <Typography.Title heading={4}>{t('收货地址')}</Typography.Title>
                <Button theme="solid" type="primary" size="small" onClick={() => setAdding(true)}>
                    {t('新增地址')}
                </Button>
            </div>

            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : rows.length === 0 ? (
                <Empty title={t('暂无地址')} />
            ) : (
                <div className="pc-card-grid">
                    {rows.map((row, index) => (
                        <Card key={row.addressId ?? row.id ?? index} className="pc-order-card">
                            <div className="pc-addr-name">
                                {row.consignee}
                                {row.isDefault ? <Tag size="small" color="green">{t('默认')}</Tag> : null}
                            </div>
                            <div className="pc-muted">{row.mobile ?? row.telephone}</div>
                            <div className="pc-muted">{[row.regionName, row.address].filter(Boolean).join(' ')}</div>
                            <div className="pc-order-actions">
                                <Button size="small" theme="light" onClick={() => setDeleting(row)}>{t('删除')}</Button>
                            </div>
                        </Card>
                    ))}
                </div>
            )}

            <Modal visible={!!deleting} title={t('确认操作')} onOk={() => void remove()} onCancel={() => setDeleting(null)}>
                {t('确定要删除该地址吗？')}
            </Modal>

            <Modal visible={adding} title={t('新增地址')} onOk={() => void create()} onCancel={() => setAdding(false)}>
                <div className="pc-form">
                    <Input placeholder={t('收货人')} value={form.consignee} onChange={(v) => setForm({ ...form, consignee: String(v) })} />
                    <Input placeholder={t('联系电话')} value={form.mobile} onChange={(v) => setForm({ ...form, mobile: String(v) })} />
                    <Input placeholder={t('详细地址')} value={form.address} onChange={(v) => setForm({ ...form, address: String(v) })} />
                </div>
            </Modal>
        </div>
    );
}
