<script setup lang="ts">
/**
 * TigIcon - 项目业务图标组件
 *
 * 用于渲染 assets/icons/ 目录下的 SVG 图标
 * 支持子目录结构，如 im/up.svg -> name="im/up"
 *
 * @example
 * <tig-icon name="cart" />
 * <tig-icon name="search" :size="24" color="#ff0000" />
 */
import { computed, inject, type Component } from "vue";

export interface TigIconProps {
    /** 图标名称，对应 assets/icons/ 下的路径，如 'cart'、'im/up' */
    name: string;
    /** 图标大小，默认 16px */
    size?: number | string;
    /** 图标颜色，默认 currentColor（继承字体颜色） */
    color?: string;
}

const props = withDefaults(defineProps<TigIconProps>(), {
    size: 16,
    color: "currentColor"
});

// 从 provide 注入图标集
const icons = inject<Record<string, { default: Component }>>("tig-icons", {});

// 计算当前图标组件
const iconComponent = computed(() => {
    // 构建匹配的 key
    // Nuxt 中 import.meta.glob("~/assets/icons/**/*.svg") 返回的 key 格式可能是：
    // - /assets/icons/xxx.svg
    // - ./assets/icons/xxx.svg
    // - ~/assets/icons/xxx.svg
    const possibleKeys = [
        `/assets/icons/${props.name}.svg`,
        `./assets/icons/${props.name}.svg`,
        `~/assets/icons/${props.name}.svg`,
        `/app/assets/icons/${props.name}.svg`,
        `../assets/icons/${props.name}.svg`
    ];

    for (const key of possibleKeys) {
        if (icons[key]?.default) {
            return icons[key].default;
        }
    }

    // 尝试遍历所有 key 找匹配的
    for (const [key, value] of Object.entries(icons)) {
        if (key.endsWith(`/${props.name}.svg`) || key.endsWith(`\\${props.name}.svg`)) {
            return value.default;
        }
    }

    return null;
});

// 计算样式
const iconStyle = computed(() => ({
    width: typeof props.size === "number" ? `${props.size}px` : props.size,
    height: typeof props.size === "number" ? `${props.size}px` : props.size,
    color: props.color
}));

// 开发环境下，图标不存在时输出警告
if (import.meta.env.DEV && !iconComponent.value) {
    console.warn(`[TigIcon] Icon "${props.name}" not found in assets/icons/`);
}
</script>

<template>
    <component :is="iconComponent" v-if="iconComponent" class="tig-icon" :style="iconStyle" aria-hidden="true" />
    <span v-else class="tig-icon tig-icon--empty" :style="iconStyle" />
</template>

<style scoped>
.tig-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    vertical-align: middle;
    flex-shrink: 0;
    color: currentColor;
}

.tig-icon--empty {
    background-color: currentColor;
    opacity: 0.1;
    border-radius: 2px;
}

/* 确保 SVG 继承尺寸 */
.tig-icon :deep(svg) {
    width: 100%;
    height: 100%;
}
</style>
