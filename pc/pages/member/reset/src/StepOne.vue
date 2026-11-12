<template>
    <div>
        <template v-if="commonStore.openEmailRegister === 1">
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
                                <template #prefix>
                                    <div class="area-code-select" v-if="isOverseas() && commonStore.isOpenMobileAreaCode">
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
                            <el-input v-model="formState.code" class="input-border-style" :placeholder="$t('验证码')" @blur="blurFun('code', formState.code)">
                                <template #suffix>
                                    <MobileCode
                                        class="mobile-code-text"
                                        event="forgetPassword"
                                        :mobile="isOverseas() && commonStore.isOpenMobileAreaCode ? mobileAreaCode + formState.mobile : formState.mobile"
                                        v-model:mobileCodeSended="mobileCodeSended"
                                        :requestApi="sendMobileCode"
                                        @mobileErrorCallback="mobileErrorCallback"
                                    ></MobileCode>
                                </template>
                            </el-input>
                            <div class="error-notice">
                                {{ formState.tigTips.code?.message }}
                            </div>
                        </div>
                    </div>
                </div>
            </Transition>
            <Transition name="slide-fade2" @after-leave="show = true">
                <div class="form-box" v-if="formState.registType && show">
                    <div class="form-item">
                        <div class="form-item-label">{{ $t("邮箱") }}</div>
                        <div class="form-item-input">
                            <el-input
                                v-model="formState.email"
                                class="input-border-style"
                                :placeholder="$t('邮箱')"
                                @blur="blurFun('email', formState.email)"
                            />
                            <!-- <TigTips :info="formState.tigTips.email"></TigTips> -->
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
                            <el-input v-model="formState.code" class="input-border-style" :placeholder="$t('验证码')" @blur="blurFun('code', formState.code)">
                                <template #suffix>
                                    <MobileCode
                                        event="forgetPassword"
                                        class="mobile-code-text"
                                        :email="formState.email"
                                        v-model:mobileCodeSended="mobileCodeSended"
                                        :requestApi="sendEmailCode"
                                        @mobileErrorCallback="mobileErrorCallback"
                                    ></MobileCode>
                                </template>
                            </el-input>
                            <div class="error-notice">
                                {{ formState.tigTips.code?.message }}
                            </div>
                        </div>
                    </div>
                </div>
            </Transition>
            <div class="btn-box">
                <el-button :loading="loading" type="primary" class="submit-button" style="width: 100%" @click="onSubmit">{{ $t("下 一 步") }}</el-button>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * 找回密码第一步组件
 * - 输入手机号或邮箱进行身份验证
 * - 发送验证码并校验
 * - 验证通过后进入下一步设置新密码
 */
import { ref } from "vue";
import MobileAreaCode from "@/components/form/src/MobileAreaCode.vue";
import { checkMobile, sendMobileCode, sendEmailCode, checkEmail } from "@/api/user/login";
import { MobileCode } from "@/components/form";
import { ElMessage } from "element-plus";
import { useCommonStore } from "~/store/common";
import { isOverseas } from "~/utils/util";

const step = defineModel("step");

const emit = defineEmits(["success"]);

const commonStore: any = useCommonStore();
const { t } = useI18n();
const mobileCodeSended = ref(false);
const mobileAreaCode = ref("86");
const formRef = shallowRef();
const validateFun = (prop: any, isValid: boolean, message: string) => {
    if (!isValid) {
        formState.value.tigTips[prop] = { type: "error", message: message };
    } else {
        formState.value.tigTips[prop] = { type: "valid", message: message };
    }
};
const menuList = [
    { label: t("手机号找回"), value: "" },
    { label: t("邮箱找回"), value: "email" }
];

// const focusFun = (val: any) => {
//     formState.value.tigTips[val] = { type: "info", message: focusList[val] };
// };

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

const validateMobile = (rule: any, value: any, callback: any) => {
    const regex = commonStore.isOpenMobileAreaCode ? /^(\+?[1-9]\d{1,14})$/ : /^(?:\+?86)?1[3-9]\d{9}$/;
    if (!value) {
        return callback(new Error(t("手机号不能为空")));
    } else if (!isOverseas() && !regex.test(value)) {
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
const show = ref(true);
const registerTypeChange = (type: string) => {
    formState.value.registType = type;
    show.value = false;
    formState.value.tigTips = {};
    if (type === "email") {
        formState.value.mobile = "";
        formState.value.code = "";
        formState.value.tigTips.mobile = { type: "info", message: focusList.mobile };
    } else {
        formState.value.email = "";
        formState.value.code = "";
        formState.value.tigTips.email = { type: "info", message: focusList.email };
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
    mobile: t("请填写手机号码"),
    email: t("请填写邮箱地址"),
    code: t("请输入验证码"),
    password: t("6-20个大小写英文字母、符号或数字的组合")
});
const formRules = reactive<any>({
    mobile: [{ validator: validateMobile, trigger: "blur", required: true }],
    email: [{ validator: validateMail, trigger: "blur", required: true }],
    code: [{ validator: validateCode, trigger: "blur", required: true }],
    password: [{ validator: validatePassword, trigger: "blur", required: true }]
});
const loading = ref<boolean>(false);
const formState: any = ref({
    tigTips: {},
    mobile: "",
    registType: "",
    code: ""
});
const onSubmit = async () => {
    const fieldsToValidate = formState.value.registType === "" ? ["mobile", "code"] : ["email", "code"];
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
        ElMessage.error(t("请检查表单信息"));
        return;
    }
    try {
        loading.value = true;
        let result;
        if (formState.value.registType === "email") {
            // 邮箱找回
            const tempEmail = {
                email: formState.value.email,
                code: formState.value.code
            };
            // 这里可以加邮箱校验接口，如有需要
            result = await checkEmail(tempEmail);
        } else {
            // 手机号找回
            const temp = {
                mobile: isOverseas() && commonStore.isOpenMobileAreaCode ? mobileAreaCode.value + formState.value.mobile : formState.value.mobile,
                code: formState.value.code
            };
            result = await checkMobile(temp);
        }
        emit("success", result);
        step.value = 2;
    } catch (error: any) {
        ElMessage.error(error.message || t("操作失败"));
        formState.value.code = "";
        formState.value.tigTips = {};
    } finally {
        loading.value = false;
    }
};
const mobileErrorCallback = (e: any) => {
    ElMessage.error(e);
};
</script>

<style lang="less" scoped>
.content {
    width: 380px;
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

.input-border-style {
    height: 50px;
}
.area-code-input {
    :deep(.el-input__inner) {
        margin-left: 10px;
    }
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

.btn-box {
    padding: 20px 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    row-gap: 15px;
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
