<template>
    <CommonHeader v-if="!registrationClosed" title="注册"></CommonHeader>
    <CommonLoginHeader
        v-if="!registrationClosed"
        type="register"
        :height="headerHeight"
        :shadow="headerShadow"
        :logo-width="logoWidth"
        :logo-height="logoHeight"
        :classic="loginMode === 1"
    ></CommonLoginHeader>
    <div v-if="!registrationClosed" class="full-height-div">
        <Transition name="slide-fade" @after-leave="contentChange = true">
            <div data-form-rev="ZmFocDIyODI" class="register-info" v-show="showWechat === false && contentChange">
                <!-- 邮箱注册仅跨境版支持；非跨境即使后台曾开启也不展示切换入口 -->
                <template v-if="isOverseas() && commonStore.openEmailRegister === 1">
                    <div class="register-menu">
                        <div class="menu">
                            <template v-for="(item, index) in menuList" :key="index">
                                <div class="menu-item" :class="{ active: item.value === formState.registType }" @click="registerTypeChange(item.value)">
                                    {{ $t(item.label) }}
                                    <div class="line" v-if="item.value === formState.registType"></div>
                                </div>
                            </template>
                        </div>
                    </div>
                </template>
                <template v-else>
                    <div class="title">{{ $t("会员注册") }}</div>
                </template>
                <div class="content">
                    <Transition name="slide-fade" @after-leave="show = true">
                        <div class="form-box" v-if="!formState.registType && show">
                            <div class="form-item">
                                <div class="form-item-label">{{ $t("手机号码") }}</div>
                                <div class="form-item-input area-code-input">
                                    <el-input
                                        v-model="formState.mobile"
                                        class="input-border-style"
                                        :placeholder="$t('手机号码')"
                                        @blur="blurFun('mobile', formState.mobile)"
                                    >
                                        <template #prefix v-if="isOverseas() && commonStore.isOpenMobileAreaCode">
                                            <div class="area-code-select">
                                                <MobileAreaCode v-model="mobileAreaCode"></MobileAreaCode>
                                            </div>
                                        </template>
                                    </el-input>
                                    <div class="error-notice">
                                        {{ formState.tigTips.mobile?.message }}
                                    </div>
                                </div>
                            </div>
                            <div class="form-item">
                                <div class="form-item-label">
                                    {{ $t("验证码") }}
                                </div>
                                <div class="form-item-input">
                                    <el-input
                                        v-model="formState.mobileCode"
                                        class="input-border-style"
                                        :placeholder="$t('验证码')"
                                        @blur="blurFun('mobileCode', formState.mobileCode)"
                                    >
                                        <template #suffix>
                                            <MobileCode
                                                class="mobile-code-text"
                                                :mobile="moblieNum"
                                                v-model:mobileCodeSended="mobileCodeSended"
                                                :requestApi="sendMobileCode"
                                                @mobileErrorCallback="mobileErrorCallback"
                                            ></MobileCode>
                                        </template>
                                    </el-input>
                                    <div class="error-notice">
                                        {{ formState.tigTips.mobileCode?.message }}
                                    </div>
                                </div>
                            </div>
                            <div class="form-item">
                                <div class="form-item-label">
                                    <div>{{ $t("设置密码") }}</div>
                                </div>
                                <div class="form-item-input">
                                    <el-input
                                        v-model="formState.password"
                                        class="input-border-style"
                                        show-password
                                        type="password"
                                        :placeholder="$t('设置密码')"
                                        @blur="blurFun('password', formState.password)"
                                    />
                                    <div class="error-notice">
                                        {{ formState.tigTips.password?.message }}
                                    </div>
                                </div>
                            </div>
                        </div>
                    </Transition>
                    <Transition name="slide-fade2" @after-leave="show = true">
                        <div class="form-box" v-if="isOverseas() && formState.registType && show">
                            <div class="form-item">
                                <div class="form-item-label">{{ $t("邮箱") }}</div>
                                <div class="form-item-input">
                                    <el-input
                                        v-model="formState.email"
                                        class="input-border-style"
                                        :placeholder="$t('邮箱')"
                                        @blur="blurFun('email', formState.email)"
                                    />
                                    <div class="error-notice">
                                        {{ formState.tigTips.email?.message }}
                                    </div>
                                </div>
                            </div>
                            <div class="form-item">
                                <div class="form-item-label">
                                    {{ $t("验证码") }}
                                </div>
                                <div class="form-item-input">
                                    <el-input
                                        v-model="formState.emailCode"
                                        class="input-border-style"
                                        :placeholder="$t('验证码')"
                                        @blur="blurFun('emailCode', formState.emailCode)"
                                    >
                                        <template #suffix>
                                            <MobileCode
                                                class="mobile-code-text"
                                                :email="formState.email"
                                                :isEmail="true"
                                                v-model:mobileCodeSended="mobileCodeSended"
                                                :requestApi="sendEmailCode"
                                                @mobileErrorCallback="mobileErrorCallback"
                                            ></MobileCode>
                                        </template>
                                    </el-input>
                                    <div class="error-notice">
                                        {{ formState.tigTips.emailCode?.message }}
                                    </div>
                                </div>
                            </div>
                            <div class="form-item">
                                <div class="form-item-label">
                                    <div>{{ $t("设置密码") }}</div>
                                </div>
                                <div class="form-item-input">
                                    <el-input
                                        v-model="formState.password"
                                        class="input-border-style"
                                        show-password
                                        type="password"
                                        :placeholder="$t('设置密码')"
                                        @blur="blurFun('password', formState.password)"
                                    />
                                    <div class="error-notice">
                                        {{ formState.tigTips.password?.message }}
                                    </div>
                                </div>
                            </div>
                        </div>
                    </Transition>
                </div>
                <div class="btn-box">
                    <div class="extra">
                        {{ $t("点击注册，表示您同意") }}
                        <NuxtLink class="font-color" target="_blank" :to="'/article/issue/info?articleSn=zcxy'">
                            《{{ $t("{0}注册协议！", [commonStore.shopName]) }}》
                        </NuxtLink>
                    </div>
                    <el-button :loading="loading" type="primary" class="login-btn" @click="onSubmit">{{ $t("同意协议并注册") }}</el-button>

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
                        <div class="login-img" v-if="commonStore.openWechatOauth === 1 && commonStore.openWechatPcLogin === 1" @click="showWechatLogin(false)">
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
                <div class="login-box">
                    <NuxtLink class="font-color" to="/member/login">{{ $t("已有账号？去登录！") }}</NuxtLink>
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
    <CommonPageFooter v-if="!registrationClosed" :bg="'white'" :type="0" :mt30="false"></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * @description 用户注册页面
 * - 支持手机号注册；邮箱注册仅跨境版且 openEmailRegister 开启时可用
 * - 手机号注册需要发送验证码进行验证
 * - 邮箱注册需要发送邮箱验证码
 * - 支持微信扫码注册（通过OAuth）
 * - 包含表单验证，确保密码强度和格式正确
 * - 海外版支持国际区号选择
 * - 与登录页共用 PC 登录页装修（home/home/pcLogin），应用头部高度/阴影
 */
