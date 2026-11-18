/**
 * PC 旧版自定义模块一（pcCustomAd1）- 预览组件
 *
 * 渲染规格（权威）：前台 pc-semi 对 type=pcCustomAd1 的实际渲染——
 * contract.ts `adaptPcCustomAd1`（:417）原样透传 { picList, picList2, picList3, frame }，
 * 由 modules.tsx `ContractCustomAd1`（:5676）渲染，逐字照抄旧版 CustomAd1.vue
 * 的固定三列编辑版式（不是三组图平铺）：
 * - 左列 .left_banner：只出 picList[0]（原版 270×400 槽位）；
 * - 中列「每日推荐」：picList2[0] 为大图（.brand_banner），picList2[1..] 全部
 *   逐条出行卡（标题 picTitle + 副标题 picDesc + 90×90 图）；
 * - 右列「热销精品」：picList3[0] 为大图且图下带标题/副标题，picList3[1..2]
 *   出行卡，picList3[3..4] 出底部 .life_pic 卡（图 + 标题 + 「GO >>」角标）；
 * - 两列标题文案是前台写死的「每日推荐 / 热销精品」，不读数据；
 * - frame 前台适配器透传但 ContractCustomAd1 不消费，本预览同样不消费；
 * - 三组全空时前台整模块不渲染，编辑器内以空态提示替代。
 *
 * 已知偏差：标题旁的 iconfont-pc 字形（icon-jingpin / icon-jingpintuijian）
 * 在后台字体环境里不保证可用，本预览只保留三角装饰 + 文案（见同线 MANIFEST）。
 */
import { useMemo, type ReactNode } from 'react';
import { useI18n } from '../../../../i18n';
import './pcCustomAd1.css';

type AnyRecord = Record<string, any>;

/** 空图片占位（与新版预览同款内联 SVG）。 */
const IMAGE_PLACEHOLDER =
  'data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect fill="%23f0f0f0" width="100" height="100"/></svg>';

