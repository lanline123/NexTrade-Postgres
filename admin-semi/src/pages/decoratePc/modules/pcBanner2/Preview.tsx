/**
 * PC 旧版轮播 Banner2（pcBanner2）- 预览组件
 *
 * 渲染规格（权威）：前台 pc-semi 对 type=pcBanner2 的实际渲染——
 * contract.ts `adaptPcBanner2`（:165）把数据适配为 Banner2 计划
 * { picList, number: slideNum ?? number, slideGap, frame }，
 * 再由 modules.tsx `Banner2`（:3046）渲染：
 * - 只消费 picList（一屏 perView 张、逐格步进轮播）；bannerStyle / showCat /
 *   logo / picList2 / picList3 前台一概不消费，本预览同样忽略；
 * - perView = clamp(Number(number) || 1, 1, 3)；
 * - 间距 slideGap：有限数取 max(0, slideGap)，缺省 12；perView=1 时不生效（0）；
 * - 页数 = picList.length - perView + 1（逐格步进，不是整页分组）；
 * - 5000ms 自动播放、悬停暂停、箭头 + 可点击圆点；
 * - frame 走 px() 语义：缺值按 0、背景缺省 transparent；frame.boxHeight > 0 时
 *   图片固定高度 cover，否则高度跟随图片比例；
 * - picList 为空时前台整模块不渲染，编辑器内以空态提示替代。
 *
 * 与新版 pc-banner2 预览的差异（不可直接复用，依据见同线 MANIFEST）：
 * 新版预览是整页分组轮播（pages = ceil(count/perView)），且其默认 frame
 * （boxPadding 10 / boxRadius 8 / 背景 #eeeeee）来自旧编辑器默认值，
 * 与前台 Banner2 的 px() 口径不同；本组件按前台口径独立实现，
 * 仅沿用同款等价轮播写法（track 百分比位移 + .swiper-button-* 类名）。
 */
import { useEffect, useMemo, useState, type CSSProperties, type ReactNode } from 'react';
import { useI18n } from '../../../../i18n';
import './pcBanner2.css';

type AnyRecord = Record<string, any>;

/** 空图片占位（与新版预览同款内联 SVG）。 */
const IMAGE_PLACEHOLDER =
  'data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect fill="%23f0f0f0" width="100" height="100"/></svg>';

/** 本模块 def 的 defaults（逐字段一致）。 */
const pcBanner2LegacyDefaults: AnyRecord = {
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

export default function PcBanner2LegacyPreview({ data }: { data: Record<string, any> }): ReactNode {
  const { t } = useI18n();

  const moduleConfig = useMemo(() => deepMerge(pcBanner2LegacyDefaults, data), [data]);
  const picList = useMemo(
    () => (Array.isArray(moduleConfig.picList) ? moduleConfig.picList : []) as AnyRecord[],
    [moduleConfig.picList],
  );

  // 前台 adaptPcBanner2：number = slideNum ?? number；Banner2：perView = clamp(number || 1, 1, 3)
  const perView = useMemo(() => {
    const raw = Number(moduleConfig.slideNum ?? moduleConfig.number);
    if (!Number.isFinite(raw) || raw < 1) return 1;
    return Math.min(3, Math.floor(raw));
  }, [moduleConfig.slideNum, moduleConfig.number]);

  const gap = useMemo(() => {
    if (perView <= 1) return 0;
    const raw = Number(moduleConfig.slideGap);
    return Number.isFinite(raw) ? Math.max(0, raw) : 12;
  }, [moduleConfig.slideGap, perView]);

  const frame = (moduleConfig.frame || {}) as AnyRecord;
  const boxHeight = Number(frame.boxHeight) > 0 ? Number(frame.boxHeight) : 0;

  const cssVars = useMemo(
    () =>
      ({
        '--dec-box-padding': `${px(frame.boxPadding)}px`,
        '--dec-box-padding-top': `${px(frame.boxPaddingTop)}px`,
        '--dec-box-padding-bottom': `${px(frame.boxPaddingBottom)}px`,
        '--dec-box-radius': `${px(frame.boxRadius)}px`,
        '--dec-bg-color': frame.backgroundColor || 'transparent',
      }) as CSSProperties,
    [frame],
  );

  const count = picList.length;
  const pages = Math.max(1, count - perView + 1);

  const [index, setIndex] = useState(0);
  const [hovered, setHovered] = useState(false);

  useEffect(() => {
    setIndex(0);
  }, [count, perView]);

  // autoplay：5000ms（前台另有图片就绪门控，编辑器等价实现不引入）
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
    width: `${(count / perView) * 100}%`,
    transform: `translateX(-${(Math.min(index, pages - 1) * 100) / Math.max(count, 1)}%)`,
  };

  return (
    <div className={`dec-pc-banner2-legacy dec-pc-banner2-legacy--num-${perView}`} style={cssVars}>
      {!count ? (
        <div className="dec-pc-banner2-legacy__empty">
          <div className="dec-pc-banner2-legacy__empty-text">{t('点击添加轮播Banner2')}</div>
        </div>
      ) : (
        <div className="dec-pc-banner2-legacy__inner">
          <div
            className="dec-pc-banner2-legacy__carousel"
            onMouseEnter={() => setHovered(true)}
            onMouseLeave={() => setHovered(false)}
          >
            <div className="dec-pc-banner2-legacy__track" style={trackStyle}>
              {picList.map((pic, i) => (
                <div
                  className="dec-pc-banner2-legacy__slide"
                  key={pic?.picId ?? i}
                  style={{
                    width: `${100 / Math.max(count, 1)}%`,
                    paddingRight: perView > 1 && gap > 0 ? `${gap}px` : undefined,
                  }}
                >
                  <a
                    className="dec-pc-banner2-legacy__link"
                    href={pic?.picLink?.link || 'javascript:;'}
                    target="_blank"
                    rel="noopener noreferrer"
                    title={pic?.picTitle || ''}
                  >
                    <img
                      className="dec-pc-banner2-legacy__img"
                      src={formatImage(pic)}
                      alt={pic?.picTitle || pic?.picName || ''}
                      style={boxHeight ? { height: `${boxHeight}px`, objectFit: 'cover' } : undefined}
                    />
                  </a>
                </div>
              ))}
            </div>

            {pages > 1 ? (
              <>
                <div className="swiper-button-prev" onClick={goPrev} />
                <div className="swiper-button-next" onClick={goNext} />
                <div className="dec-pc-banner2-legacy__dots">
                  {Array.from({ length: pages }, (_, dot) => (
                    <span
                      key={dot}
                      className={`dec-pc-banner2-legacy__dot${dot === index ? ' is-active' : ''}`}
                      onClick={() => setIndex(dot)}
                    />
                  ))}
                </div>
              </>
            ) : null}
          </div>
        </div>
      )}
    </div>
  );
}
