<template>
    <div class="cc-collapse-item" :class="{ 'is-active': isActive, 'is-disabled': props.disabled }">
        <div class="cc-collapse-item__header" :class="{ 'is-active': isActive }" role="button" tabindex="0" @click="onToggle" @keydown.enter="onToggle">
            <div class="cc-collapse-item__title">
                <slot name="title">
                    {{ props.title }}
                </slot>
            </div>
            <slot name="icon" :is-active="isActive" />
        </div>
        <div class="cc-collapse-item__wrap" :class="{ 'is-active': isActive }">
            <div class="cc-collapse-item__content-wrap">
                <div class="cc-collapse-item__content">
                    <slot />
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, getCurrentInstance, inject } from "vue";

type CollapseName = string | number;

interface CollapseContext {
    isItemActive: (name: CollapseName) => boolean;
    toggleItem: (name: CollapseName) => void;
}

const props = defineProps<{
    name?: CollapseName;
    title?: string;
    disabled?: boolean;
}>();

const collapseContext = inject<CollapseContext>("custom-collapse-context");
const instance = getCurrentInstance();

const itemName = computed<CollapseName>(() => {
    if (props.name !== undefined) return props.name;
    return instance?.uid ?? 0;
});

const isActive = computed<boolean>(() => {
    if (!collapseContext) return false;
    return collapseContext.isItemActive(itemName.value);
});

const onToggle = (): void => {
    if (props.disabled || !collapseContext) return;
    collapseContext.toggleItem(itemName.value);
};
</script>

<style scoped lang="less">
.cc-collapse-item {
    border-bottom: 1px solid #e5e5e5;
}

.cc-collapse-item__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 16px 0;
    cursor: pointer;
}

.cc-collapse-item__title {
    flex: 1;
    min-width: 0;
    color: #333;
}

.cc-collapse-item__wrap {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows 0.3s ease-in-out;
    background-color: transparent;
    will-change: grid-template-rows;

    &.is-active {
        grid-template-rows: 1fr;
    }
}

.cc-collapse-item__content-wrap {
    min-height: 0;
    overflow: hidden;
}

.cc-collapse-item__content {
    padding-bottom: 16px;
}
</style>
