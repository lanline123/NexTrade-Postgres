/**
 * PC 旧版自定义模块二（pcCustomAd2）- 预览组件
 *
 * 渲染规格（权威）：前台 pc-semi 对 type=pcCustomAd2 的实际渲染——
 * contract.ts `adaptPcCustomAd2`（:431）适配为 ContractCustomAd 计划
 * { picList, picList2, picList3, groups, links: toLinks(linkList), frame }，
 * 由 modules.tsx `ContractCustomAd`（:5561）渲染，逐字照抄旧版 CustomAd2.vue：
 * - 左主区（写死渐变底 #b96131 → #6a452b）：
 *   - .left_banner：主图 = picList[0]（旧版 picList 亦可为单个对象，两种形态
 *     都兼容），图固定 316×450；图下 .banner_link 链接行只出 linkList 前 6 条；
 *   - .center_con：picList2 按下标逐格取图——[0] 大图（.global_pic），
 *     [1..2] 左列下行两格，[3..4] 右列两格；每格 = 标题 picTitle + 副标题
 *     picDesc + 图（proLink 三处同构）；
 * - 右列 .global_category：picList3 全部逐条出卡（标题 + 副标题 + 色条 + 图）；
 * - linkList 的数据形态是 { link: { link, title }, linkTitle }，前台 toLinks
 *   （contract.ts:108）归一为 { link, text }（text 取 linkTitle → link.title →
 *   title，空文案空链接的条目被过滤）——本预览内置同口径归一；
 * - frame 前台适配器透传但 ContractCustomAd 不消费，本预览同样不消费；
 * - 主图与 picList2 / picList3 全空时前台整模块不渲染（链接行不单独成立），
 *   编辑器内以空态提示替代。
 */
import { useMemo, type CSSProperties, type ReactNode } from 'react';
import { useI18n } from '../../../../i18n';
import './pcCustomAd2.css';

type AnyRecord = Record<string, any>;

/** 空图片占位（与新版预览同款内联 SVG）。 */
const IMAGE_PLACEHOLDER =
  'data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect fill="%23f0f0f0" width="100" height="100"/></svg>';

