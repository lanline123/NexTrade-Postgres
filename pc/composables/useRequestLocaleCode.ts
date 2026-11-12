/**
 * 当前 Nuxt 请求已经确定的语言码。
 *
 * SSR 首访时，`useCookie` 新建的引用只能稳定读取传入请求里的 Cookie；语言初始化在
 * 本次响应阶段写入的 Cookie 不应再被同一请求里的业务接口当作唯一数据源。使用
 * `useState` 可以让语言 Store 与统一请求封装共享同一个请求级值，并自动随 SSR 状态水合。
 */
export const useRequestLocaleCode = () => useState<string>("i18n:request-locale-code", () => "");
