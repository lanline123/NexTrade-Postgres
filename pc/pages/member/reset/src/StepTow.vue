<template>
    <div class="content">
        <div class="form-box">
            <div class="form-item">
                <div class="form-item-label">{{ $t("新密码") }}</div>
                <div class="form-item-input area-code-input">
                    <el-input
                        v-model="formState.password"
                        class="input-border-style"
                        show-password
                        type="password"
                       :placeholder="$t('新密码')"
                        @blur="blurFun('password', formState.password)"
                    />
                    <div class="error-notice">
                        {{ formState.tigTips.password?.message }}
                    </div>
                </div>
            </div>
            <div class="form-item">
                <div class="form-item-label">{{ $t("确认新密码") }}</div>
                <div class="form-item-input area-code-input">
                    <el-input
                        v-model="formState.password2"
                        class="input-border-style"
                        show-password
                        type="password"
                        :placeholder="$t('确认新密码')"
                        @blur="blurFun('password2', formState.password2)"
                    />
                    <div class="error-notice">
                        {{ formState.tigTips.password2?.message }}
                    </div>
                </div>
            </div>
        </div>
        <div class="btn-box">
            <el-button :loading="loading" type="primary" class="submit-button" style="width: 100%" @click="onSubmit">{{ $t("确 认") }}</el-button>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * @description 找回密码第二步组件
 * - 设置新的登录密码
 * - 验证两次密码输入一致性
 * - 提交新密码完成重置
 */
import { ref } from "vue";
import { forgetPassword } from "@/api/user/login";
import { ElMessage } from "element-plus";

const step = defineModel("step");
const mobileKey = defineModel({ type: String });

const { t } = useI18n();
const validateFun = (prop: any, isValid: boolean, message: string) => {
    if (!isValid) {
        formState.value.tigTips[prop] = { type: "error", message: message };
    } else {
        formState.value.tigTips[prop] = { type: "valid", message: message };
    }
};
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

const validatePasswordAgain = (rule: any, value: any, callback: any) => {
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
    } else if (value != formState.value.password) {
        return callback(new Error(t("两次输入密码不一致")));
    } else {
        callback();
    }
};
const focusList = reactive<any>({
    password: t("6-20个大小写英文字母、符号或数字的组合"),
    password2: t("请输入和上次相同的密码")
});
const formRules = reactive<any>({
    password: [{ validator: validatePassword, trigger: "blur", required: true }],
    password2: [{ validator: validatePasswordAgain, trigger: "blur", required: true }]
});
const loading = ref<boolean>(false);
const formState: any = ref({
    tigTips: {},
    password: "",
    password2: ""
});
const router = useRouter();
const onSubmit = async () => {
    // 根据注册类型决定需要校验的字段
    const fieldsToValidate = ["password", "password2"];
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
        const result = await forgetPassword({
            mobileKey: mobileKey.value,
            password: formState.value.password //新密码
        });
        ElMessage.success(t("重置密码成功"));
        router.push("/member/login");
    } catch (error: any) {
        ElMessage.error(error.message);
        step.value = 1;
        mobileKey.value = "";
        formState.value.password = "";
        formState.value.password2 = "";
    } finally {
        loading.value = false;
    }
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
.btn-box {
    padding: 20px 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    row-gap: 15px;
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
:deep(.el-input__inner) {
    &::placeholder {
        color: #505259 !important;
    }
}
</style>
