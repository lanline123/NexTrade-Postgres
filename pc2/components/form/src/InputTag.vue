<template>
    <el-select
        class="input-tag-warp"
        style="width: 100%"
        :teleported="false"
        v-model="tagValue"
        multiple
        filterable
        allow-create
        default-first-option
        :reserve-keyword="false"
        placeholder=""
        popper-class="input-tag-popper-warp"
        @change="onChange"
        :filter-method="onfilter"
        @keyup.enter="onEnter"
    >
    </el-select>
</template>
<script setup lang="ts">
import { ref, toRefs, onUnmounted, computed } from "vue";
const props = defineProps({
    modelValue: { type: [String, Array], required: true },
    splitString: {
        type: String,
        value: ","
    }
});
const emit = defineEmits(["update:modelValue"]);
const tagValue = ref();
const modelValue = ref();
const splitString = ref();
modelValue.value = props.modelValue;
splitString.value = props.splitString || ",";
if (typeof modelValue.value === "string") {
    tagValue.value = modelValue.value ? modelValue.value.split(splitString.value) : [];
} else {
    tagValue.value = modelValue.value;
}
const onChange = (value) => {
    if (typeof modelValue.value === "string") {
        emit("update:modelValue", tagValue.value.join(splitString.value));
    } else {
        emit("update:modelValue", tagValue.value);
    }
};
const filterInput = ref();
const onfilter = (e) => {
    if (e) filterInput.value = e;
};
const onEnter = () => {
    // alert(filterInput.value)
    if (tagValue.value.indexOf(filterInput.value) === -1) {
        if (filterInput.value) {
            tagValue.value.push(filterInput.value);
            filterInput.value = "";
        }
    }
};
</script>
<style lang="less" scoped>
.input-tag-warp :deep(.el-tag) {
    border: none !important;
}

.input-tag-warp :deep(.input-tag-popper-warp) {
    display: none;
}

.input-tag-warp :deep(.el-input__suffix) {
    display: none;
}
</style>
