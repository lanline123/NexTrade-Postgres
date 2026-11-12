import type { Metadata } from 'next';
import InquiryList from '@/components/member/InquiryList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '询价单 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/inquiry/List.vue */
export default function Page() {
    return <InquiryList />;
}
