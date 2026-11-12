<template>
    <TotalContainer :module="containerModule" :class="'module-ad-con module-notice'" :style="containerStyle">
        <div class="notice-ad-warp">
            <div class="notice-con">
                <div class="notice-item">
                    <div class="item-content">
                        <div class="item-notice-con" :style="itemFormat.itemBackgroundColor + itemFormat.itemHeight + itemFormat.itemRadius">
                            <div v-if="module?.icoPic?.picUrl" class="item-notice-ico">
                                <img class="item-img" :src="imageFormat(module.icoPic.picUrl)" alt="" />
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
import { computed } from "vue";
import TotalContainer from "~/decorate/components/shop/TotalContainer.vue";
import { imageFormat } from "~/utils/format";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

const frame = computed(() => props.module?.frame ?? {});

const containerModule = computed(() => {
    const data = props.module ?? {};

    return {
        boxPadding: frame.value?.boxPadding ?? data?.boxPadding ?? 10,
        boxPaddingTop: frame.value?.boxPaddingTop ?? data?.boxPaddingTop ?? 5,
        boxPaddingBottom: frame.value?.boxPaddingBottom ?? data?.boxPaddingBottom ?? 5,
        boxRadius: frame.value?.boxRadius ?? data?.boxRadius ?? 0,
        backgroundColor: frame.value?.backgroundColor || data?.backgroundColor || "#ffffff",
        innerPadding: frame.value?.innerPadding ?? data?.innerPadding ?? 0
    };
});

const containerStyle = computed(() => ({
    backgroundColor: containerModule.value.backgroundColor,
    borderRadius: `${containerModule.value.boxRadius}px`,
    overflow: containerModule.value.boxRadius ? "hidden" : undefined
}));

const itemFormat = computed(() => {
    return {
        itemBackgroundColor: `background-color:${frame.value?.itemBackgroundColor ?? "#fff8e6"};`,
        itemRadius: `border-radius:${frame.value?.itemRadius ?? 4}px;`,
        itemHeight: `height:${frame.value?.itemHeight ?? 35}px;`,
        textColor: `color:${frame.value?.textColor ?? "#555555"};`
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
                object-fit: contain;
            }
        }

        .item-notice-list {
            margin-left: 8px;
            flex: 1;
            overflow: hidden;

            .item-notice-input {
                // 全局 html 字号为 12px，此处显式对齐后台装修预览的 14px
                font-size: 14px;
                white-space: nowrap;
                overflow: hidden;
                text-overflow: ellipsis;
            }
        }
    }
}
</style>
