<template>
    <div class="login-center flex align-center justify-end">
        <div class="login_border login-body-ZHYyMjgyaGw">
            <template v-if="loginType == 1">
                <div class="login-main">
                    <div class="login-title-box">
                        <div class="login-title">{{ $t("欢迎登录") }}</div>
                        <template v-if="commonStore.shopRegClosed === 0">
                            <div class="register-box">
                                <span class="">{{ $t("没有账号？") }}</span>
                                <NuxtLink class="register-main-text" to="/member/register/">{{ $t("去注册") }}</NuxtLink>
                            </div>
                        </template>
                    </div>
                    <div class="login-top flex align-center justify-between">
                        <div class="login-tab flex align-center hand-pointer">
                            <div :class="tabIndex == 1 ? 'login-on' : ''" class="login_mode" @click="changTab(1)">
                                {{ $t("普通登录") }}
                            </div>
                            <div :class="tabIndex == 2 ? 'login-on' : ''" class="login_mode" @click="changTab(2)">
                                {{ $t("手机登录") }}
                            </div>
                        </div>
                    </div>
                    <div class="login-body">
                        <div v-if="tabIndex == 1" class="login-mode-normal">
                            <div class="login-input flex align-center">
                                <el-input
                                    v-model="username"
                                    clearable
                                    :placeholder="$t(commonStore.openEmailRegister === 1 ? '用户名/手机/邮箱' : '用户名/手机')"
                                    type="text"
                                >
                                    <template #prefix>
                                        <img src="/assets/images/user/login/ico_user.png" />
                                    </template>
                                </el-input>
                            </div>
                            <div class="login-input flex align-center">
                                <el-input v-model="password" :placeholder="$t('密码')" show-password type="password">
                                    <template #prefix>
                                        <img src="/assets/images/user/login/ico_pwd.png" />
                                    </template>
                                </el-input>
                            </div>
                        </div>
                        <div v-show="tabIndex === 2" class="login-mode-normal">
                            <div class="login-input flex align-center">
                                <el-input v-model="mobile" clearable :placeholder="$t('您的手机号')" type="text">
                                    <template #prefix>
                                        <template v-if="commonStore.isOpenMobileAreaCode">
                                            <MobileAreaCode v-model="mobileAreaCode"></MobileAreaCode>
                                        </template>
                                        <template v-else>
                                            <img src="/assets/images/user/login/iconfont-shouji.png" />
                                        </template>
                                    </template>
                                </el-input>
                            </div>

                            <div class="login-input flex align-center">
                                <el-input v-model="mobileCode" :placeholder="$t('请输入验证码')" type="text">
                                    <template #prefix>
                                        <img src="/assets/images/user/login/iconfont-duanxin.png" />
                                    </template>
                                    <template #suffix>
                                        <MobileCode
                                            :mobile="commonStore.isOpenMobileAreaCode ? mobileAreaCode + mobile : mobile"
                                            v-model:mobileCode="mobileCode"
                                            v-model:mobileCodeSended="mobileCodeSended"
                                            :requestApi="sendMobileCode"
                                            class="mobile-code-text"
                                            @mobileErrorCallback="mobileErrorCallback"
                                        ></MobileCode>
                                    </template>
                                </el-input>
                            </div>
                        </div>
                        <template v-if="errorMessage">
                            <div class="error-notice">
                                {{ errorMessage }}
                            </div>
                        </template>

                        <div class="agreement-box">
                            <el-checkbox v-model="agreementChecked">
                                <span>{{ $t("我已阅读并同意") }}</span>
                                <NuxtLink class="agreement-link" target="_blank" :to="'/article/issue/info?articleSn=fwxy'">《{{ $t("服务协议") }}》 </NuxtLink>
                                <span>{{ $t("和") }}</span>
                                <NuxtLink class="agreement-link" target="_blank" :to="'/article/issue/info?articleSn=ysxy'">《{{ $t("隐私协议") }}》 </NuxtLink>
                            </el-checkbox>
                        </div>

                        <div class="login_last">
                            <el-button :disabled="isloginDisabled" :loading="loginLoading" class="login_btn submit_btn" type="primary" @click="onLoginClick">
                                {{ $t("登录") }}
                            </el-button>
                        </div>

                        <template v-if="tabIndex === 1">
                            <div class="forget-text">
                                <a @click="$router.push('/member/reset')">{{ $t("忘记密码？") }}</a>
                            </div>
                        </template>

                        <template v-if="showOauth === 1 && commonStore.openWechatOauth === 1 && commonStore.openWechatPcLogin === 1">
                            <div class="login_bottom">
                                <div class="tit">{{ $t("合作网站账号登录") }}</div>
                                <li class="other-login">
                                    <template v-if="wechatLogin == 1">
                                        <a @click="showWechatLogin"> <img height="32" src="/assets/images/user/login/ico_wx.png" width="32" /></a>
                                    </template>
                                </li>
                            </div>
                        </template>

                        <Verify
                            ref="verify"
                            :imgSize="{ width: '310px', height: '155px' }"
                            captchaType="blockPuzzle"
                            mode="pop"
                            @okCallback="okCallback"
                        ></Verify>
                    </div>

                    <template v-if="commonStore.openWechatOauth === 1 && commonStore.openWechatPcLogin === 1">
                        <div @click="showWechatLogin" class="qiehuan1"></div>
                    </template>
                </div>
            </template>

            <template v-if="loginType == 2">
                <div class="login-main wechat-main">
                    <div class="tow-title">{{ $t("使用微信扫码安全登录") }}</div>
                    <div>
                        <el-image fit="cover" class="two-image" :src="loginUrl">
                            <template #placeholder
                                ><div class="image-text">{{ $t("加载中...") }}</div></template
                            >
                            <template #error
                                ><div class="image-text">{{ $t("加载中...") }}</div></template
                            >
                        </el-image>
                    </div>
                    <div v-if="qrExpired" class="wechat-expired">{{ $t("二维码已失效") }}</div>
                    <div>
                        <el-button style="width: 140px" type="primary" @click="showWechatLogin()">{{ $t("刷新二维码") }}</el-button>
                    </div>
                    <div @click="cancelWechat" class="qiehuan"></div>
                </div>
            </template>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { onMounted, onUnmounted, ref } from "vue";
