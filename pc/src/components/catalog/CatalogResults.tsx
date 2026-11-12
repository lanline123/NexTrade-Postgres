'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import FilterBar from '@/components/catalog/FilterBar';
import ProductResults from '@/components/catalog/ProductResults';
import {
    fetchAttrSuggestions,
    fetchFilterOptions,
    type CatalogQuery,
    type FilterOptions,
} from '@/lib/catalog';

/**
 * 检索页通用外壳：筛选栏 + 结果网格。
 * Vue 基准：pages/search/components/search.vue（Filter.vue + CatGoodsList.vue 的组合层）。
 *
 * 行为对齐 Vue：
 * - 关键字/分类/品牌/价格/属性任一变化 → 回到第 1 页重新取数；
 * - 筛选项随「当前条件」重新拉（后端返回的是该条件下的同层分类与可选品牌）；
 * - 属性建议走 `getAttributeSuggestions`，只作为 `attrs` 行展示。
 * 不写 URL query（Vue 会同步路由）：当前站内其它页仍是客户端状态，
 * 若要支持「分享带筛选的链接」，再在父级接管 query 即可。
 */
interface Props {
    keyword?: string;
    initialCat?: string;
    /** 店铺内分类（Vue pages/shopCategory 用这个字段） */
    shopCategoryId?: number;
    shopId?: number;
    searchToken?: string;
    emptyText?: string;
    pageSize?: number;
    /** 是否展示筛选栏（店铺首页/图搜页不需要） */
    showFilter?: boolean;
}

const EMPTY_OPTIONS: FilterOptions = { categories: [], brands: [], attrs: [], maxPrice: 0 };

export default function CatalogResults({
    keyword,
    initialCat,
    shopCategoryId,
    shopId,
    searchToken,
    emptyText,
    pageSize = 20,
    showFilter = true,
}: Props) {
    const { t } = useTranslation();
    const [query, setQuery] = useState<CatalogQuery>({
        keyword: keyword || undefined,
        cat: initialCat || undefined,
        shopCategoryId,
        shopId,
        searchToken,
    });
    const [options, setOptions] = useState<FilterOptions>(EMPTY_OPTIONS);
    const [total, setTotal] = useState(0);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        setQuery((prev) => ({
            ...prev,
            keyword: keyword || undefined,
            cat: initialCat || undefined,
            shopCategoryId,
            shopId,
            searchToken,
        }));
    }, [keyword, initialCat, shopCategoryId, shopId, searchToken]);

    const filterKey = useMemo(
        () => JSON.stringify({ keyword: query.keyword, cat: query.cat, brand: query.brand, min: query.min, max: query.max, shopId }),
        [query.keyword, query.cat, query.brand, query.min, query.max, shopId],
    );

    useEffect(() => {
        if (!showFilter) return;
        let cancelled = false;
        (async () => {
            try {
                const [base, attrs] = await Promise.all([
                    fetchFilterOptions(query),
                    fetchAttrSuggestions(query).catch(() => [] as FilterOptions['attrs']),
                ]);
                if (cancelled) return;
                setOptions({ ...base, attrs });
            } catch {
                if (!cancelled) setOptions(EMPTY_OPTIONS);
            }
        })();
        return () => {
            cancelled = true;
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [filterKey, showFilter]);

    const change = useCallback((patch: Partial<CatalogQuery>) => {
        setQuery((prev) => ({ ...prev, ...patch }));
    }, []);

    const reset = useCallback(() => {
        setQuery({ keyword: keyword || undefined, shopId, searchToken, page: 1 });
    }, [keyword, shopId, searchToken]);

    return (
        <div className="pc-catalog">
            {showFilter ? (
                <FilterBar
                    options={options}
                    query={query}
                    total={total}
                    loading={loading}
                    onChange={change}
                    onReset={reset}
                />
            ) : null}
            <ProductResults
                query={query}
                pageSize={pageSize}
                emptyText={emptyText ?? t('暂无商品')}
                onTotal={setTotal}
                onLoading={setLoading}
            />
        </div>
    );
}
