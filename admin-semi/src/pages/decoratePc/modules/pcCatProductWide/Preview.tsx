/**
 * PC 分类商品-宽（pcCatProductWide）- 预览组件
 *
 * 渲染规格对齐前台 pc-semi ContractCatProductWide
 * （pc-semi/src/decorate/components/modules.tsx:5340，逐字复刻旧版
 * components/home/CatProductWide.vue）：
 *   .mod_index_floor（高 430，三栏）
 *     左栏（236）：linear-gradient(30deg, color, color2) 渐变 + 标题 + enTitle 英文副标题
 *       + SpecialSwiper（picList 层叠轮播，3s、悬停暂停、计数器）+ 底部 linkList 评论条
 *     中栏（704）：子分类关键词行（前 8）+ picList2 促销卡（前 6，副标题 color 着色）
 *     右栏（250）：热门商品榜（前 5）：60×60 图 + 名称（预售标）+ 价格
 *
 * 取数（与前台契约层 adaptPcCatProductWide + useDecorateProductList 一致）：
 *   productIds 非空 → /product/product/list?ids=逗号串&size=个数
 *   否则 → categoryId=主分类&page=1&size=8，展示前 5。
 *
 * 与前台差异：
 * 1. 子分类关键词：前台消费后端富化的 childCategoryList；编辑器带该字段时优先，
 *    否则按 categoryId 从分类树取子级，并按 def 的 childCategoryIds（选择子分类显示）
 *    非空时过滤——等价于富化口径；
 * 2. 链接一律 javascript:; 不跳转；iconfont（标题箭头/评论图标）改内联 SVG；
 * 3. 活动边框图（useProductStyleMap）编辑器不取，热门商品不叠边框图；
 * 4. picList 为空时轮播区渲染一张占位图（前台为空转盘面），缺图走内联 SVG 占位。
 */
import { useEffect, useMemo, useState } from 'react';
import { apiGet } from '../../../../api';
import { useI18n } from '../../../../i18n';
import './pcCatProductWide.css';

type AnyRecord = Record<string, any>;

const EMPTY_IMG =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='100' height='100'%3E%3Crect fill='%23f2f4f6' width='100' height='100'/%3E%3Cpath d='M28 62l13-16 10 11 7-7 14 12H28z' fill='%23c0c8d4'/%3E%3Ccircle cx='40' cy='38' r='6' fill='%23c0c8d4'/%3E%3C/svg%3E";

/** 与 def defaults 逐字段一致（modules/pcCatProductWide/index.tsx）。 */
const DEFAULTS: AnyRecord = {
  categoryId: 0,
  childCategoryIds: [],
  title: '',
  shortTitle: '',
  enTitle: '',
  color: '',
  color2: '',
  logo: { picUrl: '', picThumb: '' },
  picList: [],
  picList2: [],
  productIds: [],
  linkList: [],
};

function isPlainObject(value: unknown): value is AnyRecord {
  return Object.prototype.toString.call(value) === '[object Object]';
}

function deepMerge<T extends AnyRecord>(defaults: T, source?: AnyRecord | null): T {
  const out: AnyRecord = Array.isArray(defaults) ? [...defaults] : { ...defaults };
  if (!source) return out as T;
  Object.keys(source).forEach((key) => {
    const base = out[key];
    const next = source[key];
    if (isPlainObject(base) && isPlainObject(next)) {
      out[key] = deepMerge(base as AnyRecord, next as AnyRecord);
    } else if (next !== undefined) {
      out[key] = next;
    }
  });
  return out as T;
}

function picSrc(item?: AnyRecord | null): string {
  if (!item) return EMPTY_IMG;
  return item.picUrl || item.picThumb || item.picLarge || EMPTY_IMG;
}

function titleTextOf(raw: any): string {
  return typeof raw === 'string' ? raw : (raw?.titleText as string) || '';
}

function formatPrice(price?: number | string): string {
  if (price === undefined || price === null) return '¥0.00';
  const num = typeof price === 'string' ? parseFloat(price) : price;
  return `¥${Number.isFinite(num) ? num.toFixed(2) : '0.00'}`;
}

/* ---------- 分类树（子分类关键词兜底取数，与 PcDecorateEditor useCategoryTree 同口径） ---------- */

let categoryTreeCache: AnyRecord[] | null = null;

