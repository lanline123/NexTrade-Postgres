/**
 * PC 旧版「分类导航3」pcCategoryNav3 - 预览组件
 *
 * 渲染规格（权威）= PC 前台 pc-semi 对同 type 的实际渲染：
 *   contract.ts `adaptPcCategoryNav3`（:474）→ modules.tsx `CategoryNav3`（:3125）
 *   - 适配器输出 module = { title: toTitle(titleText ?? titleName ?? title), titleText,
 *     picList, showMore(布尔), moreText, moreLink, backgroundColor, imageBackgroundColor, frame }；
 *     CategoryNav3 本体：一屏 6 卡横向滚动（CSS 变量驱动容器边距/圆角/背景/高度，
 *     高度缺省 352），卡片 = 图片（picUrl||picThumb，底色 pic.imageBackgroundColor
 *     || module.imageBackgroundColor）+ picTitle + 可选 picDesc；
 *     标题走 DecModuleTitle（fallbackText「热门推荐」）。
 *   - 注意：modules.tsx:5499 注释把三者并称 ContractCategoryHot 是旧说法；
 *     pcCategoryNav3 早已改指 CategoryNav3（图片版），勿按热门分类卡片实现。
 *
 * 与新版 pc-category-nav3 预览的复用裁决（依据见同线 MANIFEST）：版式与前台组件
 * 同为 CategoryNav3，滚动/页码/箭头逻辑照新版预览复刻，但数据口径不同，不可直接套用：
 *   - 旧版 def 无 frame / title 对象 / backgroundColor 字段：frame 缺省按前台
 *     CategoryNav3 的兜底（boxRadius 0 而非新版默认的 8、boxHeight 非正数回 352、
 *     背景回 rgba(0,0,0,0.02)）。
 *   - 标题按适配器口径组装：平铺的 showMore/showMoreText/moreLink 在前台不被
 *     折进标题（适配器只透传 title 对象内的 showMore/moreLink），本预览同样不折，
 *     避免预览里出现前台没有的「更多」链接。
 *   - def 中的 bannerStyle / showCat / logo / picList2 / picList3 以及 rowNum
 *     前台均不消费（适配器丢弃、本体写死一屏 6 卡），本预览同样不渲染。
 *   - 编辑态不做自动轮播（与新版预览一致；前台有 5 秒自动翻屏）。
 *   - 链接一律 javascript:; 不跳转（编辑器统一口径）。
 */
import { type CSSProperties, useEffect, useRef, useState } from 'react';
import { useI18n } from '../../../../i18n';
import ModuleTitlePreview from '../../previews/shared/ModuleTitlePreview';
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
}

interface PcCategoryNav3Config {
  frame: AnyRecord;
  titleText: string;
  imageBackgroundColor: string;
  picList: PcCategoryNav3Item[];
  backgroundColor: string;
}

/**
 * 默认值 = 旧版 def defaults 中与渲染有关的字段（picList / imageBackgroundColor /
 * titleText），其余按前台 CategoryNav3 的缺省兜底：frame 全缺省
 * （boxRadius 0、boxHeight 352、背景 rgba(0,0,0,0.02)）、backgroundColor 空
 * （由 frame.backgroundColor / 兜底色接管）。
 */
const DEFAULTS: PcCategoryNav3Config = {
  frame: {
    boxPadding: 0,
    boxPaddingTop: 0,
    boxPaddingBottom: 0,
    boxRadius: 0,
    backgroundColor: 'rgba(0, 0, 0, 0.02)',
    boxHeight: 352,
  },
  titleText: '',
  imageBackgroundColor: '',
  picList: [],
  backgroundColor: '',
};

const EMPTY_IMG =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='180' height='180'%3E%3Crect fill='%23f0f0f0' width='180' height='180'/%3E%3C/svg%3E";

/** 深合并：frame 逐字段合并，picList 数组整体覆盖（与同库预览一致）。 */
function mergeConfig(data: AnyRecord | undefined): PcCategoryNav3Config {
  const source = data || {};
  return {
    ...DEFAULTS,
    ...source,
    frame: { ...DEFAULTS.frame, ...(source.frame || {}) },
    picList: Array.isArray(source.picList) ? source.picList : DEFAULTS.picList,
  } as PcCategoryNav3Config;
}

function formatImage(url?: string): string {
  return url && String(url).trim() ? String(url).trim() : EMPTY_IMG;
}

