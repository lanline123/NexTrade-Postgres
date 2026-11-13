import type { CSSProperties, ReactNode } from 'react';
import { useI18n } from '../../../../i18n';
import './module-title.shared.css';

/**
 * 模块标题预览 —— 单一来源（此前 6 个预览文件各抄一份，字号/行高/对齐已经漂移）。
 *
 * 数值基准 = PC 前台 ShopTitle（pc-semi `top-bar`，decorate-source.scss「shop/Title.vue scoped」）：
 * 容器 padding 10px、行高 32px、主标题 18px/700、描述 13px、图标 32px。
 * 对齐（1 左 / 2 居中 / 3 右）与样式（1 / 2 竖线 / 3 下划线）语义与前台逐条一致。
 *
 * 右侧内容按契约 moreType 二选：1 = 更多链接、2 = 倒计时（秒杀/拼团）。
 * 倒计时只接收目标时间戳与当前时间，活动数据由调用方模块负责取。
 */
const pad2 = (n: number) => String(n).padStart(2, '0');

function splitCountdown(ms: number) {
  const total = Math.max(0, Math.floor(ms / 1000));
  return {
    days: Math.floor(total / 86400),
    hours: Math.floor((total % 86400) / 3600),
    minutes: Math.floor((total % 3600) / 60),
    seconds: total % 60,
  };
}

export default function ModuleTitlePreview({
  title,
  fallbackText,
  countdownTargetTs = 0,
  now = 0,
  countdownLabelFallback,
}: {
  title?: Record<string, any> | null;
  fallbackText?: string;
  /** moreType=2 时的活动结束时间戳（毫秒）；0 表示无活动 */
  countdownTargetTs?: number;
  /** 与 countdownTargetTs 同一时钟的当前时间戳（毫秒） */
  now?: number;
  /** countdownLabel 为空时的兜底文案（秒杀「倒计时」/ 拼团「距结束」） */
  countdownLabelFallback?: string;
}): ReactNode {
  const { t } = useI18n();
  if (!title || Number(title.showTitle) !== 1) return null;

  const titleStyle = Number(title.titleStyle) || 1;
  const align = Number(title.titleAlign) || 1;
  const descLayout = Number(title.descLayout ?? 1);
  const titleFontSize = Number(title.titleFontSize) || 18;
  const descFontSize = Number(title.descFontSize) || 13;
  const titleIcon = title.titleBackgroundPic?.picUrl || title.titleBackgroundPic?.picThumb || '';
  const descBelow = descLayout === 2;
  const showMore = Number(title.showMore) === 1;
  const isCountdown = Number(title.moreType) === 2;

  const containerStyle: CSSProperties = {};
  if (title.titleBackground2 && title.titleBackground) {
    containerStyle.background = `linear-gradient(to right, ${title.titleBackground}, ${title.titleBackground2})`;
  } else if (title.titleBackground) {
    containerStyle.backgroundColor = title.titleBackground;
  }
  if (title.titleRadius) {
    containerStyle.borderRadius = `${Number(title.titleRadius)}px ${Number(title.titleRadius)}px 0 0`;
  }
  const padTop = Number(title.titlePaddingTop) || 0;
  const padBottom = Number(title.titlePaddingBottom) || 0;
  if (padTop) containerStyle.paddingTop = `${padTop}px`;
  if (padBottom) containerStyle.paddingBottom = `${padBottom}px`;
  // 标题左右边距：未配置时按旧 CSS 的 10px 走（零跳变），配置 0 则紧贴模块最左/最右。
  // 始终写内联（含 0），否则会被 .dec-module-title 的 padding:10px 覆盖回去。
  const padLeft = title.titlePaddingLeft == null || title.titlePaddingLeft === '' ? 10 : Number(title.titlePaddingLeft);
  const padRight = title.titlePaddingRight == null || title.titlePaddingRight === '' ? 10 : Number(title.titlePaddingRight);
  containerStyle.paddingLeft = `${Number.isFinite(padLeft) ? padLeft : 10}px`;
  containerStyle.paddingRight = `${Number.isFinite(padRight) ? padRight : 10}px`;

  const customLabel = String(title.countdownLabel || '').trim();
  let countdownLabel = customLabel || countdownLabelFallback || t('距结束');
  if (!customLabel && countdownTargetTs && countdownTargetTs - now <= 0) countdownLabel = t('已结束');
  const parts = splitCountdown(countdownTargetTs ? countdownTargetTs - now : 0);

  return (
    <div
      className={[
        'dec-module-title',
        `dec-module-title--style-${titleStyle}`,
        `dec-module-title--align-${align}`,
        descBelow ? 'dec-module-title--desc-below' : '',
      ]
        .filter(Boolean)
        .join(' ')}
      style={containerStyle}
    >
      <div className={`dec-module-title__left${descBelow ? ' dec-module-title__left--wrap' : ''}`}>
        {titleIcon ? <img className="dec-module-title__icon" src={titleIcon} alt="" /> : null}
        <div
          className="dec-module-title__text"
          style={{ color: title.titleColor || '#2a3145', fontSize: `${titleFontSize}px` }}
        >
          {title.titleText || fallbackText}
        </div>
        {title.descText ? (
          <div
            className="dec-module-title__desc"
            style={{ color: title.descColor || '#aaaaaa', fontSize: `${descFontSize}px` }}
          >
            {title.descText}
          </div>
        ) : null}
      </div>

      {showMore && isCountdown ? (
        <div className="dec-module-title__right">
          <div className="dec-module-title__countdown" style={{ color: title.moreColor || '#1a1a1a' }}>
            <div className="dec-module-title__countdown-box">
              <span className="dec-module-title__countdown-label">{countdownLabel}: </span>
              {parts.days > 0 ? (
                <>
                  <span className="dec-module-title__countdown-item dec-module-title__countdown-item--days">{parts.days}</span>
                  <span className="dec-module-title__countdown-daytext">{t('天')}</span>
                </>
              ) : null}
              <span className="dec-module-title__countdown-item">{pad2(parts.hours)}</span>
              <span className="dec-module-title__countdown-sep">:</span>
              <span className="dec-module-title__countdown-item">{pad2(parts.minutes)}</span>
              <span className="dec-module-title__countdown-sep">:</span>
              <span className="dec-module-title__countdown-item">{pad2(parts.seconds)}</span>
            </div>
          </div>
        </div>
      ) : null}

      {showMore && !isCountdown ? (
        <div className="dec-module-title__right">
          <a
            className="dec-module-title__more"
            href={title.moreLink?.link || 'javascript:;'}
            style={{ color: title.moreColor || '#969799' }}
          >
            <span>{title.moreLink?.title || t('更多')}</span>
            <i className="dec-module-title__arrow">›</i>
          </a>
        </div>
      ) : null}
    </div>
  );
}