<template>
    <TotalContainer :module="module.frame">
        <!-- 左右内边距：与后台预览 __content padding 对齐，让「左右内边距」在前台生效 -->
        <div :style="innerPaddingStyle">
            <Title :module="module.title"></Title>
            <template v-if="module.picList && module.picList.length > 0">
                <div class="image-list" :style="containerStyle">
                    <template v-if="module.picList">
                        <template v-if="module.picType == 5">
                            <div class="image-square-layout image-square-layout-height2">
                                <div class="image-ad-item">
                                    <NuxtLink v-if="module.picList[0].picLink" :to="urlFormat(module.picList[0].picLink)" target="_blank" class="item-content">
                                        <imageBox :src="module.picList[0].picUrl" />
                                    </NuxtLink>
                                    <div v-else class="item-content">
                                        <imageBox :src="module.picList[0].picUrl" />
                                    </div>
                                </div>
                                <div class="grid-layout-2">
                                    <div class="image-ad-item">
                                        <NuxtLink v-if="module.picList[1].picLink" :to="urlFormat(module.picList[1].picLink)" target="_blank" class="item-content">
                                            <imageBox :src="module.picList[1].picUrl" />
                                        </NuxtLink>
                                        <div v-else class="item-content">
                                            <imageBox :src="module.picList[1].picUrl" />
                                        </div>
                                    </div>
                                    <div class="image-ad-item">
                                        <NuxtLink v-if="module.picList[2].picLink" :to="urlFormat(module.picList[2].picLink)" target="_blank" class="item-content">
                                            <imageBox :src="module.picList[2].picUrl" />
                                        </NuxtLink>
                                        <div v-else class="item-content">
                                            <imageBox :src="module.picList[2].picUrl" />
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </template>
                        <template v-else-if="module.picType == 7">
                            <div class="image-square-layout2">
                                <div class="image-ad-item">
                                    <NuxtLink v-if="module.picList[0].picLink" :to="urlFormat(module.picList[0].picLink)" target="_blank" class="item-content">
                                        <imageBox :src="module.picList[0].picUrl" />
                                    </NuxtLink>
                                    <div v-else class="item-content">
                                        <imageBox :src="module.picList[0].picUrl" />
                                    </div>
                                </div>
                                <div class="grid-layout-2">
                                    <div class="image-ad-item">
                                        <NuxtLink v-if="module.picList[1].picLink" :to="urlFormat(module.picList[1].picLink)" target="_blank" class="item-content">
                                            <imageBox :src="module.picList[1].picUrl" />
                                        </NuxtLink>
                                        <div v-else class="item-content">
                                            <imageBox :src="module.picList[1].picUrl" />
                                        </div>
                                    </div>
                                    <div class="image-ad-item">
                                        <NuxtLink v-if="module.picList[2].picLink" :to="urlFormat(module.picList[2].picLink)" target="_blank" class="item-content">
                                            <imageBox :src="module.picList[2].picUrl" />
                                        </NuxtLink>
                                        <div v-else class="item-content">
                                            <imageBox :src="module.picList[2].picUrl" />
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </template>
                        <template v-else-if="module.picType == 6">
                            <div class="image-square-layout image-square-layout3 image-square-layout-height2">
                                <div class="image-ad-item">
                                    <NuxtLink v-if="module.picList[0].picLink" :to="urlFormat(module.picList[0].picLink)" target="_blank" class="item-content">
                                        <imageBox :src="module.picList[0].picUrl" />
                                    </NuxtLink>
                                    <div v-else class="item-content">
                                        <imageBox :src="module.picList[0].picUrl" />
                                    </div>
                                </div>
                                <div class="grid-layout-2">
                                    <div>
                                        <div class="image-ad-item">
                                            <NuxtLink
                                                v-if="module.picList[1].picLink"
                                                :to="urlFormat(module.picList[1].picLink)"
                                                target="_blank"
                                                class="item-content"
                                            >
                                                <imageBox :src="module.picList[1].picUrl" />
                                            </NuxtLink>
                                            <div v-else class="item-content">
                                                <imageBox :src="module.picList[1].picUrl" />
                                            </div>
                                        </div>
                                    </div>
                                    <div class="grid-layout-3">
                                        <div class="image-ad-item">
                                            <NuxtLink
                                                v-if="module.picList[2].picLink"
                                                :to="urlFormat(module.picList[2].picLink)"
                                                target="_blank"
                                                class="item-content"
                                            >
                                                <imageBox :src="module.picList[2].picUrl" />
                                            </NuxtLink>
                                            <div v-else class="item-content">
                                                <imageBox :src="module.picList[2].picUrl" />
                                            </div>
                                        </div>
                                        <div class="image-ad-item">
                                            <NuxtLink
                                                v-if="module.picList[3].picLink"
                                                :to="urlFormat(module.picList[3].picLink)"
                                                target="_blank"
                                                class="item-content"
                                            >
                                                <imageBox :src="module.picList[3].picUrl" />
                                            </NuxtLink>
                                            <div v-else class="item-content">
                                                <imageBox :src="module.picList[3].picUrl" />
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </template>
                        <template v-else>
                            <div :class="'image-square-layout-' + module.picType + ' image-square-layout-height1'">
                                <div v-for="(pic, key) in module.picList" :key="key" class="image-ad-item">
                                    <NuxtLink v-if="pic.picLink" :to="urlFormat(pic.picLink)" target="_blank" class="item-content">
                                        <div class="item-img-a">
                                            <el-image class="item-img" fit="fill" :src="imageFormat(pic.picUrl)" alt="" />
                                        </div>
                                    </NuxtLink>
                                    <div v-else class="item-content">
                                        <div class="item-img-a">
                                            <el-image class="item-img" fit="fill" :src="imageFormat(pic.picUrl)" alt="" />
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </template>
                    </template>
                </div>
            </template>
        </div>
    </TotalContainer>
