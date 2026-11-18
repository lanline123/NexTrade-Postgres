/**
 * PC 旧版推荐商品模块（pcRecommendProduct）实时预览。
 *
 * 前台渲染（权威）：pc-semi `decorate/components/modules.tsx` 的
 * `ContractRecommendProduct` 组件（4778 行起），经 `contract.ts` 的
 * `adaptPcRecommendProduct`（355 行起）适配——旧版首页「猜你喜欢」只有
 * 这一套 `.mod_you_like` 结构（与新版 pc-recommend-product 的卡片列表
 * RecommendProduct 是两个组件、两套 DOM，不可复用新版预览）。
 * DOM/类名逐层照前台：mod_you_like > h3.com-pc-title + .you_like_list
 * > ul > li > .item（.flip_wrap[.pro_pic/.pro_name/.pro_price] +
 * .operate_area），样式在本目录 pcRecommendProduct.css（移植前台
 * decorate-source.scss 中 `.pc-home .mod_you_like` 段，去掉 .pc-home 前缀）。
 *
 * 取数口径与新版 pc-recommend-product 预览一致（编辑器同族既有做法）：
 * `GET /product/product/list`，参数键映射 ids（逗号串）/ categoryId /
 * introType / size，并带 productStatus=1 & checkStatus=1。选品优先级：
 *  1. productIds 非空 → ids 拉取并按配置顺序回排（前台 selectType=1 口径，
 *     旧版存量数据常有 ids 却无 productSelectType，故 ids 优先于 selectType）；
 *  2. productSelectType=2 → categoryId；=3 → introType（productTag）；
 *  3. 其余（缺省 0）→ 默认推荐流，仅按 productNumber 取量。
 * size 一律取 productNumber（缺省 50，def 口径）；前台是 20/页滚动翻页到
 * maxNum，编辑器画布不做滚动翻页，一次取齐（同新版预览）。
 *
 * 与前台的两点已知偏差：
 *  1. 布局：前台 decorate-source.scss 把列表落成 228px 定宽 flex-wrap +
 *     299px 定高卡（瀑布感已被前台磨平）；本预览按模块本名「推荐商品-
 *     瀑布流」与编辑器口径用 CSS columns 多列不等高（图片自然比例、
 *     break-inside: avoid），卡片内部结构与前台逐层一致。
 *  2. 标题显隐：前台只看 titleName 非空（adapt 不透传 showTitle）；本预览
 *     额外尊重编辑器 def 的 showTitle 开关——存量数据无此字段时按显示
 *     处理（与前台一致），显式置 0/false 时隐藏，改设置即时生效。
 *  另：活动边框图（product-style-border，前台 useProductStyleMap）编辑器
 *  同族预览均未实现，本预览同样略过；com-pc-title 两侧的雪碧图装饰在
 *  admin 无该资源，css 里以同尺寸的 CSS 线条纹样替代。
 */
import { useEffect, useState } from 'react';
import type { SyntheticEvent } from 'react';
import { apiGet } from '../../../../api';
import { useI18n } from '../../../../i18n';
import './pcRecommendProduct.css';

interface PreviewProduct {
  productId?: number | string;
  productSn?: string;
  productName?: string;
  picUrl?: string;
  picThumb?: string;
  picLarge?: string;
  productPrice?: number | string;
  productStock?: number | string;
  productType?: number | string;
  deliveryMode?: number | string;
  deliveryType?: number | string;
  skuList?: unknown[];
  productSkuList?: unknown[];
}

/** 旧版 pcRecommendProduct def 的 defaults（对应 modules/pcRecommendProduct/index.tsx，products 补齐前台 adapt 的全字段兜底）。 */
const RECOMMEND_DEFAULTS = {
  titleName: '',
  showTitle: undefined as number | boolean | undefined,
  products: {
    productSelectType: 0,
    productIds: [] as Array<number | string>,
    productCategoryId: undefined as number | string | undefined,
    productTag: '',
    productNumber: 50,
  },
};

type RecommendConfig = typeof RECOMMEND_DEFAULTS;

/** 深合并默认值：对象递归、数组覆盖（同同族预览）。 */
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

const EMPTY_IMG =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='100' height='100'%3E%3Crect fill='%23f2f4f6' width='100' height='100'/%3E%3Cpath d='M28 62l13-16 10 11 7-7 14 12H28z' fill='%23c0c8d4'/%3E%3Ccircle cx='40' cy='38' r='6' fill='%23c0c8d4'/%3E%3C/svg%3E";

function handleImgError(e: SyntheticEvent<HTMLImageElement>) {
  const img = e.currentTarget;
  if (!img || img.src === EMPTY_IMG) return;
  img.onerror = null;
  img.src = EMPTY_IMG;
}

/** 三段式价格（¥ / 整数 / 小数），口径同前台 FormatPrice 的 .util/.num/.decimals。 */
function priceParts(price: string | number | undefined): { int: string; dec: string } {
  const num = Number(price);
  const text = Number.isFinite(num) ? num.toFixed(2) : '0.00';
  const [int, dec] = text.split('.');
  return { int: int || '0', dec: dec || '00' };
}