function findChildren(tree: AnyRecord[], categoryId: number): AnyRecord[] {
  for (const node of tree) {
    if (Number(node.categoryId) === categoryId) {
      return Array.isArray(node.children) ? node.children : [];
    }
    if (Array.isArray(node.children)) {
      const found = findChildren(node.children, categoryId);
      if (found.length) return found;
    }
  }
  return [];
}

function useCategoryChildren(categoryId: number): AnyRecord[] {
  const [children, setChildren] = useState<AnyRecord[]>([]);
  useEffect(() => {
    if (!categoryId) {
      setChildren([]);
      return undefined;
    }
    let alive = true;
    const apply = (tree: AnyRecord[]) => {
      if (alive) setChildren(findChildren(tree, categoryId));
    };
    if (categoryTreeCache) {
      apply(categoryTreeCache);
    } else {
      apiGet<any[]>('/product/category/getAllCategory')
        .then((raw) => {
          categoryTreeCache = Array.isArray(raw) ? raw : [];
          apply(categoryTreeCache);
        })
        .catch(() => {
          if (alive) setChildren([]);
        });
    }
    return () => {
      alive = false;
    };
  }, [categoryId]);
  return children;
}

/* ---------- 热门商品取数（ids 优先，否则主分类；口径同前台契约层） ---------- */

