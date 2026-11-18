/**
 * PC 旧版分组轮播 Banner（pcGroupBanner）- 预览组件
 *
 * 渲染规格（权威）：前台 pc-semi 对 type=pcGroupBanner 的实际渲染——
 * contract.ts `adaptPcGroupBanner`（:179）先把 picList 按 number 截取
 * （number > 0 时 banners = picList.slice(0, number)，否则全量），
 * 再由 modules.tsx `GroupBanner`（:3341）渲染。注意适配后的计划里不再带
 * number，所以 GroupBanner 的布局 perView 恒为缺省 3：
 * - 展示图 = 截取后的 picList（最多 number 张），每张按 3 列宽排布
 *   （calc((100% - 2*30px) / 3)），图少于 3 张时右侧留空、不拉伸；
 * - 间距固定 30px（原版 PcGroupBanner swiper spaceBetween）；
 * - 逐格步进轮播：页数 = 图数 - 3 + 1，5000ms 自动播放，箭头翻页，无指示点；
 * - 高度 = frame.slideHeight（>0 时），缺省 195px；
 * - 图片圆角：picRadioStyle 2=8px / 1=0，老数据无此字段时跟随 frame.boxRadius；
 * - frame 其余字段走 px() 语义（缺值 0、背景缺省 transparent）；
 * - bannerStyle / showCat / logo / picList2 / picList3 前台一概不消费；
 * - 截取后无图时前台整模块不渲染，编辑器内以空态提示替代。
 *
 * 与新版 pc-group-banner 的 number 语义差异（不可直接复用，依据见同线 MANIFEST）：
 * 新版 number 是「轮播每屏张数」（全部图片入轨、按 number 分页轮播）；
 * 旧版 number 是「入选张数上限」（先截取、布局恒 3 列）。两者数据语义不同，
 * 本组件按前台旧链口径独立实现，仅沿用同款等价轮播写法。
 */
import { useEffect, useMemo, useState, type CSSProperties, type ReactNode } from 'react';
import { useI18n } from '../../../../i18n';
import './pcGroupBanner.css';

type AnyRecord = Record<string, any>;

/** 空图片占位（与新版预览同款内联 SVG）。 */
const IMAGE_PLACEHOLDER =
  'data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect fill="%23f0f0f0" width="100" height="100"/></svg>';

/** 本模块 def 的 defaults（逐字段一致；number 在 def 里缺省 undefined）。 */
const pcGroupBannerLegacyDefaults: AnyRecord = {
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

/** 前台 GroupBanner 的布局列数：适配计划不带 number，恒为缺省 3。 */
const PER_VIEW = 3;
/** 前台写死间距（原版 swiper spaceBetween: 30）。 */
const GAP = 30;

export default function PcGroupBannerLegacyPreview({ data }: { data: Record<string, any> }): ReactNode {
  const { t } = useI18n();

  const moduleConfig = useMemo(() => deepMerge(pcGroupBannerLegacyDefaults, data), [data]);

  // 前台 adaptPcGroupBanner：number > 0 时先截取 picList
  const banners = useMemo(() => {
    const pics = (Array.isArray(moduleConfig.picList) ? moduleConfig.picList : []) as AnyRecord[];
    const number = Number(moduleConfig.number);
    return Number.isFinite(number) && number > 0 ? pics.slice(0, Math.floor(number)) : pics;
  }, [moduleConfig.picList, moduleConfig.number]);

  const frame = (moduleConfig.frame || {}) as AnyRecord;
  const slideHeight = px(frame.slideHeight) > 0 ? px(frame.slideHeight) : 195;

  const cssVars = useMemo(() => {
    // 图片边角：取原始数据的 picRadioStyle（2=圆角 8px、1=直角 0），无此字段回退容器圆角
    const rawStyle = Number((data || {}).picRadioStyle);
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
  }, [frame, data, slideHeight]);

  const count = banners.length;
  const pages = Math.max(1, count - PER_VIEW + 1);

  const [index, setIndex] = useState(0);
  const [hovered, setHovered] = useState(false);

  useEffect(() => {
    setIndex(0);
  }, [count]);

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
    width: `${(count / PER_VIEW) * 100}%`,
    transform: `translateX(-${(Math.min(index, pages - 1) * 100) / Math.max(count, 1)}%)`,
  };

  return (
    <div className="dec-pc-group-banner-legacy" style={cssVars}>
      {!count ? (
        <div className="dec-pc-group-banner-legacy__empty">
          <div className="dec-pc-group-banner-legacy__empty-text">{t('点击添加分组轮播Banner')}</div>
        </div>
      ) : (
        <div className="dec-pc-group-banner-legacy__inner">
          <div
            className="dec-pc-group-banner-legacy__carousel"
            onMouseEnter={() => setHovered(true)}
            onMouseLeave={() => setHovered(false)}
          >
            <div className="dec-pc-group-banner-legacy__track" style={trackStyle}>
              {banners.map((pic, i) => (
                <div
                  className="dec-pc-group-banner-legacy__slide"
                  key={pic?.picId ?? i}
                  style={{ width: `${100 / Math.max(count, 1)}%`, paddingRight: `${GAP}px` }}
                >
                  <a
                    className="dec-pc-group-banner-legacy__link"
                    href={pic?.picLink?.link || 'javascript:;'}
                    target="_blank"
                    rel="noopener noreferrer"
                    title={pic?.picTitle || ''}
                  >
                    <img className="dec-pc-group-banner-legacy__img" src={formatImage(pic)} alt={pic?.picTitle || ''} />
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
      )}
    </div>
  );
}
