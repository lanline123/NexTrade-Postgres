/**
 * PC 旧版秒杀模块（pcSeckill）实时预览。
 *
 * 前台渲染（权威）：pc-semi `decorate/components/modules.tsx` 的 `Seckill` 组件
 * （2230 行起），经 `contract.ts` 的 `adaptPcSeckill`（193 行起）适配。
 * 编辑器取数口径与新版 pc-home-seckill 预览一致（同族既有做法）：
 * `GET /promotion/seckill/listForHomePreview?page=1&size=15`，倒计时取列表
 * 第一条活动的结束时间；DOM/类名与新版预览同构，样式 import 新版目录的
 * `pcSeckill.css`（单一数据源）。
 *
 * 与新版预览的差异（旧版数据结构决定）：
 *  1. 旧版 def 只有 `{ seckillTitle }` 一个字段。前台 adapt 的标题口径是
 *     `toTitle(m.seckillTitle ?? m.title)`：seckillTitle 存在时整组标题只取它
 *     （showTitle=1、titleText=seckillTitle，其余视觉字段走缺省），不存在时才用
 *     title 对象。本预览据此构造合并基线，不能直接套新版预览的固定默认标题，
 *     否则用户改的活动标题永远显示不出来。编辑器载入期的 normalizeLegacyTitle
 *     只认新版 type（pc-home-seckill），不覆盖旧版，故该归一在本文件内完成。
 *  2. 购买按钮按前台 Seckill 的存量口径：buyBtnStyle 缺省且无 buyBtnSet 标记时
 *     按 4（加号实心）解析（旧版编辑器从未暴露该设置，前台一直显示加购钮）；
 *     有 buyBtnSet 后 0 才表示隐藏。
 *  3. showName / showPrice 按前台消费（!== 0 && !== false 才显示）；商品间距
 *     按前台回退链 goodsGap ?? goodsPadding ?? 10。
 *  4. style / goodsStyle / goodsRadioStyle / goodsNameRow / textAlign /
 *     textWeight / goodsNamePadding 等卡片视觉字段：前台仅作 CSS class 钩子
 *     （ad-goods_style__ 等）与 style 显式给出时的列数切换；编辑器同族新版预览
 *     同样不消费，本预览保持一致（横向滚动的 6 卡布局即前台无 style 时的
 *     「预览布局」，前台代码已按此对齐）。
 */
import { useEffect, useRef, useState } from 'react';
import { usePreviewDemoProducts, demoEndTimeString } from '../../../../components/demoProductFill';
import { apiGet } from '../../../../api';
import { useI18n } from '../../../../i18n';
import ModuleTitlePreview from '../../previews/shared/ModuleTitlePreview';
import { PcBuyButtonVisual, resolvePcBuyBtn, usePcGlobalBuyStyle } from '../../previews/shared/PcBuyButton';
import '../pc-home-seckill/pcSeckill.css';

interface PreviewProduct {
  productId?: number;
  productName?: string;
  picUrl?: string;
  picThumb?: string;
  picLarge?: string;
  productPrice?: number | string;
  marketPrice?: number | string;
  seckillPrice?: number | string;
  seckkillData?: { seckillEndTime?: string };
}

const SECKILL_DEFAULTS = {
  frame: {
    boxPadding: 0,
    boxPaddingTop: 0,
    boxPaddingBottom: 0,
    boxRadius: 0,
    backgroundColor: '#fafafa',
    leftAndRightPadding: 0,
  },
  seckillTitle: '全场优惠限时9折起',
  title: {
    showTitle: 1,
    titleStyle: 1,
    titleAlign: 1,
    titleBackground: '',
    titleBackground2: '',
    titleBackgroundPic: { picUrl: '', picThumb: '' },
    titleRadius: 0,
    titlePaddingTop: 20,
    titlePaddingBottom: 20,
    titleText: '全场优惠限时9折起',
    titleColor: '#1a1a1a',
    descText: '',
    descColor: '#aaaaaa',
    showMore: 0,
    moreType: 2,
    countdownLabel: '倒计时',
    moreLink: { link: '', title: '' },
    moreColor: '#1a1a1a',
  },
  goodsGap: 10,
};

type SeckillConfig = typeof SECKILL_DEFAULTS;

/** 深合并默认值：对象递归、数组覆盖（同新版预览）。 */
function deepMerge<T extends Record<string, any>>(defaults: T, data?: Record<string, any>): T {
  const out: Record<string, any> = Array.isArray(defaults) ? [...(defaults as any)] : { ...defaults };
  Object.entries(data || {}).forEach(([key, value]) => {
    const base = out[key];
    if (value && typeof value === 'object' && !Array.isArray(value) && base && typeof base === 'object' && !Array.isArray(base)) {
      out[key] = deepMerge(base as Record<string, any>, value as Record<string, any>);
    } else if (value !== undefined) {
      out[key] = value;
    }
  });
  return out as T;
}

