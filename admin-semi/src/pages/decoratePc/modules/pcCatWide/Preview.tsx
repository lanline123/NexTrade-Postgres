/**
 * PC 分类模块-宽（pcCatWide）- 预览组件
 *
 * 渲染规格对齐前台 pc-semi ContractCategoryHot（pcCategoryNav / pcCategoryNav3 /
 * pcCatWide 共用，pc-semi/src/decorate/components/modules.tsx:5499）+
 * 契约层 adaptPcCatWide（contract.ts:498）：
 *   取热门分类 → 按 catIds 过滤（catIds 为空则全部）→ 6 列网格：
 *   48×48 分类图（object-fit contain、圆角 8）+ 14px 分类名。
 *   pcCatWide 的契约不下发 title / moreLink，故无标题行、无「更多」。
 *
 * 取数：前台走 /api/category/category/hot；编辑器与 pc-category-nav 预览同口径，
 *   用 /product/category/list?page=1&size=100&parentId=-1 后前端筛 isHot=1 && isShow=1
 *   （后端树接口不含 isHot），再按 catIds 过滤，保持热门列表原始顺序。
 *
 * 与前台差异：
 * 1. 前台加载中/无命中时整块不渲染（return null）；编辑器显示加载/空态占位文案，
 *    否则模块在画布上不可见、不可选中；
 * 2. 链接一律 javascript:; 不跳转；缺图走内联 SVG 占位。
 */
import { useEffect, useMemo, useState } from 'react';
import { apiGet } from '../../../../api';
import { useI18n } from '../../../../i18n';
import './pcCatWide.css';

type AnyRecord = Record<string, any>;

const EMPTY_IMG =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='100' height='100'%3E%3Crect fill='%23f2f4f6' width='100' height='100'/%3E%3Cpath d='M28 62l13-16 10 11 7-7 14 12H28z' fill='%23c0c8d4'/%3E%3Ccircle cx='40' cy='38' r='6' fill='%23c0c8d4'/%3E%3C/svg%3E";

/** 与 def defaults 逐字段一致（modules/pcCatWide/index.tsx）。 */
const DEFAULTS: AnyRecord = { catIds: [] };

function isPlainObject(value: unknown): value is AnyRecord {
  return Object.prototype.toString.call(value) === '[object Object]';
}

function deepMerge<T extends AnyRecord>(defaults: T, source?: AnyRecord | null): T {
  const out: AnyRecord = Array.isArray(defaults) ? [...defaults] : { ...defaults };
  if (!source) return out as T;
  Object.keys(source).forEach((key) => {
    const base = out[key];
    const next = source[key];
    if (isPlainObject(base) && isPlainObject(next)) {
      out[key] = deepMerge(base as AnyRecord, next as AnyRecord);
    } else if (next !== undefined) {
      out[key] = next;
    }
  });
  return out as T;
}

function catPic(category?: AnyRecord | null): string {
  if (!category) return EMPTY_IMG;
  return category.categoryPic || category.picUrl || category.picThumb || category.pic || EMPTY_IMG;
}

export default function PcCatWidePreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const config = useMemo(() => deepMerge(DEFAULTS, data), [JSON.stringify(data)]);
  const catIds: number[] = Array.isArray(config.catIds)
    ? config.catIds.map((v: any) => Number(v)).filter((v: number) => Number.isFinite(v))
    : [];

  const [categories, setCategories] = useState<AnyRecord[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let alive = true;
    setLoading(true);
    apiGet<{ records?: AnyRecord[]; list?: AnyRecord[] }>('/product/category/list', {
      page: 1,
      size: 100,
      parentId: -1,
    })
      .then((res) => {
        if (!alive) return;
        const all = res?.records || res?.list || [];
        setCategories(all.filter((c) => Number(c.isHot) === 1 && Number(c.isShow) === 1));
      })
      .catch(() => {
        if (alive) setCategories([]);
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, []);

  const filtered = catIds.length
    ? categories.filter((c) => catIds.includes(Number(c.categoryId)))
    : categories;

  if (loading) {
    return (
      <div className="dec-pc-cat-wide-cat">
        <div className="dec-pc-cat-wide-cat__empty">{t('加载中...')}</div>
      </div>
    );
  }

  if (!filtered.length) {
    return (
      <div className="dec-pc-cat-wide-cat">
        <div className="dec-pc-cat-wide-cat__empty">
          {catIds.length ? t('所选分类不在热门分类中（请在分类管理中勾选"热门"）') : t('暂无热门分类（请在分类管理中勾选"热门"）')}
        </div>
      </div>
    );
  }

  return (
    <div className="dec-pc-cat-wide-cat">
      <div className="dec-pc-cat-wide-cat__grid">
        {filtered.map((category, i) => (
          <a
            className="dec-pc-cat-wide-cat__item"
            key={String(category.categoryId ?? i)}
            href="javascript:;"
          >
            <img
              className="dec-pc-cat-wide-cat__img"
              src={catPic(category)}
              alt={category.categoryName || ''}
            />
            <span className="dec-pc-cat-wide-cat__name">{category.categoryName || t('分类')}</span>
          </a>
        ))}
      </div>
    </div>
  );
}
