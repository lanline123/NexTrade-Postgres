<template>
    <div class="decorate-renderer">
        <template v-for="(entry, index) in resolvedModules" :key="entry.key">
            <div class="decorate-renderer__module" :class="{ 'is-last': index === resolvedModules.length - 1 }">
                <component :is="entry.component" :module="entry.item.module" />
            </div>
        </template>
    </div>
</template>

<script lang="ts" setup>
import { computed, defineAsyncComponent, type Component } from "vue";
import { normalizeModuleType, type DecorateModuleLike } from "../normalize";
import { getPcCriticalComponent, getPcModuleDefinition } from "../registry/pc";

const props = defineProps<{
    modules: DecorateModuleLike[];
    criticalModuleCount?: number;
}>();

// 缓存异步组件，避免计算属性重复执行时反复创建实例
const asyncComponentCache = new Map<string, ReturnType<typeof defineAsyncComponent>>();

const safeModules = computed(() => (Array.isArray(props.modules) ? props.modules : []));
const safeCriticalModuleCount = computed(() => Math.max(0, props.criticalModuleCount ?? 0));

const renderableModules = computed(() => {
    return safeModules.value.filter((item: DecorateModuleLike) => {
        if (!getIsShow(item)) return false;
        const type = normalizeModuleType(item);
        const def = getPcModuleDefinition(type);
        if (!def) {
            warnUnregisteredType(type);
            return false;
        }
        return true;
    });
});

// 未注册类型会被静默丢弃（页面只是少一块），开发态提示一次，便于定位注册表缺件
const warnedTypes = new Set<string>();
function warnUnregisteredType(type: string): void {
    if (!import.meta.dev || !type || warnedTypes.has(type)) return;
    warnedTypes.add(type);
    console.warn(`[DecorateRenderer] 装修模块类型 "${type}" 未在 decorate/registry/pc.ts 注册，已跳过渲染。`);
}

const resolvedModules = computed(() =>
    renderableModules.value.map((item, index) => ({
        item,
        key: getKey(item, index),
        component: resolveComponent(item, index)
    }))
);

function getIsShow(item: DecorateModuleLike): boolean {
    return item?.isShow !== false;
}

function getKey(item: DecorateModuleLike, index: number): string | number {
    return item?.moduleIndex ?? `${normalizeModuleType(item)}-${index}`;
}

function resolveComponent(item: DecorateModuleLike, index: number): Component | null {
    const type = normalizeModuleType(item);
    const def = getPcModuleDefinition(type);
    if (!def) return null;

    if (index < safeCriticalModuleCount.value) {
        const criticalComponent = getPcCriticalComponent(type);
        if (criticalComponent) {
            return criticalComponent;
        }
    }

    const cached = asyncComponentCache.get(type);
    if (cached) return cached;
    const asyncComp = defineAsyncComponent(def.getComponent);
    asyncComponentCache.set(type, asyncComp);
    return asyncComp;
}
</script>

<style lang="less" scoped>
.decorate-renderer__module {
    margin-bottom: var(--decorate-module-gap, 0px);
}
.decorate-renderer__module.is-last {
    margin-bottom: 0;
}
</style>
