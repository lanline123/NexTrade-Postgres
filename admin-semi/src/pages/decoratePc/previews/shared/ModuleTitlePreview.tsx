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
 * 新增设置（老数据缺省时回退默认值，不产生视觉跳变）：
 * - titleFontSize：主标题字号（默认 18）
 * - descFontSize ：副标题字号（默认 13）
 * - descLayout   ：副标题排版，1 = 跟随主标题后方（默认，同前台旧行为）、2 = 另起一行在下方
 */
export default function ModuleTitlePreview({
  title,
  fallbackText,
}: {
  title?: Record<string, any> | null;
  fallbackText?: string;
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

  const containerStyle: CSSProperties = {};
  if (title.titleBackground2 && title.titleBackground) {
    containerStyle.background = `linear-gradient(to right, ${title.titleBackground}, ${title.titleBackground2})`;
  } else if (title.titleBackground) {
    containerStyle.backgroundColor = title.titleBackground;
  }
  if (title.titleRadius) {
    containerStyle.borderRadius = `${Number(title.titleRadius)}px ${Number(title.titleRadius)}px 0 0`;
  }

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

      {title.showMore ? (
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
