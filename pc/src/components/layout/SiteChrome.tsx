'use client';

import { usePathname } from 'next/navigation';
import type { ReactNode } from 'react';
import SiteHeader from '@/components/layout/SiteHeader';
import SiteFooter from '@/components/layout/SiteFooter';

/**
 * 站点头尾 + 主内容区。
 *
 * Vue 里有几个「独立白页」不带头尾（营业执照公示 `/shop/license`、安装向导 `/install`）。
 * App Router 的 layout 逐层继承，没法在某条路由上「取消」根 layout 的 chrome，
 * 所以把 chrome 收在这个客户端组件里，按路径判断要不要渲染头尾。
 */
const BARE_PATHS = ['/shop/license', '/install'];

export default function SiteChrome({ children }: { children: ReactNode }) {
    const pathname = usePathname() ?? '';
    const bare = BARE_PATHS.some((path) => pathname === path || pathname.startsWith(`${path}/`));

    if (bare) return <main className="pc-main pc-main-bare">{children}</main>;

    return (
        <>
            <SiteHeader />
            <main className="pc-main">{children}</main>
            <SiteFooter />
        </>
    );
}
