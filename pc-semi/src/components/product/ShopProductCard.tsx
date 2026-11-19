'use client';

/**
 * 店铺装修商品卡片（源码对照：java-b2b2c-overseas-v5.8.61/view/pc/components/shop/product/）
 *
 * 旧版店铺装修的 7 个组件里，商品卡片本体有 3 种形态，本次合并为一个组件 + variant 区分：
 *   - ProductCard.vue      → variant='standard'（竖排：图上 / 名 / 简介 / 价格+购买按钮）
 *   - ProductCardLand.vue  → variant='land'（横排：左图 260px / 右侧信息）
 *   - ProductCardSlide.vue → variant='slide'（竖排 + 固定宽度，横向滑动用）
 *
 * 布局要点（逐条对齐旧版 .less）：
 *   - .product-card 外层只负责 padding（module.goodsPadding）与宽度（100 / size %）；
 *     边框/圆角 class（goodsStyle / goodsRadioStyle）落在内层 .car-info 上。
 *   - 名称：16px / line-height 20px；goodsNameRow=1 → 高 20px，否则 40px；textWeight≠1 → bold。
 *   - 简介：standard 两行截断（-webkit-line-clamp:2），land / slide 单行省略。
 *   - 价格：24px、var(--price)，textWeight≠1 → bold。
 *   - 购买按钮：全站统一枚举 buyBtnStyle（0 隐藏 / 1-8 旧版 ProductButton / 9 圆圈购物车 /
 *     10 自定义图片 / -1 跟随全局），经共享组件 BuyButton 归一化渲染，
 *     复用 modules-source.scss 的 .dec-buy-btn 系列 class。
 *   - 商品样式边框图（productStyle.borderImage）叠在图片上。
 *
 * 复用的 class：product-card-border-2 / -3 / -4（goodsStyle）、box-radius / image-box-radius（goodsRadioStyle）。
 */

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { Toast } from '@douyinfe/semi-ui';
import { apiPost } from '@/lib/client-api';
import { splitDisplayPrice } from '@/lib/currency';
import { BuyButtonVisual, buyBtnClassName, resolveBuyBtnStyle } from '@/decorate/components/BuyButton';
import { gateCopyForPayload } from '@/lib/price-gate';
import '@/components/product/shop-product-card.scss';

export type ShopCardVariant = 'standard' | 'land' | 'slide';

export interface ShopProductItem {
  productId?: number | string;
  productSn?: string;
  seoLink?: string;
  productName?: string;
  productBrief?: string;
  brief?: string;
  productPrice?: number | string;
  price?: number | string;
  picThumb?: string;
  picUrl?: string;
  productStock?: number | string;
  productSku?: unknown[];
  productType?: number | string;
  deliveryMode?: number | string;
  deliveryType?: number | string;
}

/** 装修模块里与卡片外观相关的字段（旧版 ShopProduct.vue 透传给各卡片组件的 module） */
export interface ShopCardModule {
  goodsStyle?: number;
  goodsRadioStyle?: number;
  goodsPadding?: number;
  goodsNameRow?: number;
  goodsNamePadding?: number;
  textAlign?: number;
  textWeight?: number;
  showName?: number | boolean;
  showBrief?: number | boolean;
  showPrice?: number | boolean;
  /** 购买按钮：全站统一枚举（见 @/decorate/components/BuyButton），可为数字/旧对象型，由 resolveBuyBtnStyle 读时归一化 */
  buyBtnStyle?: unknown;
  /** 样式 10（自定义图片）时模块自带图标 {picUrl, picThumb} */
  buyBtnIcon?: unknown;
}

/** 商品样式边框图（/api/product/productStyle/batchActiveStyle 返回的 borderImage） */
export type ProductStyleMap = Record<number, { borderImage?: string } | undefined>;

/** 旧版 ProductCard.vue 的 border/radio class 映射（源码 components/shop/product/ProductCard.vue） */
function cardBorderClass(goodsStyle: number): string {
  if (goodsStyle === 2) return 'product-card-border-2';
  if (goodsStyle === 3) return 'product-card-border-3';
  if (goodsStyle === 1) return '';
  return 'product-card-border-4';
}

function splitPrice(value: unknown): { symbol: string; integer: string; decimals: string; suffix: string } {
  return splitDisplayPrice(value);
}

