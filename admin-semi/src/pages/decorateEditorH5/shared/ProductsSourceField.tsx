/**
 * 商品来源设置控件（对齐 view/admin src/modules/src/commonProduct/Edit.vue CommonProductEdit）
 *
 * 选商品 / 按分类（分类树）/ 按标签（商品标签）三种来源。
 * 选商品 2026-11-15 合一：弃旧 ProductPicker（按钮+表格+SideSheet），改走与 PC 编辑器
 * 相同的 ProductPickGrid（卡片网格）+ 共享 ProductPickerModal（卡片选品弹窗）。
 * 数据契约不变：模块数据仍只写 productIds（H5 数据本就没有 productItems 展示缓存字段），
 * 网格需要的展示信息只在本组件内用 useState 做本地缓存，不落模块数据。
 */
import { useEffect, useState } from 'react';
import { Input, Radio, RadioGroup, Select, Spin } from '@douyinfe/semi-ui';
import { apiGet } from '../../../api';
import { useI18n } from '../../../i18n';
import ProductPickGrid, { mergePickedItems } from '../../../components/ProductPickGrid';
import ProductPickerModal from '../../../components/ProductPickerModal';

export default function ProductsSourceField({
    data,
    onChange,
    field = 'products',
}: {
    data: Record<string, any>;
    onChange: (patch: Record<string, any>) => void;
    field?: string;
}) {
    const { t } = useI18n();
    const products = (data?.[field] || {}) as Record<string, any>;
    const type = Number(products.productSelectType || 1);
    const [categories, setCategories] = useState<Array<{ label: string; value: number }>>([]);
    const [loadingCategory, setLoadingCategory] = useState(false);
    const [pickerVisible, setPickerVisible] = useState(false);
    // 展示信息本地缓存（仅供 ProductPickGrid 渲染卡片，不写回模块数据）
    const [itemsCache, setItemsCache] = useState<any[]>([]);

    const set = (patch: Record<string, any>) => onChange({ [field]: { ...products, ...patch } });

    useEffect(() => {
        setLoadingCategory(true);
        apiGet<any[]>('/product/category/getAllCategory')
            .then((tree) => {
                const flat: Array<{ label: string; value: number }> = [];
                const walk = (nodes: any[], prefix = '') => {
                    (Array.isArray(nodes) ? nodes : []).forEach((node) => {
                        flat.push({
                            label: prefix ? `${prefix} / ${node.categoryName}` : node.categoryName,
                            value: node.categoryId,
                        });
                        if (Array.isArray(node.children) && node.children.length) walk(node.children, node.categoryName);
                    });
                };
                walk(Array.isArray(tree) ? tree : []);
                setCategories(flat);
            })
            .catch(() => undefined)
            .finally(() => setLoadingCategory(false));
    }, []);

    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <RadioGroup value={type} onChange={(e: any) => set({ productSelectType: Number(e.target.value) })}>
                <Radio value={1}>{t('选商品')}</Radio>
                <Radio value={2}>{t('按分类')}</Radio>
                <Radio value={3}>{t('按标签')}</Radio>
            </RadioGroup>
            {type === 1 ? (
                <>
                    <ProductPickGrid
                        ids={Array.isArray(products.productIds) ? products.productIds.map(Number) : []}
                        items={itemsCache}
                        onChange={(nextIds, nextItems) => {
                            setItemsCache(nextItems);
                            const currentIds = Array.isArray(products.productIds) ? products.productIds.map(Number) : [];
                            // 网格的历史数据自动补全只更新展示缓存，ids 未变时不写数据（避免无谓补丁）
                            if (nextIds.join(',') !== currentIds.join(',')) set({ productIds: nextIds });
                        }}
                        onRequestPick={() => setPickerVisible(true)}
                    />
                    {pickerVisible && (
                        <ProductPickerModal
                            visible
                            selectedIds={Array.isArray(products.productIds) ? products.productIds.map(Number) : []}
                            max={50}
                            onOk={(nextIds, rows) => {
                                setItemsCache(mergePickedItems(nextIds, rows, itemsCache));
                                set({ productIds: nextIds });
                                setPickerVisible(false);
                            }}
                            onCancel={() => setPickerVisible(false)}
                        />
                    )}
                </>
            ) : null}
            {type === 2 ? (
                <Spin spinning={loadingCategory}>
                    <Select
                        value={products.productCategoryId ? Number(products.productCategoryId) : undefined}
                        placeholder={t('选择分类')}
                        showClear
                        filter
                        style={{ width: '100%' }}
                        optionList={categories}
                        onChange={(next) => set({ productCategoryId: Number(next) || 0 })}
                    />
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 8 }}>
                        <span style={{ color: '#646a73', fontSize: 12 }}>{t('商品数量')}</span>
                        <Input
                            value={String(products.productNumber ?? 6)}
                            onChange={(v) => set({ productNumber: Number(v) || 0 })}
                            style={{ width: 90 }}
                        />
                    </div>
                </Spin>
            ) : null}
            {type === 3 ? (
                <>
                    <Input
                        value={String(products.productTag ?? '')}
                        placeholder={t('商品标签（introType：new/recommend/hot）')}
                        onChange={(v) => set({ productTag: v })}
                    />
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                        <span style={{ color: '#646a73', fontSize: 12 }}>{t('商品数量')}</span>
                        <Input
                            value={String(products.productNumber ?? 6)}
                            onChange={(v) => set({ productNumber: Number(v) || 0 })}
                            style={{ width: 90 }}
                        />
                    </div>
                </>
            ) : null}
        </div>
    );
}