</template>

<script lang="ts" setup>
import { computed } from "vue";
import TotalContainer from "~/decorate/components/shop/TotalContainer.vue";
import Title from "~/decorate/components/shop/Title.vue";
import imageBox from "./pcImageSquareAd/components/imageBox.vue";
import { urlFormat, imageFormat } from "~/utils/format";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

const containerStyle = computed(() => ({
    "--img-gap": (props.module.imgPadding ?? 0) + "px"
}));

const innerPaddingStyle = computed(() => {
    const frame = props.module.frame ?? {};
    const padding = frame.leftAndRightPadding ?? frame.innerPadding ?? 0;
    return { padding: padding + "px" };
});
</script>

<style lang="less" scoped>
// 容器圆角裁切：与后台预览（__content overflow:hidden）一致，让「圆角」设置真正生效
:deep(.base-class) {
    overflow: hidden;
}
.image-list {
    box-sizing: border-box;
}
.image-ad-item {
    height: 100%;
    width: 100%;
}
.item-content {
    height: 100%;
    width: 100%;
    overflow: hidden;
    box-sizing: border-box;
}
.item-img-a {
    height: 100%;
    width: 100%;
}
.item-img {
    width: 100%;
}
.image-square-layout-height1 {
    height: auto;
    width: 100%;
}
.image-square-layout-height2 {
    height: auto;
    width: 100%;
}

.image-square-layout-1 {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--img-gap, 0);
}
.image-square-layout-2 {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--img-gap, 0);
}
.image-square-layout-3 {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--img-gap, 0);
}
.image-square-layout-4 {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    grid-template-rows: repeat(2, 1fr);
    gap: var(--img-gap, 0);

    .image-ad-item:nth-child(2) {
        grid-column: 1 / 2;
    }
    .image-ad-item:nth-child(3) {
        grid-row: 1 / 2;
        grid-column: 2 / 2;
    }
}

.image-square-layout {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--img-gap, 0);

    & .grid-layout-2 {
        display: grid;
        grid-template-rows: repeat(2, 1fr);
        gap: var(--img-gap, 0);
    }

    &.image-square-layout3 {
        & .grid-layout-3 {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: var(--img-gap, 0);
        }
    }
}

.image-square-layout2 {
    display: grid;
    gap: var(--img-gap, 0);

    & .grid-layout-2 {
        display: grid;
        grid-template-columns: repeat(2, 1fr);
        gap: var(--img-gap, 0);
    }
}
</style>
