<template>
    <PcSeckillLegacy :module="module" />
</template>

<script lang="ts" setup>
// pc 端装修秒杀组件（过渡层）
// 说明：wrapper 只做委托，真正的渲染/取数/倒计时都在 PcSeckillLegacy（对齐 vortmall 金标准结构）。
import PcSeckillLegacy from "~/decorate/components/pc/PcSeckillLegacy.vue";
import type { PcSeckillModuleDecor } from "~/types/decoration/pcSeckillModule";

defineProps<{
    module: PcSeckillModuleDecor;
}>();
</script>
