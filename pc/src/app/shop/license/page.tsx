import type { Metadata } from 'next';
import ShopLicense from '@/components/shop/ShopLicense';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '网店经营者营业执照信息 · NexTrade PC' };

/** Vue 基准：pages/shop/license.vue（独立白页，无站点头尾，`?shopId=`） */
export default function Page({ searchParams }: { searchParams?: { shopId?: string } }) {
    return <ShopLicense shopId={Number(searchParams?.shopId) || 0} />;
}
