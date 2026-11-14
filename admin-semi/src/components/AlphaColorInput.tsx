/**
 * 带透明度的颜色选择控件（自 PC 装修编辑器抽出，两端编辑器共用）：
 * 棋盘格色块触发器 + 弹层（SV 面板 / 色相条 / 透明度条 / 值输入 / 预置颜色）。
 * 样式为 decorate-editor.css 与 decorateEditorH5/styles/editor.less 中的 .pchead-color-* / .pchead-alpha-color__*。
 */
import React, { useState } from 'react';
import { Input, Popover } from '@douyinfe/semi-ui';
import { useI18n } from '../i18n';

function parseColorValue(value: string): { hex: string; alpha: number } {
  const raw = String(value || '').trim();
  if (/^transparent$/i.test(raw)) return { hex: '#ffffff', alpha: 0 };
  const hexMatch = raw.match(/^#([0-9a-f]{3,8})$/i);
  if (hexMatch) {
    let hex = hexMatch[1];
    if (hex.length === 3 || hex.length === 4) hex = hex.split('').map((c) => c + c).join('');
    if (hex.length === 6) return { hex: `#${hex.slice(0, 6).toLowerCase()}`, alpha: 100 };
    if (hex.length === 8) {
      return {
        hex: `#${hex.slice(0, 6).toLowerCase()}`,
        alpha: Math.round((parseInt(hex.slice(6, 8), 16) / 255) * 100),
      };
    }
  }
  const rgbMatch = raw.match(/^rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})(?:\s*,\s*([\d.]+))?\s*\)$/i);
  if (rgbMatch) {
    const hex = `#${[1, 2, 3].map((i) => Math.min(255, Number(rgbMatch[i]) || 0).toString(16).padStart(2, '0')).join('')}`;
    const alpha = rgbMatch[4] == null ? 100 : Math.round(Math.max(0, Math.min(1, Number(rgbMatch[4]))) * 100);
    return { hex, alpha };
  }
  return { hex: '#ffffff', alpha: 100 };
}

function formatColorValue(hex: string, alpha: number): string {
  if (alpha >= 100) return hex;
  const rgb = hex.replace('#', '');
  const r = parseInt(rgb.slice(0, 2), 16) || 0;
  const g = parseInt(rgb.slice(2, 4), 16) || 0;
  const b = parseInt(rgb.slice(4, 6), 16) || 0;
  return `rgba(${r}, ${g}, ${b}, ${(alpha / 100).toFixed(2)})`;
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const rgb = String(hex || '#ffffff').replace('#', '');
  return {
    r: parseInt(rgb.slice(0, 2), 16) || 0,
    g: parseInt(rgb.slice(2, 4), 16) || 0,
    b: parseInt(rgb.slice(4, 6), 16) || 0,
  };
}

function rgbToHsv(r: number, g: number, b: number): { h: number; s: number; v: number } {
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const d = max - min;
  let h = 0;
  if (d !== 0) {
    if (max === rn) h = ((gn - bn) / d) % 6;
    else if (max === gn) h = (bn - rn) / d + 2;
    else h = (rn - gn) / d + 4;
    h *= 60;
    if (h < 0) h += 360;
  }
  return { h, s: max === 0 ? 0 : d / max, v: max };
}

function hsvToHex(h: number, s: number, v: number): string {
  const c = v * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = v - c;
  let r = 0;
  let g = 0;
  let b = 0;
  if (h < 60) [r, g, b] = [c, x, 0];
  else if (h < 120) [r, g, b] = [x, c, 0];
  else if (h < 180) [r, g, b] = [0, c, x];
  else if (h < 240) [r, g, b] = [0, x, c];
  else if (h < 300) [r, g, b] = [x, 0, c];
  else [r, g, b] = [c, 0, x];
  const to = (n: number) => Math.round((n + m) * 255).toString(16).padStart(2, '0');
  return `#${to(r)}${to(g)}${to(b)}`;
}

