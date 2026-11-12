<template>
    <TotalContainer v-if="module && frameModule" :module="frameModule" :class="'module-ad-con module-white_line'">
        <div class="white-line-warp">
            <div class="white-line-con">
                <div v-if="lineType == 2" :class="'white-line-item line-style-' + lineStyle">
                    <div class="item-line"></div>
                </div>
            </div>
        </div>
    </TotalContainer>
</template>

<script lang="ts" setup>
/**
 * PC端装修分割线组件
 * - 用于在模块之间添加分割线
 * - lineType: 1=无线条, 2=显示线条
 * - lineStyle: 1=实线, 2=虚线, 3=点线
 */
import { computed } from "vue";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

// 获取 lineType，默认为 2（显示线条）
const lineType = computed(() => {
    return Number(props.module?.lineType) || 2;
});

// 获取 lineStyle，默认为 1（实线）
const lineStyle = computed(() => {
    return Number(props.module?.lineStyle) || 1;
});

// 处理 frame 数据，如果没有则提供默认值
const frameModule = computed(() => {
    if (props.module?.frame) {
        return props.module.frame;
    }
    // 返回默认的 frame 结构
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
/*线条*/
.white-line-warp {
    width: 100%;
    box-sizing: border-box;

    .white-line-con {
        padding: 15px 0;

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