/** 本模块 def 的 defaults（逐字段一致）。 */
const pcCustomAd2LegacyDefaults: AnyRecord = {
  linkList: [],
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

/** linkList 条目的链接值提取（字符串直用，对象取 link / href）。 */
function linkHrefOf(value: any): string {
  if (!value) return '';
  if (typeof value === 'string') return value;
  if (typeof value === 'object') return String(value.link || value.href || '');
  return '';
}

/** 前台 toLinks（contract.ts:108）同口径：{link:{link,title}, linkTitle} → {link, text}。 */
function toLinks(value: any): Array<{ link: string; text: string }> {
  if (!Array.isArray(value)) return [];
  return value
    .map((item: any) => ({
      link: linkHrefOf(item?.link),
      text: String(item?.linkTitle || item?.link?.title || item?.title || ''),
    }))
    .filter((item) => item.text || item.link);
}

export default function PcCustomAd2LegacyPreview({ data }: { data: Record<string, any> }): ReactNode {
  const { t } = useI18n();

  const moduleConfig = useMemo(() => deepMerge(pcCustomAd2LegacyDefaults, data), [data]);

  // 主图：旧版 picList 是单个对象，编辑器形态是数组——两种都兼容（前台同处理）
  const mainPicture = useMemo(() => {
    const raw = moduleConfig.picList;
    return (Array.isArray(raw) ? raw[0] : raw) as AnyRecord | undefined;
  }, [moduleConfig.picList]);
  const second = useMemo(
    () => (Array.isArray(moduleConfig.picList2) ? moduleConfig.picList2 : []) as AnyRecord[],
    [moduleConfig.picList2],
  );
  const third = useMemo(
    () => (Array.isArray(moduleConfig.picList3) ? moduleConfig.picList3 : []) as AnyRecord[],
    [moduleConfig.picList3],
  );
  const links = useMemo(() => toLinks(moduleConfig.linkList), [moduleConfig.linkList]);

  if (!mainPicture && !second.length && !third.length) {
    return (
      <div className="dec-pc-custom-ad2-legacy">
        <div className="dec-pc-custom-ad2-legacy__empty">
          <div className="dec-pc-custom-ad2-legacy__empty-text">{t('点击添加自定义模块二')}</div>
        </div>
      </div>
    );
  }

  /** .pro_shoppe / .global_pic 内部的 <a>：标题 + 副标题 + 图（前台 proLink，三处复用）。 */
  const proLink = (pic: AnyRecord | undefined, variant: 'big' | 'cell'): ReactNode =>
    pic ? (
      <a
        className="dec-pc-custom-ad2-legacy__pro-link"
        href={pic?.picLink?.link || 'javascript:;'}
        target="_blank"
        rel="noopener noreferrer"
        title={pic?.picTitle || ''}
      >
        <p className="dec-pc-custom-ad2-legacy__pro-tit">{pic?.picTitle || ''}</p>
        <p className="dec-pc-custom-ad2-legacy__pro-sub">{pic?.picDesc || ''}</p>
        <img
          className={`dec-pc-custom-ad2-legacy__pro-img dec-pc-custom-ad2-legacy__pro-img--${variant}`}
          src={formatImage(pic)}
          alt={pic?.picTitle || ''}
        />
      </a>
    ) : null;

  const gradientStyle: CSSProperties = { background: 'linear-gradient(#b96131 0%, #6a452b 100%)' };

  return (
    <div className="dec-pc-custom-ad2-legacy">
      <div className="dec-pc-custom-ad2-legacy__food" style={gradientStyle}>
        <div className="dec-pc-custom-ad2-legacy__left-banner">
          {mainPicture ? (
            <a
              href={mainPicture?.picLink?.link || 'javascript:;'}
              target="_blank"
              rel="noopener noreferrer"
              title={mainPicture?.picTitle || ''}
            >
              <img
                className="dec-pc-custom-ad2-legacy__main-img"
                style={{ width: 316, height: 450 }}
                src={formatImage(mainPicture)}
                alt={mainPicture?.picTitle || ''}
              />
            </a>
          ) : null}
          <div className="dec-pc-custom-ad2-legacy__banner-link">
            <ul className="dec-pc-custom-ad2-legacy__link-list">
              {links.slice(0, 6).map((item, i) => (
                <li key={i}>
                  <a href={item.link || 'javascript:;'} target="_blank" rel="noopener noreferrer" title={item.text}>
                    {item.text}
                  </a>
                </li>
              ))}
            </ul>
          </div>
        </div>

        <div className="dec-pc-custom-ad2-legacy__center">
          <div className="dec-pc-custom-ad2-legacy__center-left">
            <div className="dec-pc-custom-ad2-legacy__shoppe">{proLink(second[0], 'big')}</div>
            <div className="dec-pc-custom-ad2-legacy__pro-wrap">
              <div className="dec-pc-custom-ad2-legacy__shoppe">{proLink(second[1], 'cell')}</div>
              <div className="dec-pc-custom-ad2-legacy__shoppe">{proLink(second[2], 'cell')}</div>
            </div>
          </div>
          <div className="dec-pc-custom-ad2-legacy__center-right">
            <div className="dec-pc-custom-ad2-legacy__shoppe">{proLink(second[3], 'cell')}</div>
            <div className="dec-pc-custom-ad2-legacy__shoppe">{proLink(second[4], 'cell')}</div>
          </div>
        </div>
      </div>

      <div className="dec-pc-custom-ad2-legacy__category">
        <ul className="dec-pc-custom-ad2-legacy__category-list">
          {third.map((pic, i) => (
            <li key={pic?.picId ?? i}>
              <a
                className="dec-pc-custom-ad2-legacy__category-a"
                href={pic?.picLink?.link || 'javascript:;'}
                target="_blank"
                rel="noopener noreferrer"
                title={pic?.picTitle || ''}
              >
                <p className="dec-pc-custom-ad2-legacy__pro-tit dec-pc-custom-ad2-legacy__pro-tit--dark">{pic?.picTitle || ''}</p>
                <p className="dec-pc-custom-ad2-legacy__pro-sub dec-pc-custom-ad2-legacy__pro-sub--dark">{pic?.picDesc || ''}</p>
                <p className="dec-pc-custom-ad2-legacy__pro-color" />
                <img className="dec-pc-custom-ad2-legacy__category-img" src={formatImage(pic)} alt={pic?.picTitle || ''} />
              </a>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
