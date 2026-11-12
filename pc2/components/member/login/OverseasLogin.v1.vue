<template>
    <div class="overseas-login-box flex align-center justify-end">
        <div class="login-content">
            <Transition name="slide-fade" @after-leave="contentChange = true">
                <div class="login-main" v-show="showWechat === false && contentChange">
                    <div class="menu">
                        <template v-for="(item, index) in menuList" :key="index">
                            <div class="menu-item" :class="{ active: item.value === activeMenuValue }" @click="handleChangeMenu(item.value)">
                                {{ $t(item.name) }}
                            </div>
                        </template>
                    </div>
                    <div class="content">
                        <Transition name="slide-fade" @after-leave="show = true">
                            <div class="form-box" v-if="activeMenuValue === 'password' && show">
                                <div class="form-item">
                                    <div class="form-item-label">{{ $t("用户名/手机") }}</div>
                                    <div class="form-item-input">
                                        <el-input
                                            v-model="accountForm.username"
                                            clearable
                                            :placeholder="$t(commonStore.openEmailRegister === 1 ? '用户名/手机/邮箱' : '用户名/手机')"
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
                                                <template v-if="commonStore.isOpenMobileAreaCode">
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
                                                    :mobile="commonStore.isOpenMobileAreaCode ? mobileAreaCode + mobileForm.mobile : mobileForm.mobile"
                                                    v-model:mobileCode="mobileForm.mobileCode"
                                                    v-model:mobileCodeSended="mobileCodeSended"
                                                    :requestApi="sendMobileCode"
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
                            {{ errorMessage }}
                        </div>
                    </template>

                    <div class="agreement-box">
                        <el-checkbox v-model="agreementChecked">
                            <span>我已阅读并同意</span>
                            <NuxtLink class="agreement-link" target="_blank" :to="'/article/issue/info?articleSn=fwxy'">《{{ $t("服务协议") }}》 </NuxtLink>
                            <span>和</span>
                            <NuxtLink class="agreement-link" target="_blank" :to="'/article/issue/info?articleSn=ysxy'">《{{ $t("隐私协议") }}》 </NuxtLink>
                        </el-checkbox>
                    </div>

                    <div class="btn-box">
                        <el-button :disabled="isloginDisabled" :loading="loginLoading" class="login-btn" type="primary" @click="onLoginClick">
                            {{ $t("登录") }}
                        </el-button>

                        <template
                            v-if="
                                (commonStore.openWechatOauth === 1 && commonStore.openWechatPcLogin === 1) ||
                                commonStore.googleLoginOn === 1 ||
                                commonStore.facebookLoginOn === 1
                            "
                        >
                            <div class="text">{{ $t("或") }}</div>
                        </template>
                        <div class="other-login-img">
                            <div
                                class="login-img"
                                v-if="commonStore.openWechatOauth === 1 && commonStore.openWechatPcLogin === 1"
                                @click="showWechatLogin(false)"
                            >
                                <img src="~/assets/images/user/login/ico_wx.png" alt="" class="login-img" />
                            </div>
                            <div class="login-img googleRight" v-if="commonStore.googleLoginOn" @click="handleOtherLogin('google')">
                                <img src="~/assets/images/user/login/google.png" alt="" class="login-img" />
                            </div>
                            <div class="login-img" v-if="commonStore.facebookLoginOn" @click="handleOtherLogin('facebook')">
                                <img src="~/assets/images/user/login/Facebook.png" alt="" class="login-img" />
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
                            <!-- <template v-if="true"> -->
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
import { getWechatLoginUrl, sendMobileCode, userSignin, checkWechatEvent, getOauth, getOauthCallback } from "@/api/user/login";
import { useCommonStore } from "~/store/common";
import { useUserStore } from "~/store/user";
import QRCode from "qrcode";
import Verify from "@/components/verifition/Verify.vue";

const { t } = useI18n();

const commonStore = useCommonStore();

const userStore = useUserStore();

const csrfToken = defineModel<string>("csrfToken");

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
    username: "",
    password: ""
});

const mobileForm = reactive({
    mobile: "",
    mobileCode: ""
});

const contentChange = ref(true);

const mobileAreaCode = ref("86");
const mobileCodeSended = ref(false);
const errorMessage = ref("");

const agreementChecked = ref(false);

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
const returnUrl = route.query.returnUrl ? String(route.query.returnUrl) : "/";
const signin = async () => {
    try {
        loginLoading.value = true;
        const result = await userSignin(
            {
                loginType: activeMenuValue.value,
                username: accountForm.username,
                password: accountForm.password,
                mobile: commonStore.isOpenMobileAreaCode ? mobileAreaCode.value + mobileForm.mobile : mobileForm.mobile,
                mobileCode: mobileForm.mobileCode,
                verifyToken: verifyToken.value
            },
            {
                "X-CSRF-Token": csrfToken.value
            }
        );
        userStore.setToken(result.token);
        userStore.userInfoLoaded = false;
        navigateTo(returnUrl, { replace: true });
    } catch (error: any) {
        if (error.code == 1002) {
            verify.value && verify.value.show();
        } else if (error.code > 0) {
            errorMessage.value = error.message;
        }
    } finally {
        loginLoading.value = false;
    }
};

