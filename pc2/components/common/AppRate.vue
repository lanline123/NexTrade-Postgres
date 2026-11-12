<template>
    <div class="app-rate" :class="{ disabled: props.disabled }" :style="{ '--rate-size': iconSize, '--rate-color': props.textColor }">
        <template v-if="props.disabled">
            <span class="static-stars" :style="{ '--fill-width': `${fillPercent}%` }">
                <span class="base">★★★★★</span>
                <span class="fill">★★★★★</span>
            </span>
        </template>
        <template v-else>
            <button
                v-for="idx in props.max"
                :key="idx"
                type="button"
                class="star-btn"
                :class="{ active: idx <= displayActiveValue }"
                @click="setValue(idx)"
                @mouseenter="hoverValue = idx"
                @mouseleave="hoverValue = null"
            >
                ★
            </button>
        </template>

        <span v-if="props.showText" class="rate-text">{{ displayText }}</span>
    </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";

const props = withDefaults(
    defineProps<{
        modelValue?: number | string;
        disabled?: boolean;
        showText?: boolean;
        texts?: string[];
        textColor?: string;
        size?: number | string;
        max?: number;
    }>(),
    {
        modelValue: 0,
        disabled: false,
        showText: false,
        texts: () => [],
        textColor: "#ff9900",
        size: 14,
        max: 5
    }
);

const emit = defineEmits<{
    (e: "update:modelValue" | "change", value: number): void;
}>();

const hoverValue = ref<number | null>(null);

const numericValue = computed<number>(() => {
    const value = typeof props.modelValue === "string" ? parseFloat(props.modelValue) : props.modelValue;
    if (Number.isNaN(value)) return 0;
    return Math.max(0, Math.min(props.max, Number(value)));
});

const fillPercent = computed<number>(() => {
    if (props.max <= 0) return 0;
    return (numericValue.value / props.max) * 100;
});

const displayActiveValue = computed<number>(() => {
    return hoverValue.value ?? Math.round(numericValue.value);
});

const iconSize = computed<string>(() => {
    if (typeof props.size === "number") return `${props.size}px`;
    if (props.size === "large") return "18px";
    if (props.size === "small") return "12px";
    const parsed = Number(props.size);
    if (Number.isNaN(parsed)) return "14px";
    return `${parsed}px`;
});

const displayText = computed<string>(() => {
    const current = hoverValue.value ?? Math.round(numericValue.value);
    if (!props.texts.length) return "";
    if (current < 1 || current > props.texts.length) return "";
    return props.texts[current - 1] ?? "";
});

const setValue = (value: number): void => {
    if (props.disabled) return;
    emit("update:modelValue", value);
    emit("change", value);
};
</script>

<style scoped lang="less">
.app-rate {
    --rate-size: 14px;
    --rate-color: #ff9900;
    --rate-void-color: #f1f2f5;
    display: inline-flex;
    align-items: center;
    line-height: 1.1;
}

.static-stars {
    position: relative;
    display: inline-block;
    font-size: var(--rate-size);
    line-height: 1;

    .base {
        color: var(--rate-void-color);
    }

    .fill {
        position: absolute;
        top: 0;
        left: 0;
        color: var(--rate-color);
        width: var(--fill-width, 0%);
        overflow: hidden;
        white-space: nowrap;
    }
}

.star-btn {
    border: none;
    background: transparent;
    padding: 0;
    margin: 0 2px 0 0;
    color: var(--rate-void-color);
    font-size: var(--rate-size);
    line-height: 1;
    cursor: pointer;

    &.active {
        color: var(--rate-color);
    }
}

.disabled .star-btn {
    cursor: default;
}

.rate-text {
    margin-left: 10px;
    color: #666;
    font-size: 14px;
    line-height: 1.2;
}
</style>
