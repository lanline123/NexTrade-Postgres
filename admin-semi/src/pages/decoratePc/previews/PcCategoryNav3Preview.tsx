/**
 * PC 分类导航3 模块 - 预览组件（React 迁移自 5.8.61 旧版
 * `view/admin/src/views/decorate/decorate/editor/modules/pcHome/pc-category-nav3/Preview.vue`）
 *
 * DOM / class 与旧版逐行对齐：
 *   .dec-pc-category-nav3                   外层（CSS 变量驱动 padding/高度/圆角/背景）
 *   .dec-pc-category-nav3__top/title/more   顶栏：24px 标题 + 右侧「更多 >」
 *   .dec-pc-category-nav3__list-wrap        列表区
 *   .dec-pc-category-nav3__empty            空态（请在设置中添加内容）
 *   .dec-pc-category-nav3__list/scroll      列表（横向滚动，一屏 6 卡）
 *   .dec-pc-category-nav3__item/link/image/img/name/desc/desc-text
 *
 * 说明：旧版图片仅原样返回 URL（不补 storageUrl 前缀），本组件保持一致。
 * 超过一屏（6 张）时显示左右翻屏箭头 + 页码，让运营一眼看出后面还有卡片；
 * 编辑态**不做**自动轮播（会干扰编辑），自动轮播只在前台 C 端。
 */
import { type CSSProperties, useEffect, useRef, useState } from 'react';
import { useI18n } from '../../../i18n';
import './pcCategoryNav3.css';

type AnyRecord = Record<string, any>;

interface PcCategoryNav3Item {
  picId?: string | number;
  picUrl?: string;
  picThumb?: string;
  picTitle?: string;
  picDesc?: string;
  picLink?: string | { link?: string };
  productName?: string;
  imageBackgroundColor?: string;
  gradientColorA?: string;
}

interface PcCategoryNav3Config {
  frame: AnyRecord;
  titleText: string;
  showMore: number;
  showMoreText: string;
  moreLink: { link?: string; name?: string };
  imageBackgroundColor: string;
  picList: PcCategoryNav3Item[];
  backgroundColor: string;
}

/** 旧版 `pcCategoryNav3Defaults`（逐字段对齐 index.ts）。 */
const DEFAULTS: PcCategoryNav3Config = {
  frame: {
    boxPadding: 0,
    boxPaddingTop: 0,
    boxPaddingBottom: 0,
    boxRadius: 8,
    backgroundColor: 'rgba(0, 0, 0, 0.02)',
    boxHeight: 352,
  },
  titleText: '热门推荐',
  showMore: 1,
  showMoreText: '更多',
  moreLink: { link: '', name: '' },
  imageBackgroundColor: '',
  picList: [],
  backgroundColor: 'rgba(0, 0, 0, 0.02)',
};

const EMPTY_IMG =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='180' height='180'%3E%3Crect fill='%23f0f0f0' width='180' height='180'/%3E%3C/svg%3E";

function mergeConfig(data: AnyRecord | undefined): PcCategoryNav3Config {
  const source = data || {};
  return {
    ...DEFAULTS,
    ...source,
    frame: { ...DEFAULTS.frame, ...(source.frame || {}) },
    // 深合并：picList 为数组（整体覆盖，与旧版 deepMerge 行为一致）
    picList: Array.isArray(source.picList) ? source.picList : DEFAULTS.picList,
    moreLink: { ...DEFAULTS.moreLink, ...(source.moreLink || {}) },
  } as PcCategoryNav3Config;
}

function toHref(link?: string | { link?: string }): string {
  if (!link) return 'javascript:;';
  if (typeof link === 'string') return link || 'javascript:;';
  return link.link || 'javascript:;';
}

function formatImage(url?: string): string {
  return url && String(url).trim() ? String(url).trim() : EMPTY_IMG;
}

