import Link from 'next/link';

/** 全局 404。Vue 基准：pages/[...slug].vue（原站的兜底页）。 */
export default function NotFound() {
    return (
        <div className="pc-placeholder">
            <h1>404</h1>
            <p>页面不存在或已被移除。</p>
            <p>
                <Link href="/">返回首页</Link>
                <span> · </span>
                <Link href="/member/order/list">我的订单</Link>
                <span> · </span>
                <Link href="/list">去逛商品</Link>
            </p>
        </div>
    );
}
