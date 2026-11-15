import type { Metadata } from 'next';
import PcFooter from '@/components/PcFooter';
import RegisterClient from '@/components/RegisterClient';
import { getHeaderBootstrap, getPcLoginDecorate } from '@/lib/api';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '注册 · 万物新生' };

/**
 * PC 注册页（与登录页同构：双栏卡片整屏构图）
 *
 * 不再渲染独立白条 PcLoginHeader：logo 与「已有账号？去登录」互切链接由
 * RegisterClient 的悬浮顶栏（.login-card__topbar）承载，与登录页完全一致。
 * 店铺 logo / 名称来自 getHeaderBootstrap（common/config/base）；
 * 登录页装修配置仅作传参兼容保留（RegisterClient 不再使用其背景图/背景色）。
 * PcFooter 保留在整屏主体之下（滚动可见）。
 */
export default async function RegisterPage() {
  const [decorate, bootstrap] = await Promise.all([
    getPcLoginDecorate().catch(() => null),
    getHeaderBootstrap().catch(() => null),
  ]);
  const base = bootstrap?.base || null;
  return (
    <>
      <RegisterClient
        shopName={String(base?.shopName || base?.shopTitle || '万物新生')}
        shopLogo={String(base?.shopLogo || '')}
        decorate={decorate}
      />
      <PcFooter />
    </>
  );
}
