<template>
    <PcGrouponLegacy :module="module" />
</template>

<script lang="ts" setup>
// pc 端装修拼团组件（过渡层）
// 说明：wrapper 只做委托，真正的渲染/取数/倒计时都在 PcGrouponLegacy（对齐 vortmall 金标准结构）。
import PcGrouponLegacy from "~/decorate/components/pc/PcGrouponLegacy.vue";
import type { DecorateModulePayload } from "~/types/decoration/decorateModule";

defineProps<{
    module: DecorateModulePayload;
}>();
</script>
