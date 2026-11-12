<template>
    <div class="info-card">
        <span class="cu" @click="show">
            <slot></slot>
        </span>
        <el-dialog v-model="dialogTableVisible" :title="act == 'add' ? $t('新增卡片') : $t('编辑卡片')" width="600">
            <el-form v-if="!loading" ref="formRef" :model="formState" class="form-body" label-suffix="：" label-width="auto">
                <el-form-item v-show="page" :rules="[{ required: true, message: $t('请选择卡类型') }]" :label="$t('卡类型')">
                    <el-radio-group v-model="formState.accountType" @change="onChange">
                        <el-radio-button :value="1">{{ $t("银行卡") }}</el-radio-button>
                        <el-radio-button :value="2">{{ $t("支付宝") }}</el-radio-button>
                        <el-radio-button :value="3">{{ $t("微信") }}</el-radio-button>
                    </el-radio-group>
                </el-form-item>
                <el-form-item :rules="[{ required: true, message: $t('请输入真实姓名') }]" :label="$t('真实姓名')" prop="accountName">
                    <el-input v-model="formState.accountName" :placeholder="$t('请输入真实姓名')" />
                </el-form-item>
                <el-form-item :rules="identityRules" :label="$t('身份证号')" prop="identity">
                    <el-input v-model="formState.identity" maxlength="18" :placeholder="$t('请输入身份证号')" />
                </el-form-item>
                <el-form-item :label="accountPlaceholder + $t('号')" :rules="accountNoRules" prop="accountNo">
                    <el-input v-model="formState.accountNo" :placeholder="$t('请输入') + accountPlaceholder" />
                </el-form-item>
                <el-form-item v-if="formState.accountType === 1" :label="$t('银行卡详情')" prop="bankName">
                    <el-input v-model="formState.bankName" :rows="4" :placeholder="$t('请输入银行卡详情')" type="textarea" />
                </el-form-item>
            </el-form>
            <template #footer>
                <div class="dialog-footer">
                    <el-button type="primary" @click="onSubmit">{{ $t("保存") }}</el-button>
                </div>
            </template>
        </el-dialog>
    </div>
</template>
<script lang="ts" setup>
/**
 * @description 银行卡/账号信息编辑弹窗组件
 * - 支持新增和编辑银行卡或提现账号信息
 * - 根据账号类型显示不同的表单字段
 * - 包含表单验证，确保必填项已填写
 */
import { ref } from "vue";
import type { AccountData } from "~/types/user/account";
import { getAccount, updateAccount } from "~/api/user/account";
const { t } = useI18n();
const emit = defineEmits(["loadFilter"]);
const props = defineProps({
    act: {
        type: String,
        default: ""
    },
    id: {
        type: Number,
        default: 0
    },
    type: {
        type: Number,
        default: 1
    },
    page: {
        type: Number,
        default: 1
    }
});
const formState = ref<AccountData>({
    accountType: 1
} as AccountData);
const loading = ref<boolean>(true);
const accountPlaceholder = computed(() => {
    switch (formState.value.accountType) {
        case 1:
            return t("银行卡");
        case 2:
            return t("支付宝");
        case 3:
            return t("微信");
        default:
            return "";
    }
});
const onChange = async () => {
    await formRef.value.resetFields();
};
const identityRules = computed(() => {
    return [
        { required: true, message: t("请输入身份证号"), trigger: "blur" }, // 必填验证
        {
            validator: (rule: any, value: any, callback: any) => {
                const regExp = /^[1-9]\d{5}[1-9]\d{3}((0\d)|(1[0-2]))(([0|1|2]\d)|3[0-1])\d{3}([0-9]|X)$/;
                if (!regExp.test(value)) {
                    callback(new Error(t("请输入正确的身份证号码")));
                } else {
                    callback();
                }
            },
            trigger: "blur" // 触发方式，可以是 blur、change等
        }
    ];
});
const accountNoRules = computed(() => {
    return [
        { required: true, message: t("请输入") + accountPlaceholder.value + t("号"), trigger: "blur" }, // 必填验证
        {
            validator: (rule: any, value: any, callback: any) => {
                const regExp = /^\d{6,}$/;
                if (!regExp.test(value) && formState.value.accountType === 1) {
                    callback(new Error(t("请输入正确的银行卡号")));
                } else {
                    callback();
                }
            },
            trigger: "blur" // 触发方式，可以是 blur、change等
        }
    ];
});

const formRef = shallowRef();
const onSubmit = async () => {
    await formRef.value.validate();
    try {
        const result = await updateAccount({ ...formState.value }, props.act == "add" ? "createAccount" : "updateAccount");
        await formRef.value.resetFields();
        message.success(t("操作成功"));
        dialogTableVisible.value = false;
        formState.value.accountType = 1;
        emit("loadFilter");
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
const fetchInfo = async () => {
    try {
        loading.value = true;
        const result = await getAccount({ accountId: props.id });
        Object.assign(formState.value, result);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

const dialogTableVisible = ref(false);

const show = async () => {
    dialogTableVisible.value = true;
    if (formRef.value) await formRef.value.resetFields();
    formState.value.accountType = props.type;
    if (props.act == "edit") {
        await fetchInfo();
    } else {
        loading.value = false;
    }
};
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";
.cu {
    cursor: pointer;
}
.info-card {
    :deep(.el-overlay) {
        z-index: 500 !important;
    }
}
</style>
