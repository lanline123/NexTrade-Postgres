import type { Metadata } from 'next';
import Link from 'next/link';
import { findMemberSource } from '@/lib/member-menu';

export const metadata: Metadata = {
    title: '会员中心 · 待迁移'
};

/**
 * 会员中心子路由兜底：Vue 版 66 个 .vue 对应的 URL 全部有 React 承接，不会 404。
 * 逐页迁移时在 src/app/member/<具体路由>/page.tsx 建文件，本兜底自动让位。
 */
export default function MemberSubPage({ params }: { params: { slug?: string[] } }) {
    const slug = (params.slug ?? []).join('/');
    const source = findMemberSource(slug);

    return (
        <div className="pc-placeholder">
            <h1>会员中心 / {slug}</h1>
            <p>
                路由已由 React 侧承接，页面内容待按 Vue 版 1:1 重写。
            </p>
            <p>
                原 Vue 页面：<code>{source ?? `pages/member/${slug}(对应文件)`}</code>
            </p>
            <p>
                多语言用 <code>useTranslation()</code> 的 <code>t()</code>，不要引入 Vue 的 <code>$t</code>。
            </p>
            <Link href="/member">← 返回会员中心</Link>
        </div>
    );
}