function useHotProducts(productIds: Array<string | number>, categoryId: number): AnyRecord[] {
  const [products, setProducts] = useState<AnyRecord[]>([]);
  const idsKey = productIds.join(',');
  useEffect(() => {
    let alive = true;
    const run = async () => {
      const param: AnyRecord = { productStatus: 1, checkStatus: 1 };
      if (productIds.length) {
        param.ids = idsKey;
        param.size = productIds.length;
      } else if (categoryId) {
        param.page = 1;
        param.categoryId = categoryId;
        param.size = 8;
      } else {
        if (alive) setProducts([]);
        return;
      }
      try {
        const result = await apiGet<{ records?: AnyRecord[]; list?: AnyRecord[] }>('/product/product/list', param);
        if (!alive) return;
        const records = result.records || result.list || [];
        if (productIds.length) {
          const order = productIds.map((v) => String(v));
          const sorted = [...records].sort((a, b) => {
            const ka = order.indexOf(String(a.productId ?? a.id ?? ''));
            const kb = order.indexOf(String(b.productId ?? b.id ?? ''));
            return (ka < 0 ? order.length : ka) - (kb < 0 ? order.length : kb);
          });
          setProducts(sorted);
        } else {
          setProducts(records);
        }
      } catch {
        if (alive) setProducts([]);
      }
    };
    void run();
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [idsKey, categoryId]);
  return products;
}

/* ---------- SpecialSwiper（前台 ContractSpecialSwiper 的层叠样式逐行移植） ---------- */

interface SlideStyle {
  width: string;
  height: string;
  left: string;
  top: string;
  opacity: number;
  zIndex: number;
}

const SWIPER_BASE_STYLE: SlideStyle = { width: '195px', height: '225px', left: '0px', top: '0px', opacity: 1, zIndex: 100 };

function swiperSlideStyles(count: number, current: number): SlideStyle[] {
  const out: SlideStyle[] = [];
  for (let index = 0; index < count; index += 1) {
    if (index === current) {
      out[index] = { ...SWIPER_BASE_STYLE };
    } else if (index === current - 1 || (current === 0 && index === count - 1)) {
      out[index] = { width: '155px', height: '225px', left: '20px', top: '-18px', opacity: 0.5, zIndex: 80 };
    } else if (index === current + 1 || (current === count - 1 && index === 0)) {
      out[index] = { width: '175px', height: '225px', left: '10px', top: '-10px', opacity: 0.8, zIndex: 90 };
    } else {
      out[index] = { ...SWIPER_BASE_STYLE, opacity: 0, zIndex: 0 };
    }
  }
  return out;
}

function SpecialSwiper({ slides, activeColor }: { slides: AnyRecord[]; activeColor?: string }) {
  const [current, setCurrent] = useState(0);
  const [paused, setPaused] = useState(false);
  useEffect(() => {
    setCurrent(0);
  }, [slides.length]);
  useEffect(() => {
    if (paused || slides.length <= 1) return undefined;
    const timer = window.setInterval(() => {
      setCurrent((prev) => (prev + 1) % slides.length);
    }, 3000);
    return () => window.clearInterval(timer);
  }, [paused, slides.length]);

  if (!slides.length) {
    return (
      <div className="dec-pc-cat-wide__slider">
        <ul>
          <li style={{ ...SWIPER_BASE_STYLE }}>
            <span className="dec-pc-cat-wide__slider-ph">
              <img src={EMPTY_IMG} alt="" width={130} height={130} />
            </span>
          </li>
        </ul>
      </div>
    );
  }

  const styles = swiperSlideStyles(slides.length, current);
  const navigate = (direction: number) => {
    setCurrent((prev) => (prev + direction + slides.length) % slides.length);
  };

  return (
    <div
      className="dec-pc-cat-wide__slider"
      onMouseEnter={() => setPaused(true)}
      onMouseLeave={() => setPaused(false)}
    >
      <ul>
        {slides.map((slide, i) => (
          <li key={i} style={styles[i]}>
            <a href="javascript:;" className="dec-pc-cat-wide__slider-a">
              <p className="dec-pc-cat-wide__slider-caption" style={activeColor ? { color: activeColor } : undefined}>
                {slide?.picTitle || ''}
              </p>
              <p className="dec-pc-cat-wide__slider-sub">{slide?.picDesc || ''}</p>
              <img src={picSrc(slide)} alt={slide?.picTitle || ''} width={130} height={130} />
            </a>
          </li>
        ))}
      </ul>
      <div className="dec-pc-cat-wide__slider-turn">
        <div className="dec-pc-cat-wide__slider-btn" onClick={() => navigate(-1)}>
          &lt;
        </div>
        <div className="dec-pc-cat-wide__slider-num">
          <span>{current + 1}</span> / <em>{slides.length}</em>
        </div>
        <div className="dec-pc-cat-wide__slider-btn" onClick={() => navigate(1)}>
          &gt;
        </div>
      </div>
    </div>
  );
}

/** 标题右侧圆角箭头（前台 iconfont-pc icon-yuanjiaojiantouyou 的等价内联 SVG）。 */
function TitleArrow() {
  return (
    <svg className="dec-pc-cat-wide__tit-arrow" width="12" height="12" viewBox="0 0 1024 1024" aria-hidden="true">
      <path
        d="M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64zm193.5 456.9-192 128c-12.9 8.6-30.9 5-39.5-7.9-8.6-12.9-5.1-30.3 7.9-39.5l141.6-94.4-141.6-94.4c-12.9-8.6-16.5-26.6-7.9-39.5 8.6-12.9 26.6-16.5 39.5-7.9l192 128c7.8 5.2 12.5 13.9 12.5 23.1s-4.7 17.9-12.5 23.1z"
        fill="currentColor"
      />
    </svg>
  );
}

/** 评论图标（前台 iconfont-pc icon-pinlun 的等价内联 SVG）。 */
function CommentIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 1024 1024" aria-hidden="true">
      <path
        d="M128 96h768a32 32 0 0 1 32 32v576a32 32 0 0 1-32 32H544L320 928V736H128a32 32 0 0 1-32-32V128a32 32 0 0 1 32-32zm96 192v64h576v-64H224zm0 160v64h448v-64H224z"
        fill="currentColor"
      />
    </svg>
  );
}

