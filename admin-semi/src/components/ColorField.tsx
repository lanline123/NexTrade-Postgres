/**
 * 颜色字段外壳（共享）：「重置」文字链 + AlphaColorInput 色块触发器。
 *
 * 芯（取色弹层）一直是共享的 components/AlphaColorInput.tsx，但外壳此前两端各写一份：
 * PC 在 PcDecorateEditor.tsx 的字段渲染器内（color / colorPair），
 * H5 在 decorateEditorH5/shared/settingWidgets.tsx 的 ColorField 内（.dec-color-* 类）。
 * 本文件收敛单色（ColorField）与双色对（ColorPairField）两种外壳：
 * 类名沿用 .dec-color-group/.dec-color-button/.dec-color-reset/.dec-color-trigger/
 * .dec-color-group--dual/.dec-color-sep（PC 样式在 pages/decorate-editor.css，
 * H5 样式在 decorateEditorH5 的 decorate.css/editor.less，两端同名同义）。
 * 2026-11-15 收尾批次：PC 字段渲染器的 color/colorPair 与 H5 styleEdits 的
 * 背景渐变双色对均已改引本文件，两端不再各存一份外壳实现。
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

/**
 * 双色对外壳（共享）：一个「重置」链同时把两色复位 + 两颗 AlphaColorInput 以「-」分隔。
 * 收敛自 PC 字段渲染器 colorPair 与 H5 styleEdits 背景渐变颜色两处同构实现。
 * onReset 可选：调用方需要把两次复位合并为一次数据补丁时传入（如 H5 styleEdits
 * 的渐变色对是单个 onChange 补丁）；不传则默认分别以 resetValue/resetValueSecond 回调。
 */
export function ColorPairField({
  value,
  secondValue,
  onChange,
  onSecondChange,
  resetValue = '',
  resetValueSecond = '',
  onReset,
}: {
  value: string;
  secondValue: string;
  onChange: (value: string) => void;
  onSecondChange: (value: string) => void;
  resetValue?: string;
  resetValueSecond?: string;
  onReset?: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="dec-color-group dec-color-group--dual">
      <div className="dec-color-button">
        <a
          className="dec-color-reset"
          onClick={() => {
            if (onReset) {
              onReset();
              return;
            }
            onChange(resetValue);
            onSecondChange(resetValueSecond);
          }}
        >
          {t('重置')}
        </a>
        <span className="dec-color-trigger">
          <AlphaColorInput value={value} onChange={onChange} />
        </span>
        <span className="dec-color-sep">-</span>
        <span className="dec-color-trigger">
          <AlphaColorInput value={secondValue} onChange={onSecondChange} />
        </span>
      </div>
    </div>
  );
}