export function ShopProductCard({
  product,
  module = {},
  variant = 'standard',
  outWidth,
  styleMap,
}: {
  product: ShopProductItem;
  module?: ShopCardModule;
  variant?: ShopCardVariant;
  /** slide 形态的固定宽度（源码 ProductCardSlide.vue 的 outWidth，如 '260px' / '33.3%'） */
  outWidth?: string;
  styleMap?: ProductStyleMap;
}) {
  const [adding, setAdding] = useState(false);

  const goodsStyle = Number(module.goodsStyle ?? 0);
  const goodsRadioStyle = Number(module.goodsRadioStyle ?? 0);
  const borderClass = cardBorderClass(goodsStyle);
  const radiusClass = goodsRadioStyle === 2 ? 'box-radius' : '';
  const imageRadiusClass = goodsRadioStyle === 2 ? 'image-box-radius' : '';

  const textAlign = Number(module.textAlign ?? 1) === 1 ? 'left' : 'center';
  const bold = Number(module.textWeight ?? 1) !== 1;
  const nameHeight = Number(module.goodsNameRow ?? 2) === 1 ? 20 : 40;
  const goodsPadding = Number(module.goodsPadding ?? 0);
  const showName = module.showName !== 0 && module.showName !== false;
  const showBrief = module.showBrief !== 0 && module.showBrief !== false;
  const showPrice = module.showPrice !== 0 && module.showPrice !== false;
  /* 购买按钮（全站统一共享组件）：读时归一化统一枚举，支持 -1 跟随全局 / 9 圆圈购物车 / 10 自定义图片 */
  const buyBtn = resolveBuyBtnStyle(module.buyBtnStyle, { moduleIcon: module.buyBtnIcon });

  const href = product.seoLink || `/product/${product.productSn || product.productId || ''}`;
  const image = product.picUrl || product.picThumb || '';
  const brief = product.productBrief || product.brief || '';
  const price = splitPrice(product.productPrice ?? product.price);
  /* 价格门控（第 2 期）：后端置空价格时价格位渲染门控文案、撤下购买钮
     （与共享 ProductCard 同口径；旧载荷无 priceVisible 时按可见处理） */
  const priceGated = (product as any).priceVisible === false;
  const gateCopy = priceGated ? gateCopyForPayload((product as any).priceGate) : null;
  const priceNode = showPrice ? (
    gateCopy ? (
      <Link href={gateCopy.href} target="_blank" onClick={(event) => event.stopPropagation()}>
        <span className="shop-card__gate">{gateCopy.text}</span>
      </Link>
    ) : (
      <ShopPrice symbol={price.symbol} integer={price.integer} decimals={price.decimals} suffix={price.suffix} bold={bold} />
    )
  ) : null;
  const borderImage = styleMap?.[Number(product.productId)]?.borderImage;

  /** 旧版 ProductCard.vue：有 SKU 或 productType∈{5,6} 走详情页，其余直接加购 */
  const needsDetail =
    (Array.isArray(product.productSku) && product.productSku.length > 0) || [5, 6].includes(Number(product.productType));

  async function addToCart() {
    if (adding || !product.productId) return;
    setAdding(true);
    try {
      await apiPost('cart/cart/addToCart', { id: Number(product.productId), number: 1 });
      Toast.success({ content: '已加入购物车' });
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : '加入购物车失败' });
    } finally {
      setAdding(false);
    }
  }

  const buyButton = priceGated ? null : buyBtn.show ? (
    needsDetail ? (
      <Link
        href={href}
        target="_blank"
        className={`shop-card__buy ${buyBtnClassName(buyBtn.style)}`}
        style={buyBtn.iconColor ? { color: buyBtn.iconColor } : undefined}
      >
        <BuyButtonVisual style={buyBtn.style} customIcon={buyBtn.customIcon} />
      </Link>
    ) : (
      <button
        type="button"
        className={`shop-card__buy ${buyBtnClassName(buyBtn.style)}`}
        style={buyBtn.iconColor ? { color: buyBtn.iconColor } : undefined}
        onClick={addToCart}
        disabled={adding}
      >
        <BuyButtonVisual style={buyBtn.style} customIcon={buyBtn.customIcon} />
      </button>
    )
  ) : null;

  const nameNode = showName ? (
    <Link
      className="shop-card__title"
      href={href}
      target="_blank"
      style={{ fontWeight: bold ? 'bold' : 'normal', height: nameHeight }}
    >
      {Number(product.deliveryMode) === 2 && Number(product.deliveryType) === 1 ? (
        <span className="shop-card__cycle-tag">周期购</span>
      ) : null}
      {Number(product.deliveryMode) === 2 && Number(product.deliveryType) === 2 ? (
        <span className="shop-card__cycle-tag">次卡兑换</span>
      ) : null}
      {product.productName || ''}
    </Link>
  ) : null;

  const briefNode = showBrief && brief ? <div className="shop-card__brief">{brief}</div> : null;

  const photoNode = (
    <div className="shop-card__photo">
      <Link className="shop-card__link" href={href} target="_blank">
        <img className={`shop-card__img ${imageRadiusClass}`} src={image} alt={product.productName || ''} loading="lazy" />
      </Link>
      {borderImage ? <img className="shop-card__style-border" src={borderImage} alt="" /> : null}
    </div>
  );

  if (variant === 'land') {
    // 旧版 ProductCardLand.vue：左图固定 260px，右侧信息区
    return (
      <div className="shop-card shop-card--land" style={{ padding: `${goodsPadding}px` }}>
        <div className={`shop-card__info ${borderClass} ${radiusClass}`}>
          {photoNode}
          <div className="shop-card__right">
            <div className="shop-card__body" style={{ padding: Number(module.goodsNamePadding ?? 1) === 1 ? '0 10px' : undefined }}>
              <div className="shop-card__name" style={{ textAlign }}>
                {nameNode}
              </div>
              <div className="shop-card__action">
                {priceNode}
                {buyButton}
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (variant === 'slide') {
    // 旧版 ProductCardSlide.vue：竖排 + 固定宽度（min/max width = outWidth）
    return (
      <div
        className="shop-card shop-card--slide"
        style={{ padding: `${goodsPadding}px`, minWidth: outWidth, maxWidth: outWidth }}
      >
        <div className={`shop-card__info ${borderClass} ${radiusClass}`}>
          {photoNode}
          <div className="shop-card__body" style={{ padding: Number(module.goodsNamePadding ?? 1) === 1 ? '0 10px' : undefined }}>
            <div className="shop-card__name" style={{ textAlign }}>
              {nameNode}
            </div>
            {briefNode}
            <div className="shop-card__action">
              {priceNode}
              {buyButton}
            </div>
          </div>
        </div>
      </div>
    );
  }

  // 旧版 ProductCard.vue：标准竖排卡片
  return (
    <div className="shop-card" style={{ padding: `${goodsPadding}px` }}>
      <div className={`shop-card__info ${borderClass} ${radiusClass}`}>
        {photoNode}
        <div className="shop-card__body" style={{ padding: Number(module.goodsNamePadding ?? 1) === 1 ? '0 10px' : undefined }}>
          <div className="shop-card__name" style={{ textAlign }}>
            {nameNode}
          </div>
          {briefNode}
          <div className="shop-card__action">
            {priceNode}
            {buyButton}
          </div>
        </div>
      </div>
    </div>
  );
}

/** 价格（旧版 <FormatPrice>：￥ + 整数 + 小数，24px / var(--price)） */
/** 旧版 FormatPrice 的 符号 / 整数 / 小数 三段结构（源码 components/format/Price.vue） */
function ShopPrice({ symbol, integer, decimals, suffix, bold }: { symbol: string; integer: string; decimals: string; suffix?: string; bold: boolean }) {
  return (
    <div className="shop-card__price" style={{ fontWeight: bold ? 'bold' : 'normal' }}>
      <span className="shop-card__price-sym">{symbol || '￥'}</span>
      <span className="shop-card__price-int">{integer}</span>
      <span className="shop-card__price-dec">{decimals}</span>
      {suffix ? <span className="shop-card__price-dec">{suffix}</span> : null}
    </div>
  );
}

/** 购买按钮视觉已统一到共享组件 @/decorate/components/BuyButton 的 BuyButtonVisual（原本地 BuyButtonIcon 删除）。 */

/**
 * 商品样式边框图（旧版 hooks/useProductStyle.ts → /api/product/productStyle/batchActiveStyle）。
 * 按 productIds 批量取一次，失败静默降级（无边框图）。
 */
export function useProductStyleMap(products: ShopProductItem[]): ProductStyleMap {
  const [styleMap, setStyleMap] = useState<ProductStyleMap>({});
  const ids = products.map((item) => Number(item.productId)).filter(Boolean);
  const key = ids.join(',');

  useEffect(() => {
    if (!key) {
      setStyleMap({});
      return undefined;
    }
    let cancelled = false;
    fetch(`/api/product/productStyle/batchActiveStyle?productIds=${key}`)
      .then((response) => response.json())
      .then((json) => {
        if (cancelled) return;
        const data = json?.data || json || {};
        if (data && typeof data === 'object') setStyleMap(data as ProductStyleMap);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [key]);

  return styleMap;
}
