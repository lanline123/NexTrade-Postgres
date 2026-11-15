'use client';

/**
 * 搜索结果商品网格。
 *
 * 原先这里有一套独立的 `.pc-product-grid__card` 卡片（14px 圆角、hover 上浮 6px、
 * 价格写死红色 #e5484d），与购物车 / 会员中心 / 商品详情的卡片视觉不一致。
 * 现统一走 `src/components/product/ProductCard.tsx`（对齐 Java 原版
 * components/product/ProductCard.vue），本组件只保留「网格 + 空态」职责，
 * props 签名不变，调用方 app/search/page.tsx 无需改动。
 */

import { useRef } from 'react';
import type { ProductListItem } from '@/lib/api';
import ProductCard, { type ProductCardItem } from '@/components/product/ProductCard';
import { useProductColumns } from '@/components/product/use-product-columns';
import { mapSearchLegacyBuyStyle, resolveBuyBtnStyle, type ResolvedBuyBtn } from '@/decorate/components/BuyButton';
import '@/components/product/product-card.scss';

import { useTranslation } from 'react-i18next';
/** 搜索装修的购买按钮原始配置（服务端页面算好后透传，客户端在此归一化）。 */
export interface ProductGridBuyBtnConfig {
  /** 原始 buyStyle 值：legacy 时为旧 1/2/3；新版（buyStyleV>=2）为统一枚举（可为 -1 跟随全局）。 */
  raw: unknown;
  /** true = 存量旧值，先经 mapSearchLegacyBuyStyle 映射到统一枚举。 */
  legacy?: boolean;
  /** 搜索装修 showCartButton 开关：false 时不渲染购买按钮。 */
  show?: boolean;
  /** 样式 10（自定义图片）时模块自带图标 {picUrl, picThumb}。 */
  moduleIcon?: unknown;
}

interface ProductGridProps {
  products: ProductListItem[];
  /** 搜索装修 showSales：true 时展示已售数量（默认展示库存）。 */
  showSales?: boolean;
  /** 搜索装修 showShopTag：为 true 且商品带店铺信息时展示店铺角标。 */
  showShopTag?: boolean;
  /** 搜索装修购买按钮配置（不传 = 卡片维持默认 Add 圆钮行为）。 */
  buyBtn?: ProductGridBuyBtnConfig | null;
  /**
   * 额外挂到网格根节点上的旧版类名。
   * 旧版 pages/search/components/CatGoodsList.vue 的结构是
   * .goods-list__box > .goods-list-c > .wrap，.wrap 才是网格本体
   * （grid-template-columns: repeat(var(--search-grid-columns, 5), minmax(0, 1fr))），
   * 所以搜索页把 wrap 传进来。
   */
  gridClassName?: string;
  /** 旧版 SearchProductCard.vue 的 fixed-name-two-lines：搜索页固定两行名称。 */
  fixedNameTwoLines?: boolean;
}

export default function ProductGrid({
  products,
  showSales = false,
  showShopTag = false,
  buyBtn,
  gridClassName,
  fixedNameTwoLines = false,
}: ProductGridProps) {
    const { t } = useTranslation();
  const gridRef = useRef<HTMLDivElement | null>(null);
  const columns = useProductColumns(gridRef);
  /* 购买按钮（全站统一共享组件）：整表共用一份解析结果，逐卡透传给 ProductCard。 */
  const resolvedBuyBtn: ResolvedBuyBtn | undefined = buyBtn
    ? (() => {
        const resolved = resolveBuyBtnStyle(buyBtn.legacy ? mapSearchLegacyBuyStyle(Number(buyBtn.raw) || 1) : buyBtn.raw, {
          moduleIcon: buyBtn.moduleIcon,
        });
        return buyBtn.show === false ? { ...resolved, show: false } : resolved;
      })()
    : undefined;

  return (
    <div
      className={`product-card-grid${gridClassName ? ` ${gridClassName}` : ''}`}
      ref={gridRef}
      style={{ '--you-like-columns': columns } as React.CSSProperties}
    >
      {products.map((product) => (
        <ProductCard
          key={product.productId}
          product={product as ProductCardItem}
          salesMode={showSales ? 'sales' : 'stock'}
          showShopTag={showShopTag}
          fixedNameTwoLines={fixedNameTwoLines}
          activityInfo={(product as ProductCardItem).activityInfo}
          buyBtn={resolvedBuyBtn}
        />
      ))}
      {!products.length ? <div className="search-empty"><span className="search-empty__text">{t('没有找到商品')}</span></div> : null}
    </div>
  );
}