/** 秒杀预览数据（原版 listForHomePreview，口径同新版预览）。 */
function useSeckillPreview() {
  const [rows, setRows] = useState<PreviewProduct[]>([]);
  useEffect(() => {
    let alive = true;
    apiGet<{ records?: PreviewProduct[]; list?: PreviewProduct[] }>('/promotion/seckill/listForHomePreview', {
      page: 1,
      size: 15,
    })
      .then((res) => {
        if (alive) setRows(res.records || res.list || []);
      })
      .catch(() => {
        if (alive) setRows([]);
      });
    return () => {
      alive = false;
    };
  }, []);
  return rows;
}

/** 1s 心跳，驱动倒计时实时走秒。 */
function useNowTick(intervalMs = 1000) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), intervalMs);
    return () => window.clearInterval(timer);
  }, [intervalMs]);
  return now;
}

/** "YYYY-MM-DD HH:mm:ss" → 时间戳（无效返回 0）。 */
function dateStringToTimestamp(dateString?: string | null): number {
  if (!dateString) return 0;
  const parts = String(dateString).split(' ');
  const datePart = parts[0] || '';
  const timePart = parts[1] || '';
  const [year, month, day] = datePart.split('-');
  const [hour, minute, second] = timePart ? timePart.split(':') : ['0', '0', '0'];
  return new Date(Number(year), Number(month) - 1, Number(day), Number(hour), Number(minute), Number(second)).getTime();
}

/** 三段式价格（int/dec）。 */
function priceParts(price: string | number): { int: string; dec: string } {
  const num = Number(price);
  const text = Number.isFinite(num) ? num.toFixed(2) : String(price ?? '0.00');
  const [int, dec] = text.split('.');
  return { int: int || '0', dec: dec || '00' };
}

function SwitchArrow({
  dir,
  left,
  right,
  onClick,
}: {
  dir: 'prev' | 'next';
  left?: number;
  right?: number;
  onClick?: () => void;
}) {
  return (
    <div className={`pc-seckill__switch pc-seckill__switch--${dir}`} style={{ left, right }} onClick={onClick}>
      <span />
    </div>
  );
}

const EMPTY_IMG =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='180' height='180'%3E%3Crect fill='%23f0f0f0' width='180' height='180'/%3E%3C/svg%3E";

