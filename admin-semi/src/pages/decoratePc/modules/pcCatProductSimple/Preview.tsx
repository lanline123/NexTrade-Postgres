/**
 * PC 分类商品-简约（pcCatProductSimple）- 预览组件
 *
 * 渲染规格对齐前台 pc-semi ContractCatProductSimple
 * （pc-semi/src/decorate/components/modules.tsx:4679，逐条复刻旧版
 * components/home/CatProductSimple.vue）：
 *   .simple-goodsbox
 *     标题行：标题（24px）+ 右侧「更多 ›」
 *     商品网格 4 列：图 180×180 / 名称 / 简介 / 价格行（现价粗体 + 市场价弱化）
 *     baseStyle=2：左侧大图（picList[0]，230px）+ 右侧网格（图高 210）
 *
 * 取数（与前台 useDecorateProductList / buildDecorateProductListQuery 一致，
 * 参数映射同编辑器 pc-product 预览）：/product/product/list
 *   productSelectType 1 → ids=逗号串&size=个数
 *   2 → categoryId=productCategoryId&page=1&size=productNumber
 *   3 → introType=productTag&page=1&size=productNumber
 *
 * 与前台差异：
 * 1. 前台无商品时整块不渲染（return null）；编辑器保留标题行 + 空态占位，
 *    否则模块在画布上不可见、不可选中（与 pc-recommend-product 预览同口径）；
 * 2. 活动边框图（useProductStyleMap）编辑器不取，商品图不叠边框；
 * 3. 链接一律 javascript:; 不跳转；市场价前台走 FormatPrice 的 displayPrice
 *    缩写口径，编辑器用统一 formatPrice 全额显示。
 */
import { useEffect, useMemo, useState } from 'react';
import { apiGet } from '../../../../api';
import { useI18n } from '../../../../i18n';
import './pcCatProductSimple.css';

type AnyRecord = Record<string, any>;

const EMPTY_IMG =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='100' height='100'%3E%3Crect fill='%23f2f4f6' width='100' height='100'/%3E%3Cpath d='M28 62l13-16 10 11 7-7 14 12H28z' fill='%23c0c8d4'/%3E%3Ccircle cx='40' cy='38' r='6' fill='%23c0c8d4'/%3E%3C/svg%3E";

/** 与 def defaults 逐字段一致（modules/pcCatProductSimple/index.tsx）。 */
const DEFAULTS: AnyRecord = {
  baseStyle: 1,
  title: '',
  products: { productSelectType: 1, productCategoryId: 1, productNumber: 8, productIds: [], productTag: '' },
  moreLink: '',
  picList: [],
};

interface ProductItem {
  productId?: number | string;
  id?: number | string;
  productName?: string;
  productBrief?: string;
  brief?: string;
  productPrice?: number | string;
  marketPrice?: number | string;
  picUrl?: string;
  picThumb?: string;
  picLarge?: string;
}

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

