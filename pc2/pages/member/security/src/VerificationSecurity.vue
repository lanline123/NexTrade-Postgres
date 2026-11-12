<template>
    <el-dialog v-model="dialogVisible" :close-on-click-modal="false" :title="params.title" width="450">
        <div style="padding: 10px">
            <el-form ref="formRef" :model="formState" :rules="formRules" class="form-body" label-suffix="：">
                <template v-if="params.type == 1">
                    <template v-if="oldMobile">
                        <el-form-item :label="$t('原手机号')">
                            {{ formState.mobile? phoneEncrypt(formState.mobile): "" }}
                        </el-form-item>
                        <el-form-item :rules="[{ required: true, message: $t('请输入验证码') }]" label="" prop="mobileCode">
                            <el-input v-model="formState.mobileCode" :placeholder="$t('请输入验证码')" type="text" size="large">
                                <template #suffix>
                                    <MobileCode
                                        v-model:mobile="formState.mobile"
                                        v-model:mobileCode="formState.mobileCode"
                                        v-model:mobileCodeSended="mobileCodeSended"
                                        :requestApi="sendMobileCodeByMobileValidate"
                                        :need-mobile="false"
                                        :showMessage="true"
                                        class="mobile-code-text"
                                        @mobileErrorCallback="mobileErrorCallback"
                                    >
                                    </MobileCode>
                                </template>
                            </el-input>
                        </el-form-item>
                        <el-button :disabled="isDisabled" style="width: 100%" type="primary" @click="nextStep" size="large">{{ $t("下一步") }}</el-button>
                    </template>
                    <template v-else>
                        <!-- :rules="[
                                { required: true, message: $t('请输入手机号码') },
                                { pattern: commonStore.isOpenMobileAreaCode ?/^[1-9]\d{10,14}$/: /^1[3-9]\d{9}$/, message: $t('请输入有效手机号'), trigger: ['blur', 'change'] }
                            ]" -->
                        <el-form-item label="" prop="newMobile">
                            <el-input v-model="formState.newMobile" clearable :placeholder="$t('请输入手机号码')" size="large">
                                <template v-if="commonStore.isOpenMobileAreaCode" #prefix>
                                    <MobileAreaCode v-model="mobileAreaCode"></MobileAreaCode>
                                </template>
                            </el-input>
                        </el-form-item>
                        <el-form-item :rules="[{ required: true, message: $t('请输入验证码') }]" label="" prop="newMobileCode">
                            <el-input v-model="formState.newMobileCode" :placeholder="$t('请输入验证码')" type="text" size="large">
                                <template #suffix>
                                    <MobileCode
                                        :mobile="commonStore.isOpenMobileAreaCode ? mobileAreaCode + formState.newMobile : formState.newMobile"
                                        v-model:mobileCode="formState.newMobileCode"
                                        v-model:mobileCodeSended="newMobileCodeSended"
                                        :requestApi="sendMobileCodeByMobileMobile"
                                        :showMessage="true"
                                        class="mobile-code-text"
                                        @mobileErrorCallback="mobileErrorCallback"
                                    >
                                    </MobileCode>
                                </template>
                            </el-input>
                        </el-form-item>
                        <el-button :disabled="isNewDisabled" style="width: 100%" type="primary" @click="onsubmit" size="large">{{ $t("提交") }}</el-button>
                    </template>
                </template>
                <template v-else-if="params.type == 2">
                    <el-form-item :label="$t('手机号')">
                        {{ formState.mobile }}
                    </el-form-item>
                    <el-form-item :rules="[{ required: true, message: $t('请输入验证码') }]" label="" prop="mobileCode">
                        <el-input v-model="formState.mobileCode" :placeholder="$t('请输入验证码')" type="text" size="large">
                            <template #suffix>
                                <MobileCode
                                    v-model:mobile="formState.mobile"
                                    v-model:mobileCode="formState.mobileCode"
                                    v-model:mobileCodeSended="passwordMobileCodeSended"
                                    :requestApi="sendMobileCodeByMobilePassword"
                                    :need-mobile="false"
                                    :showMessage="true"
                                    class="mobile-code-text"
                                    @mobileErrorCallback="mobileErrorCallback"
                                ></MobileCode>
                            </template>
                        </el-input>
                    </el-form-item>
                    <el-form-item label="" prop="password">
                        <el-input v-model="formState.password" clearable :placeholder="$t('请输入新密码')" size="large" />
                    </el-form-item>
                    <el-form-item label="" prop="confirmPassword">
                        <el-input v-model="formState.confirmPassword" clearable :placeholder="$t('请再次输入新密码')" size="large" />
                    </el-form-item>
                    <el-button :disabled="isPasswordDisabled" style="width: 100%" type="primary" size="large" @click="onsubmitPassword">{{
                        $t("提交")
                    }}</el-button>
                </template>
                <template v-else-if="params.type == 3">
                    <!-- :rules="[
                            { required: true, message: $t('请输入手机号码') },
                            { pattern: commonStore.isOpenMobileAreaCode ?/^[1-9]\d{10,14}$/: /^1[3-9]\d{9}$/, message: $t('请输入有效手机号'), trigger: ['blur', 'change'] }
                        ]" -->
                    <el-form-item label="" prop="newMobile">
                        <el-input v-model="formState.newMobile" clearable :placeholder="$t('请输入手机号码')" size="large">
                            <template v-if="commonStore.isOpenMobileAreaCode" #prefix>
                                <MobileAreaCode v-model="mobileAreaCode"></MobileAreaCode>
                            </template>
                        </el-input>
                    </el-form-item>
                    <el-form-item :rules="[{ required: true, message: $t('验证码不能为空') }]" label="" prop="newMobileCode">
                        <el-input v-model="formState.newMobileCode" :placeholder="$t('请输入验证码')" type="text" size="large">
                            <template #suffix>
                                <MobileCode
                                    :mobile="commonStore.isOpenMobileAreaCode ? mobileAreaCode + formState.newMobile : formState.newMobile"
                                    v-model:mobileCode="formState.newMobileCode"
                                    v-model:mobileCodeSended="newMobileCodeSended"
                                    :requestApi="sendMobileCodeByMobileMobile"
                                    :showMessage="true"
                                    class="mobile-code-text"
                                    @mobileErrorCallback="mobileErrorCallback"
                                ></MobileCode>
                            </template>
                        </el-input>
                    </el-form-item>
                    <el-button :disabled="isNewDisabled" style="width: 100%" type="primary" @click="onsubmit" size="large">{{ $t("提交") }}</el-button>
                </template>
                <template v-else-if="params.type == 4">
                    <template v-if="onEmail">
                        <el-form-item :label="$t('原邮箱')">
                            {{ formState.email }}
                        </el-form-item>
                        <el-form-item :rules="[{ required: true, message: $t('验证码不能为空') }]" label="" prop="emailCode">
                            <el-input v-model="formState.emailCode" :placeholder="$t('请输入验证码')" type="text" size="large">
                                <template #suffix>
                                    <MobileCode
                                        :email="formState.email"
                                        v-model:mobileCode="formState.emailCode"
                                        v-model:mobileCodeSended="newMobileCodeSended"
                                        :requestApi="sendEmailCodeByEmailValidate"
                                        :showMessage="true"
                                        class="mobile-code-text"
                                        @mobileErrorCallback="mobileErrorCallback"
                                    ></MobileCode>
                                </template>
                            </el-input>
                        </el-form-item>
                        <el-button :disabled="isDisabledEmail" style="width: 100%" type="primary" @click="nextStepEmail" size="large">{{
                            $t("下一步")
                        }}</el-button>
                    </template>
                    <template v-else>
                        <el-form-item label="" prop="newEmail">
                            <el-input v-model="formState.newEmail" clearable :placeholder="$t('请输入邮箱')" size="large" @input="onInputEmail"></el-input>
                        </el-form-item>
                        <el-form-item :rules="[{ required: true, message: $t('请输入验证码') }]" label="" prop="newEmailCode">
                            <el-input v-model="formState.newEmailCode" :placeholder="$t('请输入验证码')" type="text" size="large">
                                <template #suffix>
                                    <MobileCode
                                        :email="formState.newEmail"
                                        v-model:mobileCode="formState.newEmailCode"
                                        v-model:mobileCodeSended="newMobileCodeSended"
                                        :requestApi="sendEmailCodeByModifyEmail"
                                        :showMessage="true"
                                        class="mobile-code-text"
                                        @mobileErrorCallback="mobileErrorCallback"
                                    >
                                    </MobileCode>
                                </template>
                            </el-input>
                        </el-form-item>
                        <el-button style="width: 100%" type="primary" @click="onsubmitEmail" size="large">{{ $t("提交") }}</el-button>
                    </template>
                </template>
                <template v-else-if="params.type == 5">
                    <el-form-item label="" prop="email">
                        <el-input v-model="formState.email" clearable :placeholder="$t('请输入邮箱')" size="large"></el-input>
                    </el-form-item>
                    <el-form-item :rules="[{ required: true, message: $t('请输入验证码') }]" label="" prop="emailCode">
                        <el-input v-model="formState.emailCode" :placeholder="$t('请输入验证码')" type="text" size="large">
                            <template #suffix>
                                <MobileCode
                                    :email="formState.email"
                                    v-model:mobileCode="formState.emailCode"
                                    v-model:mobileCodeSended="newMobileCodeSended"
                                    :requestApi="sendEmailCodeByModifyEmail"
                                    :showMessage="true"
                                    class="mobile-code-text"
                                    @mobileErrorCallback="mobileErrorCallback"
                                >
                                </MobileCode>
                            </template>
                        </el-input>
                    </el-form-item>
                    <el-button style="width: 100%" type="primary" @click="bindEmail" size="large">{{ $t("提交") }}</el-button>
                </template>
            </el-form>
        </div>
    </el-dialog>
    <span @click="showDialog">
        <slot></slot>
    </span>
