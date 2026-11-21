/**
 * 商品来源设置控件（对齐 view/admin src/modules/src/commonProduct/Edit.vue CommonProductEdit）
 *
 * 选商品 / 按分类（分类树）/ 按标签（商品标签）三种来源。
 * 2026-11-21 我方新增第四种来源「全部商品」（productSelectType=4，论坛 B 类
 * 237824）：无 ids/分类/标签过滤，按现有分页口径取全站上架商品（含门店商品），
 * 只配商品数量；原版 CommonProductEdit 仅三种来源，此为我方扩展。
 * 选商品 2026-11-15 合一：弃旧 ProductPicker（按钮+表格+SideSheet），改走与 PC 编辑器
 * 相同的 ProductPickGrid（卡片网格）+ 共享 ProductPickerModal（卡片选品弹窗）。
 * 数据契约不变：模块数据仍只写 productIds（H5 数据本就没有 productItems 展示缓存字段），
 * 网格需要的展示信息只在本组件内用 useState 做本地缓存，不落模块数据。
 *
 * 2026-11-17 波次 1 对齐修复（审计 batch-1 product #14/#15/#16、batch-3 productA1 #1/#2/#5）：
 * - 按分类：原版 CommonProductEdit 用 SelectCategory（el-cascader 级联，checkStrictly，
 *   emitPath 默认开），products.productCategoryId 存**分类路径数组**（defaultProducts
 *   默认 []，原版 Preview 亦按 Array.isArray 处理）。此前我方用扁平单选 Select 写标量、
 *   读数组时 Number([1,3]) 得 NaN 显示空白——已改 Semi Cascader（changeOnSelect 对应
 *   checkStrictly），读写均为路径数组；存量标量数据（旧我方版本或 SelectCategory
 *   onChange 落定的末级 id）读取时在分类树中反查完整路径展示，不丢不报错，
 *   用户不操作则不回写、原值保留。
 * - 按标签：原版 ProductTagSelect 固定枚举 ""（请选择）/hot（热销商品）/best（推荐商品）
 *   /new（新品商品）。此前我方为自由文本 Input（占位提示误写 recommend）——已恢复枚举
 *   下拉；存量非枚举自由文本值以原样附加选项兼容显示，用户改选枚举即按原版口径落库。
 * - 商品数量：原版为 number 输入（min=1、step=1 按步校正、宽 100px）。已换 InputNumber
 *   同口径；去掉此前 `?? 6` 的显示兜底（与默认数据 productNumber=0 不符），显示存量原值。
 */
import { useEffect, useMemo, useState } from 'react';
import { Cascader, InputNumber, Radio, RadioGroup, Select, Spin } from '@douyinfe/semi-ui';
import { apiGet } from '../../../api';
import { useI18n } from '../../../i18n';
import ProductPickGrid, { mergePickedItems } from '../../../components/ProductPickGrid';
import ProductPickerModal from '../../../components/ProductPickerModal';

interface CategoryNode {
    label: string;
    value: number;
    children?: CategoryNode[];
}

function toCategoryTree(nodes: any[]): CategoryNode[] {
    return (Array.isArray(nodes) ? nodes : []).map((node) => ({
        label: node.categoryName,
        value: Number(node.categoryId),
        children:
            Array.isArray(node.children) && node.children.length ? toCategoryTree(node.children) : undefined,
    }));
}

/** 在分类树中反查某分类 id 的完整路径（存量标量数据兼容显示用） */
function findCategoryPath(nodes: CategoryNode[], target: number, trail: number[] = []): number[] | null {
    for (const node of nodes) {
        const next = [...trail, node.value];
        if (node.value === target) return next;
        if (node.children) {
            const hit = findCategoryPath(node.children, target, next);
            if (hit) return hit;
        }
    }
    return null;
}

/**
 * 存储值 → 级联路径数组：
 * - 数组（原版口径）原样归一化为 number[]；
 * - 标量（存量旧数据）→ 在分类树中反查路径，查不到时退回 [标量] 本身（不丢值、不报错）；
 * - 空/0/非法 → []。
 */
