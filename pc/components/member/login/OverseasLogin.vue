<template>
    <div class="overseas-login-box flex align-center justify-center">
        <div class="login-content login-body-ZHYyMjgyaGw">
            <Transition name="slide-fade" @after-leave="contentChange = true">
                <div class="login-main" v-show="showWechat === false && contentChange">
                    <div class="menu">
                        <template v-for="(item, index) in menuList" :key="index">
                            <div class="menu-item" :class="{ active: item.value === activeMenuValue }" @click="handleChangeMenu(item.value)">
                                {{ $t(item.name) }}
                                <div class="line" v-if="item.value === activeMenuValue"></div>
                            </div>
                        </template>
                    </div>
                    <div class="content">
                        <Transition name="slide-fade" @after-leave="show = true">
                            <div class="form-box" v-if="activeMenuValue === 'password' && show">
                                <div class="form-item">
                                    <div class="form-item-label">{{ commonStore.openEmailRegister === 1 ? $t("用户名/手机/邮箱") : $t("用户名/手机") }}</div>
                                    <div class="form-item-input">
                                        <el-input
                                            v-model="accountForm.username"
                                            clearable
                                            :placeholder="$t(commonStore.openEmailRegister === 1 ? '手机号或电子邮箱地址' : '用户名/手机')"
                                            type="text"
                                        >
                                        </el-input>
                                    </div>
                                </div>
                                <div class="form-item">
                                    <div class="form-item-label">
                                        <div>{{ $t("密码") }}</div>
                                        <div class="special" @click="$router.push('reset')">{{ $t("忘记密码？") }}</div>
                                    </div>
                                    <div class="form-item-input">
                                        <el-input v-model="accountForm.password" clearable :placeholder="$t('密码')" show-password type="password"> </el-input>
                                    </div>
                                </div>
                            </div>
                        </Transition>
                        <Transition name="slide-fade2" @after-leave="show = true">
                            <div class="form-box" v-if="activeMenuValue === 'mobile' && show">
                                <div class="form-item">
                                    <div class="form-item-label">{{ $t("手机号") }}</div>
                                    <div class="form-item-input">
                                        <el-input v-model="mobileForm.mobile" clearable :placeholder="$t('您的手机号')" type="text">
                                            <template #prefix>
                                                <template v-if="isOverseas() && commonStore.isOpenMobileAreaCode">
                                                    <MobileAreaCode v-model="mobileAreaCode"></MobileAreaCode>
                                                </template>
                                            </template>
                                        </el-input>
                                    </div>
                                </div>
                                <div class="form-item">
                                    <div class="form-item-label">
                                        {{ $t("验证码") }}
                                    </div>
                                    <div class="form-item-input">
                                        <el-input v-model="mobileForm.mobileCode" :placeholder="$t('请输入验证码')" type="text">
                                            <template #suffix>
                                                <MobileCode
                                                    :mobile="
                                                        isOverseas() && commonStore.isOpenMobileAreaCode
                                                            ? mobileAreaCode + mobileForm.mobile
                                                            : mobileForm.mobile
                                                    "
                                                    v-model:mobileCode="mobileForm.mobileCode"
                                                    v-model:mobileCodeSended="mobileCodeSended"
                                                    :requestApi="sendMobileCode"
                                                    :disabled="!mobileForm.mobile"
                                                    class="mobile-code-text"
                                                    @mobileErrorCallback="mobileErrorCallback"
                                                ></MobileCode>
                                            </template>
                                        </el-input>
                                    </div>
                                </div>
                            </div>
                        </Transition>
                    </div>
                    <template v-if="errorMessage">
                        <div class="error-notice">
                            {{ $t(errorMessage) }}
                        </div>
                    </template>
                    <div class="btn-box">
                        <!-- <el-button :disabled="isloginDisabled" :loading="loginLoading" class="login-btn" type="primary" @click="onLoginClick">
                            {{ $t("登录") }}
                        </el-button> -->
                        <TigButton :disabled="isloginDisabled" :loading="loginLoading" class="login-btn" type="primary" @click="onLoginClick">
                            {{ $t("登录") }}
                        </TigButton>

                        <template
                            v-if="
                                (commonStore.openWechatOauth === 1 && commonStore.openWechatPcLogin === 1) ||
                                (isOverseas() && (commonStore.googleLoginOn === 1 || commonStore.facebookLoginOn === 1 || commonStore.twitterLoginOn === 1))
                            "
                        >
                            <div class="text">
                                <div class="line"></div>
                                <div>
                                    {{ $t("或通过以下方式加入") }}
                                </div>
                                <div class="line"></div>
                            </div>
                        </template>
                        <div class="other-login-img">
                            <div
                                class="login-img"
                                v-if="commonStore.openWechatOauth === 1 && commonStore.openWechatPcLogin === 1"
                                @click="showWechatLogin(false)"
                            >
                                <img src="~/assets/images/user/login/ico_wx.png" alt="" class="login-img" />
                                <p>{{ $t("微信") }}</p>
                            </div>
                            <div class="login-img googleRight" v-if="isOverseas() && commonStore.googleLoginOn" @click="handleOtherLogin('google')">
                                <img src="~/assets/images/user/login/google.png" alt="" class="login-img" />
                                <p>Google</p>
                            </div>
                            <div class="login-img" v-if="isOverseas() && commonStore.facebookLoginOn" @click="handleOtherLogin('facebook')">
                                <img src="~/assets/images/user/login/Facebook.png" alt="" class="login-img" />
                                <p>Facebook</p>
                            </div>
                            <div class="login-img" v-if="isOverseas() && commonStore.twitterLoginOn" @click="handleOtherLogin('twitter')">
                                <img src="~/assets/images/user/login/twitter.svg" class="login-img" mode="widthFix" />
                                <p>Twitter/X</p>
                            </div>
                        </div>
                    </div>
                    <div v-if="commonStore.shopRegClosed === 0" class="register">
                        <div class="register-text">{{ $t("还没有账号？") }}</div>
                        <div class="register-btn" @click="$router.push('register')">{{ $t("去注册") }}</div>
                    </div>
                </div>
            </Transition>
            <Transition name="slide-fade2" @after-leave="contentChange = true">
                <div :style="{ height: `${height}px` }" class="wechat-main" v-show="showWechat && contentChange">
                    <div class="wechat-main-back" @click="handleBack">
                        <span class="iconfont-pc icon-shangjiantou"></span>
                        <span class="wechat-main-back-text">{{ $t("返回") }}</span>
                    </div>
                    <div class="tow-title">{{ $t("扫码登录") }}</div>
                    <div class="wechat-desc">{{ $t("请使用微信移动端扫描二维码") }}</div>
                    <div class="two-image-box">
                        <template v-if="needRefresh">
                            <div class="refresh-con">
                                <div class="refresh-text">{{ $t("二维码已失效") }}</div>
                            </div>
                        </template>
                        <el-image fit="cover" class="two-image" :src="wechatUrl"></el-image>
                    </div>
                    <div class="refresh-qecode" @click="showWechatLogin(true)">
                        <span class="iconfont-pc icon-svg27"></span>
                        <span>{{ $t("刷新二维码") }}</span>
                    </div>
                    <div @click="cancelWechat" class="qiehuan"></div>
                </div>
            </Transition>
        </div>
    </div>
    <Verify ref="verify" :imgSize="{ width: '310px', height: '155px' }" captchaType="blockPuzzle" mode="pop" @okCallback="okCallback"></Verify>
