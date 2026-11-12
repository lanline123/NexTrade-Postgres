import MemberHome from '@/components/member/MemberHome';

// 会员中心走按需 SSR；用户态/统计由客户端组件按 token 拉取（面板按需取数）。
export const dynamic = 'force-dynamic';

export const metadata = {
    title: '会员中心 · NexTrade PC'
};

/** Vue 基准：pages/member/index.vue */
export default function MemberHomePage() {
    return <MemberHome />;
}
