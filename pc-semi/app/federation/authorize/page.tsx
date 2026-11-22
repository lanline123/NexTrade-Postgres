import type { Metadata } from 'next';
import PcHeader from '@/components/PcHeader';
import PcFooter from '@/components/PcFooter';
import FederationAuthorizeClient from '@/components/FederationAuthorizeClient';
import { getPcHeader } from '@/lib/api';

import { serverDecorateLocaleCode } from '@/i18n/server';
export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '联邦登录授权 · 万物新生', robots: { index: false, follow: false } };

/**
 * 联邦登录授权页（我方新增，路由 /federation/authorize?requestId=<id>）。
 * 中枢侧同意面：成员站发起联邦登录后，会员浏览器跳到中枢本站本页，
 * 本人登录态下查看申请方并同意/拒绝（后端 /api/federation/sso/*，
 * 设计 ~/workspace/federation-phase3/DESIGN.md §1.2）。页面壳与礼物
 * 领取页同构（顶栏 + 静态卡 + 底栏），逻辑全在客户端组件。
 */
export default async function FederationAuthorizePage() {
  const header = await getPcHeader(undefined, serverDecorateLocaleCode()).catch(() => null);
  return (
    <>
      <PcHeader config={header as any} />
      <main className="pc-static-page">
        <div className="pc-container">
          <div className="pc-static-card">
            <FederationAuthorizeClient />
          </div>
        </div>
      </main>
      <PcFooter />
    </>
  );
}
