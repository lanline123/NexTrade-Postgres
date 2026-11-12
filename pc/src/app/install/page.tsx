import type { Metadata } from 'next';
import Link from 'next/link';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '安装向导 · NexTrade PC', robots: { index: false, follow: false } };

/**
 * 安装向导。
 * Vue 基准：pages/install/（CreateData.vue 等，部署期一次性使用）。
 * 新站由后端 Rust + PostgreSQL 提供初始化，PC 前端不再承担建库流程，这里保留说明页。
 */
export default function Page() {
    return (
        <div className="pc-placeholder">
            <h1>安装向导</h1>
            <p>本版本的服务端初始化（数据库建表、管理员创建）由后端完成，前端不再执行安装流程。</p>
            <p>如需初始化数据，请在后端执行对应命令，或联系运维。</p>
            <p>
                <Link href="/">返回首页</Link>
            </p>
        </div>
    );
}
