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
    type SortField,
    type SortOrder,
} from '@/lib/catalog';

/**
 * 检索页通用外壳：筛选栏 + 结果网格。
 * Vue 基准：pages/search/components/search.vue（Filter.vue + CatGoodsList.vue 的组合层）。
 *
 * 行为对齐 Vue：
 * - 关键字/分类/品牌/价格/属性任一变化 → 回到第 1 页重新取数；
 * - 筛选项随「当前条件」重新拉（后端返回的是该条件下的同层分类与可选品牌）；
 * - 属性建议走 `getAttributeSuggestions`，只作为 `attrs` 行展示；
 * - **筛选状态同步到 URL**（cat / brand / sort / order / min / max），分享或刷新可还原。
 *
 * 关于 URL 同步的取舍：用 `history.replaceState` 而不是 `router.replace`。
 * 页面是 `force-dynamic`，`router.replace` 会触发一次 RSC 往返，筛选变慢且会闪；
 * replaceState 只改地址栏、不触发服务端渲染。代价是浏览器前进/后退不会重新筛选
 * （刷新与分享链接仍然可用），这也是筛选栏最常用的两种场景。
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

const SORTS: SortField[] = ['sale', 'price', 'time'];
const ORDERS: SortOrder[] = ['asc', 'desc'];

/** 从地址栏读出筛选条件（不含 keyword：keyword 由页面 props 传入，搜索框另有来源）。 */
function readQueryFromUrl(): Partial<CatalogQuery> {
    if (typeof window === 'undefined') return {};
    const params = new URLSearchParams(window.location.search);
    const cat = params.get('cat') ?? undefined;
    const brand = params.get('brand') ?? undefined;
    const sortRaw = params.get('sort');
    const orderRaw = params.get('order');
    const min = Number(params.get('min')) || 0;
    const max = Number(params.get('max')) || 0;
    return {
        cat: cat || undefined,
        brand: brand || undefined,
        sort: SORTS.includes(sortRaw as SortField) ? (sortRaw as SortField) : '',
        order: ORDERS.includes(orderRaw as SortOrder) ? (orderRaw as SortOrder) : '',
        min: min > 0 ? min : undefined,
        max: max > 0 ? max : undefined,
    };
}

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
    const [query, setQuery] = useState<CatalogQuery>(() => {
        const fromUrl = readQueryFromUrl();
        return {
            keyword: keyword || undefined,
            cat: fromUrl.cat ?? (initialCat || undefined),
            brand: fromUrl.brand,
            min: fromUrl.min,
            max: fromUrl.max,
            sort: fromUrl.sort,
            order: fromUrl.order,
            shopCategoryId,
            shopId,
            searchToken,
        };
    });
    const [options, setOptions] = useState<FilterOptions>(EMPTY_OPTIONS);
    const [total, setTotal] = useState(0);
    const [loading, setLoading] = useState(true);

    // 页面 props 变化（服务端重渲染、切换到别的分类页）时，只覆盖「来源在服务端」的字段，
    // 不碰 URL 带来的筛选，避免把用户刚选的品牌/价格冲掉。
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

    /** 把筛选写回地址栏（不触发服务端渲染）。 */
    const syncUrl = useCallback((next: CatalogQuery) => {
        if (typeof window === 'undefined') return;
        const params = new URLSearchParams(window.location.search);
        const put = (key: string, value?: string | number) => {
            if (value === undefined || value === null || value === '' || Number(value) === 0) params.delete(key);
            else params.set(key, String(value));
        };
        put('cat', next.cat);
        put('brand', next.brand);
        put('sort', next.sort);
        put('order', next.order);
        put('min', next.min);
        put('max', next.max);
        const qs = params.toString();
        window.history.replaceState(null, '', qs ? `${window.location.pathname}?${qs}` : window.location.pathname);
    }, []);

    const change = useCallback(
        (patch: Partial<CatalogQuery>) => {
            const next = { ...query, ...patch };
            setQuery(next);
            syncUrl(next);
        },
        [query, syncUrl],
    );

    const reset = useCallback(() => {
        const next: CatalogQuery = { keyword: keyword || undefined, shopId, searchToken, page: 1 };
        setQuery(next);
        syncUrl(next);
    }, [keyword, shopId, searchToken, syncUrl]);

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
