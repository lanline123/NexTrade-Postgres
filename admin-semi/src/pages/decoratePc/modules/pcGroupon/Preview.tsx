/**
 * PC 旧版拼团模块（pcGroupon）实时预览。
 *
 * 前台渲染（权威）：pc-semi `decorate/components/modules.tsx` 的 `Groupon` 组件
 * （2415 行起），经 `contract.ts` 的 `adaptPcGroupon`（230 行起）适配。
 * 编辑器取数口径与新版 pc-home-groupon 预览一致（同族既有做法）：
 * `GET /promotion/groupon/item/list?grouponPromotionId=<活动ID>&page=1&size=20`，
 * 活动来自 `data.grouponActivityItem`；DOM/类名与新版预览同构（卡片、倒计时条、
 * priceBox），样式 import 新版目录的 `pcGroupon.css`（单一数据源）。
 * 标题走共享 ModuleTitlePreview、购买按钮走共享 PcBuyButton 口径，均与新版预览一致。
 *
 * 与新版预览的差异（旧版 def 决定）：
 *  1. 合并基线是旧版 def 的 defaults（style 默认 2=一行四个、goodsPadding 5、
 *     boxPadding 10、buyBtnStyle 1 + buyBtnSet 1、title.moreType 1、titleText
 *     「标题内容」），不是新版的 defaults（style 4 / goodsPadding 0 等）。
 *  2. 列数按前台 Groupon 的代码口径映射：style 2→4 列、3→5 列、4→6 列、
 *     6→横向滑动（每屏 6 个）；无活动时 mock 商品个数 = 列数（新版预览按
 *     style 2→4 / 4→6 取 mock，此处推广到全部列数值）。
 *  3. groupType（普通/阶梯拼团）两端均不消费：前台由 itemList 返回数据驱动
 *     （minGroupNum 等），编辑器同样不分支——与新版预览一致。
 *  4. frame / goodsStyle / goodsRadioStyle / textWeight / goodsNameRow /
 *     goodsNamePadding / showBrief：前台 Groupon 仅把视觉字段作 CSS class 钩子
 *     （ad-goods_style__ 等，frame 则根本不读、壳层只读顶层 backgroundColor /
 *     boxPadding 等），编辑器新版预览同样不消费，本预览保持一致；
 *     textAlign / showName / showPrice / cardStyle 正常消费（同新版预览）。
 */
import { useEffect, useRef, useState } from 'react';
import { apiGet } from '../../../../api';
import { useI18n } from '../../../../i18n';
import ModuleTitlePreview from '../../previews/shared/ModuleTitlePreview';
import { PcBuyButtonVisual, resolvePcBuyBtn, usePcGlobalBuyStyle, type PcResolvedBuyBtn } from '../../previews/shared/PcBuyButton';
import '../pc-home-groupon/pcGroupon.css';

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

/** 旧版 pcGroupon def 的 defaults（逐字段对应 modules/pcGroupon/index.tsx）。 */
const GROUPON_DEFAULTS = {
  style: 2,
  goodsStyle: 1,
  goodsRadioStyle: 1,
  textAlign: 1,
  textWeight: 1,
  goodsNameRow: 2,
  goodsNamePadding: 1,
  showName: 1,
  showBrief: 1,
  showPrice: 1,
  goodsPadding: 5,
  buyBtnStyle: 1,
  buyBtnSet: 1,
  buyBtnIcon: { picUrl: '', picThumb: '' },
  backgroundColor: '',
  boxRadius: 0,
  innerPadding: 0,
  boxPadding: 10,
  boxPaddingTop: 5,
  boxPaddingBottom: 5,
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
  title: {
    showTitle: 0,
    titleStyle: 1,
    titleAlign: 1,
    titleBackground: '',
    titleBackground2: '',
    titleBackgroundPic: { picUrl: '', picThumb: '' },
    titleRadius: 0,
    titleText: '标题内容',
    titleColor: '',
    titleWeight: 'normal',
    descText: '',
    descColor: '#aaaaaa',
    showMore: 0,
    moreType: 1,
    countdownLabel: '',
    moreLink: [] as unknown as Record<string, any>,
    moreColor: '#000000',
    format: { titleBackground: '', titleRadius: '' },
  },
  groupType: 1,
  grouponActivityItem: null as Record<string, any> | null,
  cardStyle: 1,
};

