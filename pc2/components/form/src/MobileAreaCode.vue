<template>
    <el-select
        class="mobile-area-code"
        append-to=".app"
        :teleported="true"
        v-if="list.length > 0"
        :style="customStyle"
        v-model="mobileAreaCode"
        :placeholder="$t(placeholder)"
    >
        <template v-for="item in list" :key="item">
            <el-option :label="item.label" :value="item.code">
                <span style="font-size: 14px">{{ item.label }} ({{ $t(item.name as string) }})</span>
            </el-option>
        </template>
    </el-select>
</template>

<script setup lang="ts">
import { getmobileAreaCode } from "~/api/common";
import type { MobileAreaCodeList } from "@/types/common/common.d.ts";
import { ref, watch } from "vue";
import { computed } from "@vue/reactivity";

const props = defineProps({
    customStyle: {
        type: Object,
        default: () => ({
            "min-width": "75px"
        })
    },
    placeholder: {
        type: String,
        default: "请选择区号"
    },
    modelValue: {
        type: String,
        default: ""
    }
});
const emit = defineEmits(["update:modelValue"]);
const DEFAULT_MOBILE_AREA_CODE = "86";
const mobileAreaCode = computed({
    get() {
        return props.modelValue;
    },
    set(val) {
        emit("update:modelValue", val);
    }
});
const list = ref<MobileAreaCodeList[]>([]);
const loading = ref<boolean>(true);
const getmobileAreaCodeList = async () => {
    loading.value = true;
    list.value = [];
    try {
        const result = await getmobileAreaCode();
        list.value = result;
        const defaultItem = result.find((item) => item.code === DEFAULT_MOBILE_AREA_CODE) || result.find((item) => item.isDefault === 1);
        if ((!props.modelValue || !result.some((item) => item.code === props.modelValue)) && defaultItem) {
            emit("update:modelValue", defaultItem.code);
        }
    } catch (error) {
        console.log(error);
    } finally {
        loading.value = false;
    }
};
getmobileAreaCodeList();
watch(
    () => props.modelValue,
    (value) => {
        if (list.value.length && value && !list.value.some((item) => item.code === value)) {
            const defaultItem = list.value.find((item) => item.code === DEFAULT_MOBILE_AREA_CODE) || list.value.find((item) => item.isDefault === 1);
            if (defaultItem) emit("update:modelValue", defaultItem.code);
        }
    }
);
</script>

<style lang="less" scoped>
:deep(.el-select__wrapper) {
    box-shadow: none;
    font-size: 14px !important;
}

:deep(.el-select__input-wrapper, .el-select__placeholder) {
    font-size: 14px !important;
}

.el-select-dropdown__wrap {
    .el-select-dropdown__item {
        &:deep(.is-selected) {
            color: var(--general);
        }
    }
}
</style>
