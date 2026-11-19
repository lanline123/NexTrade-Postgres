import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { Suspense } from 'react';
import PcHeader from '@/components/PcHeaderWithData';
import PcFooter from '@/components/PcFooter';
import ProductDetailClient from '@/components/ProductDetailClient';
import { ProductSkeleton } from '@/components/product/ProductLayouts';
import VisitLogger from '@/components/VisitLogger';
import { extractProductSnFromSlug, getPcHeader, getProductDetailStyle, loadProductBySlug, resolveUiStyle, type ProductUiStyle } from '@/lib/api';
import { cookies } from 'next/headers';
import { currencyCodeFrom } from '@/lib/currency';

/**
 * 商品详情页（服务端）
 * 源码对照：pages/product/[...slug].vue —— URL 使用商品 SN（可带 "." 前缀的 SEO 片段），
 * 由 utils/seo-url.ts extractProductSnFromSlug 解析后请求 product/product/detail?sn=
 */
const SITE_URL = (process.env.PC_SITE_URL || 'https://battery.recy.cc').replace(/\/$/, '');

export function normalizeSlug(slug: string | string[] | undefined): string[] {
  if (Array.isArray(slug)) return slug.map((item) => String(item));
  if (typeof slug === 'string' && slug) return [slug];
  return [];
}

type ProductDetail = Awaited<ReturnType<typeof loadProductBySlug>>;

/**
 * 商品 meta description：简介优先；为空时取详情正文（descArr 的 html 条目）
 * 去标签后的前 150 字；再空才退关键词/商品名。避免大量商品描述=商品名。
 */
function resolveProductDescription(detail: ProductDetail): string {
  const item = detail?.item;
  if (!item) return '';
  const brief = String(item.productBrief || '').trim();
  if (brief) return brief;
  const html = (detail?.descArr || [])
    .filter((d) => d && d.type !== 'pic' && !d.pic && d.html)
    .map((d) => String(d.html))
    .join(' ');
  const text = html
    .replace(/<[^>]*>/g, ' ')
    .replace(/&nbsp;/gi, ' ')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/&quot;/gi, '"')
    .replace(/&#39;|&apos;/gi, "'")
    .replace(/&amp;/gi, '&')
    .replace(/\s+/g, ' ')
    .trim();
  if (text) return Array.from(text).slice(0, 150).join('');
  return item.keywords || item.productName || '';
}

export async function buildProductMetadata(slug: string | string[] | undefined): Promise<Metadata> {
  const sn = extractProductSnFromSlug(slug);
  const detail = await loadProductBySlug(sn);
  const item = detail?.item;
  if (!item) return { title: '商品不存在 · 万物新生' };
  const path = `/product/${normalizeSlug(slug).join('/')}`;
  const description = resolveProductDescription(detail);
  return {
    title: `${item.productName} · 万物新生`,
    description,
    alternates: { canonical: path },
    openGraph: {
      type: 'website',
      url: path,
      title: item.productName,
      description,
      images: item.picLarge ? [item.picLarge] : undefined,
    },
    twitter: {
      card: 'summary_large_image',
      title: item.productName,
      description,
      images: item.picLarge ? [item.picLarge] : undefined,
    },
  };
}

export default async function ProductDetailView({
  slug,
  uiStyleParam,
}: {
  slug: string | string[];
  uiStyleParam?: string;
}) {
  const segments = normalizeSlug(slug);
  const sn = extractProductSnFromSlug(segments);
  const detail = await loadProductBySlug(sn);
  if (!detail?.item) notFound();
  const [header, configuredStyle] = await Promise.all([
    getPcHeader().catch(() => null),
    getProductDetailStyle(),
  ]);
  const uiStyle: ProductUiStyle = resolveUiStyle(uiStyleParam, configuredStyle);
  const item = detail.item;
  // 结构化数据的价格按当前换币后的币种输出（与页面展示一致）
  let currencyRate = 1;
  let currencyCode = 'CNY';
  try {
    const raw = cookies().get('currentCurrencyData')?.value;
    if (raw) {
      const parsed = JSON.parse(decodeURIComponent(raw));
      const rate = Number(parsed?.rate);
      if (Number.isFinite(rate) && rate > 0) currencyRate = rate;
      currencyCode = currencyCodeFrom(parsed?.name, parsed?.symbol);
    }
  } catch {
    /* cookie 缺失/损坏按人民币 */
  }
  // 库存口径与详情页一致（ProductDetailClient：selectedSku.skuStock ?? item.productStock）：
  // 有 SKU 时取各 SKU 的 skuStock 汇总，无 SKU 时取 item.productStock。
  // 旧代码误读 item.stock（详情数据无此字段），导致 JSON-LD 恒为 OutOfStock。
  const skuList = detail.skuList || [];
  const totalStock = skuList.length
    ? skuList.reduce((sum, sku) => sum + (Number(sku?.skuStock) || 0), 0)
    : Number(item?.productStock || 0);
  const productJsonLd = {
    '@context': 'https://schema.org',
    '@type': 'Product',
    name: item?.productName,
    image: item?.picLarge ? [item.picLarge] : undefined,
    description: resolveProductDescription(detail),
    sku: item?.productSn,
    offers: {
      '@type': 'Offer',
      price: Number((Number(item?.productPrice || 0) * currencyRate).toFixed(2)),
      priceCurrency: currencyCode,
      availability: totalStock > 0 ? 'https://schema.org/InStock' : 'https://schema.org/OutOfStock',
      url: `${SITE_URL}/product/${segments.join('/')}`,
    },
  };

  return (
    <>
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(productJsonLd) }} />
      {/* 商品曝光埋点（对齐旧版商品页 StatisticLog productId 上报） */}
      <VisitLogger productId={item?.productId || item?.id || ''} />
      <PcHeader config={header as any} />
      <div className="pdp-page">
        <Suspense fallback={<ProductSkeleton uiStyle={uiStyle} />}>
          <ProductDetailClient detail={detail as any} uiStyle={uiStyle} />
        </Suspense>
      </div>
      <PcFooter />
    </>
  );
}
