'use client';

import { useEffect, useState } from 'react';
import { Button, Input } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import {
    brandIdOf,
    categoryIdOf,
    type CatalogQuery,
    type FilterOptions,
    type SelectedAttr,
    type SortField,
} from '@/lib/catalog';

/**
 * 商品筛选栏（列表 / 搜索 / 店铺内共用）。
 * Vue 基准：pages/search/components/{Filter.vue, SortFilter.vue}。
 *
 * 排序口径照 Vue：`sort` = sale|price|time，`order` = asc|desc；「综合」两者都空。
 * 分类单选（`cat`）、品牌多选（`brand` 逗号分隔）、价格 `min`/`max`、属性 `attrs[{attrName,attrValue[]}]`。
 */
interface Props {
    options: FilterOptions;
    query: CatalogQuery;
    total?: number;
    loading?: boolean;
    onChange: (patch: Partial<CatalogQuery>) => void;
    onReset: () => void;
}

const COLLAPSED = 10;

export default function FilterBar({ options, query, total, loading, onChange, onReset }: Props) {
    const { t } = useTranslation();
    const [catExpanded, setCatExpanded] = useState(false);
    const [brandExpanded, setBrandExpanded] = useState(false);
    const [priceMin, setPriceMin] = useState(query.min ? String(query.min) : '');
    const [priceMax, setPriceMax] = useState(query.max ? String(query.max) : '');

    useEffect(() => {
        setPriceMin(query.min ? String(query.min) : '');
        setPriceMax(query.max ? String(query.max) : '');
    }, [query.min, query.max]);

    const brandIds = query.brand ? query.brand.split(',').filter(Boolean) : [];
    const attrs = query.attrs ?? [];

    const toggleBrand = (id: string) => {
        const next = brandIds.includes(id) ? brandIds.filter((item) => item !== id) : [...brandIds, id];
        onChange({ brand: next.join(',') || undefined, page: 1 });
    };

    const toggleAttr = (name: string, value: string) => {
        const current = attrs.find((item) => item.attrName === name);
        const values = current?.attrValue ?? [];
        const nextValues = values.includes(value) ? values.filter((item) => item !== value) : [...values, value];
        const next: SelectedAttr[] = attrs
            .map((item) => (item.attrName === name ? { attrName: name, attrValue: nextValues } : item))
            .filter((item) => item.attrValue.length > 0);
        if (!current && nextValues.length) next.push({ attrName: name, attrValue: nextValues });
        onChange({ attrs: next, page: 1 });
    };

    const applyPrice = () => {
        const min = Number(priceMin) || 0;
        const max = Number(priceMax) || 0;
        if (min > 0 && max > 0 && min > max) return;
        onChange({ min: min || undefined, max: max || undefined, page: 1 });
    };

    const sortValue = query.sort && query.order ? `${query.sort}:${query.order}` : '';
    const sortItems: { key: string; label: string; patch: Partial<CatalogQuery> }[] = [
        { key: '', label: t('综合'), patch: { sort: '', order: '' } },
        { key: 'sale:desc', label: t('销量'), patch: { sort: 'sale', order: 'desc' } },
    ];

    const priceAscActive = query.sort === 'price' && query.order === 'asc';
    const priceDescActive = query.sort === 'price' && query.order === 'desc';

    const selectedChips: { key: string; label: string; clear: () => void }[] = [];
    if (query.keyword) selectedChips.push({ key: 'kw', label: query.keyword, clear: () => onChange({ keyword: undefined, page: 1 }) });
    if (query.cat) {
        const name = options.categories.find((row) => categoryIdOf(row) === query.cat)?.categoryName;
        selectedChips.push({ key: 'cat', label: name ?? query.cat, clear: () => onChange({ cat: undefined, page: 1 }) });
    }
    brandIds.forEach((id) => {
        const name = options.brands.find((row) => brandIdOf(row) === id)?.brandName ?? id;
        selectedChips.push({
            key: `b-${id}`,
            label: name,
            clear: () => {
                const next = brandIds.filter((item) => item !== id);
                onChange({ brand: next.join(',') || undefined, page: 1 });
            },
        });
    });
    if (query.min || query.max) {
        selectedChips.push({
            key: 'price',
            label: `${query.min ?? 0} - ${query.max ?? '∞'}`,
            clear: () => onChange({ min: undefined, max: undefined, page: 1 }),
        });
    }
    attrs.forEach((item) =>
        item.attrValue.forEach((value) =>
            selectedChips.push({
                key: `a-${item.attrName}-${value}`,
                label: `${item.attrName}：${value}`,
                clear: () => toggleAttr(item.attrName, value),
            }),
        ),
    );

    const catList = catExpanded ? options.categories : options.categories.slice(0, COLLAPSED);
    const brandList = brandExpanded ? options.brands : options.brands.slice(0, COLLAPSED);

    return (
        <div className="pc-filter">
            <div className="pc-filter-sort">
                <div className="pc-filter-sort-tabs">
                    {sortItems.map((item) => (
                        <span
                            key={item.key || 'default'}
                            className={`pc-sort-tab${sortValue === item.key ? ' is-active' : ''}`}
                            onClick={() => onChange({ ...item.patch, page: 1 })}
                        >
                            {item.label}
                        </span>
                    ))}
                    <span
                        className={`pc-sort-tab${priceAscActive || priceDescActive ? ' is-active' : ''}`}
                        onClick={() =>
                            // 价格：先升序，再点一次转降序，第三次仍降序（与 Vue 一致）
                            onChange({ sort: 'price', order: priceAscActive ? 'desc' : 'asc', page: 1 })
                        }
                    >
                        {t('价格')}
                        <span className="pc-sort-arrows">
                            <i className={priceAscActive ? 'is-on' : ''}>↑</i>
                            <i className={priceDescActive ? 'is-on' : ''}>↓</i>
                        </span>
                    </span>
                    <span
                        className={`pc-sort-tab${sortValue === 'time:desc' ? ' is-active' : ''}`}
                        onClick={() => onChange({ sort: 'time' as SortField, order: 'desc', page: 1 })}
                    >
                        {t('新品')}
                    </span>
                </div>
                <div className="pc-filter-total">
                    {loading ? t('加载中') : `${t('共')} ${total ?? 0} ${t('件商品')}`}
                </div>
            </div>

            {options.categories.length ? (
                <div className="pc-filter-row">
                    <span className="pc-filter-label">{t('分类')}</span>
                    <div className="pc-filter-values">
                        <span
                            className={`pc-filter-chip${!query.cat ? ' is-active' : ''}`}
                            onClick={() => onChange({ cat: undefined, page: 1 })}
                        >
                            {t('全部')}
                        </span>
                        {catList.map((row) => {
                            const id = categoryIdOf(row);
                            return (
                                <span
                                    key={id}
                                    className={`pc-filter-chip${query.cat === id ? ' is-active' : ''}`}
                                    onClick={() => onChange({ cat: query.cat === id ? undefined : id, page: 1 })}
                                >
                                    {row.categoryName}
                                </span>
                            );
                        })}
                        {options.categories.length > COLLAPSED ? (
                            <span className="pc-filter-more" onClick={() => setCatExpanded(!catExpanded)}>
                                {catExpanded ? t('收起') : t('更多')}
                            </span>
                        ) : null}
                    </div>
                </div>
            ) : null}

            {options.brands.length ? (
                <div className="pc-filter-row">
                    <span className="pc-filter-label">{t('品牌')}</span>
                    <div className="pc-filter-values">
                        {brandList.map((row) => {
                            const id = brandIdOf(row);
                            return (
                                <span
                                    key={id}
                                    className={`pc-filter-chip${brandIds.includes(id) ? ' is-active' : ''}`}
                                    onClick={() => toggleBrand(id)}
                                >
                                    {row.brandName}
                                </span>
                            );
                        })}
                        {options.brands.length > COLLAPSED ? (
                            <span className="pc-filter-more" onClick={() => setBrandExpanded(!brandExpanded)}>
                                {brandExpanded ? t('收起') : t('更多')}
                            </span>
                        ) : null}
                    </div>
                </div>
            ) : null}

            <div className="pc-filter-row">
                <span className="pc-filter-label">{t('价格')}</span>
                <div className="pc-filter-values pc-filter-price">
                    <Input
                        size="small"
                        value={priceMin}
                        placeholder={t('最低价')}
                        onChange={(v) => setPriceMin(String(v).replace(/[^\d.]/g, ''))}
                        style={{ width: 96 }}
                    />
                    <span className="pc-muted">-</span>
                    <Input
                        size="small"
                        value={priceMax}
                        placeholder={t('最高价')}
                        onChange={(v) => setPriceMax(String(v).replace(/[^\d.]/g, ''))}
                        style={{ width: 96 }}
                    />
                    <Button size="small" theme="light" onClick={applyPrice}>
                        {t('确定')}
                    </Button>
                    {options.maxPrice > 0 ? (
                        <span className="pc-muted" style={{ fontSize: 12 }}>
                            {t('最高')} {options.maxPrice}
                        </span>
                    ) : null}
                </div>
            </div>

            {options.attrs.map((attr) => (
                <div className="pc-filter-row" key={attr.name}>
                    <span className="pc-filter-label">{attr.name}</span>
                    <div className="pc-filter-values">
                        {attr.values.slice(0, 16).map((option) => {
                            const checked = attrs.some(
                                (item) => item.attrName === attr.name && item.attrValue.includes(option.value),
                            );
                            return (
                                <span
                                    key={option.value}
                                    className={`pc-filter-chip${checked ? ' is-active' : ''}`}
                                    onClick={() => toggleAttr(attr.name, option.value)}
                                >
                                    {option.value}
                                    {option.count ? <em className="pc-filter-count">({option.count})</em> : null}
                                </span>
                            );
                        })}
                    </div>
                </div>
            ))}

            {selectedChips.length ? (
                <div className="pc-filter-row pc-filter-selected">
                    <span className="pc-filter-label">{t('已选')}</span>
                    <div className="pc-filter-values">
                        {selectedChips.map((chip) => (
                            <span key={chip.key} className="pc-filter-chip is-active" onClick={chip.clear}>
                                {chip.label}
                                <em className="pc-filter-x">×</em>
                            </span>
                        ))}
                        <span className="pc-filter-more" onClick={onReset}>
                            {t('清除全部')}
                        </span>
                    </div>
                </div>
            ) : null}
        </div>
    );
}
