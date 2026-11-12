<template>
    <div :style="{ padding: itemPadding + 'px' }">
        <TotalContainer :module="frameModule" :class="['module-ad-con', 'module-pc-coupon']">
            <!-- 标题/描述：左右排列（标题在左、描述在右），与后台预览一致 -->
            <div v-if="showTitle" class="dec-pc-coupon__header">
                <div class="dec-pc-coupon__header-title" :style="{ color: titleColor }">{{ title }}</div>
                <div v-if="desc" class="dec-pc-coupon__header-desc">{{ desc }}</div>
            </div>
            <Coupon :max-count="4"></Coupon>
        </TotalContainer>
    </div>
</template>

<script lang="ts" setup>
// pc-home-coupon：外层容器消费 frame（边距/背景/圆角）+ 可选标题，内部委托 home/Coupon.vue 取数渲染领券中心。
import { computed, type PropType } from "vue";

// 路径映射：vortmall 的 ~/components/shop/components 与 ~/pages/activity/components 在 NexTrade 下统一为 ~/components/shop/src。
import TotalContainer from "~/decorate/components/shop/TotalContainer.vue";
import Coupon from "~/components/home/Coupon.vue";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    },
    shopId: {
        type: [String, Number] as PropType<string | number>,
        default: -1
    }
});

const frameModule = computed(() => {
    const frame = props.module ?? {};
    return {
        boxPaddingTop: frame.boxPaddingTop ?? 0,
        boxPaddingBottom: frame.boxPaddingBottom ?? 0,
        boxPadding: frame.boxPadding ?? 0,
        backgroundColor: frame.backgroundColor ?? "transparent",
        // 圆角作用在最外层容器（base-class）；默认 8
        boxRadius: frame.boxRadius ?? 8
    };
});

// 组件边距
const itemPadding = computed(() => props.module?.itemPadding ?? 0);

const showTitle = computed(() => props.module?.showTitle == 1);
const title = computed(() => props.module?.title || "");
const desc = computed(() => props.module?.desc || "");
const titleColor = computed(() => props.module?.titleColor ?? "#2a3145");
</script>

<style lang="less" scoped>
// 标题/描述区：左右排列，与后台预览 dec-pc-coupon__header 一致
.dec-pc-coupon__header {
    display: flex;
    flex-direction: row;
    flex-wrap: nowrap;
    align-items: center;
    padding: 10px 0;
    box-sizing: border-box;

    &-title {
        line-height: 20px;
        font-size: 16px;
        font-weight: bold;
        color: #2a3145;
        word-break: break-all;
    }

    &-desc {
        margin-left: 5px;
        line-height: 18px;
        color: #8c8c8c;
        font-size: 12px;
        word-break: break-all;
        position: relative;
        top: 1px;
    }
}

// 圆角裁切：作用在最外层容器（base-class），让「圆角」扩展设置生效
:deep(.base-class) {
    overflow: hidden;
}

// 领券中心自带 margin-top（首页其它位置需要），装修容器内需归零以贴合 frame 边距
:deep(.mod_coupon) {
    margin-top: 0 !important;
}
</style>
