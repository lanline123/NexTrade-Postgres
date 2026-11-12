import type { Metadata } from 'next';
import RechargeChannelPage from '@/components/member/RechargeChannelPage';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '充值支付 · NexTrade PC', robots: { index: false, follow: false } };

/**
 * Vue 基准：pages/member/account/src/{alipay,paypal,wechat,yabanpay,yunpay,lakala}.vue
 * （这 6 个文件在 Vue 里是真实路由，路径含 `/src/`，不在路由盘点脚本的常规目录里）
 */
export default function Page({
    params,
    searchParams
}: {
    params: { channel?: string };
    searchParams?: { id?: string; type?: string; payCode?: string };
}) {
    return (
        <RechargeChannelPage
            channel={(params.channel ?? '').toLowerCase()}
            orderId={Number(searchParams?.id) || 0}
            payCode={searchParams?.payCode}
        />
    );
}
