<template>
    <TotalContainer :module="module.frame" :class="'module-ad-con module-notice'">
        <div class="notice-ad-warp">
            <div class="notice-con">
                <div class="notice-item">
                    <div class="item-content">
                        <div class="item-notice-con" :style="itemFormat.itemBackgroundColor + itemFormat.itemHeight + itemFormat.itemRadius">
                            <div class="item-notice-ico" v-if="module?.icoPic?.picUrl">
                                <el-image class="item-img" fit="contain" :src="imageFormat(module.icoPic.picUrl)" />
                            </div>
                            <div class="item-notice-list">
                                <div class="item-notice-input" :style="itemFormat.textColor">{{ $t(module?.text || "") }}</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </TotalContainer>
</template>
<script lang="ts" setup>
/**
 * PC端装修通知公告组件
 * - 展示单行滚动公告信息
 * - 支持自定义公告图标、背景色、文字颜色、圆角等
 * - 文本超出时自动省略显示
 */
import { computed } from "vue";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";
import { imageFormat } from "@/utils/format";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

const itemFormat = computed(() => {
    return {
        itemBackgroundColor: props.module?.frame?.itemBackgroundColor ? `background-color:${props.module.frame.itemBackgroundColor};` : "",
        itemRadius: props.module?.frame?.itemRadius ? `border-radius:${props.module.frame.itemRadius}px;` : "",
        itemHeight: props.module?.frame?.itemHeight ? `height:${props.module.frame.itemHeight}px;` : "height:20px;",
        textColor: props.module?.frame?.textColor ? `color:${props.module.frame.textColor};` : ""
    };
});
</script>
<style lang="less" scoped>
.notice-ad-warp {
    width: 100%;
    box-sizing: border-box;

    .notice-con {
        width: 100%;
    }

    .item-notice-con {
        background: #fff;
        height: 20px;
        display: flex;
        align-items: center;
        padding: 5px 0;
        color: #555555;

        .item-notice-ico {
            margin-left: 10px;
            width: 16px;
            height: 16px;
            flex-shrink: 0;

            .item-img {
                width: 100%;
                height: 100%;
            }
        }

        .item-notice-list {
            margin-left: 8px;
            flex: 1;
            overflow: hidden;

            .item-notice-input {
                white-space: nowrap;
                overflow: hidden;
                text-overflow: ellipsis;
            }
        }
    }
}
</style>
