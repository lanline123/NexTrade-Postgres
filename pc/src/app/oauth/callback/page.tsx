import type { Metadata } from 'next';
import Link from 'next/link';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '登录回调 · NexTrade PC', robots: { index: false, follow: false } };

/**
 * 第三方登录回调。
 * Vue 基准：pages/oauth/callback.vue。
 * 微信/支付宝等第三方登录需要开放平台配置与后端换取 token，当前未接入，
 * 这里给出明确说明而不是空白页。接入时在此读取 code 并调后端回调接口。
 */
export default function Page() {
    return (
        <div className="pc-placeholder">
            <h1>登录回调</h1>
            <p>第三方登录（微信/支付宝等）尚未接入，当前请使用账号密码或手机验证码登录。</p>
            <p>
                <Link href="/member/login">去登录</Link>
                <span> · </span>
                <Link href="/">返回首页</Link>
            </p>
        </div>
    );
}