export default function PcCatProductWidePreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const config = useMemo(() => deepMerge(DEFAULTS, data), [JSON.stringify(data)]);
  const titleText = titleTextOf(config.title);
  const enTitle = typeof config.enTitle === 'string' ? config.enTitle : '';
  const color = typeof config.color === 'string' ? config.color : '';
  const color2 = typeof config.color2 === 'string' ? config.color2 : '';
  const picList: AnyRecord[] = Array.isArray(config.picList) ? config.picList : [];
  const picList2: AnyRecord[] = Array.isArray(config.picList2) ? config.picList2 : [];
  const linkList: AnyRecord[] = Array.isArray(config.linkList) ? config.linkList : [];
  const productIds: Array<string | number> = Array.isArray(config.productIds) ? config.productIds : [];
  const categoryId = Number(Array.isArray(config.categoryId) ? config.categoryId[0] : config.categoryId) || 0;

  // 子分类关键词：存量 childCategoryList 优先；否则分类树子级 + childCategoryIds 过滤
  const storedChildren: AnyRecord[] = Array.isArray(config.childCategoryList) ? config.childCategoryList : [];
  const treeChildren = useCategoryChildren(storedChildren.length ? 0 : categoryId);
  let childCategories = storedChildren.length ? storedChildren : treeChildren;
  const childIds: number[] = Array.isArray(config.childCategoryIds)
    ? config.childCategoryIds.map((v: any) => Number(v)).filter((v: number) => v > 0)
    : [];
  if (!storedChildren.length && childIds.length) {
    childCategories = childCategories.filter((c) => childIds.includes(Number(c.categoryId)));
  }

  const products = useHotProducts(productIds, categoryId);
  const hotList = products.slice(0, 5);

  const leftStyle = color || color2
    ? { background: `linear-gradient(30deg, ${color || color2} 0%, ${color2 || color} 100%)` }
    : undefined;
  const textColorStyle = color ? { color } : undefined;

  return (
    <div className="dec-pc-cat-wide">
      <div className="dec-pc-cat-wide__floor">
        <div className="dec-pc-cat-wide__left" style={leftStyle}>
          <h4>
            <a href="javascript:;" title={titleText}>
              <span>{titleText}</span>
              <TitleArrow />
            </a>
          </h4>
          <p className="dec-pc-cat-wide__en">{enTitle}</p>
          <SpecialSwiper slides={picList} activeColor={color || undefined} />
          <div className="dec-pc-cat-wide__comment">
            <div className="dec-pc-cat-wide__comment-left" style={color ? { borderColor: color } : undefined}>
              <CommentIcon />
            </div>
            <div className="dec-pc-cat-wide__comment-list">
              <ul>
                {linkList.map((item, i) => (
                  <li key={i}>
                    <a href="javascript:;" title={item?.linkTitle || ''}>
                      {item?.linkTitle || ''}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </div>
        <div className="dec-pc-cat-wide__center">
          <div className="dec-pc-cat-wide__keywords">
            <ul>
              {childCategories.map((item, i) => (
                <li key={String(item?.categoryId ?? i)}>
                  {i < 8 ? (
                    <>
                      {i !== 0 ? '|' : ''}
                      <a className="dec-pc-cat-wide__hot-link" href="javascript:;" title={item?.categoryName || ''}>
                        {item?.categoryName || ''}
                      </a>
                    </>
                  ) : null}
                </li>
              ))}
            </ul>
          </div>
          <div className="dec-pc-cat-wide__goods">
            {picList2.map((item, i) => (
              <div className="dec-pc-cat-wide__pro-wrap" key={i}>
                {i < 6 ? (
                  <a href="javascript:;" title={item?.picTitle || ''}>
                    <p className="dec-pc-cat-wide__pro-tit">{item?.picTitle || ''}</p>
                    <p className="dec-pc-cat-wide__pro-sub" style={textColorStyle}>
                      {item?.picDesc || ''}
                    </p>
                    <span className="dec-pc-cat-wide__pro-img">
                      <img src={picSrc(item)} alt={item?.picTitle || ''} />
                    </span>
                  </a>
                ) : null}
              </div>
            ))}
          </div>
        </div>
        <div className="dec-pc-cat-wide__hot">
          <p className="dec-pc-cat-wide__hot-tit">{t('热门商品')}</p>
          <div className="dec-pc-cat-wide__rank">
            <ul>
              {hotList.map((item, i) => {
                const presale = Boolean(item.saleMode || item.productSaleMode);
                return (
                  <li key={String(item.productId ?? item.id ?? i)}>
                    <a href="javascript:;" className="dec-pc-cat-wide__rank-a" title={item.productName || ''}>
                      <span className="dec-pc-cat-wide__rank-img">
                        <img src={picSrc(item)} alt={item.productName || ''} width={60} height={60} />
                      </span>
                      <div className="dec-pc-cat-wide__rank-detail">
                        <p className="dec-pc-cat-wide__rank-name">
                          {presale ? (
                            <>
                              <span className="dec-pc-cat-wide__presale">{t('预售')}</span>
                              <span>|</span>
                            </>
                          ) : null}
                          {item.productName || ''}
                        </p>
                        <div className="dec-pc-cat-wide__rank-price">{formatPrice(item.productPrice)}</div>
                      </div>
                    </a>
                  </li>
                );
              })}
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}
