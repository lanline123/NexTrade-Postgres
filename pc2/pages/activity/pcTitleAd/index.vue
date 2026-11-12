<template>
    <TotalContainer :module="module.frame" :class="'module-ad-con module-title_ad ad-title_align__' + (module?.title?.titleAlign || 1)">
        <!-- 占位显示：没有标题文本时 -->
        <div v-if="!module?.title?.titleText || module.title.titleText.trim() === ''" class="module-ad-empty empty-title_ad">
            <div class="image-empty-bg"></div>
            <div class="desc">{{ $t("点击添加标题") }}</div>
        </div>
        <!-- 有标题时显示标题内容 -->
        <div v-else :class="'title-ad-warp ad-title_align__' + (module?.title?.titleAlign || 1)" :style="paddingStyle.paddingLeftRight">
            <div class="title-ad-con">
                <div class="title-ad-item">
                    <div class="item-content">
                        <div :class="'image-ad-title ad-title_style__' + (module?.title?.titleStyle || 1)">
                            <div class="image-ad-title-l" :style="allFormat.titleStyle">
                                <div class="image-ad-title-h" :style="allFormat.titleColor">{{ module?.title?.titleText || "" }}</div>
                                <div class="image-ad-title-d" :style="allFormat.descColor">{{ module?.title?.descText || "" }}</div>
                            </div>
                            <template v-if="module?.title?.showMore == 1">
                                <div class="image-ad-title-r">
                                    <div class="more-link" :style="allFormat.moreColor">
                                        <div class="flex align-center">
                                            <span>{{ $t("查看更多") }}</span>
                                            <i class="iconfont-pc icon-login-jt" />
                                        </div>
                                    </div>
                                </div>
                            </template>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </TotalContainer>
</template>

<script lang="ts" setup>
/**
 * PC端装修标题广告组件
 * - 展示模块标题和描述文字
 * - 支持左对齐(1)和居中对齐(2)两种布局
 * - 支持自定义背景色（含渐变）、圆角、文字颜色
 * - 可配置"查看更多"链接
 */
import { computed } from "vue";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";

const module = defineModel("module", {
    type: Object,
    default: () => ({})
})


const allFormat = computed(() => {
    const title = module.value?.title;
    const titleRadius = title?.titleRadius || 0;
    const titleBackground = title?.titleBackground || "";
    const titleBackground2 = title?.titleBackground2 || "";

    // 标题背景样式
    let backgroundStyle = "";
    if (titleBackground2 && titleBackground) {
        backgroundStyle = `background: linear-gradient(to right, ${titleBackground}, ${titleBackground2});`;
    } else if (titleBackground) {
        backgroundStyle = `background-color: ${titleBackground};`;
    }

    return {
        descColor: title?.descColor ? `color: ${title.descColor};` : "",
        moreColor: title?.moreColor ? `color: ${title.moreColor};` : "",
        titleColor: title?.titleColor ? `color: ${title.titleColor};` : "",
        titleStyle: `border-radius: ${titleRadius}px; ${backgroundStyle}`
    };
});

const paddingStyle = computed(() => {
    const leftAndRightPadding = module.value?.frame?.leftAndRightPadding || 0;
    return {
        paddingLeftRight: `padding-left: ${leftAndRightPadding}px; padding-right: ${leftAndRightPadding}px;`
    };
});
</script>
<style lang="less" scoped>
.title-ad-warp {
    width: 100%;
    box-sizing: border-box;

    .title-ad-item {
        padding: 0;
        min-height: 20px;

        .item-content {
            width: 100%;
        }
    }

    .image-ad-title {
        display: flex;
        align-items: center;
        justify-content: space-between;
        width: 100%;
        position: relative;
        padding: 10px 0 !important;

        .image-ad-title-l {
            display: flex;
            flex-direction: column;
            gap: 8px;
            flex: 1;
            overflow: hidden;

            .image-ad-title-h {
                line-height: 20px;
                font-size: 16px;
                font-weight: bold;
                color: #323233;
                word-break: break-all;
            }

            .image-ad-title-d {
                line-height: 18px;
                padding-top: 8px;
                color: #8c8c8c;
                font-size: 12px;
                word-break: break-all;
            }
        }

        .image-ad-title-r {
            .more-link {
                cursor: pointer;
                display: flex;
                align-items: center;
                gap: 4px;
                text-decoration: none;

                .iconfont-pc {
                    font-size: 12px;
                }
            }
        }
    }
}

.title-ad-warp.ad-title_align__2 .image-ad-title {
    justify-content: center;
}

.title-ad-warp.ad-title_align__2 .image-ad-title .image-ad-title-l {
    flex: none;
}

.title-ad-warp.ad-title_align__2 .image-ad-title .image-ad-title-r {
    position: absolute;
    right: 0;
}
</style>