type GrouponConfig = typeof GROUPON_DEFAULTS;

/** 深合并默认值：对象递归、数组覆盖（同新版预览）。 */
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

/** 拼团商品（promotion/groupon/item/list）；无活动 ID 时不请求（同新版预览）。 */
function useGrouponItems(promotionId?: number) {
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

/** 无活动时的 mock 商品（与新版预览的 mockProducts 逐字段一致）。 */
const GROUPON_MOCK_PRODUCTS: PreviewProduct[] = [
  { productId: 1, productName: '拼团商品示例一号', picThumb: 'https://dummyimage.com/200x200/f5f5f5/999&text=1', minGroupPrice: '99.00', productPrice: '199.00', minGroupNum: '2' },
  { productId: 2, productName: '拼团商品示例二号', picThumb: 'https://dummyimage.com/200x200/f5f5f5/999&text=2', minGroupPrice: '128.00', productPrice: '256.00', minGroupNum: '3' },
  { productId: 3, productName: '拼团商品示例三号', picThumb: 'https://dummyimage.com/200x200/f5f5f5/999&text=3', minGroupPrice: '68.00', productPrice: '168.00', minGroupNum: '2' },
  { productId: 4, productName: '拼团商品示例四号', picThumb: 'https://dummyimage.com/200x200/f5f5f5/999&text=4', minGroupPrice: '188.00', productPrice: '388.00', minGroupNum: '5' },
  { productId: 5, productName: '拼团商品示例五号', picThumb: 'https://dummyimage.com/200x200/f5f5f5/999&text=5', minGroupPrice: '58.00', productPrice: '118.00', minGroupNum: '2' },
  { productId: 6, productName: '拼团商品示例六号', picThumb: 'https://dummyimage.com/200x200/f5f5f5/999&text=6', minGroupPrice: '88.00', productPrice: '168.00', minGroupNum: '3' },
];

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

/** 拼团商品卡（与新版预览 GrouponCard 同构：cardStyle 1 图片倒计时条 + priceBox；其余简版价格行）。 */
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

export default function PcGrouponLegacyPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const config = deepMerge<GrouponConfig>(GROUPON_DEFAULTS as GrouponConfig, data);
  const activity = (config.grouponActivityItem || null) as Record<string, any> | null;
  const activityId = Number(activity?.grouponPromotionId ?? activity?.grouponId) || undefined;
  const fetched = useGrouponItems(activityId);
  const now = useNowTick();
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const style = Number(config.style ?? 2);
  const isScrollLayout = style === 6;
  // 列数按前台 Groupon 代码口径：2→4 列、3→5 列、4→6 列、6→横滑 6 个，其余回退 4 列
  const columns = style === 2 ? 4 : style === 3 ? 5 : style === 4 ? 6 : 4;
  const cardStyle = Number(config.cardStyle ?? 1);
  const textAlign = Number(config.textAlign ?? 1);
  /* 购买按钮（全站统一共享组件）：旧版 def 自带 buyBtnSet=1，显式样式直接生效；
     存量无标记时 0/缺省按 4 解析，与前台同口径 */
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

  // 无活动时按列数取 mock（原版 displayProducts 的推广口径，见文件头第 2 点）
  const baseRows: PreviewProduct[] = fetched.length
    ? fetched
    : GROUPON_MOCK_PRODUCTS.slice(0, isScrollLayout ? 6 : columns);
  const rows = baseRows.map((r) => ({ ...r, startTime: activity?.startTime || '', endTime: activity?.endTime || '' }));

  // 卡片倒计时文案（进行中→距结束，未开始→距开始，已结束→空），同新版预览
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

  // 列宽（原版 itemWidth：gap 参与宽度补偿，同新版预览）
  const gapComp = (perRow: number) =>
    goodsPadding > 0 ? `calc(${100 / perRow}% - ${(goodsPadding * (perRow - 1)) / perRow}px)` : `${100 / perRow}%`;
  const itemWidth = gapComp(columns);

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
        {/* 标题走共享 ModuleTitlePreview（与前台 DecModuleTitle 同源）；
            旧版 title 缺省 showTitle=0，此时不渲染，与前台一致 */}
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