import { getQuickLoginSetting, getWechatLoginUrl, sendMobileCode, userSignin, checkWechatEvent } from "@/api/user/login";
import Verify from "@/components/verifition/Verify.vue";
import MobileAreaCode from "@/components/form/src/MobileAreaCode.vue";
import { MobileCode } from "@/components/form";
import { useUserStore } from "@/store/user";
import { useCommonStore } from "~/store/common";
import QRCode from "qrcode";
import { csrfCreate } from "~/api/user/login";

const { t } = useI18n();
const userStore = useUserStore();
const route = useRoute();
const returnUrl = route.query.returnUrl ? String(route.query.returnUrl) : "/";
const commonStore = useCommonStore();
commonStore.loadNav();
const tabIndex = ref(1);
const username = ref(import.meta.env.VITE_DEFAULT_USER_NAME || "");
const password = ref(import.meta.env.VITE_DEFAULT_USER_PASSWORD || "");
const mobile = ref("");
const mobileCode = ref("");
const mobileCodeSended = ref(false);
const loginLoading = ref(false);
const verifyToken = ref("");
const verify = ref();
const errorMessage = ref("");
const showOauth = ref(0);
const wechatLogin = ref(0);
const agreementChecked = ref(false);

const mobileAreaCode = ref("86");

onMounted(() => {
    quickLoginSetting();
});

const onLoginClick = () => {
    if (!agreementChecked.value) {
        return (errorMessage.value = t("请同意服务协议和隐私政策"));
    }

    signin();
};

const csrfToken = ref("");

const getCsrfCreateData = async () => {
    try {
        const result = await csrfCreate();
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
                loginType: tabIndex.value === 1 ? "password" : "mobile",
                username: username.value,
                password: password.value,
                mobile: commonStore.isOpenMobileAreaCode ? mobileAreaCode.value + mobile.value : mobile.value,
                mobileCode: mobileCode.value,
                verifyToken: verifyToken.value
            },
            {
                "X-CSRF-Token": csrfToken.value
            }
        );
        userStore.setToken(result.token);
        await userStore.validateUserInfoOnce();
        navigateTo(returnUrl, { replace: true });
    } catch (error: any) {
        if (error.code == 1002) {
            verify.value.show();
        } else if (error.code > 0) {
            errorMessage.value = error.message;
        }
    } finally {
        loginLoading.value = false;
    }
};

const okCallback = (e: any) => {
    verifyToken.value = e.verifyToken;
    onLoginClick();
};

const isloginDisabled = computed(() => {
    if (tabIndex.value == 1) {
        return username.value == "" || password.value == "";
    } else {
        return mobile.value == "" || mobileCode.value == "" || mobileCodeSended.value == false;
    }
});

const changTab = (index: number) => {
    tabIndex.value = index;
    errorMessage.value = "";
};
const mobileErrorCallback = (e: string) => {
    errorMessage.value = e;
};

