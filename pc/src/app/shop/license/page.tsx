import type { Metadata } from 'next';
import Link from 'next/link';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '店铺资质 · NexTrade PC' };

/**
 * 店铺资质。
 * Vue 基准：pages/shop/license.vue（营业执照/资质证书展示）。
 * 当前为说明页：资质数据接口未确认，先从店铺主页进入查看。
 */
export default function Page() {
    return (
        <div className="pc-placeholder">
            <h1>店铺资质</h1>
            <p>资质证书展示待接入店铺资质接口，可从店铺主页查看店铺信息与商品。</p>
            <p>
                <Link href="/shop/list">全部店铺</Link>
                <span> · </span>
                <Link href="/">返回首页</Link>
            </p>
        </div>
    );
}
