<template>
    <button
        class="tig-button"
        :class="{ theme: theme, loading: loading }"
        :disabled="disabled || loading"
        @click="handleClick"
        @mouseleave="handleMouseleave"
        @pointerdown="handlePointerDown"
    >
        <span class="button-content">
            <span v-if="loading" class="loading-spinner"></span>
            <slot></slot>
        </span>
        <span>
            <div v-if="showRipple" class="ripple" :style="{ ...rippleStyle, ...rippleBg }"></div>
        </span>
    </button>
</template>

<script setup lang="ts">
import { useThemeStore } from "~/store/theme";

const props = defineProps({
    disabled: {
        type: Boolean,
        default: false
    },
    loading: {
        type: Boolean,
        default: false
    },
    theme: {
        type: Boolean,
        default: false
    },
    plain: {
        type: Boolean,
        default: false
    },
    rippleBgColor: {
        type: String,
        default: "rgba(0, 0, 0, 0.2)"
    },
    animation: {
        type: Boolean,
        default: true
    }
});

const themeStore = useThemeStore();

const rippleBg = computed(() => {
    let bg = props.rippleBgColor;

    if (props.theme || props.plain) {
        if (props.theme && !props.plain) {
            bg = "rgba(255, 255, 255, 0.2)";
        } else if (props.plain) {
            bg = themeStore.hexToRgba(themeStore.themeColor["--general"], 0.1);
        }
    }
    return { background: bg };
});

let timeoutIds: Array<number | ReturnType<typeof setTimeout>> = [];

const showRipple = ref(false);
const rippleStyle = reactive({
    left: "0px",
    top: "0px",
    transform: "scale(0)",
    width: "0px",
    height: "0px",
    marginLeft: "0px",
    marginTop: "0px",
    opacity: "1"
});

let buttonRect: DOMRect;
const handleClick = (event: Event) => {};

const createRipple = (x: number, y: number) => {
    const maxRadius = Math.sqrt(Math.pow(Math.max(x, buttonRect.width - x), 2) + Math.pow(Math.max(y, buttonRect.height - y), 2));

    rippleStyle.left = `${x}px`;
    rippleStyle.top = `${y}px`;
    rippleStyle.width = `${maxRadius * 2}px`;
    rippleStyle.height = `${maxRadius * 2}px`;
    rippleStyle.marginLeft = `${-maxRadius}px`;
    rippleStyle.marginTop = `${-maxRadius}px`;
    rippleStyle.transform = "scale(0)";
    rippleStyle.opacity = "1";

    showRipple.value = true;

    const timeoutId = setTimeout(() => {
        rippleStyle.transform = "scale(1)";
    }, 16);
    timeoutIds.push(timeoutId);
};

const handlePointerDown = (event: PointerEvent) => {
    if (!props.animation) {
        return;
    }

    if (props.disabled || props.loading || showRipple.value) return;

    const { clientX = 0, clientY = 0 } = event as MouseEvent;
    const target = event.currentTarget as HTMLElement;
    buttonRect = target.getBoundingClientRect();

    const x = clientX - buttonRect.left;
    const y = clientY - buttonRect.top;

    createRipple(x, y);
};

const handleMouseleave = () => {
    if (!showRipple.value) {
        return;
    }

    if (!props.animation) {
        return;
    }

    rippleStyle.opacity = "0";

    const fadeTimeoutId = setTimeout(() => {
        showRipple.value = false;
    }, 200);

    timeoutIds.push(fadeTimeoutId);
};

const clearAllTimeouts = () => {
    timeoutIds.forEach((id) => clearTimeout(id));
    timeoutIds = [];
};

onUnmounted(() => {
    clearAllTimeouts();
});
</script>

<style lang="less" scoped>
.tig-button {
    box-sizing: border-box;
    border: 1px solid #c2c4cc;
    outline: 0px;
    margin: 0px;
    background: none;
    min-width: 64px;
    color: #1a1a1a;
    cursor: pointer;
    user-select: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    position: relative;
    padding: 5px 15px;
    font-size: 14px;
    height: 30px;
    border-radius: 4px;
    overflow: hidden;
    transition:
        color 0.3s ease,
        border-color 0.3s ease,
        background-color 0.3s ease;

    &:hover:not(:disabled):not(.theme) {
        color: var(--main-bg);
        border-color: var(--main-bg);
    }

    &:disabled {
        cursor: not-allowed;
        opacity: 0.5;
    }

    &.loading {
        cursor: not-allowed;
        pointer-events: none;
        opacity: 0.5;
    }

    &.theme {
        color: var(--main-text);
        border-color: var(--main-bg);
        background-color: var(--main-bg);
        border-width: 0;

        &:hover:not(:disabled) {
            color: var(--main-text);
            background-color: var(--primary-dark-2);
            border-color: var(--primary-dark-2);
        }

        &.loading {
            opacity: 0.5;
        }

        &:disabled {
            opacity: 0.5;
        }
    }

    .button-content {
        position: relative;
        z-index: 1;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 8px;
    }

    .loading-spinner {
        display: inline-block;
        width: 14px;
        height: 14px;
        border: 2px solid transparent;
        border-top-color: currentColor;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
        flex-shrink: 0;
        order: -1;
    }

    &.theme .loading-spinner {
        border-top-color: var(--main-text);
    }

    > span:not(.button-content) {
        overflow: hidden;
        pointer-events: none;
        position: absolute;
        z-index: 0;
        inset: 0px;
        border-radius: inherit;
    }

    .ripple {
        position: absolute;
        z-index: 0;
        border-radius: 50%;
        pointer-events: none;
        will-change: transform;
        transform: translateZ(0);
        transition:
            transform 0.5s cubic-bezier(0.4, 0, 0.2, 1),
            opacity 0.2s ease-out;
    }
}

@keyframes spin {
    0% {
        transform: rotate(0deg);
    }
    100% {
        transform: rotate(360deg);
    }
}
</style>
