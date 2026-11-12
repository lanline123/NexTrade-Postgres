import MemberOverview from '@/components/MemberOverview';

// 会员中心走按需 SSR；后续接入用户态/订单统计时在此服务端组件里 await。
export const dynamic = 'force-dynamic';

export const metadata = {
    title: '会员中心 · NexTrade PC2'
};

export default function MemberHomePage() {
    return <MemberOverview />;
}
