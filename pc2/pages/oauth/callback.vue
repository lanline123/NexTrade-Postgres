<!-- OAuth 授权回跳页（跨境社交登录闭环：授权服务器 → /oauth/callback → 换 token → 落 userStore → 回原路径）。
     签名差异：o2o `getOauthCallback` 为 3 参 `(path, redirectUri, data)`（`OverseasLogin.v1.vue` 归档仍在用），本页跟随 3 参签名。 -->
<template></template>

<script setup lang="ts">
import { getOauthCallback } from "@/api/user/login";
import { useUserStore } from "~/store/user";

const { t } = useI18n();
const route = useRoute();
const userStore = useUserStore();
const oauthPath = useCookie<string>("oauthPath");

const getSingleQueryValue = (value: unknown) => {
    if (typeof value === "string") {
        return value;
    }
    if (Array.isArray(value) && typeof value[0] === "string") {
        return value[0];
    }
    return "";
};

const redirectAfterLogin = async (url: string) => {
    const target = url || "/";
    if (/^https?:\/\//i.test(target)) {
        window.location.href = target;
        return;
    }
    await navigateTo(target, { replace: true });
};

const redirectToLogin = async (messageText: string) => {
    oauthPath.value = "";
    message.error(messageText);
    await navigateTo("/member/login", { replace: true });
};

const handleOauthCallback = async () => {
    const code = getSingleQueryValue(route.query.code);
    const state = getSingleQueryValue(route.query.state);
    const oauthProvider = oauthPath.value;

    if (!oauthProvider) {
        await redirectToLogin(t("缺少授权方式"));
        return;
    }

    if (!code || !state) {
        await redirectToLogin(t("缺少授权参数"));
        return;
    }

    try {
        const redirectUri = typeof window !== "undefined" ? `${window.location.origin}/oauth/callback` : "";
        const result = await getOauthCallback(oauthProvider, redirectUri, { code, state });
        userStore.setToken(result.token);
        await userStore.validateUserInfoOnce();
        oauthPath.value = "";
        await redirectAfterLogin(result.returnUrl || "/");
    } catch (error: any) {
        console.error(error);
        await redirectToLogin(error?.message || t("授权失败"));
    }
};

onMounted(() => {
    handleOauthCallback();
});
</script>
