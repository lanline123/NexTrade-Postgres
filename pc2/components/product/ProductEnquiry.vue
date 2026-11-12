<template>
    <el-dialog v-model="show" width="500" append-to-body :z-index="2001" :style="themeStore.themeColor">
        <div class="enquiry-content">
            <div class="title">{{ $t("立即询价") }}</div>
            <div class="desc">{{ $t("你提交后，专属业务员将第一时间为你服务") }}</div>
            <el-form ref="formRef" :model="form" label-width="auto" :rules="rules">
                <el-form-item :label="$t('联系电话')" class="mobile" prop="mobile">
                    <el-input v-model="form.mobile" />
                    <!-- <div class="tip"><span class="iconfont-pc icon-jiesuo"></span> {{ $t("隐私加密保护中，对商家隐藏真实号码") }}</div> -->
                </el-form-item>
                <el-form-item :label="$t('邮箱地址')" prop="email">
                    <el-input v-model="form.email" :placeholder="$t('请输入邮箱地址')" />
                </el-form-item>
                <el-form-item :label="$t('需求信息')" prop="content">
                    <el-input class="content" v-model="form.content" type="textarea" :autosize="{ minRows: 4 }" :placeholder="$t('请简单描述你的需求')" />
                </el-form-item>
            </el-form>
            <div class="btn-box">
                <el-button class="btn" type="primary" @click="submitForm" :loading="loading">{{ $t("立即发送") }}</el-button>
            </div>
        </div>
    </el-dialog>
</template>

<script setup lang="ts">
import { enquiry } from "@/api/product/product";
import type { FormInstance } from "element-plus";
import { useI18n } from "vue-i18n";
import { useThemeStore } from "~/store/theme";
const { t } = useI18n();
const themeStore = useThemeStore();

const props = defineProps({
    modelValue: {
        type: Boolean,
        default: false
    },
    productId: {
        type: Number,
        required: true
    }
});
const emit = defineEmits(["update:modelValue"]);
const show = computed({
    get: () => props.modelValue,
    set: (value) => {
        emit("update:modelValue", value);
    }
});

const formRef = ref<FormInstance>();
const rules = {
    mobile: [
        { required: true, message: t("请输入联系电话"), trigger: "blur" },
        { pattern: /^1[3-9]\d{9}$/, message: t("请输入正确的手机号码"), trigger: "blur" }
    ],
    email: [
        {
            validator: (_rule: unknown, value: string, callback: (error?: Error) => void) => {
                if (!value || /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) {
                    callback();
                } else {
                    callback(new Error(t("请输入正确的邮箱格式")));
                }
            },
            trigger: "blur"
        }
    ]
};
const form = reactive({
    mobile: "",
    email: "",
    content: ""
});
const loading = ref(false);

const submitForm = async () => {
    if (!formRef.value) return;
    await formRef.value.validate((valid, fields) => {
        if (valid) {
            sendEnquiry();
        } else {
            console.error("error submit!", fields);
        }
    });
};

const sendEnquiry = async () => {
    loading.value = true;
    try {
        await enquiry({ ...form, productId: props.productId });
        emit("update:modelValue", false);
        formRef.value!.resetFields();
        ElMessage.success(t("商家已收到您的联系方式，会尽快联系您!"));
    } catch (error: any) {
        console.error(error);
        ElMessage.error(error.message);
    } finally {
        loading.value = false;
    }
};
</script>

<style lang="less" scoped>
.enquiry-content {
    padding: 0 30px;

    .title {
        font-weight: 500;
        font-size: 18px;
        color: #333;
        margin-bottom: 15px;
    }
    .desc {
        font-weight: 400;
        font-size: 12px;
        color: #666;
        margin-bottom: 18px;
    }

    .tip {
        color: var(--general);
        font-size: 12px;
        .icon-jiesuo {
            position: relative;
            top: 2px;
        }
    }
    .btn-box {
        padding-top: 10px;
        width: 100%;
        display: flex;
        justify-content: center;
        margin-bottom: 15px;

        .btn {
            height: 40px;
            min-width: 110px;
        }
    }

    .content {
        :deep(.el-textarea__inner::placeholder) {
            font-size: 12px;
        }
    }
}
</style>
