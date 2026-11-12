'use client';

import type { ReactNode } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { Layout, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { MEMBER_MENU } from '@/lib/member-menu';

/** 会员中心外壳：对 /member/** 所有路由生效，侧栏按 Vue 版 25 个子目录组织。 */
export default function MemberShell({ children }: { children: ReactNode }) {
    const pathname = usePathname();
    const { t } = useTranslation();

    return (
        <Layout className="pc-member">
            {/* 站点 Header 由根 layout 统一提供，会员区只保留侧栏 + 内容 */}
            <div className="pc-member-body">
                <aside className="pc-member-sider">
                    {MEMBER_MENU.map((group) => (
                        <div key={group.group} className="pc-member-group">
                            <Typography.Text type="tertiary">{group.group}</Typography.Text>
                            {group.items.map((item) => (
                                <Link
                                    key={item.path}
                                    href={item.path}
                                    className={`pc-member-link${pathname === item.path ? ' is-active' : ''}`}
                                >
                                    {t(item.label)}
                                </Link>
                            ))}
                        </div>
                    ))}
                </aside>
                <Layout.Content className="pc-member-content">{children}</Layout.Content>
            </div>
        </Layout>
    );
}
