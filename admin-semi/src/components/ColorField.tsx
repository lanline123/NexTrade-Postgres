/**
 * 颜色字段外壳（共享）：「重置」文字链 + AlphaColorInput 色块触发器。
 *
 * 芯（取色弹层）一直是共享的 components/AlphaColorInput.tsx，但外壳此前两端各写一份：
 * PC 在 PcDecorateEditor.tsx:6960-6992 的字段渲染器内（color / colorPair），
 * H5 在 decorateEditorH5/shared/settingWidgets.tsx 的 ColorField 内（.dec-color-* 类）。
 * 本组件收敛单色外壳：沿用 H5 的 .dec-color-group/.dec-color-button/.dec-color-reset/
 * .dec-color-trigger 类名（样式在 decorateEditorH5/styles/editor.less），H5 已改用；
 * PC 侧外壳与双色对（colorPair）暂留原实现，后续迁移点即 PcDecorateEditor.tsx:6960-6992。
 */
import AlphaColorInput from './AlphaColorInput';
import { useI18n } from '../i18n';

export default function ColorField({
  value,
  onChange,
  resetValue = '',
  allowEmpty = true,
}: {
  /** 当前颜色值（空串 = 跟随主题/未设置） */
  value: string;
  onChange: (value: string) => void;
  /** 点「重置」时回到的值，默认空串 */
  resetValue?: string;
  /** 是否显示「重置」链（原版 showDecColorReset 口径） */
  allowEmpty?: boolean;
}) {
  const { t } = useI18n();
  return (
    <div className="dec-color-group">
      <div className="dec-color-button">
        {allowEmpty ? (
          <a className="dec-color-reset" onClick={() => onChange(resetValue)}>
            {t('重置')}
          </a>
        ) : null}
        <span className="dec-color-trigger">
          <AlphaColorInput value={value} onChange={onChange} />
        </span>
      </div>
    </div>
  );
}
