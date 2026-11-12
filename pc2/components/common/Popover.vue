<template>
    <el-popover
        :width="width"
        popper-style="box-shadow: rgb(14 18 22 / 35%) 0px 10px 38px -10px, rgb(14 18 22 / 20%) 0px 10px 20px -15px; padding: 20px;"
        :trigger="trigger"
        @before-enter="$emit('update:modelValue', true)"
        @before-leave="close"
        :teleported="false"
        :showAfter="showAfter"
        :hideAfter="hideAfter"
    >
        <template #reference>
            <slot name="reference"></slot>
        </template>
        <template #default>
            <slot></slot>
        </template>
    </el-popover>
</template>

<script setup lang="ts">
import type { PropType } from "vue";
type Trigger = "click" | "hover" | "focus" | "contextmenu";

const props = defineProps({
    modelValue: {
        type: Boolean,
        default: false
    },
    trigger: {
        type: String as PropType<Trigger>,
        default: "click"
    },
    width: {
        type: [Number, String],
        default: 300
    },
    showAfter: {
        type: Number,
        default: 0
    },
    hideAfter:{
        type: Number,
        default: 200
    }
});

const emit = defineEmits(["update:modelValue", "close"]);
const close = () => {
    emit("close");
    emit("update:modelValue", false);
};
</script>

<style lang="less" scoped></style>
