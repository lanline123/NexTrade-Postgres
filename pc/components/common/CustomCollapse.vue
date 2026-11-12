<template>
    <div class="cc-collapse">
        <slot />
    </div>
</template>

<script setup lang="ts">
import { provide } from "vue";

type CollapseName = string | number;
type CollapseModelValue = CollapseName | CollapseName[];

interface CollapseContext {
    isItemActive: (name: CollapseName) => boolean;
    toggleItem: (name: CollapseName) => void;
}

const props = withDefaults(
    defineProps<{
        modelValue?: CollapseModelValue;
        accordion?: boolean;
    }>(),
    {
        modelValue: () => [],
        accordion: false
    }
);

const emit = defineEmits<{
    (e: "update:modelValue" | "change", value: CollapseModelValue): void;
}>();

const normalizeToArray = (value: CollapseModelValue | undefined): CollapseName[] => {
    if (value === undefined || value === null || value === "") return [];
    return Array.isArray(value) ? value : [value];
};

const emitValue = (value: CollapseModelValue) => {
    emit("update:modelValue", value);
    emit("change", value);
};

const isItemActive = (name: CollapseName): boolean => {
    const current = normalizeToArray(props.modelValue);
    return current.includes(name);
};

const toggleItem = (name: CollapseName): void => {
    const current = normalizeToArray(props.modelValue);

    if (props.accordion) {
        const nextValue: CollapseModelValue = current[0] === name ? "" : name;
        emitValue(nextValue);
        return;
    }

    const exists = current.includes(name);
    const nextValue = exists ? current.filter((item) => item !== name) : [...current, name];
    emitValue(nextValue);
};

provide<CollapseContext>("custom-collapse-context", {
    isItemActive,
    toggleItem
});
</script>
<style scoped lang="less">
.cc-collapse {
    border-top: 1px solid #e5e5e5;
}
</style>