/** 按 ids 配置顺序回排（同新版预览 sortRecordsByIds）。 */
function sortByIds(records: PreviewProduct[], ids: Array<number | string>): PreviewProduct[] {
  const order = ids.map((v) => String(v));
  if (!order.length) return records;
  const map = new Map<string, PreviewProduct>();
  records.forEach((r) => {
    const k = r.productId === undefined || r.productId === null ? '' : String(r.productId);
    if (k) map.set(k, r);
  });
  const sorted: PreviewProduct[] = [];
  order.forEach((id) => {
    const hit = map.get(id);
    if (hit) sorted.push(hit);
  });
  records.forEach((r) => {
    const k = r.productId === undefined || r.productId === null ? '' : String(r.productId);
    if (!k || !order.includes(k)) sorted.push(r);
  });
  return sorted;
}

/** 按 products 配置取数（口径见文件头）。 */
function useRecommendProducts(products: RecommendConfig['products']) {
  const [rows, setRows] = useState<PreviewProduct[]>([]);
  const [loading, setLoading] = useState(true);
  const productsKey = products ? JSON.stringify(products) : '';

  useEffect(() => {
    let alive = true;
    const cfg = (products || {}) as Record<string, any>;
    const ids: Array<number | string> = Array.isArray(cfg.productIds) ? cfg.productIds : [];
    const selectType = Number(cfg.productSelectType) || 0;
    const number = Math.max(0, Number(cfg.productNumber) || 50) || 50;

    const param: Record<string, any> = { page: 1, productStatus: 1, checkStatus: 1 };
    if (ids.length) {
      param.ids = ids.join(',');
      param.size = ids.length;
    } else if (selectType === 2 && cfg.productCategoryId !== undefined && cfg.productCategoryId !== null && cfg.productCategoryId !== '') {
      param.categoryId = cfg.productCategoryId;
      param.size = number;
    } else if (selectType === 3 && cfg.productTag) {
      param.introType = cfg.productTag;
      param.size = number;
    } else {
      param.size = number;
    }

    setLoading(true);
    apiGet<{ records?: PreviewProduct[]; list?: PreviewProduct[] }>('/product/product/list', param)
      .then((res) => {
        if (!alive) return;
        const records = res.records || res.list || [];
        setRows(ids.length ? sortByIds(records, ids) : records);
        setLoading(false);
      })
      .catch(() => {
        if (!alive) return;
        setRows([]);
        setLoading(false);
      });
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [productsKey]);

  return { rows, loading };
}

export default function PcRecommendProductLegacyPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const config = deepMerge<RecommendConfig>(RECOMMEND_DEFAULTS, data);
  const { rows, loading } = useRecommendProducts(config.products);

  // 标题显隐：titleName 非空 + showTitle 未被显式关闭（见文件头偏差第 2 点）
  const showTitle = config.showTitle !== 0 && config.showTitle !== false;
  const titleName = typeof config.titleName === 'string' ? config.titleName : '';

  // 购买按钮文案（前台 buyTextOf：1=加入购物车，5/6=立即预约，其余=立即购买）
  const buyTextOf = (type: unknown): string => {
    const n = Number(type ?? 1);
    if (n === 1) return t('加入购物车');
    if (n === 5 || n === 6) return t('立即预约');
    return t('立即购买');
  };

  return (
    <div className="mod_you_like">
      {showTitle && titleName ? <h3 className="com-pc-title">{titleName}</h3> : null}
      <div className="you_like_list">
        {rows.length ? (
          <ul>
            {rows.map((product, index) => {
              const name = product.productName || '';
              const cycleBuy = Number(product.deliveryMode) === 2 ? Number(product.deliveryType) : 0;
              const soldOut = Number(product.productStock) === 0;
              const parts = priceParts(product.productPrice);
              return (
                <li key={product.productSn || product.productId || index}>
                  <div className="item">
                    <div className="flip_wrap">
                      <div className="pro_pic">
                        <span className="pro_pic_link">
                          <img
                            src={product.picUrl || product.picThumb || EMPTY_IMG}
                            alt={name}
                            loading="lazy"
                            onError={handleImgError}
                          />
                        </span>
                      </div>
                      <div className="pro_name">
                        {cycleBuy === 1 ? <span className="cycleBuyTag">{t('周期购')}</span> : null}
                        {cycleBuy === 2 ? <span className="cycleBuyTag">{t('次卡兑换')}</span> : null}
                        {name}
                      </div>
                      <div className="pro_price">
                        <span className="util">¥</span>
                        <span className="num">{parts.int}</span>
                        <span className="decimals">.{parts.dec}</span>
                      </div>
                    </div>
                    {/* 前台 operate_area 仅 .item:hover 显示；预览内链接一律惰性（span/div） */}
                    <div className="operate_area">
                      <span className="not_interest left-span">
                        <span>{t('查看详情')}</span>
                      </span>
                      <div className={`view_detail${soldOut ? ' is-disabled' : ''}`}>{buyTextOf(product.productType)}</div>
                    </div>
                  </div>
                </li>
              );
            })}
          </ul>
        ) : loading ? (
          <ul>
            {[1, 2, 3, 4, 5].map((i) => (
              <li key={i}>
                <div className="item">
                  <div className="you_like_skeleton-img" />
                  <div className="you_like_skeleton-line" />
                  <div className="you_like_skeleton-line" style={{ width: '55%' }} />
                </div>
              </li>
            ))}
          </ul>
        ) : (
          <div className="you_like_empty">{t('暂无商品')}</div>
        )}
      </div>
    </div>
  );
}
