<template>
    <div v-if="!ready" class="pc-header-runtime-placeholder" aria-hidden="true"></div>
    <PcHeaderOverseasRuntime
        v-else-if="styleType === 2"
        :is-white="isWhite"
        :page="page"
        :shop-search="shopSearch"
        :config="resolvedConfig"
    />
    <PcHeaderSimpleRuntime v-else :is-white="isWhite" :page="page" :shop-search="shopSearch" :config="resolvedConfig" />
</template>

<script lang="ts" setup>
/**
 * PC 头部装修运行时入口
 * 等待配置就绪后，按 styleType 渲染对应的头部。
 * 上层（PageHeader）保证有 config 才渲染本组件；“无 config 回退旧头部”由上层处理。
 */
import { computed } from "vue";
import PcHeaderOverseasRuntime from "./components/PcHeaderOverseasRuntime.vue";
import PcHeaderSimpleRuntime from "./components/PcHeaderSimpleRuntime.vue";
import type { PcHeaderConfig } from "~/types/decorate/pcHeader";
import { isOverseas } from "@/utils/util";

defineOptions({ name: "CommonPcHeaderRuntime" });

const props = defineProps<{
    isWhite: boolean;
    page: string;
    shopSearch: boolean;
    ready: boolean;
    config: PcHeaderConfig | null;
}>();

const styleType = computed<1 | 2>(() => {
    const raw = props.config?.styleType;
    if (raw === 1 || raw === 2) return raw;
    // 跨境版默认跨境风，标准版默认简约风（与 Admin 默认口径一致）
    return isOverseas() ? 2 : 1;
});

const resolvedConfig = computed<PcHeaderConfig>(() => props.config ?? ({} as PcHeaderConfig));
</script>

<style scoped lang="less">
.pc-header-runtime-placeholder {
    min-height: 168px;
}
</style>