</template>

<script setup lang="ts">
import { MobileCode } from "@/components/form";
import { getWechatLoginUrl, sendMobileCode, userSignin, checkWechatEvent, getOauth } from "@/api/user/login";
import { useCommonStore } from "~/store/common";
import { useUserStore } from "~/store/user";
import QRCode from "qrcode";
import Verify from "@/components/verifition/Verify.vue";
import MobileAreaCode from "@/components/form/src/MobileAreaCode.vue";
import { csrfCreate } from "~/api/user/login";
import { isOverseas } from "~/utils/util";

const { t } = useI18n();

const commonStore = useCommonStore();

const userStore = useUserStore();

const menuList = [
    {
        name: "账号登录",
        value: "password"
    },
    {
        name: "手机号登录",
        value: "mobile"
    }
];

const activeMenuValue = ref("password");

const accountForm = reactive({
    username: import.meta.env.VITE_DEFAULT_USER_NAME || "",
    password: import.meta.env.VITE_DEFAULT_USER_PASSWORD || ""
});

const mobileForm = reactive({
    mobile: "",
    mobileCode: ""
});

const contentChange = ref(true);

const mobileAreaCode = ref("86");
const mobileCodeSended = ref(false);
const errorMessage = ref("");

const mobileErrorCallback = (e: string) => {
    errorMessage.value = e;
};

