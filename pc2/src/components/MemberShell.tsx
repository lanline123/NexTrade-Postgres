'use client';

import type { ReactNode } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { Layout, Tag, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import LocaleSwitch from '@/components/LocaleSwitch';
import { MEMBER_MENU } from '@/lib/member-menu';

/** 会员中心外壳：对 /member/** 所有路由生效，侧栏按 Vue 版 25 个子目录组织。 */
export default function MemberShell({ children }: { children: ReactNode }) {
    const pathname = usePathname();
    const { t } = useTranslation();

    return (
        <Layout className="pc2-member">
            <Layout.Header className="pc2-topbar">
                <div className="pc2-brand">
                    NexTrade PC2
                    <small>{t('会员中心')}</small>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                    <Tag size="small" color="blue">
                        React + Semi SSR
                    </Tag>
                    <LocaleSwitch />
                </div>
            </Layout.Header>

            <div className="pc2-member-body">
                <aside className="pc2-member-sider">
                    {MEMBER_MENU.map((group) => (
                        <div key={group.group} className="pc2-member-group">
                            <Typography.Text type="tertiary">{group.group}</Typography.Text>
                            {group.items.map((item) => (
                                <Link
                                    key={item.path}
                                    href={item.path}
                                    className={`pc2-member-link${pathname === item.path ? ' is-active' : ''}`}
                                >
                                    {t(item.label)}
                                </Link>
                            ))}
                        </div>
                    ))}
                </aside>
                <Layout.Content className="pc2-member-content">{children}</Layout.Content>
            </div>
        </Layout>
    );
}
