<template>
    <TotalContainer v-if="module && frameModule" :module="frameModule" :class="'module-ad-con module-white_line'">
        <div class="white-line-warp">
            <div class="white-line-con" :style="{ height: itemHeight + 'px' }">
                <div v-if="lineType == 2" :class="'white-line-item line-style-' + lineStyle">
                    <div class="item-line"></div>
                </div>
            </div>
        </div>
    </TotalContainer>
</template>

<script lang="ts" setup>
import { computed } from "vue";
import TotalContainer from "~/decorate/components/shop/TotalContainer.vue";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

const lineType = computed(() => {
    return Number(props.module?.lineType) || 2;
});

const lineStyle = computed(() => {
    return Number(props.module?.lineStyle) || 1;
});

// 组件高度（对齐后台「容器设置」的 itemHeight，默认 10px）
const itemHeight = computed(() => {
    const raw = Number(props.module?.frame?.itemHeight);
    return Number.isFinite(raw) ? raw : 10;
});

const frameModule = computed(() => {
    if (props.module?.frame) {
        return props.module.frame;
    }
    return {
        innerPadding: 0,
        boxPaddingTop: 0,
        boxPaddingBottom: 0,
        boxPadding: 0,
        backgroundColor: "",
        boxRadius: 0
    };
});
</script>

<style lang="less" scoped>
.white-line-warp {
    width: 100%;
    box-sizing: border-box;

    .white-line-con {
        display: flex;
        align-items: center;

        .white-line-item {
            width: 100%;

            .item-line {
                height: 1px;
            }
        }

        .line-style-1 .item-line {
            border-bottom: 1px solid #e5e5e5;
        }

        .line-style-2 .item-line {
            border-bottom: 1px dashed #e5e5e5;
        }

        .line-style-3 .item-line {
            border-bottom: 1px dotted #e5e5e5;
        }
    }
}
</style>