import { ref } from "vue";
import { getWechatLoginUrl, checkWechatEvent, getOauth } from "@/api/user/login";
import MobileAreaCode from "@/components/form/src/MobileAreaCode.vue";
import { sendMobileCode, userRegist, sendEmailCode } from "@/api/user/regist";
import QRCode from "qrcode";
import { MobileCode } from "@/components/form";
import { ElMessage } from "element-plus";
import { useUserStore } from "@/store/user";
import { useCommonStore } from "~/store/common";
import { isOverseas } from "@/utils/util";
import { usePcLoginDecorate } from "~/composables/usePcLoginDecorate";

const route = useRoute();
const contentChange = ref(true);
const commonStore = useCommonStore();
const registrationClosed = computed(() => commonStore.shopRegClosed === 1);
let registrationRedirecting = false;
const redirectFromClosedRegistration = async () => {
    if (registrationRedirecting) return;
    registrationRedirecting = true;
    const previousRoute = window.history.state?.back;
    const targetRoute =
        typeof previousRoute === "string" && previousRoute.startsWith("/") && !previousRoute.startsWith("/member/register") ? previousRoute : "/";
    await navigateTo(targetRoute, { replace: true });
};
const userStore = useUserStore();

const { t } = useI18n();

// PC 登录页装修配置与派生（与登录页共用），注册页取头部高度/阴影 + 经典标记
const { loginMode, headerHeight, headerShadow, logoWidth, logoHeight } = await usePcLoginDecorate();

