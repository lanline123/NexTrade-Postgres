<template>
    <CommonHeader title="绑定手机"></CommonHeader>
    <CommonLoginHeader type="register"></CommonLoginHeader>
    <div class="full-height-div">
        <div class="register-info">
            <div class="title">{{ $t("绑定手机") }}</div>
            <el-form
                ref="formRef"
                :hide-required-asterisk="true"
                :model="formState"
                :rules="formRules"
                :show-message="false"
                class="form-body"
                label-width="auto"
                @validate="validateFun"
            >
                <el-form-item v-if="formState.registType === 'mobile' || !formState.registType" prop="mobile">
                    <div class="label">{{ $t("手机号") }}</div>
                    <el-input v-model="formState.mobile" class="input-border-style" style="" @focus="focusFun('mobile')">
                        <template #prefix>
                            <div class="area-code-select">
                                <el-select v-model="formState.areaCode" :placeholder="$t('请选择区号')">
                                    <el-option :label="'(+86)' + $t('中国')" value="+86" />
                                    <el-option :label="'(+61)' + $t('澳洲')" value="+61" />
                                </el-select>
                            </div>
                        </template>
                    </el-input>
                    <TigTips :info="formState.tigTips.mobile"></TigTips>
                </el-form-item>
                <el-form-item v-else prop="mail">
                    <div class="label">{{ $t("邮箱") }}</div>
                    <el-input v-model="formState.mail" class="input-border-style" @focus="focusFun('mail')" />
                    <TigTips :info="formState.tigTips.mail"></TigTips>
                </el-form-item>
                <div style="width: 100%; display: flex; justify-content: center; margin-top: -20px" v-if="false">
                    <div style="width: 330px; text-align: right; color: #06c">
                        <el-button v-if="formState.registType === 'mail'" link type="primary" @click="formState.registType = 'mobile'">
                            {{ $t("手机验证") }}</el-button
                        >
                        <el-button v-else link type="primary" @click="formState.registType = 'mail'">{{ $t("邮箱验证") }}</el-button>
                    </div>
                </div>
                <el-form-item prop="mobileCode">
                    <div class="label">{{ $t("验证码") }}</div>
                    <el-input v-model="formState.mobileCode" class="input-border-style" @focus="focusFun('mobileCode')">
                        <template #suffix>
                            <MobileCode
                                class="mobile-code-text"
                                v-model:mobile="formState.mobile"
                                v-model:mobileCode="formState.mobileCode"
                                v-model:mobileCodeSended="mobileCodeSended"
                                :requestApi="sendMobileCode"
                                @mobileErrorCallback="mobileErrorCallback"
                            ></MobileCode>
                        </template>
                    </el-input>
                    <TigTips :info="formState.tigTips.mobileCode"></TigTips>
                </el-form-item>
                <el-form-item prop="password">
                    <div class="label">{{ $t("设置密码") }}</div>
                    <el-input v-model="formState.password" class="input-border-style" show-password type="password" @focus="focusFun('password')" />
                    <TigTips :info="formState.tigTips.password"></TigTips>
                </el-form-item>
                <el-form-item label="" prop="firstWord">
                    <el-button :loading="loading" type="primary" class="submit-button" style="width: 330px" @click="onSubmit">{{ $t("确认") }}</el-button>
                </el-form-item>
            </el-form>
        </div>
    </div>
    <CommonPageFooter :bg="'white'" :type="0"></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * @description 绑定手机号页面
 * - 用于第三方登录后绑定手机号
 * - 发送手机验证码进行验证
 * - 绑定成功后更新用户状态
 */
import { ref } from "vue";
import { sendMobileCode, userBindMobile } from "~/api/user/regist";
import { MobileCode } from "~/components/form";
import { ElMessage } from "element-plus";
import { useUserStore } from "~/store/user";

const userStore = useUserStore();

const { t } = useI18n();

const mobileCodeSended = ref(false);

const formRef = shallowRef();

const validateFun = (prop: any, isValid: boolean, message: string) => {
    if (!isValid) {
        formState.value.tigTips[prop] = { type: "error", message: message };
    } else {
        formState.value.tigTips[prop] = { type: "valid", message: message };
    }
};

const focusFun = (val: any) => {
    formState.value.tigTips[val] = { type: "info", message: focusList[val] };
};

const validateMobile = (rule: any, value: any, callback: any) => {
    if (!value) {
        return callback(new Error(t("手机号不能为空")));
    } else if (/^(?=.*\D).+$/.test(value)) {
        return callback(new Error(t("格式错误，请输入正确的手机号码")));
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
const focusList = reactive<any>({
    mobile: t("请填写正确的手机号码，以便接收订单通知，找回密码等"),
    mail: t("请填写正确的邮箱地址，以便接收订单通知，找回密码等"),
    mobileCode: t("请输入验证码"),
    password: t("6-20个大小写英文字母、符号或数字的组合")
});
const formRules = reactive<any>({
    mobile: [{ validator: validateMobile, trigger: "blur", required: true }],
    mail: [{ validator: validateMail, trigger: "blur", required: true }],
    mobileCode: [{ validator: validateCode, trigger: "blur", required: true }],
    password: [{ validator: validatePassword, trigger: "blur", required: true }]
});
const loading = ref<boolean>(false);
const formState: any = ref({
    tigTips: {},
    areaCode: "+86"
});

const onSubmit = async () => {
    await formRef.value.validate();
    try {
        loading.value = true;
        const result = await userBindMobile({ ...formState.value, openData: userStore.openData });
        ElMessage.success(t("绑定成功"));
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
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.full-height-div {
    display: flex;
    min-height: calc(100vh - 170px - 104px); /* 设置 div 的高度为视口的 100% */
    background-color: white;
    justify-content: center;

    .register-info {
        width: 500px;
        margin: 50px 0;

        .title {
            font-size: 24px;
            font-weight: bold;
            text-align: center;
        }
    }

    .form-body {
        margin-top: 30px;
        display: flex;
        justify-content: center;
        align-items: center;
        flex-direction: column;
        gap: 15px;

        .get-check {
            width: 110px;
            background-color: #f3f3f3;
            cursor: pointer;
            margin-left: 10px;
            color: #666;
            height: 50px;
            text-align: center;
            line-height: 50px;
            border: 1px solid #dedede;
            border-radius: 4px;
        }
    }
}

.input-border-style {
    height: 50px;
    width: 330px;
}

.area-code-select {
    width: 120px;
    padding: 10px 0;
    :deep(.el-select__wrapper) {
        box-shadow: none !important;
    }
}

.label {
    position: absolute;
    left: -100px;
    width: 80px;
    text-align: right;
    color: #666;
}
.submit-button {
    height: 50px;
}
</style>