</template>
<script lang="ts" setup>
/**
 * 安全验证组件
 * - 支持修改密码、换绑手机、换绑邮箱等安全操作
 * - 多步骤验证流程：原手机/邮箱验证 -> 新手机/邮箱绑定
 * - 支持验证码发送和校验
 */
import { ref } from "vue";
import type { SecurityFormState } from "~/types/user/security.d";
import {
    checkModifyPasswordMobileCode,
    mobileMobile,
    mobileValidate,
    sendMobileCodeByMobileMobile,
    sendMobileCodeByMobilePassword,
    sendMobileCodeByMobileValidate,
    sendEmailCodeByEmailValidate,
    emailValidate,
    sendEmailCodeByModifyEmail,
    emailEmailValidate
} from "~/api/user/security";
import { MobileCode } from "~/components/form";
import MobileAreaCode from "@/components/form/src/MobileAreaCode.vue";
import { useUserStore } from "~/store/user";
import { useCommonStore } from "~/store/common";
import { isOverseas } from "~/utils/util";

const commonStore: any = useCommonStore();
const { t } = useI18n();

const mobileAreaCode = ref("");
const mobileCodeSended = ref(false);
const isDisabled = computed(() => {
    return formState.value.mobile == "" || formState.value.mobileCode == "" || mobileCodeSended.value == false;
});
const newMobileCodeSended = ref(false);
const isNewDisabled = computed(() => {
    return formState.value.newMobile == "" || formState.value.newMobileCode == "" || newMobileCodeSended.value == false;
});
const passwordMobileCodeSended = ref(false);
const isPasswordDisabled = computed(() => {
    return formState.value.mobile == "" || formState.value.mobileCode == "" || passwordMobileCodeSended.value == false;
});
const isDisabledEmail = computed(() => {
    return formState.value.email == "" || formState.value.newEmail == "";
});
const onInputEmail = () => {
    formState.value.email = formState.value.newEmail;
};
const phoneEncrypt = (phone: string) => {
    return phone.replace(/^(\d{3})\d{4}(\d{4})$/, "$1****$2").replace(/^(\d{3})\d{4}(\d{4})$/, "$1****$2");
};
const props = defineProps({
    params: {
        type: Object,
        default: {}
    }
});
const dialogVisible = ref(false);
const oldMobile = ref(true);
const onEmail = ref(true);
const formState = ref<SecurityFormState>({});

