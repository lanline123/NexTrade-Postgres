/**
 * PC 旧版轮播 Banner（pcBanner）- 预览组件
 *
 * 渲染规格（权威）：前台 pc-semi 对 type=pcBanner 的实际渲染——
 * contract.ts `adaptPcBanner`（:143）按 bannerStyle 分流：
 *
 * - bannerStyle = 2（简约）→ GroupBanner 计划 { banners: picList, frame }，
 *   由 modules.tsx `GroupBanner`（:3341）渲染：全量 picList、布局恒 3 列、
 *   间距 30、逐格步进轮播（5000ms / 箭头 / 无指示点）、高度 frame.slideHeight
 *   缺省 195、图片圆角 picRadioStyle 优先其次 frame.boxRadius。本文件内以
 *   同口径的 GroupCarousel 子组件实现（与 pcGroupBanner 预览同构，差别仅在
 *   这里不按 number 截取——适配器对 pcBanner 不传 number 也不截取）。
 *
 * - bannerStyle = 1（默认，缺省）→ ContractBanner 计划
 *   { main: picList, recommend: picList2, icons: picList3, frame }，
 *   由 modules.tsx `ContractBanner`（:5007）渲染，是原版 Banner.vue 的
 *   静态首屏复刻（三块在原版是 swiper，pc-semi 静态页只渲染第一屏）：
 *   左分类列只留同宽空框（分类菜单由 C 端头部承载）；中部主图只出
 *   picList[0]；其右为 picList2 前 3 张（每屏一组）；右列是 picList3 图标行
 *   （32×32 图 + picTitle 文案，即原 CommonSmallPersonalCenter 的 .vip-list）。
 *   三组全空时前台整模块不渲染，编辑器内以空态提示替代。
 *
 * - showCat / logo 前台不消费（注释明示「C 端头部已承载，降级省略」），
 *   本预览同样不渲染。
 *
 * 复用裁决：pcBanner 与 pcBanner2 的 def 字段几乎相同，但前台渲染器完全
 * 不同（pcBanner2 → Banner2 轮播；pcBanner → ContractBanner 静态首屏 /
 * GroupBanner 分流），且新版 pc-banner2 预览对应的是 Banner2 的编辑器形态，
 * 与本模块无一分支同构，故不复用、独立实现（详见同线 MANIFEST）。
 */
import { useEffect, useMemo, useState, type CSSProperties, type ReactNode } from 'react';
import { useI18n } from '../../../../i18n';
import './pcBanner.css';

type AnyRecord = Record<string, any>;

/** 空图片占位（与新版预览同款内联 SVG）。 */
const IMAGE_PLACEHOLDER =
  'data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect fill="%23f0f0f0" width="100" height="100"/></svg>';

/** 本模块 def 的 defaults（逐字段一致）。 */
const pcBannerLegacyDefaults: AnyRecord = {
  bannerStyle: 1,
  showCat: 1,
  logo: { picUrl: '', picThumb: '' },
  picList: [],
  picList2: [],
  picList3: [],
};

/** 与新版预览同款 deepMerge：嵌套普通对象递归合并，数组整体覆盖，`undefined` 保留默认值。 */
function deepMerge<T extends AnyRecord>(defaults: T, source?: AnyRecord): T {
  const result: AnyRecord = { ...defaults };
  if (!source || typeof source !== 'object') return result as T;
  for (const key of Object.keys(source)) {
    const sourceValue = source[key];
    const defaultValue = (defaults as AnyRecord)[key];
    if (
      sourceValue !== null &&
      typeof sourceValue === 'object' &&
      !Array.isArray(sourceValue) &&
      defaultValue !== null &&
      typeof defaultValue === 'object' &&
      !Array.isArray(defaultValue)
    ) {
      result[key] = deepMerge(defaultValue as AnyRecord, sourceValue as AnyRecord);
    } else if (sourceValue !== undefined) {
      result[key] = sourceValue;
    }
  }
  return result as T;
}

