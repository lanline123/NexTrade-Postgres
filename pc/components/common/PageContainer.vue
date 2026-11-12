<script setup lang="ts">
/**
 * 全站统一响应式内容容器（对齐首页 1264–1600 流式）。
 * 底层复用全局 .container + --pc-home-* 变量，与存量 <div class="container"> 同一套宽度，可无缝共存、渐进迁移。
 * variant 仅就近覆盖 --pc-home-* 变量，不产生第二套宽度体系。
 */
interface Props {
    // normal=跟随全局(1264–1600 流式,默认) | wide=显式封顶 1600 | narrow=阅读/表单窄栏 | full=通栏
    variant?: "normal" | "wide" | "narrow" | "full";
    // 横向 flex 布局（如会员中心：侧栏 + 内容）
    flex?: boolean;
    tag?: string;
}
withDefaults(defineProps<Props>(), { variant: "normal", flex: false, tag: "div" });
</script>

<template>
    <component :is="tag" class="container" :class="[variant !== 'normal' ? `container--${variant}` : '', { flex }]">
        <slot />
    </component>
</template>

<style scoped lang="less">
.container--wide {
    --pc-home-max-width: 1600px;
    --pc-home-min-width: 1264px;
    --pc-home-padding-x: 38px;
}
.container--narrow {
    --pc-home-max-width: 1200px;
    --pc-home-min-width: auto;
}
.container--full {
    --pc-home-max-width: none;
    --pc-home-min-width: auto;
    --pc-home-padding-x: 0;
}
</style>
