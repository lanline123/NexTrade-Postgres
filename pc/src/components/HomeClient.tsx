'use client';

import { Button, Card, Space, Table, Tag, Typography } from '@douyinfe/semi-ui';
import LocaleSwitch from '@/components/LocaleSwitch';
import type { MigrationRoute } from '@/lib/routes';

type HomeClientProps = {
    routes: MigrationRoute[];
    done: number;
    renderedAt: string;
};

const STATUS_META: Record<MigrationRoute['status'], { color: 'green' | 'blue' | 'amber'; text: string }> = {
    done: { color: 'green', text: '已迁移' },
    in_progress: { color: 'amber', text: '迁移中' },
    pending: { color: 'blue', text: '待迁移' }
};

export default function HomeClient({ routes, done, renderedAt }: HomeClientProps) {
    const columns = [
        {
            title: '路由',
            dataIndex: 'path',
            width: 160,
            render: (text: string) => <Typography.Text strong>{text}</Typography.Text>
        },
        { title: '页面', dataIndex: 'label', width: 150 },
        {
            title: '原 Vue 路径',
            dataIndex: 'source',
            render: (text: string) => <code>{text}</code>
        },
        { title: '.vue 数', dataIndex: 'vueFiles', width: 100 },
        {
            title: '状态',
            dataIndex: 'status',
            width: 110,
            render: (_: string, record: MigrationRoute) => {
                const meta = STATUS_META[record.status];
                return (
                    <Tag color={meta.color} size="small">
                        {meta.text}
                    </Tag>
                );
            }
        }
    ];

    return (
        <>
            <header className="pc-topbar">
                <div className="pc-brand">
                    NexTrade PC
                    <small>React 18 + Next 14 SSR + Semi Design</small>
                </div>
                <Space>
                    <LocaleSwitch />
                    <Button theme="solid" type="primary" onClick={() => window.location.reload()}>
                        刷新验证 SSR
                    </Button>
                </Space>
            </header>

            <main className="pc-shell">
                <div className="pc-hero">
                    <Typography.Title heading={2}>PC 前端重构 · 迁移看板</Typography.Title>
                    <Typography.Paragraph type="tertiary">
                        pc 目录原地重写：Vue 3 源码保留在 pages/、components/、decorate/ 作 1:1 参照（运行时依赖已移除，不参与构建），
                        React 侧代码在 src/app/。
                    </Typography.Paragraph>
                </div>

                <div className="pc-meta">
                    <span>
                        服务端渲染时间：<code>{renderedAt}</code>（刷新会变 = SSR 生效）
                    </span>
                    <span>
                        进度：<code>{done}/{routes.length}</code> 个顶层路由已迁移
                    </span>
                    <span>
                        参照源码：<code>447 个 .vue</code>
                    </span>
                </div>

                <Card title="路由迁移清单（按 .vue 文件数降序 = 建议开工顺序）">
                    <Table columns={columns} dataSource={routes} rowKey="path" pagination={false} />
                </Card>

                <div className="pc-note">
                    <strong>迁移约定</strong>
                    <br />
                    1. 每迁一页，在 <code>src/app/&lt;路由&gt;/page.tsx</code> 落地，并把 <code>src/lib/routes.ts</code> 里对应 status 改为 done。
                    <br />
                    2. 需要交互 / Semi 组件的抽成 <code>'use client'</code> 组件；数据获取留在服务端组件里 await。
                    <br />
                    3. 多语言统一 <code>react-i18next</code> 的 <code>useTranslation() / t()</code>，语言 cookie 为 <code>pc_locale</code>，SSR 首屏按 cookie 出正确语种。
                    <br />
                    4. 本地起服务 <code>npm run dev</code>（3006 端口）；Vue 源码只用来读，不再运行。
                </div>
            </main>
        </>
    );
}