const quickLoginSetting = async () => {
    const result = await getQuickLoginSetting();
    showOauth.value = result.showOauth;
    wechatLogin.value = result.wechatLogin;
};

const wechatLoding = ref(false);
const loginType = ref(1);
const loginUrl = ref("");
const ticket = ref("");
const qrExpired = ref(false);
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
    qrExpired.value = true;
};

const scheduleWechatEvent = () => {
    if (pollingFinished || loginType.value !== 2 || !ticket.value) return;
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

const showWechatLogin = async () => {
    if (wechatLoding.value) return;
    const session = ++pollingSession;
    wechatLoding.value = true;
    stopWechatPolling();
    loginType.value = 2;
    qrExpired.value = false;
    try {
        const result = await getWechatLoginUrl();
        if (session !== pollingSession) return;
        if (result.url) {
            ticket.value = result.ticket;

            const qrCodeImage = await generateQRCodeImage(result.url);
            if (session !== pollingSession) return;
            loginUrl.value = qrCodeImage;
            // 新二维码从零开始计时，采用串行递归定时器，避免慢请求重叠
            pollingStartedAt = Date.now();
            pollingInFlight = false;
            pollingFinished = false;
            pollingErrorCount = 0;
            scheduleWechatEvent();
        }
    } catch (e) {
        console.log(e);
        stopWechatPolling();
    } finally {
        wechatLoding.value = false;
    }
};
const cancelWechat = () => {
    pollingSession++;
    loginType.value = 1;
    stopWechatPolling();
    qrExpired.value = false;
};
const generateQRCodeImage = async (url: string) => {
    const data = url; // 替换为你要转换的链接
    let opts = {
        margin: 10 //二维码留白边距
    };
    return QRCode.toDataURL(data, opts);
};

const wechatEvent = async () => {
    if (pollingFinished || pollingInFlight || loginType.value !== 2 || !ticket.value) return;
    const activeTicket = ticket.value;
    if (Date.now() - pollingStartedAt >= WECHAT_POLL_TIMEOUT) {
        expireWechatQrCode();
        return;
    }

    pollingInFlight = true;
    try {
        const result = await checkWechatEvent({ key: activeTicket });
        if (pollingFinished || loginType.value !== 2 || ticket.value !== activeTicket) return;

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
            // 连续失败（例如二维码 key 已失效）时停止，避免无限请求
            expireWechatQrCode();
        } else if (!pollingFinished && loginType.value === 2) {
            // 短暂网络错误最多重试两次，仍保持串行请求
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

// 定义一个方法来处理回车键事件
function handleEnter(event: any) {
    if (event.key === "Enter" && !isloginDisabled.value) {
        console.log("Enter key pressed");
        onLoginClick();
        // 在这里处理你的逻辑
    }
}

// 在组件挂载时添加全局事件监听器
onMounted(() => {
    document.addEventListener("keydown", handleEnter);
});

// 在组件卸载时移除全局事件监听器
onUnmounted(() => {
    document.removeEventListener("keydown", handleEnter);
    stopWechatPolling();
});
</script>
<style lang="less" scoped>
.agreement-box {
    padding: 10px 0 0px;

    .agreement-link {
        color: var(--general);
        cursor: pointer;
    }

    :deep(.el-checkbox__inner) {
        &::after {
            border-width: 1.5px;
        }
    }
}

.wechat-expired {
    margin-top: 8px;
    color: #999;
    font-size: 13px;
    text-align: center;
}

.login-center {
    width: 100%;
    max-width: 1230px;
    padding: 0 20px;
    box-sizing: border-box;

    .forget-text {
        padding: 15px 0 0;
        text-align: center;

        a {
            color: #666;
        }
    }

    .login_border {
        position: relative;

        .login-main {
            background: #ffffff;
            min-height: 280px;
            padding-bottom: 5px;
            position: relative;
            min-width: 380px;
            padding: 20px 0 40px;
            border-radius: 8px;
            overflow: hidden;

            .login-title-box {
                display: flex;
                padding: 0 25px;
                align-items: center;

                .login-title {
                    font-size: 20px;
                    font-weight: bold;
                    color: #333;
                    flex: 1;
                }

                .register-box {
                    .register-main-text {
                        color: var(--general);
                    }
                }
            }

            .login-top {
                height: 46px;
                padding: 10px 25px 0;

                .login-tab {
                    width: 100%;
                    border-bottom: 1px solid #e7e7e7;

                    .login_mode {
                        height: 46px;
                        line-height: 46px;
                        text-align: center;
                        font-size: 14px;
                        color: #333;
                        flex: 1;
                        font-weight: bold;
                    }

                    .login-on {
                        color: var(--general);
                        position: relative;

                        &::after {
                            content: "";
                            position: absolute;
                            left: 0;
                            right: 0;
                            bottom: 0px;
                            height: 2px;
                            background-color: var(--general);
                        }
                    }
                }
            }

            .login-body {
                overflow: visible;
                padding: 30px 25px 0;

                .login-mode-normal {
                    display: flex;
                    flex-direction: column;
                    gap: 20px;

                    .login-input {
                        width: 100%;
                        border-radius: 3px;

                        :deep(.el-select__wrapper) {
                            box-shadow: none;
                        }

                        img {
                            width: 20px;
                            height: 20px;
                            margin-right: 8px;
                        }

                        .el-input {
                            height: 36px;
                            border-radius: 2px;
                        }

                        :deep(.el-input__suffix-inner) > :first-child {
                            margin-left: 0;
                        }
                    }
                }

                .sendMobileMsg {
                    margin-top: 20px;

                    .input-box {
                        width: 60%;
                        border: 1px solid #d6d6d6;
                        border-radius: 3px;
                        padding: 5px 8px;
                        height: 24px;
                        line-height: 24px;

                        img {
                            width: 22px;
                            height: 22px;
                            margin-right: 8px;
                        }

                        input {
                            width: 80%;
                            height: 24px;
                        }
                    }

                    .btn {
                        background: #f0f0f0 none repeat scroll 0 0;
                        border: 1px solid #d6d6d6;
                        border-radius: 3px;
                        cursor: pointer;
                        height: 34px;
                        margin-left: 4px;
                        width: 90px;
                        display: inline-block;
                        text-align: center;
                        line-height: 34px;
                        vertical-align: middle;
                    }
                }

                .error-notice {
                    color: rgba(250, 44, 25, 1);
                    padding-top: 10px;
                }

                .login-forgot {
                    margin-top: 10px;

                    .forgot-txt {
                        &:hover {
                            color: var(--general) !important;
                        }
                    }
                }

                .login_last {
                    margin-top: 16px;

                    .login_btn {
                        width: 100%;
                        background: var(--general) none repeat scroll 0 0;
                        border: 0 none;
                        border-radius: 3px;
                        color: var(--main-text);
                        cursor: pointer;
                        font-size: 16px;
                        height: 40px;
                    }

                    .login_btn.is-disabled {
                        opacity: 0.6;
                        cursor: not-allowed;
                    }
                }

                .login_bottom {
                    margin-top: 14px;
                    display: flex;
                    flex-direction: column;
                    align-items: center;
                    color: #666;

                    .tit {
                        display: flex;
                        justify-content: center;
                        width: 100%;
                        align-items: center;
                        column-gap: 12px;

                        &::before {
                            content: "";
                            height: 1px;
                            flex: 1;
                            background-color: #e5e5e5;
                        }

                        &::after {
                            content: "";
                            height: 1px;
                            flex: 1;
                            background-color: #e5e5e5;
                        }
                    }

                    .other-login {
                        margin-top: 15px;
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        column-gap: 20px;
                    }
                }

                .send-valicode {
                    margin-left: 10px;
                    height: 36px;
                }

                :deep(.mobile-code-text .code-btn) {
                    color: var(--general);
                    font-size: 12px;
                }
            }

            .qiehuan1 {
                background: url("/assets/images/user/login/login-code.png") no-repeat scroll right top;
                bottom: 0px;
                cursor: pointer;
                height: 60px;
                position: absolute;
                right: 0px;
                width: 60px;
            }

            .qiehuan {
                background: url("/assets/images/user/login/login-code.png") no-repeat scroll left bottom;
                bottom: 0px;
                cursor: pointer;
                height: 60px;
                position: absolute;
                right: 0px;
                width: 60px;
            }
        }

        .wechat-main {
            display: flex;
            justify-content: center;
            align-items: center;
            flex-direction: column;
            gap: 18px;

            .tow-title {
                font-size: 16px;
                line-height: 20px;
                color: #858585;
                height: 20px;
                font-weight: bold;
            }

            .two-image {
                width: 140px;
                height: 140px;
                border: 1px solid #d5d5d5;
                border-radius: 2px;

                .image-text {
                    align-items: center;
                    color: #a8a8a8;
                    display: flex;
                    font-size: 14px;
                    height: 100%;
                    justify-content: center;
                    width: 100%;
                }
            }
        }
    }
}
</style>
