'use client';

import Link from 'next/link';
import { Button, Card, Tag, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { MEMBER_MENU } from '@/lib/member-menu';

/** 会员中心首页概览。数据接口尚未接入，先把结构与多语言链路跑通。 */
export default function MemberOverview() {
    const { t } = useTranslation();
    const total = MEMBER_MENU.reduce((sum, group) => sum + group.items.length, 0);

    return (
        <div className="pc2-member-page">
            <Typography.Title heading={3}>{t('会员中心')}</Typography.Title>
            <Typography.Paragraph type="tertiary">
                React 18 + Next 14 SSR + Semi Design 重写版。左侧 {total} 个入口已按 Vue 版目录建好路由，
                未逐页迁移的子页由 [...]slug 占位承接，不会出现 404。
            </Typography.Paragraph>

            <div className="pc2-overview-grid">
                {MEMBER_MENU.map((group) => (
                    <Card key={group.group} title={group.group} className="pc2-overview-card">
                        <div className="pc2-overview-links">
                            {group.items.map((item) => (
                                <Link key={item.path} href={item.path} className="pc2-overview-link">
                                    {t(item.label)}
                                </Link>
                            ))}
                        </div>
                    </Card>
                ))}
            </div>

            <Card className="pc2-overview-note" title="迁移进度">
                <div style={{ display: 'flex', alignItems: 'center', gap: 12, flexWrap: 'wrap' }}>
                    <Tag color="green" size="small">
                        已完成：外壳 / 侧栏 / 首页 / 占位路由 / react-i18next
                    </Tag>
                    <Tag color="blue" size="small">
                        待迁移：Vue 侧 66 个 .vue
                    </Tag>
                    <Link href="/">
                        <Button theme="light" type="primary" size="small">
                            返回迁移看板
                        </Button>
                    </Link>
                </div>
                <Typography.Paragraph type="tertiary" style={{ marginTop: 12 }}>
                    逐页重写时：在 src/app/member/&lt;子路由&gt;/page.tsx 落地实现，占位路由自动让位；
                    文案统一用 useTranslation() 的 t()，不要再引入 Vue 的 $t。
                </Typography.Paragraph>
            </Card>
        </div>
    );
}