const mobileCodeSended = ref(false);

const mobileAreaCode = ref("86");

const formRef = shallowRef();

const menuList = [
    { label: t("手机号注册"), value: "" },
    { label: t("邮箱注册"), value: "email" }
];

const blurFun = (label: any, value: any) => {
    formRules[label].forEach((rule: any) => {
        rule.validator(rule, value, (err: any) => {
            if (err) {
                formState.value.tigTips[label] = { type: "error", message: err.message };
            } else {
                formState.value.tigTips[label] = { type: "valid", message: "" };
            }
        });
    });
};
const show = ref(true);
const registerTypeChange = (type: string) => {
    // 非跨境不支持邮箱注册
    if (type === "email" && !isOverseas()) return;
    formState.value.registType = type;
    show.value = false;
    formState.value.tigTips = {};
    if (type === "email") {
        formState.value.mobile = "";
        formState.value.mobileCode = "";
        formState.value.tigTips.mobile = { type: "info", message: focusList.mobile };
    } else {
        formState.value.email = "";
        formState.value.tigTips.email = { type: "info", message: focusList.email };
    }
};

const validateMobile = (rule: any, value: any, callback: any) => {
    const regex = commonStore.isOpenMobileAreaCode ? /^(\+?[1-9]\d{1,14})$/ : /^(?:\+?86)?1[3-9]\d{9}$/;
    if (!value) {
        callback(new Error(t("手机号不能为空")));
    } else if (!isOverseas() && !regex.test(value)) {
        callback(new Error(t("格式错误，请输入正确的手机号码")));
    } else {
        callback();
    }
};
const validateMail = (rule: any, value: any, callback: any) => {
    if (!value) {
        return callback(new Error(t("邮箱不能为空")));
    } else if (!/^\w+([-+.]\w+)*@\w+([-.]\w+)*\.\w+([-.]\w+)*$/.test(value)) {
        return callback(new Error(t("格式错误，请输入正确的邮箱地址")));
    } else {
        callback();
    }
};
const validateCode = (rule: any, value: any, callback: any) => {
    if (!value) {
        return callback(new Error(t("验证码不能为空")));
    } else if (value.length != 6) {
        return callback(new Error(t("请输入6位验证码")));
    } else {
        callback();
    }
};
const validatePassword = (rule: any, value: any, callback: any) => {
    if (!value) {
        return callback(new Error(t("密码不能为空")));
    } else if (value.length > 20 || value.length < 6) {
        return callback(new Error(t("密码应为6-20位字符")));
    } else if (/\s/.test(value)) {
        return callback(new Error(t("密码中不允许有空格")));
    } else if (/^\d+$/.test(value)) {
        return callback(new Error(t("密码不能全为数字")));
    } else if (!/^(?=.*[\d\W]).+$/.test(value)) {
        return callback(new Error(t("密码不能全为字母")));
    } else if (!/^(?=.*[a-zA-Z0-9]).+$/.test(value)) {
        return callback(new Error(t("密码不能全为符号")));
    } else if (!/^(?!([a-zA-Z0-9])\1*$).+$/.test(value)) {
        return callback(new Error(t("密码不能全为相同字符或数字")));
    } else {
        callback();
    }
};
const focusList: any = {
    mobile: t("请填写正确的手机号码，以便接收订单通知，找回密码等"),
    email: t("请填写正确的邮箱地址，以便接收订单通知，找回密码等"),
    mobileCode: t("请输入验证码"),
    password: t("6-20个大小写英文字母、符号或数字的组合")
};
const formRules = {
    mobile: [{ validator: validateMobile, trigger: "blur", required: true }],
    email: [{ validator: validateMail, trigger: "blur", required: true }],
    mobileCode: [{ validator: validateCode, trigger: "blur", required: true }],
    emailCode: [{ validator: validateCode, trigger: "blur", required: true }],
    password: [{ validator: validatePassword, trigger: "blur", required: true }]
};
const loading = ref<boolean>(false);

const formState: any = ref({
    tigTips: {},
    registType: ""
});

const moblieNum = computed(() => {
    return isOverseas() && commonStore.isOpenMobileAreaCode ? mobileAreaCode.value + formState.value.mobile : formState.value.mobile;
});

