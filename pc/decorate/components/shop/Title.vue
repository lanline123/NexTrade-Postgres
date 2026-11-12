<template>
    <div
        v-if="module.showTitle == 1"
        :style="{
            background: module.titleBackground2
                ? 'linear-gradient(to right, ' + module.titleBackground + ', ' + module.titleBackground2 + ')'
                : module.titleBackground,
            'border-top-left-radius': module.titleRadius + 'px',
            'border-top-right-radius': module.titleRadius + 'px'
        }"
        class="top-bar"
        :class="[`top-bar--style-${module.titleStyle || 1}`, `top-bar--align-${module.titleAlign || 1}`]"
    >
        <div class="left-title">
            <el-image v-if="module.titleBackgroundPic?.picUrl" class="image-size" :src="imageFormat(module.titleBackgroundPic.picUrl)"></el-image>
            <div :style="{ color: module.titleColor }" class="title-text">{{ module.titleText }}</div>
            <div v-if="module.descText" :style="{ color: module.descColor }" class="desc-text">
                {{ module.descText }}
            </div>
        </div>
        <NuxtLink v-if="module.showMore == 1" target="_blank" :to="urlFormat(module.moreLink)" :style="{ color: module.moreColor }" class="right-title">
            <span> {{ moreText }}</span>
            <SvgIcon class="icon" name="arrow-right" width="16" height="16" />
        </NuxtLink>
    </div>
</template>
<script setup lang="ts">
import { computed, ref } from "vue";
import { imageFormat, urlFormat } from "~/utils/format";

const { t } = useI18n();

const props = defineProps({
    module: { type: Object, default: () => ({}) },
    rightText: { type: String, default: "查看更多" }
});

const rightText = ref(props.rightText);

// 优先使用后台「更多链接」配置的文字；未配置时回退到默认文案（默认文案做 i18n）
const moreText = computed(() => {
    const title = props.module?.moreLink?.title;
    return title ? title : t(rightText.value);
});
</script>
<style scoped lang="less">
.top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 10px;
    box-sizing: border-box;
    line-height: 32px;
    position: relative;

    .left-title {
        display: flex;
        flex-wrap: nowrap;
        align-items: center;
        gap: 6px;

        .image-size {
            width: 32px;
            height: 32px;
            flex-shrink: 0;
        }

        .title-text {
            font-weight: 700;
            color: #2a3145;
            font-size: 18px;
            position: relative;
        }

        .desc-text {
            color: #aaa;
            font-size: 13px;
            position: relative;
            top: 1px;
        }
    }

    .right-title {
        display: flex;
        align-items: center;
        font-size: 14px;
        color: #1a1a1a;
        text-decoration: none;
        cursor: pointer;

        &:hover {
            color: var(--main-bg);
        }

        :deep(svg) {
            margin-left: 2px;
            position: relative;

            path {
                stroke: currentColor;
            }
        }
    }

    // Style 2: 左侧装饰线
    &.top-bar--style-2 {
        .title-text {
            padding-left: 10px;

            &::before {
                content: "";
                position: absolute;
                left: 0;
                top: 50%;
                transform: translateY(-50%);
                width: 3px;
                height: 16px;
                background: linear-gradient(180deg, #155bd4 0%, #64b5f6 100%);
                border-radius: 2px;
            }
        }
    }

    // Style 3: 下划线装饰
    &.top-bar--style-3 {
        .title-text {
            &::after {
                content: "";
                position: absolute;
                left: 0;
                bottom: -2px;
                width: 100%;
                height: 2px;
                background: linear-gradient(90deg, #155bd4 0%, transparent 100%);
                border-radius: 1px;
            }
        }
    }

    // Align 1: 左对齐（默认）
    &.top-bar--align-1 {
        justify-content: space-between;
    }

    // Align 2: 居中
    &.top-bar--align-2 {
        justify-content: center;

        .left-title {
            flex-direction: column;
            align-items: center;
            text-align: center;
        }

        .desc-text {
            margin-left: 0;
            margin-top: 2px;
        }

        .right-title {
            position: absolute;
            right: 10px;
        }
    }

    // Align 3: 右对齐
    &.top-bar--align-3 {
        flex-direction: row-reverse;

        .left-title {
            flex-direction: row-reverse;
        }

        .image-size {
            margin-right: 0;
            margin-left: 5px;
        }

        .desc-text {
            margin-left: 0;
            margin-right: 5px;
        }

        .right-title {
            margin-right: auto;
        }
    }
}
</style>