const show = ref(true);
const handleChangeMenu = (value: string) => {
    activeMenuValue.value = value;
    show.value = false;
};

const verify = ref();
const verifyToken = ref("");
const okCallback = (e: any) => {
    verifyToken.value = e.verifyToken;
    onLoginClick();
};

const loginLoading = ref(false);
const isloginDisabled = computed(() => {
    if (activeMenuValue.value === "password") {
        return accountForm.username == "" || accountForm.password == "";
    } else {
        return mobileForm.mobile == "" || mobileForm.mobileCode == "" || mobileCodeSended.value == false;
    }
});

const route = useRoute();
const getReturnUrl = () => {
    const queryReturnUrl = route.query.returnUrl;
    if (typeof queryReturnUrl === "string" && queryReturnUrl) {
        return queryReturnUrl;
    }
    if (Array.isArray(queryReturnUrl) && queryReturnUrl[0]) {
        return queryReturnUrl[0];
    }
    // 登录页本身没有 returnUrl 时，交给后端走默认跳转。
    if (route.path.startsWith("/member/")) {
        return "";
    }
    return route.fullPath || route.path || "";
};

const csrfToken = ref("");

const getCsrfCreateData = async () => {
    try {
        const result = (await csrfCreate()) as string;
        csrfToken.value = result;
    } catch (error) {
        console.error("获取CSRF Token失败:", error);
    }
};

const signin = async () => {
    try {
        loginLoading.value = true;

        await getCsrfCreateData();

        const result = await userSignin(
            {
                loginType: activeMenuValue.value,
                username: accountForm.username,
                password: accountForm.password,
                mobile: isOverseas() && commonStore.isOpenMobileAreaCode ? mobileAreaCode.value + mobileForm.mobile : mobileForm.mobile,
                mobileCode: mobileForm.mobileCode,
                verifyToken: verifyToken.value
            },
            {
                "X-CSRF-Token": csrfToken.value
            }
        );
        userStore.setToken(result.token);
        await userStore.validateUserInfoOnce();
        navigateTo(getReturnUrl() || "/", { replace: true });
    } catch (error: any) {
        if (error.code == 1002) {
            errorMessage.value = error.message;
            verify.value && verify.value.show();
        } else if (error.code > 0) {
            errorMessage.value = error.message;
        }
    } finally {
        loginLoading.value = false;
    }
};

const onLoginClick = () => {
    signin();
};

const oauthPath = useCookie("oauthPath");
const handleOtherLogin = async (path: string) => {
    oauthPath.value = path;
    try {
        const result = await getOauth(path, getReturnUrl());
        window.location.href = result;
    } catch (error) {
        console.error(error);
    }
};

const ticket = ref("");
const wechatUrl = ref("");
const showWechat = ref(false);
const needRefresh = ref(false);
const WECHAT_POLL_INTERVAL = 2000;
const WECHAT_POLL_TIMEOUT = 60 * 1000;
let timerID: ReturnType<typeof setTimeout> | null = null;
let pollingStartedAt = 0;
let pollingInFlight = false;
let pollingFinished = false;
let pollingErrorCount = 0;
let pollingSession = 0;

