<template>
    <div class="input-box" :class="{ rounded: rounded }">
        <el-input-number v-model="number" :step="step" :max="max" :min="min" @change="onChange" />
    </div>
</template>
<script lang="ts" setup>
const props = defineProps({
    modelValue: { type: Number, default: 1 },
    max: Number,
    min: { type: Number, default: 1 },
    step: { type: Number, default: 1 },
    // 新版 UI 圆角步进器样式
    rounded: { type: Boolean, default: false }
});
const number = defineModel<number>("modelValue");
const emit = defineEmits(["change"]);
const onChange = (value: any) => {
    emit("change");
    if (!Number.isInteger(value) || value < 1) {
        number.value = props.min;
    }
};
</script>
<style lang="less" scoped>
.input-box {
    :deep(.el-input-number__decrease) {
        border-radius: 0;
    }

    :deep(.el-input) {
        border-radius: 0;
    }

    :deep(.el-input-number .el-input__wrapper) {
        border-radius: 0;
    }

    :deep(.el-input-number .el-input__wrapper) {
        box-shadow: 0 0 0 1px #e7e7e7 inset !important;
    }

    :deep(.el-input-number__decrease:hover),
    :deep(.el-input-number__increase:hover) {
        color: #333;
    }
}

// 新版 UI：圆角步进器（− 数字 +），与商品详情新版样式保持一致
.input-box.rounded {
    :deep(.el-input-number) {
        width: 120px;
        height: 40px;
        line-height: 40px;
    }

    :deep(.el-input-number .el-input__wrapper) {
        border-radius: 10px !important;
        // box-shadow: 0 0 0 1px #e7e7e7 inset !important;
        box-shadow: none !important;
        border: 1px solid rgba(31, 31, 51, .1) !important;
    }

    :deep(.el-input__inner) {
        color: rgba(9, 9, 10, 0.92);
        font-size: 14px;
        font-weight: 600;
    }

    :deep(.el-input-number__decrease),
    :deep(.el-input-number__increase) {
        width: 38px;
        background: transparent !important;
        color: rgba(9, 9, 10, 0.92);
        border: none;
    }

    :deep(.el-input-number__decrease) {
        border-right: 1px solid rgba(31, 31, 51, .1) !important;
        border-radius: 10px 0 0 10px !important;
    }

    :deep(.el-input-number__increase) {
        border-left: 1px solid rgba(31, 31, 51, .1) !important;
        border-radius: 0 10px 10px 0 !important;
    }
}
</style>
