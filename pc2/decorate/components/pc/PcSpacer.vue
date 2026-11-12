<template>
    <div class="pc-spacer module-ad-con module-white_blank" :style="outerStyle">
        <div class="pc-spacer__content" :style="contentStyle"></div>
    </div>
</template>

<script lang="ts" setup>
import { computed } from "vue";

interface SpacerFrame {
    boxPadding?: number;
    boxPaddingTop?: number;
    boxPaddingBottom?: number;
    boxRadius?: number;
}

interface SpacerModule {
    frame?: SpacerFrame;
    backgroundColor?: string;
    blankHeight?: number;
}

const props = defineProps<{
    module?: SpacerModule;
}>();

const outerStyle = computed(() => {
    const frame = props.module?.frame ?? {};
    return {
        paddingLeft: `${frame.boxPadding ?? 0}px`,
        paddingRight: `${frame.boxPadding ?? 0}px`,
        paddingTop: `${frame.boxPaddingTop ?? 0}px`,
        paddingBottom: `${frame.boxPaddingBottom ?? 0}px`,
        boxSizing: "border-box" as const,
        width: "100%"
    };
});

const contentStyle = computed(() => {
    const frame = props.module?.frame ?? {};
    const height = props.module?.blankHeight ?? 30;
    const bg = props.module?.backgroundColor;
    return {
        width: "100%",
        height: `${height}px`,
        minHeight: "10px",
        backgroundColor: bg || "transparent",
        borderRadius: `${frame.boxRadius ?? 0}px`,
        boxSizing: "border-box" as const
    };
});
</script>

<style lang="less" scoped>
.pc-spacer {
    width: 100%;
}
</style>
