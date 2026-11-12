import type { Metadata } from 'next';
import InvoiceList from '@/components/member/InvoiceList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '发票管理 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/userInvoice/List.vue、orderInvoice/List.vue */
export default function Page() {
    return <InvoiceList />;
}