/** 与前台 picSrc 同序兜底：picUrl → picThumb → 占位图。 */
function formatImage(pic?: AnyRecord | null): string {
  const url = pic?.picUrl || pic?.picThumb;
  return url ? String(url) : IMAGE_PLACEHOLDER;
}

/** 前台 px()：有限数原样，否则 0。 */
function px(value: unknown): number {
  const n = Number(value);
  return Number.isFinite(n) ? n : 0;
}

/* ================= bannerStyle=2：GroupBanner 同口径轮播 ================= */

const GROUP_PER_VIEW = 3;
const GROUP_GAP = 30;

function GroupCarousel({ pics, frame, rawData }: { pics: AnyRecord[]; frame: AnyRecord; rawData: AnyRecord }) {
  const slideHeight = px(frame.slideHeight) > 0 ? px(frame.slideHeight) : 195;

  const cssVars = useMemo(() => {
    const rawStyle = Number((rawData || {}).picRadioStyle);
    const itemRadius = rawStyle === 2 ? 8 : rawStyle === 1 ? 0 : px(frame.boxRadius);
    return {
      '--dec-box-padding': `${px(frame.boxPadding)}px`,
      '--dec-box-padding-top': `${px(frame.boxPaddingTop)}px`,
      '--dec-box-padding-bottom': `${px(frame.boxPaddingBottom)}px`,
      '--dec-box-radius': `${px(frame.boxRadius)}px`,
      '--dec-item-radius': `${itemRadius}px`,
      '--dec-bg-color': frame.backgroundColor || 'transparent',
      '--dec-slide-height': `${slideHeight}px`,
    } as CSSProperties;
  }, [frame, rawData, slideHeight]);

  const count = pics.length;
  const pages = Math.max(1, count - GROUP_PER_VIEW + 1);

  const [index, setIndex] = useState(0);
  const [hovered, setHovered] = useState(false);

  useEffect(() => {
    setIndex(0);
  }, [count]);

  useEffect(() => {
    if (hovered || pages <= 1) return undefined;
    const timer = window.setInterval(() => {
      setIndex((current) => (current + 1) % pages);
    }, 5000);
    return () => window.clearInterval(timer);
  }, [hovered, pages]);

  const goPrev = () => setIndex((current) => (current - 1 + pages) % pages);
  const goNext = () => setIndex((current) => (current + 1) % pages);

  const trackStyle: CSSProperties = {
    width: `${(count / GROUP_PER_VIEW) * 100}%`,
    transform: `translateX(-${(Math.min(index, pages - 1) * 100) / Math.max(count, 1)}%)`,
  };

  return (
    <div className="dec-pc-banner-legacy__group" style={cssVars}>
      <div className="dec-pc-banner-legacy__group-inner">
        <div
          className="dec-pc-banner-legacy__group-carousel"
          onMouseEnter={() => setHovered(true)}
          onMouseLeave={() => setHovered(false)}
        >
          <div className="dec-pc-banner-legacy__group-track" style={trackStyle}>
            {pics.map((pic, i) => (
              <div
                className="dec-pc-banner-legacy__group-slide"
                key={pic?.picId ?? i}
                style={{ width: `${100 / Math.max(count, 1)}%`, paddingRight: `${GROUP_GAP}px` }}
              >
                <a
                  className="dec-pc-banner-legacy__group-link"
                  href={pic?.picLink?.link || 'javascript:;'}
                  target="_blank"
                  rel="noopener noreferrer"
                  title={pic?.picTitle || ''}
                >
                  <img className="dec-pc-banner-legacy__group-img" src={formatImage(pic)} alt={pic?.picTitle || ''} />
                </a>
              </div>
            ))}
          </div>

          {pages > 1 ? (
            <>
              <div className="swiper-button-prev" onClick={goPrev} />
              <div className="swiper-button-next" onClick={goNext} />
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
}

/* ================= bannerStyle=1：ContractBanner 静态首屏 ================= */

export default function PcBannerLegacyPreview({ data }: { data: Record<string, any> }): ReactNode {
  const { t } = useI18n();

  const moduleConfig = useMemo(() => deepMerge(pcBannerLegacyDefaults, data), [data]);
  const main = useMemo(
    () => (Array.isArray(moduleConfig.picList) ? moduleConfig.picList : []) as AnyRecord[],
    [moduleConfig.picList],
  );
  const recommend = useMemo(
    () => (Array.isArray(moduleConfig.picList2) ? moduleConfig.picList2 : []) as AnyRecord[],
    [moduleConfig.picList2],
  );
  const icons = useMemo(
    () => (Array.isArray(moduleConfig.picList3) ? moduleConfig.picList3 : []) as AnyRecord[],
    [moduleConfig.picList3],
  );
  const frame = (moduleConfig.frame || {}) as AnyRecord;

  if (!main.length && !recommend.length && !icons.length) {
    return (
      <div className="dec-pc-banner-legacy">
        <div className="dec-pc-banner-legacy__empty">
          <div className="dec-pc-banner-legacy__empty-text">{t('点击添加轮播Banner')}</div>
        </div>
      </div>
    );
  }

  // 简约风格：GroupBanner 平铺轮播 picList（无图时前台不渲染，这里回退空态）
  if (Number(moduleConfig.bannerStyle ?? 1) === 2) {
    if (!main.length) {
      return (
        <div className="dec-pc-banner-legacy">
          <div className="dec-pc-banner-legacy__empty">
            <div className="dec-pc-banner-legacy__empty-text">{t('点击添加轮播Banner')}</div>
          </div>
        </div>
      );
    }
    return (
      <div className="dec-pc-banner-legacy">
        <GroupCarousel pics={main} frame={frame} rawData={data || {}} />
      </div>
    );
  }

  const hero = main[0];
  const firstSlide = recommend.slice(0, 3);

  return (
    <div className="dec-pc-banner-legacy">
      <div className="dec-pc-banner-legacy__row">
        {/* 左分类列：前台只留同宽空框（分类菜单由 C 端头部承载） */}
        <div className="dec-pc-banner-legacy__left" />
        <div className="dec-pc-banner-legacy__middle">
          {hero ? (
            <div className="dec-pc-banner-legacy__hero">
              <a
                className="dec-pc-banner-legacy__hero-link"
                href={hero?.picLink?.link || 'javascript:;'}
                target="_blank"
                rel="noopener noreferrer"
                title={hero?.picTitle || ''}
              >
                <img className="dec-pc-banner-legacy__hero-img" src={formatImage(hero)} alt={hero?.picTitle || ''} />
              </a>
            </div>
          ) : null}
          {firstSlide.length ? (
            <div className="dec-pc-banner-legacy__recommend">
              {firstSlide.map((pic, i) => (
                <a
                  className="dec-pc-banner-legacy__recommend-item"
                  key={pic?.picId ?? i}
                  href={pic?.picLink?.link || 'javascript:;'}
                  target="_blank"
                  rel="noopener noreferrer"
                  title={pic?.picTitle || ''}
                >
                  <img className="dec-pc-banner-legacy__recommend-img" src={formatImage(pic)} alt={pic?.picTitle || ''} />
                </a>
              ))}
            </div>
          ) : null}
        </div>
        <div className="dec-pc-banner-legacy__right">
          {icons.length ? (
            <div className="dec-pc-banner-legacy__vip-list">
              {icons.map((pic, i) => (
                <div className="dec-pc-banner-legacy__lysy" key={pic?.picId ?? i}>
                  <a
                    href={pic?.picLink?.link || 'javascript:;'}
                    target="_blank"
                    rel="noopener noreferrer"
                    title={pic?.picTitle || ''}
                  >
                    <div>
                      <img
                        className="dec-pc-banner-legacy__lysy-img"
                        style={{ width: 32, height: 32 }}
                        src={formatImage(pic)}
                        alt={pic?.picTitle || ''}
                      />
                    </div>
                    <p className="dec-pc-banner-legacy__lysy-text">{pic?.picTitle || ''}</p>
                  </a>
                </div>
              ))}
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
