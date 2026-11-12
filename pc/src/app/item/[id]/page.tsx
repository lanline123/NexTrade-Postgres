import type { Metadata } from 'next';
import ProductShell from '@/components/product/ProductShell';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '商品详情 · NexTrade PC' };

/**
 * Vue 基准：pages/item/[id].vue —— 读后台「商品页风格」切换三套 UI：
 * 1=经典(old) 2=新版(new) 3=跨境(overseas)；?uiStyle= 强制预览。
 * 之前被重定向到 /product/:id（写死跨境版），现恢复为真实页面以支持风格切换。
 */
export default function Page({
    params,
    searchParams
}: {
    params: { id?: string };
    searchParams?: { uiStyle?: string; grouponPromotionRecordId?: string; type?: string };
}) {
    const preview = searchParams?.uiStyle;
    const force = preview === 'old' || preview === 'new' || preview === 'overseas' ? preview : undefined;
    return (
        <ProductShell
            idOrSn={decodeURIComponent(params.id ?? '')}
            forceVersion={force}
            grouponRecordId={Number(searchParams?.grouponPromotionRecordId) || 0}
            invited={(searchParams?.type ?? '') === 'invited'}
        />
    );
}
