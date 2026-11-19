'use client';

import { splitCurrencyPrice, splitDisplayPrice } from '@/lib/currency';
import { PRICE_GATE_COPY, usePriceAccess } from '@/lib/price-gate';

/**
 * 价格展示（对照旧版 `view/pc/components/format/Price.vue`）
 *
 * 旧版 DOM：`div.price.align-bottom > div.util(币种符号) + div.num(整数) + div.num.decimal(小数)`
 * 三段各带自己的字号，价格样式在 member.less 的 `.price` 系列里定义（baseline 对齐）。
 * 这里只搬结构，不引入任何新样式。
 *
 * 价格门控（2026-11-20 对齐原版）：原版共享价格组件在 isIdentity 开启
 * 且访客未认证时，把整个价格位换成文案（未登录「登录查看」/ 已登录
 * 「实名查看」）。我方后端在无看价权时把价格置空（null），故本组件在
 * 「值缺失（null/undefined/''）且当前访客无看价权」时渲染同一文案，
 * 覆盖装修秒杀/团购/契约模块等直接透传价格的调用点；有真实价（含 0）
 * 或访客有看价权时渲染与此前完全一致。文案口径见 lib/price-gate.ts。
 */
export interface FormatPriceProps {
  /** 金额，字符串或数字；门控置空时为 null */
  value: number | string | null | undefined;
  /** 覆盖币种符号（默认跟随当前换币设置） */
  currency?: string;
  /** 划线原价等展示场景：按后台「价格显示设置」缩写（≥1万显示 XX万 等），金额段带单位后缀 */
  displayPrice?: boolean;
  /** 整数段字号：旧版用 `:fontStyle="{ fontSize: '24px', fontWeight: '400' }"` 传进来 */
  fontSize?: string;
  fontWeight?: number | string;
  className?: string;
}

export default function FormatPrice({ value, currency, fontSize, fontWeight, className, displayPrice }: FormatPriceProps) {
  const priceAccess = usePriceAccess();
  const fontStyle = fontSize || fontWeight ? { fontSize, fontWeight } : undefined;
  const rootStyle = fontStyle || undefined;

  /* 门控：值被后端置空且访客无看价权 → 价格位渲染门控文案（原版口径）。 */
  const missing = value === undefined || value === null || value === '';
  if (missing && priceAccess !== 'visible') {
    return (
      <div className={`price align-bottom${className ? ` ${className}` : ''}`} style={rootStyle}>
        <div className="text">{PRICE_GATE_COPY[priceAccess].text}</div>
      </div>
    );
  }

  const parts = displayPrice ? splitDisplayPrice(value) : splitCurrencyPrice(value);
  const suffix = displayPrice ? (parts as { suffix?: string }).suffix || '' : '';
  const symbol = currency || parts.symbol;
  const decimals = parts.decimals === '.00' ? '' : parts.decimals;

  return (
    <div className={`price align-bottom${className ? ` ${className}` : ''}`} style={rootStyle}>
      <div className="util align-bottom">{symbol}</div>
      <div className="num">{parts.integer}</div>
      {decimals ? <div className="num decimal">{decimals}</div> : null}
      {suffix ? <div className="num suffix">{suffix}</div> : null}
    </div>
  );
}
