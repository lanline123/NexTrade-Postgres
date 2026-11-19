import { NextRequest, NextResponse } from 'next/server';
import { LOCALE_COOKIE, localizedPath, splitLocalePrefix } from './src/i18n/locale';

/**
 * 中间件做两件事（顺序很重要）：
 *
 * 1) 旧驼峰 URL → 全小写 URL（308）。
 *    背景：2026-11-07 起 PC 路由目录统一为全小写，旧站（vue-router 大小写不敏感）对外暴露的
 *    驼峰链接需要 301/308 到新路径。**这一步必须放在中间件里**，不能写成 next.config 的
 *    redirects：Next 的 redirects 规则匹配是大小写不敏感的，`/member/coupon/List` 这条规则
 *    会连 `/member/coupon/list` 一起命中，于是把页面 308 到它自己，浏览器报
 *    ERR_TOO_MANY_REDIRECTS（实测过：/member/coupon/list、/article/news/list 等都打不开）。
 *    这里用 JS 比较 pathname 与 pathname.toLowerCase()，只有真的含大写字母才跳，不会自指。
 *
 * 2) 会员中心服务端登录守卫（对应旧版 Nuxt middleware: auth）。
 *    token 登录后同时写入 localStorage 与 accessToken cookie（LoginClient / oauth callback）。
 *    无 token 或 JWT 已过期 → 307 到 /login?redirect=<原路径+query>；有效 → 直接 SSR。
 *    注意：cookie 缺失但 localStorage 仍有 token 的窗口（例如登录超过 7 天）会被要求重新登录，
 *    由客户端 MemberAuthGuard 负责把 cookie 同步回 localStorage，尽量减少该窗口。
 */

function isExpiredToken(token: string): boolean {
  try {
    const payloadPart = token.split('.')[1] ?? '';
    const normalized = payloadPart.replace(/-/g, '+').replace(/_/g, '/');
    const payload = JSON.parse(atob(normalized)) as { exp?: number };
    if (typeof payload.exp === 'number') return payload.exp * 1000 < Date.now();
    return false;
  } catch {
    // 非标准 JWT（如后端下发的非三段式 token）：不在此判断，放行由接口层兜底
    return false;
  }
}

/**
 * 构造对外的绝对 URL。
 *
 * Next 14 的 `request.nextUrl` 用的是**进程自身监听地址**（生产由
 * `next start -H 127.0.0.1 -p 3005` 启动，见 next-server.js 的 initURL），
 * 不是反向代理转发来的请求 Host；直接 `nextUrl.clone()` 会把 308/307
 * 指到 `localhost:3005`，浏览器打不开。因此重定向必须按请求头取对外地址：
 * 优先 `x-forwarded-host/proto`（nginx 设置），回退到 `host`。
 */
function externalUrl(request: NextRequest, pathname: string, search = ''): URL {
  const proto =
    request.headers.get('x-forwarded-proto') ?? request.nextUrl.protocol.replace(/:$/, '');
  const host =
    request.headers.get('x-forwarded-host') ?? request.headers.get('host') ?? request.nextUrl.host;
  return new URL(`${pathname}${search}`, `${proto}://${host}`);
}

/** 放行并注入内部请求头：x-pathname=内部（剥前缀后）路径，供 layout 生成 canonical/hreflang。 */
function passThrough(request: NextRequest, internalPath: string, prefixLocale: string | null) {
  const requestHeaders = new Headers(request.headers);
  requestHeaders.set('x-pathname', internalPath);
  if (prefixLocale) requestHeaders.set('x-locale', prefixLocale);
  return NextResponse.next({ request: { headers: requestHeaders } });
}

export function middleware(request: NextRequest) {
  const { pathname, search } = request.nextUrl;

  // 1) 大小写归一化：只在实际存在大写字母时跳，目标就是小写路径本身，绝不会自指。
  const lowerPath = pathname.toLowerCase();
  if (pathname !== lowerPath) {
    return NextResponse.redirect(externalUrl(request, lowerPath, search), 308);
  }

  // 2) 语言前缀（SEO hreflang）：/en/... 剥离前缀 rewrite 到内部路径，
  //    并注入 x-locale 请求头（服务端语言判定最高优先级），同时种 locale cookie，
  //    让后续无前缀导航与客户端 i18n 保持该语言。无前缀路径行为与此前完全一致。
  const { locale: prefixLocale, path: internalPath } = splitLocalePrefix(pathname);

  // 3) 会员中心登录守卫（按内部路径判断；/login、/register 及其历史别名直接放行）
  if (internalPath === '/member' || internalPath.startsWith('/member/')) {
    if (/^\/member\/(login|register)\/?$/.test(internalPath)) {
      return passThrough(request, internalPath, prefixLocale);
    }
    const token = request.cookies.get('accessToken')?.value;
    if (!token || isExpiredToken(token)) {
      const url = externalUrl(request, localizedPath('/login', prefixLocale ?? undefined));
      url.searchParams.set('redirect', `${pathname}${search}`);
      return NextResponse.redirect(url);
    }
  }

  if (prefixLocale) {
    const url = request.nextUrl.clone();
    url.pathname = internalPath;
    const requestHeaders = new Headers(request.headers);
    requestHeaders.set('x-locale', prefixLocale);
    requestHeaders.set('x-pathname', internalPath);
    const response = NextResponse.rewrite(url, { request: { headers: requestHeaders } });
    response.cookies.set(LOCALE_COOKIE, prefixLocale, {
      path: '/',
      maxAge: 31536000,
      sameSite: 'lax',
    });
    return response;
  }

  return passThrough(request, pathname, null);
}

export const config = {
  /**
   * 除了静态资源与接口转发，其余路径都要过一遍大小写归一化与语言前缀处理。
   * /api、/adminapi 由 next.config 的 rewrites 转发，/static 由 Next 直供 public/，都不能在这里改写大小写。
   * /mobile、/pcmobile 是 H5 静态 SPA 与其回归壳，绝不能被语言前缀改写。
   */
  matcher: [
    '/((?!api|adminapi|static|assets|_next/static|_next/image|favicon.ico|robots.txt|sitemap.xml|mobile|pcmobile|manifest.webmanifest|icon.png|apple-icon.png).*)',
  ],
};