const onSubmit = async () => {
    // 非跨境强制走手机号注册
    const isEmailRegist = isOverseas() && formState.value.registType === "email";
    const fieldsToValidate = isEmailRegist ? ["email", "emailCode", "password"] : ["mobile", "mobileCode", "password"];
    const allFieldsValid = await Promise.all(
        fieldsToValidate.map(async (label: string) => {
            return new Promise((resolve) => {
                blurFun(label, formState.value[label]);
                // 添加一个短暂的延迟以确保验证完成
                setTimeout(() => {
                    resolve(formState.value.tigTips[label]?.type === "valid");
                }, 100);
            });
        })
    );
    if (!allFieldsValid.every((isValid) => isValid)) {
        return;
    }
    try {
        loading.value = true;
        const result = await userRegist({
            ...formState.value,
            registType: isEmailRegist ? "email" : "",
            mobile: moblieNum.value
        });
        ElMessage.success(t("注册成功"));
        userStore.setToken(result.token);
        await userStore.validateUserInfoOnce();
        navigateTo("/member/");
    } catch (error: any) {
        ElMessage.error(error.message);
    } finally {
        loading.value = false;
    }
};
const mobileErrorCallback = (e: any) => {
    ElMessage.error(e);
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
        ElMessage.error(error?.message || t("微信登录失败，请重试"));
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

const oauthPath = useCookie("oauthPath");
const getReturnUrl = () => {
    const queryReturnUrl = route.query.returnUrl;
    if (typeof queryReturnUrl === "string" && queryReturnUrl) {
        return queryReturnUrl;
    }
    if (Array.isArray(queryReturnUrl) && queryReturnUrl[0]) {
        return queryReturnUrl[0];
    }
    if (route.path.startsWith("/member/")) {
        return "";
    }
    return route.fullPath || route.path || "";
};
const handleOtherLogin = async (path: string) => {
    oauthPath.value = path;
    try {
        const result = await getOauth(path, getReturnUrl());
        window.location.href = result;
    } catch (error) {
        console.error(error);
    }
};
const height = ref(0);
onMounted(() => {
    watch(
        registrationClosed,
        (closed) => {
            if (closed) void redirectFromClosedRegistration();
        },
        { immediate: true }
    );
    if (registrationClosed.value) return;
    const el = document.querySelector(".register-info");
    height.value = el ? el.clientHeight : 0;
});
</script>
<style lang="less" scoped>
.full-height-div {
    display: flex;
    min-height: calc(100vh - 170px - 104px); /* 设置 div 的高度为视口的 100% */
    background-color: white;
    justify-content: center;

    .register-info {
        margin: 50px 0;
        min-width: 420px;

        .title {
            font-size: 24px;
            font-weight: bold;
            text-align: center;
        }

        .register-menu {
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
                        color: #1a1a1a;
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
        }

        .content {
            padding: 0 20px;
            font-size: 14px;

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
                    .form-item-input {
                        position: relative;
                        margin-bottom: 10px;
                        .error-notice {
                            position: absolute;
                            top: 43px;
                            color: rgba(250, 44, 25, 1);
                            padding-top: 10px;
                        }
                    }
                }
            }
        }
        .btn-box {
            padding: 20px;
            display: flex;
            flex-direction: column;
            align-items: center;
            row-gap: 15px;

            .extra {
                width: 100%;
            }

            .login-btn {
                width: 100%;
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
        .login-box {
            text-align: center;
        }
    }

    // 微信扫码注册与登录页使用相同的内容布局，避免切换后按默认样式堆叠
    .wechat-main {
        position: relative;

        .wechat-main-back {
            position: absolute;
            top: 0;
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

.input-border-style {
    height: 50px;
}

.area-code-select {
    width: 60px;
    padding: 10px 0;
    :deep(.el-select__wrapper) {
        box-shadow: none !important;
    }
}

.label {
    position: absolute;
    left: -117px;
    width: 100px;
    text-align: right;
    color: #666;
}
.submit-button {
    height: 50px;
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
.area-code-input {
    :deep(.el-input__inner) {
        margin-left: 10px;
    }
}
:deep(.el-input__inner) {
    &::placeholder {
        color: #505259 !important;
    }
    .code-btn {
        font-size: 15px !important;
    }
}
:deep(.el-select__wrapper) {
    font-size: 15px;
}
</style>
