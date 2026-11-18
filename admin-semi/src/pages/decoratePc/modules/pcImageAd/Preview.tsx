/**
 * PC 旧版通宽广告图（pcImageAd）- 预览组件
 *
 * 渲染规格（权威）：前台 pc-semi 对 type=pcImageAd 的实际渲染——
 * contract.ts `adaptPcImageAd`（:392）对「老链形态」数据（无 picType 字段，
 * 本模块 def 的设置项正是这一形态）适配为 ContractImageAd 计划
 * { picList, frame: m.frame || { boxPaddingTop, boxPaddingBottom } }，
 * 由 modules.tsx `ContractImageAd`（:4632）渲染，逐字照抄旧版 ImageAd.vue：
 * - 一列图片自上而下顺序铺开，每张通宽（width:100%、height:auto）；
 * - 容器只有上下内边距：取 frame.boxPaddingTop / boxPaddingBottom；
 *   数据无 frame 时适配器以模块顶层的 boxPaddingTop / boxPaddingBottom
 *   （本模块 def 的两个 slider 设置项，缺省 10）合成 frame；
 *   有 frame 时以 frame 为准（不再回退顶层字段）——本预览同口径；
 * - 内边距为有限数才生效（旧版缺值拼出 undefinedpx 被浏览器忽略，前台同处理）；
 * - picList 为空时前台渲染一个空容器（无空态文案），编辑器内以空态提示替代
 *   （编辑器惯例，属已知偏差，见同线 MANIFEST）。
 *
 * 注意：数据若带 picType（新链 pc-image-ad 的栅格/轮播字段），前台会改走
 * 新版 ImageAd 组件——那不是本模块 def 的数据形态，不在本预览范围。
 * 与新版 pc-image-ad 预览（七种 picType 排版 + title + frame 全套）不是同一
 * 组件、不可复用，本组件按老链口径独立实现。
 */
import { useMemo, type CSSProperties, type ReactNode } from 'react';
import { useI18n } from '../../../../i18n';
import './pcImageAd.css';

type AnyRecord = Record<string, any>;

/** 空图片占位（与新版预览同款内联 SVG）。 */
const IMAGE_PLACEHOLDER =
  'data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect fill="%23f0f0f0" width="100" height="100"/></svg>';

/** 本模块 def 的 defaults（逐字段一致）。 */
const pcImageAdLegacyDefaults: AnyRecord = {
  picList: [],
  boxPaddingTop: 10,
  boxPaddingBottom: 10,
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

export default function PcImageAdLegacyPreview({ data }: { data: Record<string, any> }): ReactNode {
  const { t } = useI18n();

  const moduleConfig = useMemo(() => deepMerge(pcImageAdLegacyDefaults, data), [data]);
  const picList = useMemo(
    () => (Array.isArray(moduleConfig.picList) ? moduleConfig.picList : []) as AnyRecord[],
    [moduleConfig.picList],
  );

  // 前台 adaptPcImageAd 口径：frame 存在则以 frame 为准，否则用顶层字段合成
  const boxStyle = useMemo(() => {
    const raw = (data || {}) as AnyRecord;
    const frame =
      raw.frame && typeof raw.frame === 'object'
        ? (raw.frame as AnyRecord)
        : { boxPaddingTop: moduleConfig.boxPaddingTop, boxPaddingBottom: moduleConfig.boxPaddingBottom };
    const style: CSSProperties = {};
    const padTop = Number(frame.boxPaddingTop);
    const padBottom = Number(frame.boxPaddingBottom);
    if (Number.isFinite(padTop)) style.paddingTop = `${padTop}px`;
    if (Number.isFinite(padBottom)) style.paddingBottom = `${padBottom}px`;
    return style;
  }, [data, moduleConfig.boxPaddingTop, moduleConfig.boxPaddingBottom]);

  if (!picList.length) {
    return (
      <div className="dec-pc-image-ad-legacy">
        <div className="dec-pc-image-ad-legacy__empty">
          <div className="dec-pc-image-ad-legacy__empty-text">{t('点击添加通宽广告图')}</div>
        </div>
      </div>
    );
  }

  return (
    <div className="dec-pc-image-ad-legacy">
      <div className="dec-pc-image-ad-legacy__list" style={boxStyle}>
        {picList.map((pic, i) => (
          <div className="dec-pc-image-ad-legacy__item" key={pic?.picId ?? i}>
            <a
              href={pic?.picLink?.link || 'javascript:;'}
              target="_blank"
              rel="noopener noreferrer"
              title={pic?.picTitle || ''}
            >
              <img className="dec-pc-image-ad-legacy__img" src={formatImage(pic)} alt={pic?.picTitle || ''} />
            </a>
          </div>
        ))}
      </div>
    </div>
  );
}
