<template>
    <TotalContainer v-if="module.picList && module.picList.length > 0" :module="module.frame">
        <Title :module="module.title"></Title>
        <template v-if="module.picList">
            <div class="image-hotarea-warp" :style="format.innerPadding">
                <div class="image-ad-con">
                    <template v-for="item in module.picList" :key="item.picId">
                        <div class="image-ad-item">
                            <div class="item-content" :style="format.imgPadding">
                                <div class="item-content-hotatrea">
                                    <div class="item-img-a">
                                        <el-image class="item-img" fit="contain" :src="imageFormat(item.picUrl)" />
                                    </div>
                                    <template v-if="item.hotarea">
                                        <div
                                            v-for="(subItem, index) in item.hotarea"
                                            :key="index"
                                            class="image-hotarea-link"
                                            :data-width="subItem.width"
                                            :data-height="subItem.height"
                                            :data-left="subItem.pageX"
                                            :data-top="subItem.pageY"
                                            :style="subItem.position"
                                            @click="subItem.link && handleClick(subItem.link)"
                                        >
                                            <span class="area_box_con_text">{{ subItem.link?.name }}</span>
                                        </div>
                                    </template>
                                </div>
                            </div>
                        </div>
                    </template>
                </div>
            </div>
        </template>
    </TotalContainer>
</template>

<script lang="ts" setup>
/**
 * PC端装修热区图片组件
 * - 支持在图片上绘制可点击的热区区域
 * - 热区支持跳转到指定链接（内部路由或外部URL）
 * - 热区位置通过百分比定位，自适应图片尺寸
 */
import { computed } from "vue";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";
import Title from "~/pages/activity/components/Title.vue";
import { imageFormat, urlFormat } from "@/utils/format";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

const format = computed(() => {
    return {
        imgPadding: "padding:" + (props.module?.imgPadding || 0) + "px;",
        innerPadding: props.module.frame?.innerPadding ? "padding:" + props.module.frame.innerPadding + "px;" : ""
    };
});

const handleClick = (link: any) => {
    if (!link) return;
    const url = urlFormat(link);
    if (url) {
        if (url.startsWith("http")) {
            window.open(url, "_blank");
        } else {
            navigateTo(url);
        }
    }
};
</script>

<style lang="less" scoped>
.image-hotarea-warp {
    box-sizing: border-box;
    width: 100%;

    .image-ad-con {
        width: 100%;
    }

    .image-ad-item {
        width: 100%;
        position: relative;
        margin-bottom: 10px;

        &:last-child {
            margin-bottom: 0;
        }

        .item-content {
            width: 100%;
            box-sizing: border-box;
            position: relative;

            .item-content-hotatrea {
                position: relative;
                width: 100%;
                display: flex;

                .item-img-a {
                    width: 100%;
                    position: relative;

                    .item-img {
                        width: 100%;
                        height: auto;
                        display: block;
                    }
                }

                .image-hotarea-link {
                    position: absolute;
                    cursor: pointer;
                    z-index: 10;
                    transition: opacity 0.3s;

                    &:hover {
                        opacity: 0.8;
                    }

                    .area_box_con_text {
                        display: none;
                    }
                }
            }
        }
    }
}
</style>