const onLoginClick = () => {
    if (!agreementChecked.value) {
        return (errorMessage.value = t("请同意服务协议和隐私政策"));
    }

    signin();
};

const oauthPath = useCookie("oauthPath");
const handleOtherLogin = async (path: string) => {
    oauthPath.value = path;
    try {
        const result = await getOauth(path, window.location.href);
        window.open(result, "_blank", `width=${500},height=${700},top=${screen.height / 2 - 700 / 2}`);
    } catch (error) {
        console.error(error);
    }
};
const __getOauthCallback = async (data: object) => {
    try {
        const result = await getOauthCallback(oauthPath.value as string, window.location.href, data);
        userStore.setToken(result.token);
        userStore.userInfoLoaded = false;
        navigateTo(returnUrl, { replace: true });
    } catch (error) {
        console.error(error);
        message.error(t("授权失败"));
    }
};

const ticket = ref("");
const wechatUrl = ref("");
const loginTime = ref(0);
const showWechat = ref(false);
const needRefresh = ref(false);
let timerID: any;
const showWechatLogin = async (val: boolean) => {
    try {
        const result = await getWechatLoginUrl();
        if (timerID) {
            clearInterval(timerID);
        }
        wechatUrl.value = result.url;
        ticket.value = result.ticket;
        await generateQRCodeImage(result.url);
        contentChange.value = val;
        showWechat.value = true;
        needRefresh.value = false;
        loginTime.value = 0;
        timerID = setInterval(() => {
            loginTime.value++;
            if (loginTime.value > 30) {
                needRefresh.value = true;
                clearInterval(timerID);
            }
            wechatEvent();
        }, 1000);
    } catch (error) {}
};

const generateQRCodeImage = async (url: string) => {
    const data = url; // 替换为你要转换的链接
    let opts = {
        margin: 4 //二维码留白边距
    };
    wechatUrl.value = await QRCode.toDataURL(data, opts);
};
const wechatEvent = async () => {
    const result = await checkWechatEvent({ key: ticket.value });
    if (result.type > 0 || showWechat.value === false) {
        cancelWechat();
    }
    if (result.type == 1) {
        userStore.setToken(result.token);
        userStore.userInfoLoaded = false;
        navigateTo("/member", { replace: true });
    }
    if (result.type == 2 && commonStore.wechatRegisterBindPhone === 1) {
        //跳转绑定手机页面
        userStore.openData = result.openData;
        navigateTo("/member/BindMobilePhone");
    }
};

const cancelWechat = () => {
    showWechat.value = false;
    clearInterval(timerID);
    timerID = null; // 确保在设置新的定时器前，旧的定时器已经被清除
    loginTime.value = 0;
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
    console.log("height", height.value);
});

watch(
    route.query,
    () => {
        if (route.query.code) {
            __getOauthCallback(route.query);
        }
    },
    { immediate: true }
);
</script>

<style lang="less" scoped>
.agreement-box {
    padding: 10px 20px 0px;

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

.overseas-login-box {
    width: 100%;
    max-width: 1230px;
    padding: 0 20px;
    box-sizing: border-box;

    .login-content {
        background-color: #fff;
        padding: 20px 0px 10px;
        min-width: 400px;

        .menu {
            display: flex;
            align-items: center;
            column-gap: 15px;
            padding: 0 20px;
            border-bottom: 1px solid #e7e7e7;

            .menu-item {
                font-size: 12px;
                height: 36px;
                line-height: 36px;
                cursor: pointer;

                &.active {
                    color: var(--general);
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

                    .form-item-label {
                        font-weight: 600;
                        padding-left: 2px;
                        display: flex;
                        align-items: center;
                        justify-content: space-between;

                        .special {
                            color: var(--general);
                            cursor: pointer;
                            font-weight: 400;
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
            padding: 10px 20px 20px;
            display: flex;
            flex-direction: column;
            align-items: center;
            row-gap: 15px;

            .login-btn {
                width: 100%;
                background: var(--general) none repeat scroll 0 0;
                border: 0 none;
                border-radius: 3px;
                color: var(--main-text);
                font-size: 14px;
                height: 40px;

                &.is-disabled {
                    opacity: 0.6;
                    cursor: not-allowed;
                }
            }

            .text {
                height: 25px;
                line-height: 25px;
                font-weight: 600;
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
                // margin-left: 5px;

                // border: 1px solid #828192;
                padding: 5px 0;
                // font-weight: 600;
                cursor: pointer;
                color: var(--general);

                // &:hover {
                //     background-color: #f5f5f5;
                // }

                // &:active {
                //     background-color: #e7e7e7;
                // }
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
    .googleRight {
        margin: 0 30px;
    }
    .login-img {
        width: 29px;
        // padding-right: 5px;
        position: relative;
        bottom: -1px;
    }
}
</style>
