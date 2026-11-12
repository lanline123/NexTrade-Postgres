<template>
    <div class="comment-list-ov">
        <div class="page-info-body">
            <div v-for="item in modelValue" class="comment-card">
                <div class="comment-card-evaluate">
                    <div class="option">
                        <div>{{ $t("评价") }}：</div>
                        <div :class="`icon-star-${item.commentRank}`" class="icon-star xingxing"></div>
                        <div class="ml10 time">{{ item.addTime }}</div>
                    </div>
                    <div class="right-tag">
                        <span v-for="tag in item.commentTag" class="evaluate-tips">{{ tag }}</span>
                    </div>
                </div>
                <div class="comment-image-info">
                    <NuxtLink
                        :title="item.productName"
                        :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.productSn })"
                        class="img-div"
                        target="_blank"
                    >
                        <el-image :src="productImage(item)" :title="item.productName" :alt="item.productName || ''" />
                    </NuxtLink>
                    <div class="right-div">
                        <div style="color: black">[{{ $t("商品评论") }}] {{ item.productName }}</div>
                        <div class="text">
                            {{ item.content }}
                        </div>
                        <div class="print-out">
                            <div v-for="product in item.showPics">
                                <el-image
                                    class="img"
                                    :hide-on-click-modal="true"
                                    :src="productImage(product)"
                                    :preview-src-list="[imageFormat(product.picUrl)]"
                                    :initial-index="0"
                                    fit="cover"
                                />
                            </div>
                        </div>
                        <div v-if="item.reply" class="reply">
                            <span class="name">{{ $t("客服答复") }}：</span>
                            <span class="text">{{ item.reply?.content }}</span>
                            <span class="time">{{ item.reply?.addTime }}</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import type { CommentFilterState } from "~/types/user/comment.d";
import { imageFormatList } from "~/utils/format";
import { ref } from "vue";

const props = defineProps({
    modelValue: {
        type: Array as PropType<CommentFilterState[]>,
        default: []
    }
});
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.comment-list-ov {
    .title-or-tabs {
        .tag-and-input {
            display: flex;
            flex-direction: row;
            justify-content: space-between;
            width: 100%;

            .tig-font-style {
                font-size: 14px;
            }

            .image {
                margin-bottom: 20px;
            }

            .input {
                display: flex;
                align-items: center;
            }
        }
    }

    .comment-card {
        display: flex;
        flex-direction: column;
        width: 100%;
        border-bottom: 1px solid #eeeeee;
        padding: 10px;

        .comment-card-evaluate {
            display: flex;
            color: #666;
            margin-bottom: 10px;
            flex-direction: column;
            gap: 8px;
            .option {
                display: flex;
                gap: 8px;
            }
            .right-tag {
                display: flex;
                flex: 1;
                flex-wrap: wrap;
                gap: 8px;

                .evaluate-tips {
                    background-color: #eeeeee;
                    padding: 2px 4px;
                    white-space: nowrap;
                }
            }
        }

        .comment-image-info {
            display: flex;
            gap: 10px;
            flex-direction: row;

            .img-div {
                flex: 0 0 60px;
            }

            .right-div {
                display: flex;
                flex: 1 1 auto;
                gap: 10px;
                color: #999999;
                flex-direction: column;
                overflow: auto;

                .text {
                    flex: 1 0 auto;
                    overflow-wrap: break-word;
                    word-wrap: break-word;
                }

                .print-out {
                    display: flex;
                    gap: 4px;
                    flex-wrap: wrap;

                    .img {
                        cursor: pointer;
                        width: 80px;
                        height: 80px;
                        border: 1px solid #eee;
                        margin-right: 5px;
                        flex-wrap: wrap;
                    }
                }

                .reply {
                    margin: 5px 0 5px 0;
                    padding: 10px;
                    background: none repeat scroll 0 0 var(--tag-bg);
                    border-radius: 8px;
                    -moz-border-radius: 8px;
                    -webkit-border-radius: 6px;
                    clear: both;
                    color: var(--tag-text);
                    border: 1px dashed var(--tag-bg);
                    line-height: 1.75;

                    .text {
                        color: black;
                        display: block;
                    }
                }
            }
        }
    }

    .page-info-body {
        & > div:last-child {
            border-bottom: none;
        }
    }
}
.image {
    width: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
}
</style>