/** 按 products 配置拉商品（参数映射与 pc-product 预览 / 前台构建器一致）。 */
function useProductList(products: AnyRecord | undefined): ProductItem[] {
  const [list, setList] = useState<ProductItem[]>([]);
  const productsKey = products ? JSON.stringify(products) : '';
  useEffect(() => {
    let alive = true;
    const run = async () => {
      if (!products) {
        if (alive) setList([]);
        return;
      }
      const type = Number(products.productSelectType || 0);
      if (!type) {
        if (alive) setList([]);
        return;
      }
      const param: AnyRecord = { productStatus: 1, checkStatus: 1 };
      if (type === 1) {
        const idArr: Array<string | number> = Array.isArray(products.productIds)
          ? products.productIds
          : String(products.productIds || '').split(',').filter(Boolean);
        if (!idArr.length) {
          if (alive) setList([]);
          return;
        }
        param.ids = idArr.join(',');
        param.size = idArr.length;
      } else if (type === 2) {
        if (products.productCategoryId === undefined || products.productCategoryId === null || products.productCategoryId === '') {
          if (alive) setList([]);
          return;
        }
        param.page = 1;
        param.categoryId = products.productCategoryId;
        param.size = products.productNumber ?? 8;
      } else if (type === 3) {
        if (!products.productTag || String(products.productTag).trim() === '') {
          if (alive) setList([]);
          return;
        }
        param.page = 1;
        param.introType = products.productTag;
        param.size = products.productNumber ?? 8;
      } else if (type === 4) {
        // 全部商品（我方新增）：不带任何过滤参数，仅 page+size
        param.page = 1;
        param.size = products.productNumber ?? 8;
      } else {
        if (alive) setList([]);
        return;
      }
      try {
        const result = await apiGet<{ records?: ProductItem[]; list?: ProductItem[] }>('/product/product/list', param);
        if (!alive) return;
        const records = result.records || result.list || [];
        if (type === 1 && Array.isArray(products.productIds)) {
          const order = products.productIds.map((v: any) => String(v));
          const sorted = [...records].sort((a, b) => {
            const ka = order.indexOf(String(a.productId ?? a.id ?? ''));
            const kb = order.indexOf(String(b.productId ?? b.id ?? ''));
            return (ka < 0 ? order.length : ka) - (kb < 0 ? order.length : kb);
          });
          setList(sorted);
        } else {
          setList(records);
        }
      } catch {
        if (alive) setList([]);
      }
    };
    void run();
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [productsKey]);
  return list;
}

/** 「更多」右侧箭头（前台内联 SVG 逐字复刻）。 */
function MoreArrow() {
  return (
    <svg width="15" height="17" viewBox="0 0 1024 1024" aria-hidden="true">
      <path
        d="M329.354 820.029c-6.006 6.002-9.38 14.144-9.38 22.635s3.374 16.633 9.38 22.635c12.508 12.483 32.761 12.483 45.269 0l341.333-340.821c6.003-5.994 9.376-14.13 9.376-22.613s-3.373-16.619-9.376-22.613l-341.333-341.888c-12.508-12.483-32.761-12.483-45.269 0-6.006 6.002-9.38 14.144-9.38 22.635s3.374 16.633 9.38 22.635l318.72 319.232-318.72 318.165z"
        fill="#000000"
        fillOpacity="0.2"
      />
    </svg>
  );
}

export default function PcCatProductSimplePreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const config = useMemo(() => deepMerge(DEFAULTS, data), [JSON.stringify(data)]);
  const titleText = titleTextOf(config.title);
  const baseStyle = Number(config.baseStyle) || 1;
  const pics: AnyRecord[] = Array.isArray(config.picList) ? config.picList : [];
  const firstPic = pics[0];
  const products = useProductList(config.products as AnyRecord | undefined);

  return (
    <div className="dec-pc-cat-simple">
      <div className="dec-pc-cat-simple__head">
        <div className="dec-pc-cat-simple__tit">{titleText}</div>
        <a className="dec-pc-cat-simple__more" href="javascript:;">
          {t('更多')}
          <MoreArrow />
        </a>
      </div>
      {products.length ? (
        <div className={`dec-pc-cat-simple__contnt${baseStyle === 2 ? ' is-left-layout' : ''}`}>
          {baseStyle === 2 ? (
            <div className="dec-pc-cat-simple__left">
              {firstPic ? (
                <a className="dec-pc-cat-simple__imgs" href="javascript:;">
                  <img src={picSrc(firstPic)} alt={titleText} />
                </a>
              ) : null}
            </div>
          ) : null}
          <div className="dec-pc-cat-simple__right">
            <div className="dec-pc-cat-simple__grid">
              {products.map((item, i) => (
                <a className="dec-pc-cat-simple__item" key={String(item.productId ?? item.id ?? i)} href="javascript:;">
                  <div className="dec-pc-cat-simple__pic">
                    <img src={picSrc(item)} alt={item.productName || ''} />
                  </div>
                  <div className="dec-pc-cat-simple__info">
                    <div className="dec-pc-cat-simple__name">{item.productName || ''}</div>
                    <div className="dec-pc-cat-simple__desc">{item.productBrief ?? item.brief ?? ''}</div>
                    <div className="dec-pc-cat-simple__price-box">
                      <div className="dec-pc-cat-simple__newprice">{formatPrice(item.productPrice)}</div>
                      {item.marketPrice && Number(item.marketPrice) > 0 ? (
                        <div className="dec-pc-cat-simple__oldprice">{formatPrice(item.marketPrice)}</div>
                      ) : null}
                    </div>
                  </div>
                </a>
              ))}
            </div>
          </div>
        </div>
      ) : (
        <div className="dec-pc-cat-simple__empty">
          <div className="dec-pc-cat-simple__empty-icon" />
          <div className="dec-pc-cat-simple__empty-text">{t('点击添加商品')}</div>
        </div>
      )}
    </div>
  );
}