export default function PcCategoryNav3Preview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const config = mergeConfig(data);
  const frame = config.frame || {};
  const picList = Array.isArray(config.picList) ? config.picList : [];
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const [nav, setNav] = useState({ canScroll: false, page: 1, pages: 1 });

  const update = () => {
    const el = scrollRef.current;
    if (!el) return;
    const width = Math.max(1, el.clientWidth);
    const max = el.scrollWidth - el.clientWidth;
    const pages = Math.max(1, Math.ceil(el.scrollWidth / width));
    // 按可滚动距离归一化：两屏时 max 略小于一屏宽，直接用 scrollLeft/width 会永远算成第 1 页
    const page = max > 1 ? Math.min(pages, Math.round((el.scrollLeft / max) * (pages - 1)) + 1) : 1;
    const canScroll = max > 1;
    setNav((prev) => (prev.canScroll === canScroll && prev.page === page && prev.pages === pages
      ? prev
      : { canScroll, page, pages }));
  };

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    update();
    const observer = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(update) : null;
    observer?.observe(el);
    window.addEventListener('resize', update);
    return () => {
      observer?.disconnect();
      window.removeEventListener('resize', update);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [picList.length]);

  const step = (direction: 1 | -1) => {
    const el = scrollRef.current;
    if (!el) return;
    el.scrollBy({ left: direction * el.clientWidth, behavior: 'smooth' });
  };

  // backgroundColor 优先于 frame.backgroundColor（同旧版）
  const bgColor = config.backgroundColor || frame.backgroundColor || 'rgba(0, 0, 0, 0.02)';
  const cssVars = {
    '--dec-box-padding': `${frame.boxPadding ?? 0}px`,
    '--dec-box-padding-top': `${frame.boxPaddingTop ?? 0}px`,
    '--dec-box-padding-bottom': `${frame.boxPaddingBottom ?? 0}px`,
    '--dec-box-radius': `${frame.boxRadius ?? 8}px`,
    '--dec-bg-color': bgColor,
    '--dec-box-height': `${frame.boxHeight ?? 352}px`,
  } as CSSProperties & Record<string, string>;

  const getImageBg = (item: PcCategoryNav3Item): string =>
    item.imageBackgroundColor ||
    config.imageBackgroundColor ||
    item.gradientColorA ||
    'var(--primary-light-9)';

  return (
    <div className="dec-pc-category-nav3" style={cssVars}>
      <div className="dec-pc-category-nav3__top">
        <div className="dec-pc-category-nav3__title">
          {config.titleText || t('热门推荐')}
        </div>
        <div className="dec-pc-category-nav3__top-right">
          {nav.canScroll ? (
            <span className="dec-pc-category-nav3__pager">
              {nav.page} / {nav.pages}
            </span>
          ) : null}
          {Number(config.showMore) === 1 && (
            <a
              className="dec-pc-category-nav3__more"
              href={toHref(config.moreLink)}
              target="_blank"
              rel="noopener noreferrer"
            >
              <span className="dec-pc-category-nav3__more-text">
                {config.showMoreText || t('更多')}
              </span>
              <span className="dec-pc-category-nav3__more-arrow">&gt;</span>
            </a>
          )}
        </div>
      </div>

      <div className="dec-pc-category-nav3__list-wrap">
        {picList.length === 0 ? (
          <div className="dec-pc-category-nav3__empty">
            <div className="dec-pc-category-nav3__empty-text">{t('请在设置中添加内容')}</div>
          </div>
        ) : (
          <>
            <div className="dec-pc-category-nav3__list">
              <div className="dec-pc-category-nav3__scroll" ref={scrollRef} onScroll={update}>
                {picList.map((item, index) => (
                  <div
                    className="dec-pc-category-nav3__item"
                    key={String(item.picId || index)}
                  >
                    <a
                      className="dec-pc-category-nav3__link"
                      href={toHref(item.picLink)}
                      title={String(item.productName || item.picTitle || '')}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <div
                        className="dec-pc-category-nav3__image"
                        style={{ backgroundColor: getImageBg(item) }}
                      >
                        <img
                          className="dec-pc-category-nav3__img"
                          src={formatImage(item.picUrl || item.picThumb)}
                          alt={String(item.picTitle || '')}
                        />
                      </div>
                      <div className="dec-pc-category-nav3__name">{item.picTitle}</div>
                      {item.picDesc ? (
                        <div className="dec-pc-category-nav3__desc">
                          <span className="dec-pc-category-nav3__desc-text">{item.picDesc}</span>
                        </div>
                      ) : null}
                    </a>
                  </div>
                ))}
              </div>
            </div>
          </>
        )}
      </div>
      {/* 箭头挂在模块根节点：贴模块左右外边缘 + 整块垂直居中（此前在 __list-wrap 内，
          受 16px 内边距和标题栏影响，左右各偏内 16px、垂直偏下约 36px） */}
      {nav.canScroll && picList.length > 0 ? (
        <>
          <button
            type="button"
            className="dec-pc-category-nav3__nav dec-pc-category-nav3__nav--prev"
            aria-label={t('上一屏')}
            onClick={(event) => {
              event.preventDefault();
              event.stopPropagation();
              step(-1);
            }}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M15 5l-7 7 7 7" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </button>
          <button
            type="button"
            className="dec-pc-category-nav3__nav dec-pc-category-nav3__nav--next"
            aria-label={t('下一屏')}
            onClick={(event) => {
              event.preventDefault();
              event.stopPropagation();
              step(1);
            }}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M9 5l7 7-7 7" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </button>
        </>
      ) : null}
    </div>
  );
}
