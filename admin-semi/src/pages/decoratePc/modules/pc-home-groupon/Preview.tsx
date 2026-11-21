/**
 * PC 首页拼团模块预览（迁移自旧版 `pcHome/pc-home-groupon/Preview.vue` 及 `_shared/groupon`）。
 *
 * 取数：`/promotion/groupon/item/list`（活动 ID 来自 `data.grouponActivityItem`）；
 * 未选活动或接口列表为空时以在售商品池随机真实商品补足展示（共享
 * demoProductFill 的 usePreviewDemoProducts，纯显示层不落盘；补足口径与
 * H5 拼团 Preview 定案一致——仅列表为空时补、非空不混，池空/取数失败时
 * 不补、列表为空；补足条目以真实商品记录映射拼团展示字段：主价取该商品
 * 真实拼团价字段，无则取其真实售价，划线价即其真实售价，不编造商品与价格）。
 * 倒计时由 React effect（1s 心跳）驱动，标题右侧与商品卡内同时走秒。
 *
 * props 统一为 `{ data }`，渲染前与旧版 `pcHomeGrouponDefaults` 深合并。
 */
import { useEffect, useRef, useState } from 'react';
import { apiGet } from '../../../../api';
import { usePreviewDemoProducts } from '../../../../components/demoProductFill';
import ModuleTitlePreview from '../../previews/shared/ModuleTitlePreview';
import { PcBuyButtonVisual, resolvePcBuyBtn, usePcGlobalBuyStyle, type PcResolvedBuyBtn } from '../../previews/shared/PcBuyButton';
import './pcGroupon.css';

import { useI18n } from '../../../../i18n';
interface PreviewProduct {
  productId?: number | string;
  productName?: string;
  picUrl?: string;
  picThumb?: string;
  picLarge?: string;
  productPrice?: number | string;
  marketPrice?: number | string;
  minGroupPrice?: number | string;
  minGroupNum?: number | string;
  startTime?: string;
  endTime?: string;
}

const GROUPON_DEFAULTS = {
  style: 4,
  groupType: 1,
  cardStyle: 1,
  showName: 1,
  showPrice: 1,
  textAlign: 1,
  goodsPadding: 0,
  backgroundColor: '#ffffff',
  boxRadius: 0,
  innerPadding: 0,
  boxPadding: 0,
  boxPaddingTop: 5,
  boxPaddingBottom: 5,
  title: {
    showTitle: 0,
    titleStyle: 1,
    titleAlign: 1,
    titleBackground: '',
    titleBackground2: '',
    titleBackgroundPic: { picUrl: '', picThumb: '' },
    titleRadius: 0,
    titlePaddingTop: 20,
    titlePaddingBottom: 20,
    titleText: '拼团',
    titleColor: '#2a3145',
    descText: '',
    descColor: '#aaaaaa',
    showMore: 0,
    moreType: 2,
    countdownLabel: '距结束',
    moreLink: { link: '', title: '' },
    moreColor: '#1a1a1a',
  },
  // 购买按钮默认值与新版编辑器一致：4（加号实心，显式配置）
  buyBtnStyle: 4,
  buyBtnSet: 1,
  buyBtnIcon: { picUrl: '', picThumb: '' },
  grouponActivityItem: null as Record<string, any> | null,
};

type GrouponConfig = typeof GROUPON_DEFAULTS;

