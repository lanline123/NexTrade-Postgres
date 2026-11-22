'use client';

import { useEffect } from 'react';
import { usePathname, useRouter } from 'next/navigation';

/**
 * 会员中心登录兜底守卫，对应旧版 Nuxt middleware: auth。
 *
 * 服务端拦截已由 middleware.ts（cookie 校验）完成，本组件不再渲染任何加载层：
 * - 挂载时若 localStorage 无 token 而 cookie 有（登录 7 天内换浏览器场景反向兜底），把 cookie 同步回 localStorage；
 * - 两者都无（middleware 漏网）才跳登录页。
 * 页面内容始终直出，不再出现「正在校验登录状态...」闪烁。
 */
export default function MemberAuthGuard({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const pathname = usePathname();

  useEffect(() => {
    // 未登录可达页（登录/注册/找回密码/微信首登绑手机）不做跳转，
    // 与 middleware.ts 的放行名单保持一致：找回密码与首登绑手机都没有
    // token，守卫若拦截会把用户弹回登录页、流程中断。
    if (/^\/member\/(login|register|reset|bindmobilephone)\/?$/.test(pathname || '')) return;
    let cookieToken: string | undefined;
    document.cookie.split(';').some((part) => {
      const [name, ...rest] = part.trim().split('=');
      if (name === 'accessToken') {
        cookieToken = decodeURIComponent(rest.join('='));
        return true;
      }
      return false;
    });
    const localToken = localStorage.getItem('accessToken');
    if (!localToken && cookieToken) {
      localStorage.setItem('accessToken', cookieToken);
      return;
    }
    if (!localToken && !cookieToken) {
      const redirect = `${pathname || '/member'}${window.location.search || ''}`;
      router.replace(`/login?redirect=${encodeURIComponent(redirect)}`);
    }
  }, [pathname, router]);

  return <>{children}</>;
}
