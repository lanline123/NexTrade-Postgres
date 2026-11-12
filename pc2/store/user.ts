import { deleteCookie, setCookie } from "h3";
import { getUser } from "~/api/user/user";
import { userLogout } from "~/api/user/login";
import type { UserFormState } from "~/types/user/user";
import { useCartStore } from "~/store/cart";
import {
    ACCESS_TOKEN_COOKIE,
    ACCESS_TOKEN_MAX_AGE,
    buildAccessTokenCookieOps,
    serializeAccessTokenCookieOp
} from "~/utils/accessTokenCookie";
import { runLogoutCleanup } from "~/utils/session";
import { sharedCookieDomain } from "~/utils/cookieDomain";

export const useUserStore = defineStore("user", () => {
    const userInfo = ref({} as UserFormState);

    const userInfoLoaded = ref(false);
    const userInfoValidated = ref(false);
    /** 进行中的用户信息校验，避免登录后多处并发请求互相覆盖 validated 状态 */
    let validatingPromise: Promise<void> | null = null;
    /** 用户信息请求代次；切换/清除 token 时递增，使此前发出的请求失效 */
    let userInfoRequestGeneration = 0;

    const openData = ref(null);

    // 只用来读取首屏已有 Cookie；业务侧 token 用独立 ref，避免 useCookie 再写一份 host-only
    const cookieToken = useCookie<string | null>(ACCESS_TOKEN_COOKIE, { readonly: true });
    const token = ref<string | null>(cookieToken.value ?? null);

    const persistToken = (value: string | null) => {
        let domain: string | undefined;
        domain = sharedCookieDomain();
        const ops = buildAccessTokenCookieOps(value, domain);
        if (import.meta.client) {
            for (const op of ops) document.cookie = serializeAccessTokenCookieOp(op);
        } else {
            const event = useRequestEvent();
            if (event) {
                for (const op of ops) {
                    if (op.type === "expire") {
                        deleteCookie(event, ACCESS_TOKEN_COOKIE, { path: "/", ...(op.domain ? { domain: op.domain } : {}) });
                    } else {
                        setCookie(event, ACCESS_TOKEN_COOKIE, op.value, {
                            path: "/",
                            maxAge: ACCESS_TOKEN_MAX_AGE,
                            sameSite: "lax",
                            ...(op.domain ? { domain: op.domain } : {})
                        });
                    }
                }
            }
        }
        token.value = value;
    };

    const resetUserInfoCache = () => {
        userInfoRequestGeneration++;
        userInfoLoaded.value = false;
        userInfoValidated.value = false;
        userInfo.value = {} as UserFormState;
        validatingPromise = null;
    };


    const isCurrentUserRequest = (generation: number, requestToken: string | null) => {
        return generation === userInfoRequestGeneration && requestToken === token.value;
    };

    const setToken = (newToken: string) => {
        persistToken(newToken);
        // 换票后清空旧用户信息并重新拉取，避免弹层先渲染空头像/残留上一个账号资料
        resetUserInfoCache();
    };

    const clearToken = () => {
        persistToken(null);
        resetUserInfoCache();
    };

    const isAuthenticated = computed(() => !!token.value);

    async function getUserInfo() {
        if (userInfoLoaded.value == true) {
            return;
        }
        const requestGeneration = userInfoRequestGeneration;
        const requestToken = token.value;
        try {
            const result = await getUser();
            if (!isCurrentUserRequest(requestGeneration, requestToken)) return;
            userInfo.value = result;
        } catch (error) {
            console.error("[user] getUserInfo failed", error);
        } finally {
            if (isCurrentUserRequest(requestGeneration, requestToken)) {
                userInfoLoaded.value = true;
            }
        }
    }

    async function loginOut() {
        try {
            await runLogoutCleanup(userLogout, () => {
                clearToken();
                const cartStore = useCartStore();
                cartStore.resetCart();
                navigateTo("/member/login");
            });
        } catch (error) {
            console.error(error);
        }
    }

    /**
     * 登录态下校验/拉取一次用户信息（带去重，避免重复请求）
     */
    async function validateUserInfoOnce() {
        if (!isAuthenticated.value) return;
        if (userInfoValidated.value) {
            return;
        }
        if (validatingPromise) return validatingPromise;

        const requestGeneration = userInfoRequestGeneration;
        const requestToken = token.value;
        validatingPromise = (async () => {
            try {
                const result = await getUser();
                if (!isCurrentUserRequest(requestGeneration, requestToken)) return;
                userInfo.value = result;
                userInfoLoaded.value = true;
                userInfoValidated.value = true;
            } catch (error) {
                console.error("[user] validateUserInfoOnce failed", error);
                // 失败不标记 validated，允许后续重试（例如登录后 cookie/鉴权尚未就绪）
                if (isCurrentUserRequest(requestGeneration, requestToken)) {
                    userInfoLoaded.value = true;
                }
            } finally {
                // 旧账号请求结束时不能清空新账号正在进行的校验 Promise
                if (isCurrentUserRequest(requestGeneration, requestToken)) {
                    validatingPromise = null;
                }
            }
        })();

        return validatingPromise;
    }

    return {
        userInfo,
        userInfoLoaded,
        userInfoValidated,
        openData,
        token,
        isAuthenticated,
        setToken,
        clearToken,
        getUserInfo,
        validateUserInfoOnce,
        loginOut
    };
});
