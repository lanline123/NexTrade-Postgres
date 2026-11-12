<template>
    <TotalContainer :module="module.frame" :class="'module-ad-con module-title_ad ad-title_align__' + (module?.title?.titleAlign || 1)" :style="containerStyle">
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
                        <div :class="'image-ad-title ad-title_style__' + (module?.title?.titleStyle || 1)" :style="allFormat.titleStyle">
                            <div class="image-ad-title-l">
                                <div class="image-ad-title-h" :style="allFormat.titleColor">{{ module?.title?.titleText || "" }}</div>
                                <div v-if="module?.title?.descText" class="image-ad-title-d" :style="allFormat.descColor">
                                    {{ module?.title?.descText || "" }}
                                </div>
                            </div>
                            <template v-if="module?.title?.showMore == 1">
                                <div class="image-ad-title-r">
                                    <NuxtLink class="more-link" target="_blank" :to="urlFormat(module?.title?.moreLink)" :style="allFormat.moreColor">
                                        <div class="flex align-center">
                                            <span>{{ moreText }}</span>
                                            <i class="iconfont-pc icon-login-jt" />
                                        </div>
                                    </NuxtLink>
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
import { computed } from "vue";
import TotalContainer from "~/decorate/components/shop/TotalContainer.vue";
import { urlFormat } from "~/utils/format";

const module = defineModel("module", {
    type: Object,
    default: () => ({})
});

const { t } = useI18n();
// 「查看更多」文字：优先使用后台配置的链接文字，未配置时回退默认文案
const moreText = computed(() => module.value?.title?.moreLink?.title || t("查看更多"));

const allFormat = computed(() => {
    const title = module.value?.title;
    const titleRadius = title?.titleRadius || 0;
    const titleBackground = title?.titleBackground || "";
    const titleBackground2 = title?.titleBackground2 || "";
    const leftAndRightPadding = module.value?.frame?.leftAndRightPadding || 0;

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
        titleStyle: `border-radius: ${titleRadius}px; padding: 10px ${leftAndRightPadding}px; ${backgroundStyle}`
    };
});

const containerStyle = computed(() => {
    const frame = module.value?.frame ?? {};
    const boxRadius = frame.boxRadius || 0;
    return {
        backgroundColor: frame.backgroundColor || "",
        borderRadius: `${boxRadius}px`,
        overflow: boxRadius ? "hidden" : undefined
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
        box-sizing: border-box;

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

/* 标题栏风格二：标题加粗、描述更突出（与后台预览 --style-2 保持一致） */
.image-ad-title.ad-title_style__2 {
    .image-ad-title-l .image-ad-title-h {
        font-weight: 600;
    }

    .image-ad-title-l .image-ad-title-d {
        font-size: 16px;
        font-weight: 600;
        color: #11bb55;
        padding-top: 0;
    }
}

.title-ad-warp.ad-title_align__2 .image-ad-title {
    justify-content: center;
}

.title-ad-warp.ad-title_align__2 .image-ad-title .image-ad-title-l {
    flex: none;
    align-items: center;
    text-align: center;
}

.title-ad-warp.ad-title_align__2 .image-ad-title .image-ad-title-r {
    position: absolute;
    right: 0;
}
</style>