const formRef = shallowRef();
const showDialog = async () => {
    // 已打开时不重复重置，避免输入框被清空
    if (dialogVisible.value) return;
    dialogVisible.value = true;
    oldMobile.value = true;
    if (formRef.value) await formRef.value.resetFields();
    console.log(props.params);

    formState.value.mobile = props.params.mobile;
    formState.value.email = props.params.email;
};
const nextStep = async () => {
    await formRef.value.validate();
    try {
        let temp = {
            code: formState.value.mobileCode
        };
        const result = await mobileValidate({ ...temp });
        oldMobile.value = false;
    } catch (error: any) {
        if (error.message) {
            message.error(error.message);
        }
    } finally {
    }
};
const nextStepEmail = async () => {
    await formRef.value.validate();
    try {
        let temp = {
            email: formState.value.email,
            code: formState.value.emailCode
        };
        await emailValidate({ ...temp });
        onEmail.value = false;
    } catch (error: any) {
        if (error.message) {
            message.error(error.message);
        }
    } finally {
    }
};
const emit = defineEmits(["loadFilter"]);
const onsubmit = async () => {
    await formRef.value.validate();
    try {
        let temp = {
            mobile: commonStore.isOpenMobileAreaCode ? mobileAreaCode.value + formState.value.newMobile : formState.value.newMobile,
            code: formState.value.newMobileCode
        };
        const result = await mobileMobile({ ...temp });
        message.success(t("修改成功"));
        dialogVisible.value = false;
        emit("loadFilter");
        // oldMobile.value = false;
    } catch (error: any) {
        if (error.message) {
            message.error(error.message);
        }
    } finally {
    }
};
const onsubmitEmail = async () => {
    await formRef.value.validate();
    try {
        let temp = {
            email: formState.value.newEmail,
            code: formState.value.newEmailCode
        };
        const result = await emailEmailValidate({ ...temp });
        message.success(t("修改成功"));
        dialogVisible.value = false;
        emit("loadFilter");
    } catch (error: any) {
        if (error.message) {
            message.error(error.message);
        }
    } finally {
    }
};

