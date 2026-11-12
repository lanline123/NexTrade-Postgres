<template>
    <div :class="props.class">
        <div v-if="ableSend" class="code-btn" @click.stop="onClick">{{ $t(btnText) }}</div>
        <div v-if="!ableSend" class="code-text">{{ codeText }}</div>
    </div>
    <Verify ref="verify" mode="pop" captchaType="blockPuzzle" :imgSize="{ width: '310px', height: '155px' }" @okCallback="okCallback"></Verify>
</template>
<script setup lang="ts">
import { ref, toRefs, onUnmounted, computed } from "vue";
import { message } from "ant-design-vue";
import Verify from "@/components/verifition/Verify.vue";
import { result } from "lodash-es";
import { useCommonStore } from "~/store/common";
const commonStore = useCommonStore();
const { t } = useI18n();
const props = defineProps({
    class: String,
    email: { type: String, default: "" },
    mobileCodeSended: { type: Boolean, default: false },
    showMessage: { type: Boolean, default: false },
    mobile: { type: String, default: "" },
    needMobile: { type: Boolean, default: true },
    isEmail: { type: Boolean, default: false },
    requestApi: { type: Function, default: result },
    event: { type: String, default: "login" }
});
const { mobile } = toRefs(props);
const emit = defineEmits(["mobileErrorCallback", "update:mobileCodeSended"]);
const btnText = ref("获取验证码");
const ableSend = ref(true);
const verify = ref();
const verifyToken = ref(null);
const countdown = ref(60);
const onClick = async () => {
    if (!props.email && !props.isEmail) {
        if (props.needMobile) {
            if (mobile.value == "") {
                emit("mobileErrorCallback", t("手机号不能为空！"));
                return;
            }
            if (!isOverseas() && !validatePhoneNumber(mobile.value)) {
                emit("mobileErrorCallback", t("手机号格式错误"));
                return;
            }
        }
    } else {
        if (props.email == "") {
            emit("mobileErrorCallback", t("邮箱不能为空！"));
            return;
        }
        if (!validateEmail(props.email)) {
            emit("mobileErrorCallback", t("邮箱格式错误"));
            return;
        }
    }

    if (!verifyToken.value) {
        verify.value.show();
        return;
    }

    try {
        await props.requestApi({
            ...(props.needMobile ? { mobile: mobile.value } : {}),
            email: props.email,
            verifyToken: verifyToken.value,
            event: props.event
        });
        message.success(t("验证码已发送"));
        ableSend.value = false;
        emit("update:mobileCodeSended", true);
        startCountdown();
    } catch (error: any) {
        ableSend.value = true;
        if (props.showMessage) {
            message.error(error.message);
        }
        emit("mobileErrorCallback", error.message);
    } finally {
        verifyToken.value = null;
    }
};
const validatePhoneNumber = (phoneNumber: string) => {
    const regex = commonStore.isOpenMobileAreaCode ? /^(\+?[1-9]\d{1,14})$/ : /^(?:\+?86)?1[3-9]\d{9}$/; // 国内手机号正则表达式

    if (regex.test(phoneNumber)) {
        return true;
    } else {
        return false;
    }
};
const validateEmail = (email: string) => {
    const regex = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/;
    return regex.test(email);
};
const codeText = computed(() => {
    return t("{0}秒后重发", [countdown.value]);
});
const timer = ref();
const startCountdown = () => {
    timer.value = setInterval(() => {
        countdown.value--;
        if (countdown.value <= 0) {
            clearInterval(timer.value);
            ableSend.value = true;
            countdown.value = 60;
        }
    }, 1000);
};
const okCallback = (e: any) => {
    verifyToken.value = e.verifyToken;
    onClick();
};
onUnmounted(() => {
    clearInterval(timer.value);
});
</script>
<style lang="less" scoped>
.code-btn {
    color: #333;
    cursor: pointer;
    font-size: 12px;
    font-weight: normal;

    &.disabled {
        color: #999 !important;
        cursor: not-allowed;
    }
}
</style>
<style lang="less">
// 验证码发送成功等 toast 需显示在滑块验证 / 弹窗（z-index 3000~9999）之上
.ant-message {
    z-index: 10000 !important;
}
</style>