export default function AlphaColorInput({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const parsed = parseColorValue(value);
  const { r, g, b } = hexToRgb(parsed.hex);
  const hsv = rgbToHsv(r, g, b);
  const hue = Number.isFinite(hsv.h) ? hsv.h : 0;
  const alpha = parsed.alpha;
  const emit = (nextHsv: { h: number; s: number; v: number }, nextAlpha: number) => {
    onChange(formatColorValue(hsvToHex(nextHsv.h, nextHsv.s, nextHsv.v), nextAlpha));
  };
  const clamp = (n: number, min = 0, max = 100) => Math.max(min, Math.min(max, n));

  const handleSv = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.buttons !== 1) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const sPercent = clamp(((event.clientX - rect.left) / rect.width) * 100);
    const vPercent = clamp(100 - ((event.clientY - rect.top) / rect.height) * 100);
    emit({ h: hue, s: sPercent / 100, v: vPercent / 100 }, alpha);
  };

  const handleHue = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.buttons !== 1) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const nextHue = clamp(((event.clientX - rect.left) / rect.width) * 360, 0, 360);
    emit({ h: nextHue, s: hsv.s, v: hsv.v }, alpha);
  };

  const handleAlpha = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.buttons !== 1) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const nextAlpha = clamp(((event.clientX - rect.left) / rect.width) * 100);
    onChange(formatColorValue(parsed.hex, nextAlpha));
  };

  // 原版 SelectColor 的 14 色预置（view/admin/src/components/select/src/SelectColor.vue）。
  // 第 10–13 色原为 hsv/hsva/hsl/hsla，已换算为等值 hex/rgba（本组件解析器不支持 hsv/hsl）：
  // hsv(51,100,98)→#fad400；hsva(120,40,94,0.5)→rgba(144,240,144,0.5)；
  // hsl(181,100%,37%)→#00babd；hsla(209,100%,56%,0.73)→rgba(31,147,255,0.73)。
  const presetColors = [
    '#ff4500', '#ff8c00', '#ffd700', '#90ee90', '#00ced1', '#1e90ff', '#c71585',
    'rgba(255, 69, 0, 0.68)', 'rgb(255, 120, 0)', '#fad400',
    'rgba(144, 240, 144, 0.5)', '#00babd', 'rgba(31, 147, 255, 0.73)', '#c7158577',
  ];
  const solidHex = `#${[r, g, b].map((n) => n.toString(16).padStart(2, '0')).join('')}`;
  const content = (
    <div className="pchead-color-popover">
      <div
        className="pchead-color-sv"
        style={{ backgroundColor: `hsl(${hue} 100% 50%)` }}
        onPointerDown={handleSv}
        onPointerMove={(event) => event.buttons === 1 && handleSv(event)}
        onPointerUp={(event) => handleSv(event)}
      >
        <div className="pchead-color-sv__white" />
        <div className="pchead-color-sv__black" />
        <span className="pchead-color-sv__handle" style={{ left: `${hsv.s * 100}%`, top: `${(1 - hsv.v) * 100}%` }} />
      </div>

      <div
        className="pchead-color-slider pchead-color-hue"
        onPointerDown={handleHue}
        onPointerMove={(event) => event.buttons === 1 && handleHue(event)}
        onPointerUp={(event) => handleHue(event)}
      >
        <span className="pchead-color-slider__handle" style={{ left: `${(hue / 360) * 100}%`, background: `hsl(${hue} 100% 50%)` }} />
      </div>

      <div
        className="pchead-color-slider pchead-color-alpha"
        style={{
          backgroundImage: `linear-gradient(to right, transparent, ${solidHex}), linear-gradient(45deg, #e5e5e5 25%, transparent 25%), linear-gradient(-45deg, #e5e5e5 25%, transparent 25%), linear-gradient(45deg, transparent 75%, #e5e5e5 75%), linear-gradient(-45deg, transparent 75%, #e5e5e5 75%)`,
          backgroundSize: '100% 100%, 8px 8px, 8px 8px, 8px 8px, 8px 8px',
          backgroundPosition: '0 0, 0 0, 0 4px, 4px -4px, -4px 0',
        }}
        onPointerDown={handleAlpha}
        onPointerMove={(event) => event.buttons === 1 && handleAlpha(event)}
        onPointerUp={(event) => handleAlpha(event)}
      >
        <span className="pchead-color-slider__handle" style={{ left: `${alpha}%`, background: formatColorValue(parsed.hex, alpha) }} />
      </div>

      <div className="pchead-color-popover__inputs">
        <Input
          size="small"
          value={value}
          placeholder={t('#ffffff 或 rgba(...)')}
          onChange={(next) => onChange(next)}
        />
        <input
          className="pchead-alpha-color__number"
          type="number"
          min={0}
          max={100}
          step={1}
          value={alpha}
          onChange={(e) => onChange(formatColorValue(parsed.hex, Math.max(0, Math.min(100, Number(e.target.value) || 0))))}
          aria-label="Alpha"
        />
        <span className="pchead-alpha-color__unit">%</span>
      </div>

      <div className="pchead-color-popover__label">{t('预置颜色')}</div>
      <div className="pchead-color-popover__presets">
        {presetColors.map((preset) => (
          <button
            type="button"
            key={preset}
            className="pchead-color-popover__preset"
            title={preset}
            style={{ background: preset }}
            onClick={() => {
              const next = parseColorValue(preset);
              onChange(formatColorValue(next.hex, next.alpha));
            }}
          />
        ))}
      </div>
    </div>
  );

  return (
    <Popover
      trigger="custom"
      visible={open}
      onClickOutSide={() => setOpen(false)}
      position="bottomLeft"
      showArrow
      content={content}
    >
      <button
        type="button"
        className="pchead-alpha-color__trigger"
        onClick={() => setOpen((v) => !v)}
        title={value || 'transparent'}
      >
        <span className="pchead-alpha-color__checker">
          <span style={{ background: formatColorValue(parsed.hex, parsed.alpha) }} />
        </span>
      </button>
    </Popover>
  );
}