const bindEmail = async () => {
    await formRef.value.validate();
    try {
        let temp = {
            email: formState.value.email,
            code: formState.value.emailCode
        };
        await emailEmailValidate({ ...temp });
        message.success(t("绑定成功"));
        dialogVisible.value = false;
        emit("loadFilter");
    } catch (error: any) {
        if (error.message) {
            message.error(error.message);
        }
    } finally {
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
const validatePassword2 = (rule: any, value: any, callback: any) => {
    if (!value) {
        return callback(new Error(t("请再次输入密码")));
    } else if (value != formState.value.password) {
        return callback(new Error(t("两次密码不一致")));
    } else {
        callback();
    }
};

const validateMobile = (rule: any, value: any, callback: any) => {
    const regex = commonStore.isOpenMobileAreaCode ? /^(\+?[1-9]\d{1,14})$/ : /^(?:\+86)?1(?:3[0-9]|4[5-9]|5[0-35-9]|6[5-7]|7[0-8]|8[0-9]|9[189])\d{8}$/;
    if (!value) {
        return callback(new Error(t("手机号不能为空")));
    } else if (!isOverseas() && !regex.test(value)) {
        return callback(new Error(t("格式错误，请输入正确的手机号码")));
    } else {
        callback();
    }
};

const formRules = reactive<any>({
    password: [{ validator: validatePassword, trigger: "blur", required: true }],
    confirmPassword: [{ validator: validatePassword2, trigger: "change", required: true }],
    newMobile: [{ validator: validateMobile, required: true }]
});

const onsubmitPassword = async () => {
    await formRef.value.validate();
    try {
        let temp = {
            password: formState.value.password,
            code: formState.value.mobileCode
        };
        await checkModifyPasswordMobileCode({ ...temp });
        message.success(t("修改成功"));
        await userStore.loginOut();
        dialogVisible.value = false;
    } catch (error: any) {
        if (error.message) {
            message.error(error.message);
        }
    } finally {
    }
};

const userStore = useUserStore();

userStore.getUserInfo();

const errorMessage = ref("");
const mobileErrorCallback = (e: any) => {
    errorMessage.value = e;
};
defineExpose({
    showDialog
})
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

:deep(.el-select__wrapper) {
    box-shadow: none !important;

    &.is-hovering:not(.is-focused) {
        box-shadow: none;
    }
}
:deep(.textarea__inner):hover {
    box-shadow: none;
}
:deep(.el-form-item) {
    margin-bottom: 25px;
}
</style>
