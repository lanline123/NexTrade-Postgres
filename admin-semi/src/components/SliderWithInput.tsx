/**
 * 滑杆 + 数字框联动控件（共享）：从 PC 装修编辑器 PcDecorateEditor.tsx:4612 的同名实现抽出，
 * Semi Slider + InputNumber，样式类 .pchead-slider-with-input 在 pages/decorate-editor.css:1354
 * （随 PcDecorateEditor/DecoratePcPages 静态导入，全局生效）。
 *
 * 两端均已改引本组件：PC 编辑器 PcDecorateEditor 原内部实现已删除（2026-11-15 收尾批次，
 * 实现逐行等价、props/默认值不变）；H5 装修编辑器 shared/settingWidgets 的
 * RangeField/NumberField 亦改用本组件，替代原原生 <input type="range"> + <input type="number">。
 */
import { Slider, InputNumber } from '@douyinfe/semi-ui';

export default function SliderWithInput({
  value,
  min = 0,
  max = 100,
  step = 1,
  onChange,
}: {
  value: number;
  min?: number;
  max?: number;
  step?: number;
  onChange: (value: number) => void;
}) {
  const clamp = (next: number | string | null | undefined) => {
    const numeric = Number(next);
    if (!Number.isFinite(numeric)) return value;
    return Math.min(max, Math.max(min, numeric));
  };
  return (
    <div className="pchead-slider-with-input">
      <Slider
        value={value}
        min={min}
        max={max}
        step={step}
        style={{ flex: 1, minWidth: 90 }}
        onChange={(v) => onChange(typeof v === 'number' ? v : (v as number[])[0])}
      />
      <InputNumber
        value={value}
        min={min}
        max={max}
        step={step}
        style={{ width: 72, flexShrink: 0 }}
        onChange={(v) => onChange(clamp(v))}
      />
    </div>
  );
}