/** 本模块 def 的 defaults（逐字段一致）。 */
const pcCustomAd1LegacyDefaults: AnyRecord = {
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

export default function PcCustomAd1LegacyPreview({ data }: { data: Record<string, any> }): ReactNode {
  const { t } = useI18n();

  const moduleConfig = useMemo(() => deepMerge(pcCustomAd1LegacyDefaults, data), [data]);
  const left = useMemo(
    () => (Array.isArray(moduleConfig.picList) ? moduleConfig.picList : []) as AnyRecord[],
    [moduleConfig.picList],
  );
  const rec = useMemo(
    () => (Array.isArray(moduleConfig.picList2) ? moduleConfig.picList2 : []) as AnyRecord[],
    [moduleConfig.picList2],
  );
  const hot = useMemo(
    () => (Array.isArray(moduleConfig.picList3) ? moduleConfig.picList3 : []) as AnyRecord[],
    [moduleConfig.picList3],
  );

  if (!left.length && !rec.length && !hot.length) {
    return (
      <div className="dec-pc-custom-ad1-legacy">
        <div className="dec-pc-custom-ad1-legacy__empty">
          <div className="dec-pc-custom-ad1-legacy__empty-text">{t('点击添加自定义模块一')}</div>
        </div>
      </div>
    );
  }

  /** 行卡：标题 + 副标题 + 90×90 图（前台 rowCard，中右两列三处复用）。 */
  const rowCard = (item: AnyRecord | undefined): ReactNode =>
    item ? (
      <a
        className="dec-pc-custom-ad1-legacy__row-card"
        href={item?.picLink?.link || 'javascript:;'}
        target="_blank"
        rel="noopener noreferrer"
        title={item?.picTitle || ''}
      >
        <span className="dec-pc-custom-ad1-legacy__row-text">
          <span className="dec-pc-custom-ad1-legacy__pro-tit">{item?.picTitle || ''}</span>
          <span className="dec-pc-custom-ad1-legacy__pro-sub">{item?.picDesc || ''}</span>
        </span>
        <img
          className="dec-pc-custom-ad1-legacy__row-img"
          style={{ width: 90, height: 90 }}
          src={formatImage(item)}
          alt={item?.picTitle || ''}
        />
      </a>
    ) : null;

  /** 列标题（前台 h4 > em：双三角 + 字形图标 + 写死文案；字形图标在后台省略）。 */
  const columnTitle = (text: string): ReactNode => (
    <h4 className="dec-pc-custom-ad1-legacy__h4">
      <em className="dec-pc-custom-ad1-legacy__h4-em">
        <u className="dec-pc-custom-ad1-legacy__tri dec-pc-custom-ad1-legacy__tri--left" />
        <u className="dec-pc-custom-ad1-legacy__tri dec-pc-custom-ad1-legacy__tri--right" />
        <span>{text}</span>
      </em>
    </h4>
  );

  return (
    <div className="dec-pc-custom-ad1-legacy">
      <div className="dec-pc-custom-ad1-legacy__left">
        {left[0] ? (
          <a
            href={left[0]?.picLink?.link || 'javascript:;'}
            target="_blank"
            rel="noopener noreferrer"
            title={left[0]?.picTitle || ''}
          >
            <img
              className="dec-pc-custom-ad1-legacy__left-img"
              style={{ background: '#eee' }}
              src={formatImage(left[0])}
              alt={left[0]?.picTitle || ''}
            />
          </a>
        ) : null}
      </div>

      <div className="dec-pc-custom-ad1-legacy__brand">
        {columnTitle(t('每日推荐'))}
        <div className="dec-pc-custom-ad1-legacy__brand-row">
          {rec[0] ? (
            <a
              className="dec-pc-custom-ad1-legacy__brand-banner"
              href={rec[0]?.picLink?.link || 'javascript:;'}
              target="_blank"
              rel="noopener noreferrer"
              title={rec[0]?.picTitle || ''}
            >
              <img className="dec-pc-custom-ad1-legacy__brand-img" src={formatImage(rec[0])} alt={rec[0]?.picTitle || ''} />
            </a>
          ) : null}
          <div className="dec-pc-custom-ad1-legacy__brand-right">
            <ul className="dec-pc-custom-ad1-legacy__list">
              {rec.map((item, i) =>
                i === 0 ? null : <li key={item?.picId ?? i}>{rowCard(item)}</li>,
              )}
            </ul>
          </div>
        </div>
      </div>

      <div className="dec-pc-custom-ad1-legacy__brand">
        {columnTitle(t('热销精品'))}
        <div className="dec-pc-custom-ad1-legacy__brand-row" style={{ height: 230 }}>
          {hot[0] ? (
            <a
              className="dec-pc-custom-ad1-legacy__brand-banner"
              href={hot[0]?.picLink?.link || 'javascript:;'}
              target="_blank"
              rel="noopener noreferrer"
              title={hot[0]?.picTitle || ''}
            >
              <img className="dec-pc-custom-ad1-legacy__brand-img" src={formatImage(hot[0])} alt={hot[0]?.picTitle || ''} />
              <p className="dec-pc-custom-ad1-legacy__pro-tit">{hot[0]?.picTitle || ''}</p>
              <p className="dec-pc-custom-ad1-legacy__pro-sub">{hot[0]?.picDesc || ''}</p>
            </a>
          ) : null}
          <div className="dec-pc-custom-ad1-legacy__brand-right">
            <ul className="dec-pc-custom-ad1-legacy__list">
              <li>{rowCard(hot[1])}</li>
              <li>{rowCard(hot[2])}</li>
            </ul>
          </div>
        </div>
        <div className="dec-pc-custom-ad1-legacy__life-pic">
          {hot.slice(3, 5).map((item, i) =>
            item ? (
              <a
                className="dec-pc-custom-ad1-legacy__shp"
                key={item?.picId ?? i}
                href={item?.picLink?.link || 'javascript:;'}
                target="_blank"
                rel="noopener noreferrer"
                title={item?.picTitle || ''}
              >
                <img className="dec-pc-custom-ad1-legacy__shp-img" src={formatImage(item)} alt={item?.picTitle || ''} />
                <p className="dec-pc-custom-ad1-legacy__shp-p">{item?.picTitle || ''}</p>
                <p className="dec-pc-custom-ad1-legacy__shp-go">
                  GO<em> &gt;&gt;</em>
                </p>
              </a>
            ) : null,
          )}
        </div>
      </div>
    </div>
  );
}