const clearWechatTimer = () => {
    if (timerID !== null) {
        clearTimeout(timerID);
        timerID = null;
    }
};

const stopWechatPolling = () => {
    clearWechatTimer();
    pollingFinished = true;
    pollingInFlight = false;
};

const expireWechatQrCode = () => {
    stopWechatPolling();
    needRefresh.value = true;
};

const scheduleWechatEvent = () => {
    if (pollingFinished || !showWechat.value || !ticket.value) return;
    timerID = setTimeout(() => {
        timerID = null;
        void wechatEvent();
    }, WECHAT_POLL_INTERVAL);
};

// 当前后端在用户尚未扫码时可能以“未登录”错误表示等待状态，该错误不能终止轮询
const isWechatPendingError = (error: any) => {
    const message = String(error?.message || error?.msg || error?.data?.message || error?.data?.msg || "");
    return message.includes("未登录") || message.toLowerCase().includes("not logged");
};

const showWechatLogin = async (val: boolean) => {
    const session = ++pollingSession;
    stopWechatPolling();
    try {
        const result = await getWechatLoginUrl();
        if (session !== pollingSession) return;
        wechatUrl.value = result.url;
        ticket.value = result.ticket;
        const qrCodeImage = await generateQRCodeImage(result.url);
        if (session !== pollingSession) return;
        wechatUrl.value = qrCodeImage;
        contentChange.value = val;
        showWechat.value = true;
        needRefresh.value = false;
        pollingStartedAt = Date.now();
        pollingInFlight = false;
        pollingFinished = false;
        pollingErrorCount = 0;
        scheduleWechatEvent();
    } catch (error: any) {
        errorMessage.value = error?.message || t("微信登录失败，请重试");
        stopWechatPolling();
    }
};

const generateQRCodeImage = async (url: string) => {
    const data = url; // 替换为你要转换的链接
    let opts = {
        margin: 4 //二维码留白边距
    };
    return QRCode.toDataURL(data, opts);
};
const wechatEvent = async () => {
    if (pollingFinished || pollingInFlight || !showWechat.value || !ticket.value) return;
    const activeTicket = ticket.value;
    if (Date.now() - pollingStartedAt >= WECHAT_POLL_TIMEOUT) {
        expireWechatQrCode();
        return;
    }

    pollingInFlight = true;
    try {
        const result = await checkWechatEvent({ key: activeTicket });
        if (pollingFinished || !showWechat.value || ticket.value !== activeTicket) return;

        if (![0, 1, 2].includes(result.type)) {
            expireWechatQrCode();
            return;
        }

        if (result.type === 1) {
            stopWechatPolling();
            userStore.setToken(result.token);
            await userStore.validateUserInfoOnce();
            navigateTo("/member", { replace: true });
            return;
        }
        if (result.type === 2) {
            stopWechatPolling();
            if (commonStore.wechatRegisterBindPhone === 1) {
                // 跳转绑定手机页面
                userStore.openData = result.openData;
                navigateTo("/member/BindMobilePhone");
            }
            return;
        }

        pollingErrorCount = 0;
        if (Date.now() - pollingStartedAt >= WECHAT_POLL_TIMEOUT) {
            expireWechatQrCode();
        } else {
            scheduleWechatEvent();
        }
    } catch (error: any) {
        if (pollingFinished || ticket.value !== activeTicket) return;
        if (isWechatPendingError(error)) {
            pollingErrorCount = 0;
            if (Date.now() - pollingStartedAt >= WECHAT_POLL_TIMEOUT) {
                expireWechatQrCode();
            } else {
                scheduleWechatEvent();
            }
            return;
        }
        console.error(error);
        pollingErrorCount++;
        if (pollingErrorCount >= 3) {
            expireWechatQrCode();
        } else if (!pollingFinished && showWechat.value) {
            timerID = setTimeout(
                () => {
                    timerID = null;
                    void wechatEvent();
                },
                WECHAT_POLL_INTERVAL * (pollingErrorCount + 1)
            );
        }
    } finally {
        if (ticket.value === activeTicket) pollingInFlight = false;
    }
};