export default function PcCategoryNav3LegacyPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const source: AnyRecord = data || {};
  const config = mergeConfig(data);
  const frame = config.frame || {};

  // 标题：适配器 toTitle(titleText ?? titleName ?? title) ——字符串 → {showTitle:1, titleText}；
  // 对象 → 透传（showTitle 缺省 1）；皆无 → 不出标题栏。平铺 showMore 等不折入（见文件头）。
  const rawTitle = source.titleText ?? source.titleName ?? source.title;
  const effTitle: AnyRecord =
    rawTitle && typeof rawTitle === 'object'
      ? { ...rawTitle, showTitle: rawTitle.showTitle ?? 1, titleText: rawTitle.titleText || rawTitle.title || '' }
      : typeof rawTitle === 'string' && rawTitle
        ? { showTitle: 1, titleText: rawTitle }
        : { showTitle: 0 };

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

  // 前台：backgroundColor 优先于 frame.backgroundColor，再回 rgba(0,0,0,0.02)
  const bgColor = config.backgroundColor || frame.backgroundColor || 'rgba(0, 0, 0, 0.02)';
  // 前台：boxHeight 非正数回 352；boxRadius 缺省 0（px() 兜底），与新版默认 8 不同
  const boxHeight = Number(frame.boxHeight) > 0 ? Number(frame.boxHeight) : 352;
  const cssVars = {
    '--dec-box-padding': `${frame.boxPadding ?? 0}px`,
    '--dec-box-padding-top': `${frame.boxPaddingTop ?? 0}px`,
    '--dec-box-padding-bottom': `${frame.boxPaddingBottom ?? 0}px`,
    '--dec-box-radius': `${Number(frame.boxRadius) || 0}px`,
    '--dec-bg-color': bgColor,
    '--dec-box-height': `${boxHeight}px`,
  } as CSSProperties & Record<string, string>;

  // 前台：pic.imageBackgroundColor || module.imageBackgroundColor || 样式表默认底色
  const getImageBg = (item: PcCategoryNav3Item): string =>
    item.imageBackgroundColor ||
    config.imageBackgroundColor ||
    'var(--primary-light-9)';

  return (
    <div className="dec-pcCategoryNav3" style={cssVars}>
      <ModuleTitlePreview title={effTitle} fallbackText={t('热门推荐')} />
      {nav.canScroll ? (
        <div className="dec-pcCategoryNav3__pager-row">
          <span className="dec-pcCategoryNav3__pager">
            {nav.page} / {nav.pages}
          </span>
        </div>
      ) : null}

      <div className="dec-pcCategoryNav3__list-wrap">
        {picList.length === 0 ? (
          <div className="dec-pcCategoryNav3__empty">
            <div className="dec-pcCategoryNav3__empty-text">{t('请在设置中添加内容')}</div>
          </div>
        ) : (
          <>
            <div className="dec-pcCategoryNav3__list">
              <div className="dec-pcCategoryNav3__scroll" ref={scrollRef} onScroll={update}>
                {picList.map((item, index) => (
                  <div
                    className="dec-pcCategoryNav3__item"
                    key={String(item.picId || index)}
                  >
                    <a
                      className="dec-pcCategoryNav3__link"
                      href="javascript:;"
                      title={String(item.productName || item.picTitle || '')}
                    >
                      <div
                        className="dec-pcCategoryNav3__image"
                        style={{ backgroundColor: getImageBg(item) }}
                      >
                        <img
                          className="dec-pcCategoryNav3__img"
                          src={formatImage(item.picUrl || item.picThumb)}
                          alt={String(item.picTitle || '')}
                        />
                      </div>
                      <div className="dec-pcCategoryNav3__name">{item.picTitle}</div>
                      {item.picDesc ? (
                        <div className="dec-pcCategoryNav3__desc">
                          <span className="dec-pcCategoryNav3__desc-text">{item.picDesc}</span>
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
      {/* 箭头挂在模块根节点：贴模块左右外边缘 + 整块垂直居中（与新版预览一致） */}
      {nav.canScroll && picList.length > 0 ? (
        <>
          <button
            type="button"
            className="dec-pcCategoryNav3__nav dec-pcCategoryNav3__nav--prev"
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
            className="dec-pcCategoryNav3__nav dec-pcCategoryNav3__nav--next"
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