export default function PcSeckillLegacyPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  // 标题基线按前台 adapt 口径构造（见文件头第 1 点）：seckillTitle 存在时整组标题只取它，
  // 并屏蔽 data.title 参与合并；否则 title 对象正常深合并。
  const seckillTitleText = typeof data?.seckillTitle === 'string' ? data.seckillTitle : undefined;
  const defaults: SeckillConfig = {
    ...SECKILL_DEFAULTS,
    seckillTitle: seckillTitleText ?? SECKILL_DEFAULTS.seckillTitle,
    title:
      seckillTitleText !== undefined
        ? { ...SECKILL_DEFAULTS.title, showTitle: 1, titleText: seckillTitleText }
        : SECKILL_DEFAULTS.title,
  };
  const mergeSource = seckillTitleText !== undefined ? { ...data, title: undefined } : data;
  const config = deepMerge<SeckillConfig>(defaults, mergeSource);
  const raw = config as Record<string, any>;

  /* 购买按钮（全站统一共享组件）：按前台 Seckill 存量口径解析（见文件头第 2 点） */
  const globalBuy = usePcGlobalBuyStyle();
  const buyBtn = resolvePcBuyBtn(raw.buyBtnStyle === undefined && !raw.buyBtnSet ? 4 : raw.buyBtnStyle, {
    moduleIcon: raw.buyBtnIcon,
    global: globalBuy,
    zeroAs: raw.buyBtnSet ? undefined : 4,
  });
  const frame = (config.frame || {}) as Record<string, any>;
  const title = (config.title || {}) as Record<string, any>;
  const rows = useSeckillPreview();
  // 无进行中活动时随机真实商品兜底展示（共享 demoProductFill，纯显示层不落盘；
  // 四期与 PC 拼团兜底同口径补齐；个数 6 = 本模块骨架屏既有槽位数；倒计时
  // 结束时刻用展示用动态值 demoEndTimeString 承接）
  const demoFallback = usePreviewDemoProducts(rows.length === 0, 6);
  const displayRows = rows.length
    ? rows
    : demoFallback.map((p) => ({ ...p, seckkillData: { seckillEndTime: demoEndTimeString() } }));
  const now = useNowTick();
  const listRef = useRef<HTMLDivElement | null>(null);
  const goodsGap = Number(raw.goodsGap ?? raw.goodsPadding) || 10;
  const scrollByStep = (dir: 1 | -1) => {
    const el = listRef.current;
    if (!el) return;
    const first = el.querySelector<HTMLElement>('.pc-seckill__item');
    const step = Math.max(220, Math.floor((first?.getBoundingClientRect().width || 0) + goodsGap));
    el.scrollBy({ left: dir * step, behavior: 'smooth' });
  };
  const innerPadding = Number(frame.leftAndRightPadding) || 0;
  const showName = raw.showName !== 0 && raw.showName !== false;
  const showPrice = raw.showPrice !== 0 && raw.showPrice !== false;
  const first = displayRows[0];
  const countdownTargetTs = dateStringToTimestamp(first?.seckkillData?.seckillEndTime || '');

  return (
    <div
      className="pc-seckill"
      style={{
        paddingLeft: Number(frame.boxPadding) || 0,
        paddingRight: Number(frame.boxPadding) || 0,
        paddingBottom: 20,
        marginTop: Number(frame.boxPaddingTop) || 0,
        marginBottom: Number(frame.boxPaddingBottom) || 0,
        borderRadius: Number(frame.boxRadius) || 0,
        backgroundColor: frame.backgroundColor || '#fafafa',
        ['--pc-seckill-gap' as any]: `${goodsGap}px`,
      }}
    >
      {/* 标题走共享 ModuleTitlePreview（与前台 DecModuleTitle 同源）；倒计时目标取首条活动结束时间 */}
      <div style={{ padding: `0 ${innerPadding}px` }}>
        <ModuleTitlePreview
          title={title}
          fallbackText={config.seckillTitle || t('限时秒杀')}
          countdownTargetTs={countdownTargetTs}
          now={now}
          countdownLabelFallback={t('倒计时')}
        />
      </div>

      <div className="pc-seckill__list-wrap">
        {displayRows.length > 0 ? (
          <div className="pc-seckill__list" ref={listRef} style={{ padding: `0 ${innerPadding}px` }}>
            {displayRows.slice(0, 15).map((product, index) => {
              const current = Number(product.seckillPrice ?? product.productPrice ?? 0);
              const original = Number(product.productPrice ?? product.marketPrice ?? 0);
              const discount = original > 0 && current < original ? Math.floor(((original - current) / original) * 100 * 10) / 10 : 0;
              const parts = priceParts(current);
              return (
                <div key={product.productId ?? index} className="pc-seckill__item">
                  <div className="pc-seckill__img">
                    <img src={(product.picThumb || product.picUrl) as string || EMPTY_IMG} alt={product.productName || ''} />
                  </div>
                  {discount > 0 && <div className="pc-seckill__discount">优惠{discount}%</div>}
                  {showName ? <div className="pc-seckill__name">{product.productName}</div> : null}
                  {showPrice ? (
                    <>
                      <div className="pc-seckill__price-row">
                        <div className="pc-seckill__price">
                          <span className="pc-seckill__price-currency">¥</span>
                          <span className="pc-seckill__price-int">{parts.int}</span>
                          <span className="pc-seckill__price-dec">.{parts.dec}</span>
                        </div>
                        {buyBtn.show ? (
                          <div className="pc-seckill__buy">
                            <span
                              className={`pc-seckill__add-btn${buyBtn.style >= 5 ? ' pc-seckill__add-btn--bare' : ''}`}
                              title={t('添加购物车')}
                              style={buyBtn.iconColor ? { color: buyBtn.iconColor } : undefined}
                            >
                              <PcBuyButtonVisual style={buyBtn.style} customIcon={buyBtn.customIcon} />
                            </span>
                          </div>
                        ) : null}
                      </div>
                      {original > 0 && (
                        <div className="pc-seckill__price-origin">
                          ¥{Number.isFinite(original) ? original.toFixed(2) : '0.00'}
                        </div>
                      )}
                    </>
                  ) : null}
                </div>
              );
            })}
          </div>
        ) : (
          <div className="pc-seckill__list pc-seckill__list--skeleton" style={{ padding: `0 ${innerPadding}px` }}>
            {[1, 2, 3, 4, 5, 6].map((i) => (
              <div key={i} className="pc-seckill__item">
                <div className="pc-seckill__skeleton-img" />
                <div className="pc-seckill__skeleton-line" />
              </div>
            ))}
          </div>
        )}
        {displayRows.length > 0 && (
          <>
            <SwitchArrow dir="prev" left={innerPadding} onClick={() => scrollByStep(-1)} />
            <SwitchArrow dir="next" right={innerPadding} onClick={() => scrollByStep(1)} />
          </>
        )}
      </div>
    </div>
  );
}
