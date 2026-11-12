<template>
    <TotalContainer :module="module.frame">
        <Title :module="module.title"></Title>
        <template v-if="module.picList && module.picList.length > 0">
            <div class="image-list">
                <template v-if="module.picList">
                    <template v-if="module.picType == 5">
                        <div class="image-square-layout image-square-layout-height2">
                            <div class="image-ad-item">
                                <NuxtLink
                                    v-if="module.picList[0].picLink"
                                    :to="urlFormat(module.picList[0].picLink)"
                                    target="_blank"
                                    class="item-content"
                                    :style="format.imgPadding"
                                >
                                    <imageBox :src="module.picList[0].picUrl" />
                                </NuxtLink>
                                <div v-else class="item-content" :style="format.imgPadding">
                                    <imageBox :src="module.picList[0].picUrl" />
                                </div>
                            </div>
                            <div class="grid-layout-2">
                                <div class="image-ad-item">
                                    <NuxtLink
                                        v-if="module.picList[1].picLink"
                                        :to="urlFormat(module.picList[1].picLink)"
                                        target="_blank"
                                        class="item-content"
                                        :style="format.imgPadding"
                                    >
                                        <imageBox :src="module.picList[1].picUrl" />
                                    </NuxtLink>
                                    <div v-else class="item-content" :style="format.imgPadding">
                                        <imageBox :src="module.picList[1].picUrl" />
                                    </div>
                                </div>
                                <div class="image-ad-item">
                                    <NuxtLink
                                        v-if="module.picList[2].picLink"
                                        :to="urlFormat(module.picList[2].picLink)"
                                        target="_blank"
                                        class="item-content"
                                        :style="format.imgPadding"
                                    >
                                        <imageBox :src="module.picList[2].picUrl" />
                                    </NuxtLink>
                                    <div v-else class="item-content" :style="format.imgPadding">
                                        <imageBox :src="module.picList[2].picUrl" />
                                    </div>
                                </div>
                            </div>
                        </div>
                    </template>
                    <template v-else-if="module.picType == 7">
                        <div class="image-square-layout2">
                            <div class="image-ad-item">
                                <NuxtLink
                                    v-if="module.picList[0].picLink"
                                    :to="urlFormat(module.picList[0].picLink)"
                                    target="_blank"
                                    class="item-content"
                                    :style="format.imgPadding"
                                >
                                    <imageBox :src="module.picList[0].picUrl" />
                                </NuxtLink>
                                <div v-else class="item-content" :style="format.imgPadding">
                                    <imageBox :src="module.picList[0].picUrl" />
                                </div>
                            </div>
                            <div class="grid-layout-2">
                                <div class="image-ad-item">
                                    <NuxtLink
                                        v-if="module.picList[1].picLink"
                                        :to="urlFormat(module.picList[1].picLink)"
                                        target="_blank"
                                        class="item-content"
                                        :style="format.imgPadding"
                                    >
                                        <imageBox :src="module.picList[1].picUrl" />
                                    </NuxtLink>
                                    <div v-else class="item-content" :style="format.imgPadding">
                                        <imageBox :src="module.picList[1].picUrl" />
                                    </div>
                                </div>
                                <div class="image-ad-item">
                                    <NuxtLink
                                        v-if="module.picList[2].picLink"
                                        :to="urlFormat(module.picList[2].picLink)"
                                        target="_blank"
                                        class="item-content"
                                        :style="format.imgPadding"
                                    >
                                        <imageBox :src="module.picList[2].picUrl" />
                                    </NuxtLink>
                                    <div v-else class="item-content" :style="format.imgPadding">
                                        <imageBox :src="module.picList[2].picUrl" />
                                    </div>
                                </div>
                            </div>
                        </div>
                    </template>
                    <template v-else-if="module.picType == 6">
                        <div class="image-square-layout image-square-layout3 image-square-layout-height2">
                            <div class="image-ad-item">
                                <NuxtLink
                                    v-if="module.picList[0].picLink"
                                    :to="urlFormat(module.picList[0].picLink)"
                                    target="_blank"
                                    class="item-content"
                                    :style="format.imgPadding"
                                >
                                    <imageBox :src="module.picList[0].picUrl" />
                                </NuxtLink>
                                <div v-else class="item-content" :style="format.imgPadding">
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
                                            :style="format.imgPadding"
                                        >
                                            <imageBox :src="module.picList[1].picUrl" />
                                        </NuxtLink>
                                        <div v-else class="item-content" :style="format.imgPadding">
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
                                            :style="format.imgPadding"
                                        >
                                            <imageBox :src="module.picList[2].picUrl" />
                                        </NuxtLink>
                                        <div v-else class="item-content" :style="format.imgPadding">
                                            <imageBox :src="module.picList[2].picUrl" />
                                        </div>
                                    </div>
                                    <div class="image-ad-item">
                                        <NuxtLink
                                            v-if="module.picList[3].picLink"
                                            :to="urlFormat(module.picList[3].picLink)"
                                            target="_blank"
                                            class="item-content"
                                            :style="format.imgPadding"
                                        >
                                            <imageBox :src="module.picList[3].picUrl" />
                                        </NuxtLink>
                                        <div v-else class="item-content" :style="format.imgPadding">
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
                                <NuxtLink v-if="pic.picLink" :to="urlFormat(pic.picLink)" target="_blank" class="item-content" :style="format.imgPadding">
                                    <div class="item-img-a">
                                        <el-image class="item-img" fit="fill" :src="imageFormat(pic.picUrl)" />
                                    </div>
                                </NuxtLink>
                                <div v-else class="item-content" :style="format.imgPadding">
                                    <div class="item-img-a">
                                        <el-image class="item-img" fit="fill" :src="imageFormat(pic.picUrl)" />
                                    </div>
                                </div>
                            </div>
                        </div>
                    </template>
                </template>
            </div>
        </template>
    </TotalContainer>
</template>
<script lang="ts" setup>
/**
 * PC端装修图片魔方组件
 * - 支持多种布局类型（picType 1-7）
 * - picType 1-4: 基础网格布局；picType 5/6/7: 特殊组合布局
 * - 每张图片可配置独立的跳转链接
 * - 支持自定义图片间距和圆角
 */
import { computed } from "vue";
import { urlFormat, imageFormat } from "@/utils/format";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";
import Title from "~/pages/activity/components/Title.vue";
import imageBox from "./src/imageBox.vue";

const props = defineProps({
    module: {
        type: Object,
        default: () => {}
    }
});

const format = computed(() => {
    return {
        imgPadding: "padding:" + props.module.imgPadding + "px;"
    };
});
</script>
<style lang="less" scoped>
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
    // height: 100%;
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
}
.image-square-layout-2 {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
}
.image-square-layout-3 {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
}
.image-square-layout-4 {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    grid-template-rows: repeat(2, 1fr);

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

    & .grid-layout-2 {
        display: grid;
        grid-template-rows: repeat(2, 1fr);
    }

    &.image-square-layout3 {
        & .grid-layout-3 {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
        }
    }
}

.image-square-layout2 {
    & .grid-layout-2 {
        display: grid;
        grid-template-columns: repeat(2, 1fr);
    }
}
</style>
