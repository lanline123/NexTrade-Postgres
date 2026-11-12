import { isOverseas } from "@/utils/util";

/** 未登录访问购物车时跳转登录页，并保留返回地址。 */
export default defineNuxtRouteMiddleware((to) => {
    if (!isOverseas()) return;
    const token = useCookie<string | null>("accessToken");
    if (!token.value) {
        return navigateTo(`/member/login?returnUrl=${to.fullPath}`);
    }
});
