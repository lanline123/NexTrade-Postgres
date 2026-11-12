import type { Metadata } from 'next';
import ProductShell from '@/components/product/ProductShell';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '商品详情 · NexTrade PC' };

/**
 * Vue 基准：pages/product/[...slug].vue —— SEO 路径，**写死跨境版**（ui-version="overseas"）。
 * 后台「商品页风格」切换走 /item/:id（pages/item/[id].vue）。
 * ?uiStyle=old|new|overseas 可强制预览任意一套（Vue 仅 dev 放开，这里常开便于验收）。
 */
export default function Page({
    params,
    searchParams
}: {
    params: { slug?: string[] };
    searchParams?: { uiStyle?: string };
}) {
    const idOrSn = decodeURIComponent((params.slug ?? [])[0] ?? '');
    const preview = searchParams?.uiStyle;
    const force = preview === 'old' || preview === 'new' || preview === 'overseas' ? preview : undefined;
    return <ProductShell idOrSn={idOrSn} forceVersion={force} />;
}
