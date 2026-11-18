/**
 * PC 旧版「分类导航」pcCategoryNav - 预览组件
 *
 * 渲染规格（权威）= PC 前台 pc-semi 对同 type 的实际渲染：
 *   contract.ts `adaptPcCategoryNav`（:455）→ modules.tsx `ContractCategoryHot`（:5500）
 *   - 适配器输出 module = { title: toTitle(titleName ?? title), rowNum: clamp(rowNum ?? displayNum, 6, 1..10),
 *     catIds: [], frame }，外层 Container 固定 FLAT_FRAME（无边距/无背景/无圆角）。
 *   - 渲染器调 `GET /api/category/category/hot` 取热门分类，按 catIds 过滤后
 *     以 CSS grid（repeat(rowNum, 1fr)、gap 12）平铺全部命中分类：
 *     48×48 图标（objectFit contain、圆角 8、底色恒透明——适配器不透传 imageBackgroundColor）
 *     + 下方 14px 分类名；无横向滚动、无固定高度。
 *
 * 与新版 pc-category-nav 预览不可复用的裁决（依据见同线 MANIFEST）：
 *   新版数据是 { displayNum, titleName, frame{boxHeight:328}, backgroundColor, title }，
 *   前台是横向滚动轮播（CategoryNav）；旧版数据是 { rowNum, categoryList }，
 *   前台是 grid 平铺（ContractCategoryHot），字段名与版式都不同，故另写。
 *
 * 取数口径与新版 pc-category-nav 预览一致（admin 侧等价取数）：
 *   `GET /product/category/list?page=1&size=100&parentId=-1` 后前端筛选 isHot=1 && isShow=1。
 *
 * 已知口径（与前台逐条对齐，不是缺陷）：
 *   - 编辑器的 categoryList（已选分类）在前台不生效：adaptPcCategoryNav 把 catIds
 *     硬编码为 []，前台永远渲染全部热门分类。本预览与前台一致，不做过滤。
 *   - 前台 loading / 空结果时整块不渲染（return null）；编辑器内改为显示
 *     加载中 / 暂无热门分类占位（与新版预览同惯例），否则画布上模块不可见不可选。
 *   - 标题栏仅当存量数据带 titleName / title 时出现（适配器 toTitle 语义），
 *     旧版 def 本身无标题字段，新建模块无标题栏，与前台一致。
 */
import { useEffect, useState, type SyntheticEvent } from 'react';
import { apiGet } from '../../../../api';
import { useI18n } from '../../../../i18n';
import ModuleTitlePreview from '../../previews/shared/ModuleTitlePreview';
import './pcCategoryNav.css';

type AnyRecord = Record<string, any>;

interface CategoryRow {
  categoryId?: number | string;
  categoryName?: string;
  categoryPic?: string;
  picUrl?: string;
  picThumb?: string;
  pic?: string;
  isHot?: number | string;
  isShow?: number | string;
}

const EMPTY_IMG =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='180' height='180'%3E%3Crect fill='%23f0f0f0' width='180' height='180'/%3E%3C/svg%3E";

const STORAGE_URL = (import.meta.env.VITE_STORAGE_URL as string | undefined) || '';

function formatImage(url?: string): string {
  if (!url || !String(url).trim()) return EMPTY_IMG;
  const raw = String(url).trim();
  if (/^(https?:)?\/\//i.test(raw) || raw.startsWith('data:')) return raw;
  return STORAGE_URL ? `${STORAGE_URL}${raw.replace(/^\/+/, '')}` : raw;
}

function handleImgError(event: SyntheticEvent<HTMLImageElement>): void {
  const img = event.currentTarget;
  if (img.src === EMPTY_IMG) return;
  img.onerror = null;
  img.src = EMPTY_IMG;
}

export default function PcCategoryNavLegacyPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const source: AnyRecord = data || {};

  // 每行列数：适配器 clamp(rowNum ?? displayNum, 默认 6, 1..10)。
  // 编辑器 def 默认 rowNum=5，新建模块数据里恒带该值；仅存量数据缺失时才落到前台的 6。
  const rowNum = Math.min(10, Math.max(1, Number(source.rowNum ?? source.displayNum) || 6));

  // 标题：适配器 toTitle(titleName ?? title) ——字符串 → {showTitle:1, titleText}；
  // 对象 → 透传（showTitle 缺省 1，显式 0 则整条不出）；两者皆无 → 不渲染标题栏。
  // 前台 ShopTitle 无 fallback 文案，这里同样不给 fallbackText。
  const rawTitle = source.titleName ?? source.title;
  const effTitle: AnyRecord =
    rawTitle && typeof rawTitle === 'object'
      ? { ...rawTitle, showTitle: rawTitle.showTitle ?? 1, titleText: rawTitle.titleText || rawTitle.title || '' }
      : typeof rawTitle === 'string' && rawTitle
        ? { showTitle: 1, titleText: rawTitle }
        : { showTitle: 0 };
  const showTitleBar = Number(effTitle.showTitle) === 1;

  const [rows, setRows] = useState<CategoryRow[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let alive = true;
    setLoading(true);
    apiGet<{ records?: CategoryRow[]; list?: CategoryRow[] }>('/product/category/list', {
      page: 1,
      size: 100,
      parentId: -1,
    })
      .then((res) => {
        if (!alive) return;
        const all = res?.records || res?.list || [];
        setRows(all.filter((c) => Number(c.isHot) === 1 && Number(c.isShow) === 1));
      })
      .catch(() => {
        if (alive) setRows([]);
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, []);

  return (
    <div className="dec-pcCategoryNav">
      {showTitleBar ? (
        <div className="dec-pcCategoryNav__head">
          <ModuleTitlePreview title={effTitle} />
        </div>
      ) : null}

      {loading ? (
        <div className="dec-pcCategoryNav__empty">
          <div className="dec-pcCategoryNav__empty-text">{t('加载中...')}</div>
        </div>
      ) : rows.length === 0 ? (
        <div className="dec-pcCategoryNav__empty">
          <div className="dec-pcCategoryNav__empty-text">
            {t('暂无热门分类（请在分类管理中勾选"热门"）')}
          </div>
        </div>
      ) : (
        <div
          className="dec-pcCategoryNav__grid"
          style={{ gridTemplateColumns: `repeat(${rowNum}, minmax(0, 1fr))` }}
        >
          {rows.map((category, index) => (
            <a
              key={String(category.categoryId ?? index)}
              className="dec-pcCategoryNav__item"
              href="javascript:;"
            >
              <img
                className="dec-pcCategoryNav__img"
                src={formatImage(category.categoryPic || category.picUrl || category.picThumb || category.pic)}
                alt={category.categoryName || ''}
                onError={handleImgError}
              />
              <span className="dec-pcCategoryNav__name">{category.categoryName || t('分类')}</span>
            </a>
          ))}
        </div>
      )}
    </div>
  );
}