function toCategoryPath(raw: any, tree: CategoryNode[]): number[] {
    if (Array.isArray(raw)) {
        return raw.map(Number).filter((n) => Number.isFinite(n) && n !== 0);
    }
    if (raw === undefined || raw === null || raw === '') return [];
    const scalar = Number(raw);
    if (!Number.isFinite(scalar) || scalar === 0) return [];
    return findCategoryPath(tree, scalar) ?? [scalar];
}

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
    const [categoryTree, setCategoryTree] = useState<CategoryNode[]>([]);
    const [loadingCategory, setLoadingCategory] = useState(false);
    const [pickerVisible, setPickerVisible] = useState(false);
    // 展示信息本地缓存（仅供 ProductPickGrid 渲染卡片，不写回模块数据）
    const [itemsCache, setItemsCache] = useState<any[]>([]);

    const set = (patch: Record<string, any>) => onChange({ [field]: { ...products, ...patch } });

    useEffect(() => {
        setLoadingCategory(true);
        apiGet<any[]>('/product/category/getAllCategory')
            .then((tree) => setCategoryTree(toCategoryTree(Array.isArray(tree) ? tree : [])))
            .catch(() => undefined)
            .finally(() => setLoadingCategory(false));
    }, []);

    const categoryPath = useMemo(
        () => toCategoryPath(products.productCategoryId, categoryTree),
        [products.productCategoryId, categoryTree],
    );

    // 按标签：原版 ProductTagSelect 固定枚举；存量非枚举值附加原样选项兼容显示
    const rawTag = String(products.productTag ?? '');
    const tagOptions = useMemo(() => {
        const base = [
            { label: t('请选择'), value: '' },
            { label: t('热销商品'), value: 'hot' },
            { label: t('推荐商品'), value: 'best' },
            { label: t('新品商品'), value: 'new' },
        ];
        if (rawTag && !base.some((o) => o.value === rawTag)) {
            base.push({ label: rawTag, value: rawTag });
        }
        return base;
    }, [rawTag, t]);

    const productNumber = Number(products.productNumber);
    const numberRow = (
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 8 }}>
            <span style={{ color: '#646a73', fontSize: 12 }}>{t('商品数量')}</span>
            <InputNumber
                min={1}
                step={1}
                value={Number.isFinite(productNumber) ? productNumber : undefined}
                onChange={(v) => set({ productNumber: typeof v === 'number' ? Math.round(v) : 0 })}
                style={{ width: 100 }}
            />
        </div>
    );

    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <RadioGroup
                value={type}
                onChange={(e: any) => {
                    const nextType = Number(e.target.value);
                    // 切到「全部商品」且数量未配（0/未填）时播种默认 20，避免店面 size=0 被后端 clamp 成 1 件
                    if (nextType === 4 && !(Number(products.productNumber) > 0)) {
                        set({ productSelectType: nextType, productNumber: 20 });
                    } else {
                        set({ productSelectType: nextType });
                    }
                }}
            >
                <Radio value={1}>{t('选商品')}</Radio>
                <Radio value={2}>{t('按分类')}</Radio>
                <Radio value={3}>{t('按标签')}</Radio>
                <Radio value={4}>{t('全部商品')}</Radio>
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
                    <Cascader
                        treeData={categoryTree}
                        value={categoryPath.length ? categoryPath : undefined}
                        placeholder={t('选择分类')}
                        showClear
                        changeOnSelect
                        style={{ width: '100%' }}
                        onChange={(next) =>
                            set({
                                productCategoryId: Array.isArray(next)
                                    ? next.map(Number).filter((n) => Number.isFinite(n))
                                    : [],
                            })
                        }
                    />
                    {numberRow}
                </Spin>
            ) : null}
            {type === 3 ? (
                <>
                    <Select
                        value={rawTag}
                        placeholder={t('请选择')}
                        style={{ width: '100%' }}
                        optionList={tagOptions}
                        onChange={(next) => set({ productTag: String(next ?? '') })}
                    />
                    {numberRow}
                </>
            ) : null}
            {type === 4 ? (
                <>
                    <div style={{ color: '#646a73', fontSize: 12, lineHeight: 1.6 }}>
                        {t('自动展示全站上架商品（含门店商品），无需逐个挑选；新上架商品会自动进入。')}
                    </div>
                    {numberRow}
                </>
            ) : null}
        </div>
    );
}
