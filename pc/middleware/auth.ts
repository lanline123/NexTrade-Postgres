
export default defineNuxtRouteMiddleware((to, from) => {
    // 使用 cookie 进行认证
    const token = useCookie<string | null>("accessToken");
    if (!token.value) {
        // 获取上一页的路径
        const previousPage = to.fullPath;
        // 重定向到登录页，并带上返回的路径作为参数
        return navigateTo(`/member/login?returnUrl=${previousPage}`);
    }
});