/** 深合并默认值：对象递归、数组覆盖。 */
function deepMerge<T extends Record<string, any>>(defaults: T, data?: Record<string, any>): T {
  const out: Record<string, any> = { ...defaults };
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

/** 拼团商品（promotion/groupon/item/list）；无活动 ID 时不请求。 */
function useGrouponItems(promotionId?: number) {
    const { t } = useI18n();
  const [rows, setRows] = useState<PreviewProduct[]>([]);
  useEffect(() => {
    if (!promotionId) {
      setRows([]);
      return;
    }
    let alive = true;
    apiGet<{ records?: PreviewProduct[]; list?: PreviewProduct[] }>('/promotion/groupon/item/list', {
      grouponPromotionId: promotionId,
      page: 1,
      size: 20,
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
  }, [promotionId]);
  return rows;
}

/** 1s 心跳，驱动倒计时实时走秒。 */
function useNowTick(intervalMs = 1000) {
    const { t } = useI18n();
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

function parseCountdownParts(remainingMs: number) {
  return {
    days: Math.floor(remainingMs / (1000 * 60 * 60 * 24)),
    hours: Math.floor((remainingMs % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60)),
    minutes: Math.floor((remainingMs % (1000 * 60 * 60)) / (1000 * 60)),
    seconds: Math.floor((remainingMs % (1000 * 60)) / 1000),
  };
}

const formatNumTwo = (n: number) => String(n).padStart(2, '0');

const EMPTY_IMG =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='100' height='100'%3E%3Crect fill='%23f5f5f5' width='100' height='100'/%3E%3C/svg%3E";

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
    <div className={`pc-groupon__switch pc-groupon__switch--${dir}`} style={{ left, right }} onClick={onClick}>
      <span />
    </div>
  );
}

/** 拼团商品卡（对齐旧版 GrouponItemCard：cardStyle 1 图片倒计时条 + priceBox；其余简版价格行）。 */
function GrouponCard({
  item,
  cardStyle,
  textAlign,
  buyBtn,
  showName,
  showPrice,
  countdownText,
  countdownLabel,
}: {
  item: PreviewProduct;
  cardStyle: number;
  textAlign: number;
  buyBtn: PcResolvedBuyBtn;
  showName: boolean;
  showPrice: boolean;
  countdownText: string;
  countdownLabel: string;
}) {
    const { t } = useI18n();
  const isCardStyle1 = cardStyle === 1;
  const groupNum = String(item.minGroupNum || '2');
  const groupPrice = String(item.minGroupPrice ?? item.productPrice ?? '0');
  const originPrice = String(item.productPrice ?? '0');

  return (
    <div className="pc-groupon-card">
      <div className="pc-groupon-card__image-wrap">
        <div className="pc-groupon-card__image" style={{ width: isCardStyle1 ? '90%' : '100%' }}>
          <img src={((item.picThumb || item.picUrl) as string) || EMPTY_IMG} alt={item.productName || ''} />
        </div>
      </div>
      {isCardStyle1 && (
        <div className="pc-groupon-card__countdown">
          <div className="pc-groupon-card__countdown-title">{t('拼团')}</div>
          <div className="pc-groupon-card__countdown-time">
            <div className="pc-groupon-card__countdown-label">{countdownLabel}</div>
            {countdownText ? <div className="pc-groupon-card__countdown-text">{countdownText}</div> : null}
          </div>
        </div>
      )}
      {showName && (
        <div
          className="pc-groupon-card__name"
          style={{
            marginTop: 10,
            marginBottom: isCardStyle1 ? 0 : 12,
            textAlign: textAlign === 1 ? 'left' : 'center',
            justifyContent: textAlign === 1 ? 'flex-start' : 'center',
          }}
        >
          <span className="pc-groupon-card__tag">{groupNum}人团</span>
          <span className="pc-groupon-card__name-text">{item.productName}</span>
        </div>
      )}
      {showPrice &&
        (isCardStyle1 ? (
          <div className="pc-groupon-card__price-box" style={{ marginTop: 18 }}>
            <div className="pc-groupon-card__price-box-main">
              <span className="pc-groupon-card__price-current">¥{groupPrice}</span>
              <div className="pc-groupon-card__price-origin">¥{originPrice}</div>
            </div>
            <div className="pc-groupon-card__pin">{t('拼')}</div>
          </div>
        ) : (
          <div className="pc-groupon-card__price-row" style={{ flexDirection: textAlign === 1 ? 'row' : 'column' }}>
            <div className="pc-groupon-card__price-row-main">
              <span className="pc-groupon-card__price-main">¥{groupPrice}</span>
              <span className="pc-groupon-card__price-line">¥{originPrice}</span>
            </div>
            <div className="pc-groupon-card__buy">
              {buyBtn.show ? (
                <span
                  className={`pc-groupon-card__buy-btn${buyBtn.style >= 5 ? ' pc-groupon-card__buy-btn--bare' : ''}`}
                  style={buyBtn.iconColor ? { color: buyBtn.iconColor } : undefined}
                >
                  <PcBuyButtonVisual style={buyBtn.style} customIcon={buyBtn.customIcon} />
                </span>
              ) : null}
            </div>
          </div>
        ))}
    </div>
  );
}

export default function PcGrouponPreview({ data }: { data: Record<string, any> }) {
    const { t } = useI18n();
  const config = deepMerge<GrouponConfig>(GROUPON_DEFAULTS as GrouponConfig, data);
  const activity = (config.grouponActivityItem || null) as Record<string, any> | null;
  const activityId = Number(activity?.grouponPromotionId ?? activity?.grouponId) || undefined;
  const fetched = useGrouponItems(activityId);
  const now = useNowTick();
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const style = Number(config.style ?? 4);
  const isScrollLayout = style === 6;
  const cardStyle = Number(config.cardStyle ?? 1);
  const textAlign = Number(config.textAlign ?? 1);
  /* 购买按钮（全站统一共享组件）：存量未配置（无 buyBtnSet）时 0/缺省按 4 解析，与前台同口径 */
  const globalBuy = usePcGlobalBuyStyle();
  const buyBtn = resolvePcBuyBtn(config.buyBtnStyle, {
    moduleIcon: config.buyBtnIcon,
    global: globalBuy,
    zeroAs: config.buyBtnSet ? undefined : 4,
  });
  const showName = Number(config.showName ?? 1) === 1;
  const showPrice = Number(config.showPrice ?? 1) === 1;
  const goodsPadding = Number(config.goodsPadding) || 0;

  const activityEndTs = dateStringToTimestamp(activity?.endTime || '');

  // 无活动/无数据时按 style 取在售池真实商品补足（条数沿原版 displayProducts
  // 口径：2→4 个，4→6 个，其余 6 个；仅列表为空时补、非空不混，映射口径见文件头）
  const fallback = usePreviewDemoProducts(fetched.length === 0, style === 2 ? 4 : 6);
  const baseRows: PreviewProduct[] = fetched.length
    ? fetched
    : fallback.map((p) => ({
        ...p,
        picLarge: p.picLarge || p.picUrl || p.picThumb || '',
        minGroupPrice: p.minGroupPrice ?? p.minGrouponPrice ?? p.grouponPrice ?? p.productPrice,
      }));
  const rows = baseRows.map((r) => ({ ...r, startTime: activity?.startTime || '', endTime: activity?.endTime || '' }));

  // 卡片倒计时文案（进行中→距结束，未开始→距开始，已结束→空）
  const cardCountdown = (item: PreviewProduct): { label: string; text: string } => {
    const startTs = dateStringToTimestamp(item.startTime || '');
    const endTs = dateStringToTimestamp(item.endTime || '');
    const started = !startTs || startTs < now;
    const ended = !!endTs && endTs < now;
    if (ended) return { label: '已结束', text: '' };
    const target = !started ? startTs : endTs;
    const remaining = target ? Math.max(0, target - now) : 0;
    if (!remaining) return { label: started ? t('距结束') : '距开始', text: '' };
    const { days, hours, minutes, seconds } = parseCountdownParts(remaining);
    return {
      label: started ? t('距结束') : '距开始',
      text:
        days > 0
          ? `${days}天${formatNumTwo(hours)}:${formatNumTwo(minutes)}:${formatNumTwo(seconds)}`
          : `${formatNumTwo(hours)}:${formatNumTwo(minutes)}:${formatNumTwo(seconds)}`,
    };
  };

  // 列宽（原版 itemWidth/scrollItemWidth：gap 参与宽度补偿）
  const gapComp = (perRow: number) =>
    goodsPadding > 0 ? `calc(${100 / perRow}% - ${(goodsPadding * (perRow - 1)) / perRow}px)` : `${100 / perRow}%`;
  const itemWidth = style === 2 ? gapComp(4) : style === 4 ? gapComp(6) : gapComp(5);

  return (
    <div
      className="pc-groupon"
      style={{
        width: `calc(100% - ${(Number(config.boxPadding) || 0) * 2}px)`,
        backgroundColor: config.backgroundColor || '#ffffff',
        borderRadius: Number(config.boxRadius) || 0,
        marginLeft: Number(config.boxPadding) || 0,
        marginRight: Number(config.boxPadding) || 0,
        marginTop: Number(config.boxPaddingTop ?? 5),
        marginBottom: Number(config.boxPaddingBottom ?? 5),
        padding: `0 ${Number(config.innerPadding) || 0}px`,
      }}
    >
      <div className="pc-groupon__inner">
        {/* 标题走共享 ModuleTitlePreview（与 PC 端 DecModuleTitle 同源；根节点已带
            innerPadding 内容边距，与 PC 的 shell 结构一致）。此前是本文件自维护的
            GrouponTitleBar（默认值与共享口径多处漂移）。 */}
        <ModuleTitlePreview
          title={config.title}
          fallbackText={t('拼团')}
          countdownTargetTs={activityEndTs}
          now={now}
          countdownLabelFallback={t('距结束')}
        />
        <div className="pc-groupon__list-wrap">
          {isScrollLayout ? (
            <div style={{ position: 'relative', width: '100%', overflow: 'hidden', boxSizing: 'border-box' }}>
              <div className="pc-groupon__scroll" ref={scrollRef} style={{ gap: goodsPadding, padding: goodsPadding }}>
                {rows.map((item, index) => (
                  <div key={index} className="pc-groupon__item" style={{ flex: '0 0 auto', width: gapComp(6) }}>
                    <GrouponCard
                      item={item}
                      cardStyle={cardStyle}
                      textAlign={textAlign}
                      buyBtn={buyBtn}
                      showName={showName}
                      showPrice={showPrice}
                      countdownText={cardCountdown(item).text}
                      countdownLabel={cardCountdown(item).label}
                    />
                  </div>
                ))}
              </div>
              <SwitchArrow
                dir="prev"
                left={0}
                onClick={() => {
                  const el = scrollRef.current;
                  if (!el) return;
                  const first = el.querySelector<HTMLElement>('.pc-groupon__item');
                  const step = Math.max(220, Math.floor((first?.getBoundingClientRect().width || 0) + goodsPadding));
                  el.scrollBy({ left: -step, behavior: 'smooth' });
                }}
              />
              <SwitchArrow
                dir="next"
                right={0}
                onClick={() => {
                  const el = scrollRef.current;
                  if (!el) return;
                  const first = el.querySelector<HTMLElement>('.pc-groupon__item');
                  const step = Math.max(220, Math.floor((first?.getBoundingClientRect().width || 0) + goodsPadding));
                  el.scrollBy({ left: step, behavior: 'smooth' });
                }}
              />
            </div>
          ) : (
            <div className="pc-groupon__grid" style={{ padding: goodsPadding }}>
              <div className="pc-groupon__grid-inner" style={{ gap: goodsPadding }}>
                {rows.map((item, index) => (
                  <div key={index} className="pc-groupon__item" style={{ width: itemWidth }}>
                    <GrouponCard
                      item={item}
                      cardStyle={cardStyle}
                      textAlign={textAlign}
                      buyBtn={buyBtn}
                      showName={showName}
                      showPrice={showPrice}
                      countdownText={cardCountdown(item).text}
                      countdownLabel={cardCountdown(item).label}
                    />
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
