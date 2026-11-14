/**
 * PC 文本标题模块 - 预览组件
 *
 * 标题本体走共享 ModuleTitlePreview（与图片广告/商品等模块同一套通用标题样式，
 * 与 PC 端 DecModuleTitle 同源）：标题样式 1/2/3、对齐、描述（含 descLayout）、
 * 标题图标、背景渐变、圆角、四边距、更多链接 / 倒计时（moreType=2）全部由共享
 * 组件按 title 契约渲染。此前本模块用自维护的一套标题 DOM（.dec-pc-title-ad__inner
 * 等），字号写死 16、不读标题边距、不支持描述排版，与其他模块的标题长相漂移。
 *
 * 本组件只负责：frame 容器（背景/圆角/内边距）、无标题文案时的空态，
 * 以及倒计时的每秒刷新（把 countdownEndTime 换算成毫秒时间戳传给共享组件）。
 */
import { useEffect, useState, type CSSProperties } from 'react';
import { useI18n } from '../../../i18n';
import ModuleTitlePreview from './shared/ModuleTitlePreview';
import './pcTitleAd.css';

type AnyRecord = Record<string, any>;

interface PcTitleAdConfig {
  title: AnyRecord;
  frame: AnyRecord;
}

/** 旧版 `pcTitleAdDefaults`（逐字段对齐 index.ts 的 title/frame 默认值）。 */
const DEFAULTS: PcTitleAdConfig = {
  title: {
    showTitle: 1,
    titleStyle: 1,
    titleAlign: 1,
    titleBackground: '',
    titleBackground2: '',
    titleBackgroundPic: { picUrl: '', picThumb: '' },
    titleRadius: 0,
    titleText: '标题内容',
    titleColor: '#2a3145',
    descText: '',
    descColor: '#aaaaaa',
    showMore: 0,
    moreType: 1,
    countdownLabel: '',
    moreLink: { link: '', title: '' },
    moreColor: '#969799',
  },
  frame: {
    textColor: '',
    itemBackgroundColor: '',
    backgroundColor: '#ffffff',
    innerPadding: 0,
    itemHeight: 35,
    itemRadius: 3,
    boxRadius: 0,
    boxPadding: 10,
    leftAndRightPadding: 0,
    boxPaddingTop: 5,
    boxPaddingBottom: 5,
  },
};

function mergeConfig(data: AnyRecord | undefined): PcTitleAdConfig {
  const source = data || {};
  return {
    ...DEFAULTS,
    ...source,
    title: { ...DEFAULTS.title, ...(source.title || {}) },
    frame: { ...DEFAULTS.frame, ...(source.frame || {}) },
  } as PcTitleAdConfig;
}

export default function PcTitleAdPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const config = mergeConfig(data);
  const title = config.title || {};
  const frame = config.frame || {};

  const hasTitleText = String(title.titleText ?? '').trim().length > 0;

  const cssVars = {
    '--dec-bg-color': frame.backgroundColor ?? '#ffffff',
    '--dec-box-radius': `${frame.boxRadius ?? 0}px`,
    '--dec-box-padding': `${frame.boxPadding ?? 10}px`,
    '--dec-box-padding-top': `${frame.boxPaddingTop ?? 5}px`,
    '--dec-box-padding-bottom': `${frame.boxPaddingBottom ?? 5}px`,
    '--dec-inner-padding': `${frame.leftAndRightPadding ?? frame.innerPadding ?? 0}px`,
    '--dec-left-right-padding': `${frame.leftAndRightPadding ?? 0}px`,
  } as CSSProperties & Record<string, string>;

  // 倒计时目标时间（契约 countdownEndTime 支持秒或毫秒）；共享组件只做纯渲染，
  // 每秒刷新 now 由这里负责，活动结束/无目标时不启定时器（共享组件显示 00:00:00 占位）。
  const rawEnd = Number(title.countdownEndTime ?? 0);
  const endMs = rawEnd > 0 ? (rawEnd > 1e12 ? rawEnd : rawEnd * 1000) : 0;
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!endMs || endMs <= Date.now()) return undefined;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [endMs]);

  return (
    <div className="dec-pc-title-ad" style={cssVars}>
      <div className="dec-pc-title-ad__content">
        {!hasTitleText ? (
          <div className="dec-pc-title-ad__empty">
            <div className="dec-pc-title-ad__empty-text">{t('点击添加标题')}</div>
          </div>
        ) : (
          <ModuleTitlePreview
            title={title}
            countdownTargetTs={endMs}
            now={now}
            countdownLabelFallback={t('倒计时')}
          />
        )}
      </div>
    </div>
  );
}
