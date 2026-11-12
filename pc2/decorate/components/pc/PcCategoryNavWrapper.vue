<template>
    <div class="dec-category-nav-wrapper" :style="containerStyle">
        <div class="dec-category-nav-wrapper__inner">
            <CategoryNew :model-value="module" />
        </div>
    </div>
</template>

<script lang="ts" setup>
import { computed } from "vue";
import CategoryNew from "~/components/home/CategoryNew.vue";
import type { DecorateModulePayload } from "~/types/decoration/decorateModule";
import type { PcSeckillFrameDecor } from "~/types/decoration/pcSeckillModule";

const props = defineProps<{
    module: DecorateModulePayload;
}>();

const frame = computed((): PcSeckillFrameDecor => (props.module?.frame as PcSeckillFrameDecor | undefined) ?? {});

// 容器层（扩展设置「背景颜色」）：背景色 + 圆角；上/下/左右边距作为留白，露出容器底色，
// 与内容层（CategoryNew 内 .mod_seckill 的「内容设置背景颜色」）形成两层独立显示
const containerStyle = computed(() => {
    const boxRadius = frame.value.boxRadius ?? 0;
    return {
        backgroundColor: frame.value.backgroundColor || "transparent",
        borderRadius: `${boxRadius}px`,
        overflow: boxRadius ? "hidden" : undefined,
        paddingLeft: `${frame.value.boxPadding ?? 0}px`,
        paddingRight: `${frame.value.boxPadding ?? 0}px`,
        paddingTop: `${frame.value.boxPaddingTop ?? 0}px`,
        paddingBottom: `${frame.value.boxPaddingBottom ?? 0}px`
    };
});
</script>

<style lang="less" scoped>
.dec-category-nav-wrapper {
    box-sizing: border-box;
    width: 100%;
}
.dec-category-nav-wrapper__inner {
    box-sizing: border-box;
    width: 100%;
}
</style>