const cancelWechat = () => {
    pollingSession++;
    showWechat.value = false;
    stopWechatPolling();
};

const handleBack = () => {
    contentChange.value = false;
    cancelWechat();
};

onUnmounted(() => {
    if (timerID) cancelWechat();
});

const height = ref(0);
onMounted(() => {
    const el = document.querySelector(".login-main");
    height.value = el ? el.clientHeight : 0;
});
</script>

<style lang="less" scoped>
.overseas-login-box {
    width: 100%;
    max-width: 1230px;
    font-size: 14px;
    padding: 0 20px;
    box-sizing: border-box;
    flex-direction: column;
    .login-head {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        .logo {
            height: 36px;
            img {
                height: 100%;
            }
        }
        .txt {
            font-size: 14px;
            font-weight: 400;
            line-height: 18px;
            text-align: center;
            text-underline-position: from-font;
            text-decoration-skip-ink: none;
            color: #888b94;
            margin: 6px;
            display: flex;
            align-items: center;
            gap: 5px;
        }
    }

    .login-content {
        background-color: #fff;
        padding: 20px 0px 10px;
        min-width: 420px;

        .menu {
            display: flex;
            align-items: center;
            justify-content: center;
            margin-bottom: 40px;
            gap: 25px;
            .menu-item {
                position: relative;
                height: 40px;
                cursor: pointer;
                display: flex;
                flex-direction: column;
                justify-content: flex-end;
                transition: font-size 0.1s;
                font-size: 16px;

                &.active {
                    // font-size: 16px;
                    color: #1a1a1a;
                    font-weight: 700;
                }
                @keyframes growWidth {
                    from {
                        width: 0px;
                    }
                    to {
                        width: 30px; /* 这里的宽度可以根据实际需要调整 */
                    }
                }
                .line {
                    height: 3px;
                    background-color: #1a1a1a;
                    border-radius: 10px;
                    position: absolute;
                    bottom: -10px;
                    left: calc(50% - 15px);
                    transition:
                        left 0.3s,
                        width 0.3s; /* 添加过渡效果，使平移动画更平滑 */
                    animation: growWidth 0.3s forwards; /* 添加动画效果 */
                }
            }
        }

        .content {
            padding: 0 20px;

            .form-box {
                .form-item {
                    display: flex;
                    width: 100%;
                    flex-direction: column;
                    row-gap: 10px;
                    margin-top: 20px;
                    :deep(.el-input__wrapper) {
                        padding: 14px 15px;
                        border-radius: 4px;
                        box-shadow: none;
                        border: 1px solid #c2c4cc;
                        transition: border-color 0.3s;
                        &:hover {
                            border-color: currentcolor;
                        }
                        &:focus-within {
                            border-color: currentcolor;
                        }
                    }
                    :deep(.el-input__inner) {
                        font-size: 15px !important;
                        color: #333;
                        &::placeholder {
                            color: currentcolor;
                        }
                    }

                    .form-item-label {
                        font-weight: 600;
                        padding-left: 2px;
                        color: #1a1a1a;
                        display: flex;
                        align-items: center;
                        justify-content: space-between;

                        .special {
                            font-weight: 500;
                            color: var(--general);
                            cursor: pointer;
                        }
                    }
                }
            }
        }

        .error-notice {
            color: rgba(250, 44, 25, 1);
            padding: 0 20px;
            padding-top: 10px;
        }

        .btn-box {
            padding: 20px;
            display: flex;
            flex-direction: column;
            align-items: center;
            row-gap: 15px;

            .login-btn {
                width: 100%;
                margin-top: 10px;
                background: var(--general) none repeat scroll 0 0;
                border: 0 none;
                border-radius: 3px;
                color: var(--main-text);
                font-size: 16px;
                height: 50px;

                &.is-disabled {
                    opacity: 0.6;
                    cursor: not-allowed;
                }
            }

            .text {
                width: 100%;
                font-size: 14px;
                color: #888b94;
                display: flex;
                align-items: center;
                justify-content: center;
                gap: 10px;
                margin-top: 30px;
                .line {
                    width: 22px;
                    height: 1px;
                    background-color: #c2c4cc;
                }
            }

            .other-login-btn {
                border: 1px solid #828192;
                height: 40px;
                width: 100%;
                display: flex;
                align-items: center;
                justify-content: center;
                cursor: pointer;

                &:hover {
                    background-color: #f5f5f5;
                }

                &:active {
                    background-color: #e7e7e7;
                }

                .login-img {
                    width: 20px;
                    padding-right: 5px;
                    position: relative;
                    bottom: -1px;
                }

                .btn-text {
                    font-weight: 600;
                    font-size: 12px;
                }
            }
        }

        .register {
            display: flex;
            align-items: center;
            justify-content: center;
            padding-bottom: 15px;
            font-size: 12px;

            .register-btn {
                margin-left: 5px;
                padding: 5px 0;
                cursor: pointer;
                color: var(--general);
            }
        }

        .wechat-main {
            position: relative;

            .wechat-main-back {
                position: absolute;
                top: 0px;
                left: 25px;
                color: var(--general);
                font-size: 18px;
                display: flex;
                align-items: center;
                justify-content: center;
                cursor: pointer;
                column-gap: 4px;

                .icon-shangjiantou {
                    transform: rotate(-90deg) translateX(-1px);
                    font-weight: 600;
                }
            }

            .tow-title {
                font-size: 24px;
                font-weight: 600;
                color: #000;
                padding: 80px 0 20px;
                text-align: center;
            }

            .wechat-desc {
                font-size: 16px;
                color: #858585;
                text-align: center;
                padding-bottom: 20px;
            }

            .two-image-box {
                margin: 0 auto;
                width: 220px;
                height: 220px;
                position: relative;

                .refresh-con {
                    align-items: center;
                    background: #ffffffe6;
                    bottom: 0;
                    display: flex;
                    flex-wrap: wrap;
                    justify-content: center;
                    left: 0;
                    position: absolute;
                    right: 0;
                    top: 0;
                    width: 100%;
                    z-index: 1;

                    .refresh-text {
                        font-size: 16px;
                    }
                }

                .two-image {
                    width: 100%;
                    height: 100%;
                    // border: 1px solid #d5d5d5;
                    border-radius: 2px;
                }
            }

            .refresh-qecode {
                color: var(--general);
                font-size: 16px;
                padding-top: 20px;
                cursor: pointer;
                display: flex;
                justify-content: center;
                align-items: center;
                column-gap: 5px;
            }
        }
    }

    :deep(.mobile-code-text .code-btn) {
        color: var(--general);
        font-size: 12px;
    }
}
:deep(.el-input__inner) {
    &::placeholder {
        color: #505259 !important;
    }
}

.slide-fade-enter-active {
    transition: all 0.3s ease-in-out;
}

.slide-fade-leave-active {
    transition: none;
}

.slide-fade-enter-from,
.slide-fade-leave-to {
    transform: translateX(20px);
    opacity: 0;
}

.slide-fade2-enter-active {
    transition: all 0.35s ease-in-out;
}

.slide-fade2-leave-active {
    transition: none;
}

.slide-fade2-enter-from,
.slide-fade2-leave-to {
    transform: translateX(-20px);
    opacity: 0;
}
.other-login-img {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 60px;
    margin-top: 10px;

    .login-img {
        // padding-right: 5px;
        position: relative;
        bottom: -1px;
        text-align: center;
        cursor: pointer;
        img {
            width: 25px;
        }
        p {
            font-size: 14px;
            line-height: 18px;
            margin: 12px 0;
            color: #1a1a1a;
        }
    }
}
</style>
