<template>
    <!-- D1146：评价功能关闭时整块不渲染，仅保留空态；内部各根元素维持原缩进以缩小 diff -->
    <template v-if="commentEnabled">
    <div class="compare-view-box" v-if="useToShow">
        <ul class="compare-view flex align-center justify-between">
            <li class="centage-praise">
                <span class="s-centage">{{ comment.goodPercent }}%</span>
                <span class="s-praise">{{ $t("好评度") }}</span>
            </li>
            <li class="centage-all">
                <div>
                    <div class="centage-b flex">
                        <div class="centageText">{{ $t("好评") }}：</div>
                        <span class="centage-enough"><span class="centage-show" :style="'width:' + comment.goodPercent + '%'"></span></span>
                        <span class="centage-num">{{ comment.goodPercent }}%</span>
                    </div>
                    <div class="centage-b flex">
                        <div class="centageText">{{ $t("中评") }}：</div>
                        <span class="centage-enough"><span class="centage-show" :style="'width:' + comment.moderatePercent + '%'"></span></span>
                        <span class="centage-num">{{ comment.moderatePercent }}%</span>
                    </div>
                    <div class="centage-b flex">
                        <div class="centageText">{{ $t("差评") }}：</div>
                        <span class="centage-enough"><span class="centage-show" :style="'width:' + comment.badPercent + '%'"></span></span>
                        <span class="centage-num">{{ comment.badPercent }}%</span>
                    </div>
                </div>
            </li>
            <li class="comment-go">
                <div class="go-comm-txt">
                    {{ $t("评价立送") }}<span class="font-color">5</span>{{ $t("积分") }}，{{ $t("晒单额外赠送") }}<span class="font-color">5</span
                    >{{ $t("积分") }}，{{ $t("我购买过商品") }}
                    <NuxtLink :to="urlFormat({ path: `member/comment/list` })" target="_blank" class="">
                        <el-button type="primary" size="small" class="ml10">{{ $t("我要评价") }}</el-button>
                    </NuxtLink>
                </div>
            </li>
        </ul>
    </div>

    <div class="comment-option" v-if="useToShow">
        <ul class="comment-option-sel flex">
            <li @click="commentTabChange(1)" :class="commentIndex == 1 ? 'o-over' : ''">
                <a href="javascript:;" data-kind="0" class="comment-type-btn"
                    >{{ $t("全部评论") }}(<em>{{ comment.total }}</em
                    >)</a
                >
            </li>
            <li @click="commentTabChange(2)" :class="commentIndex == 2 ? 'o-over' : ''">
                <a href="javascript:;" data-kind="1" class="comment-type-btn"
                    >{{ $t("好评") }}(<em>{{ comment.goodCount }}</em
                    >)</a
                >
            </li>
            <li @click="commentTabChange(3)" :class="commentIndex == 3 ? 'o-over' : ''">
                <a href="javascript:;" data-kind="2" class="comment-type-btn"
                    >{{ $t("中评") }}(<em>{{ comment.moderateCount }}</em
                    >)</a
                >
            </li>
            <li @click="commentTabChange(4)" :class="commentIndex == 4 ? 'o-over' : ''">
                <a href="javascript:;" data-kind="3" class="comment-type-btn"
                    >{{ $t("差评") }}(<em>{{ comment.badCount }}</em
                    >)</a
                >
            </li>
            <li @click="commentTabChange(5)" :class="commentIndex == 5 ? 'o-over' : ''">
                <a href="javascript:;" data-kind="4" class="comment-type-btn"
                    >{{ $t("有晒单评论") }}(<em>{{ comment.showCount }}</em
                    >)</a
                >
            </li>
        </ul>
    </div>
    <div :class="useToShow ? 'comnWid' : ''">
        <div class="comm_Wrap" v-for="(item, index) in commentList" v-if="commentList.length > 0">
            <div class="eachInfo flex">
                <div class="head">
                    <div class="logo">
                        <img width="88" height="88" style="border-radius: 8px" :src="imageFormat(item.avatar || '')" />
                    </div>
                    <div class="name">
                        {{ item.nickname || item.username }}
                    </div>
                </div>
                <div class="info">
                    <div class="starLi flex align-center justify-between">
                        <div class="fl flex">
                            <div class="star">
                                <span class="icon-star" :class="`icon-star-${item.commentRank}`"></span>
                            </div>
                        </div>
                        <div class="fr">
                            <span class="time">{{ item.addTime }}</span>
                        </div>
                    </div>
                    <div class="starLi g-comm-tag" v-if="item.commentTag">
                        <span v-for="tag in item.commentTag">{{ tag }}</span>
                    </div>
                    <div class="starLi">
                        <div class="fl flex">
                            <div class="msg">
                                {{ item.content }}
                            </div>
                        </div>
                    </div>
                    <div class="starLi" v-if="item.showPics">
                        <div class="fl flex">
                            <div class="pic-list flex flex-wrap">
                                <div class="pic" v-for="photo in item.showPics">
                                    <el-image
                                        :src="imageFormat(photo.picThumb)"
                                        fit="cover"
                                        class="image"
                                        loading="lazy"
                                        :preview-src-list="[imageFormat(photo.picUrl)]"
                                        :initial-index="0"
                                    />
                                </div>
                            </div>
                        </div>
                    </div>
                    <div class="starLi" v-if="item.replies && item.replies.length > 0">
                        <div class="reply" v-for="reply in item.replies" :key="reply.commentId">
                            <span class="name">{{ $t("客服回复") }}：</span>
                            <span class="text">{{ reply.content }}</span>
                            <span class="time">{{ reply.addTime }}</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        <div class="empty" v-if="commentList.length == 0 && !loading">「{{ $t("暂无评价") }}」</div>
        <div v-if="commentList.length >= total && total > 0 && props.useToShow" style="text-align: center; padding: 10px 0; color: #aaa">
            {{ $t("没有更多了") }}
        </div>
    </div>
    <!-- <div class="pagination" v-if="commentList.length > 0">
        <Pagination v-model:page="page" :size="size" :total="total" @callback="pageChange" />
    </div> -->
    </template>
    <CommonAppEmpty v-else :description="$t('评价功能已关闭')" />
</template>
 
<script lang="ts" setup>
import { computed, onMounted, ref, nextTick, watch } from "vue";
import { getComment, getCommentList } from "~/api/product/product";
import { Pagination } from "@/components/list";
import type { CommentDetail, CommentItem } from "~/types/product/product.d";
import { imageFormat } from "@/utils/format";
import { useRouter } from "vue-router";
import { urlFormat } from "@/utils/format";
import { useCommonStore } from "~/store/common";
const route = useRoute();
const commonStore = useCommonStore();
// D1146：商品评价总开关
const commentEnabled = computed(() => commonStore.productCommentEnabled === 1);
const props = defineProps({
    modelValue: {
        type: Number,
        default: 0
    },
    useToShow: {
        type: Boolean
    }
});
const id = ref<number>(props.modelValue);
// const id = ref(Number(props.product.productId));
const commentIndex = ref(1);
const loading = ref(false);
const comment = ref<CommentDetail>({});
const commentList = ref<CommentItem[]>([]);
const total = ref(0);
const page = ref<number>(1);
const size = ref(5);
const commentTabChange = async (data: any) => {
    commentIndex.value = data;
    commentList.value.length = 0;
    _getComment(id.value);
    _getCommentList(id.value);
};
const emits = defineEmits(["empty"]);
onMounted(() => {});
const _getComment = async (id: any) => {
    // D1146：关闭时不发请求（后端也会拦，这里避免无意义往返）
    if (!commentEnabled.value) return;
    loading.value = true;
    try {
        const result = await getComment(id);
        comment.value = result;
    } catch (error) {
    } finally {
        loading.value = false;
    }
};
// const _getCommentList = async (id: any) => {
//     loading.value = true;
//     try {
//         const result = await getCommentList(id, {
//             type: commentIndex.value,
//             page: page.value
//         });
//         if (props.useToShow) {
//             commentList.value = result.records;
//         } else {
//             commentList.value = result.records.slice(0, 2);
//         }
//         // if (result.total == 0) {
//         emits("empty", result.total);
//         // }
//         total.value = result.total;
//     } catch (error) {
//     } finally {
//         loading.value = false;
//     }
// };
const commentListBox = ref<HTMLElement | null>(null);
const isLoadingMore = ref(false);
// 滚动监听
const handleScroll = () => {
    if (!commentListBox.value) return;
    const el = commentListBox.value;
    if (el.scrollTop + el.clientHeight >= el.scrollHeight - 10) {
        loadMore();
    }
};

onMounted(() => {
    nextTick(() => {
        if (commentListBox.value && props.useToShow) {
            commentListBox.value.addEventListener("scroll", handleScroll);
        }
    });
});
onUnmounted(() => {
    if (commentListBox.value && props.useToShow) {
        commentListBox.value.removeEventListener("scroll", handleScroll);
    }
});

// ...existing code...
const _getCommentList = async (id: any) => {
    // D1146：关闭时不发请求（后端也会拦，这里避免无意义往返）
    if (!commentEnabled.value) return;
    loading.value = true;
    try {
        const result = await getCommentList(id, {
            type: commentIndex.value,
            page: page.value,
            size: size.value
        });
        if (props.useToShow) {
            if (page.value === 1) {
                commentList.value = result.records;
            } else {
                commentList.value = commentList.value.concat(result.records);
            }
        } else {
            commentList.value = result.records.slice(0, 2);
        }
        emits("empty", result.total);
        total.value = result.total;
    } catch (error) {
    } finally {
        loading.value = false;
    }
};
// ...existing code...

// 修改pageChange方法，分页时重置滚动条
const pageChange = () => {
    page.value = 1;
    _getCommentList(id.value);
    nextTick(() => {
        if (commentListBox.value) commentListBox.value.scrollTop = 0;
    });
};
_getComment(id.value);
_getCommentList(id.value);
// const pageChange = () => {
//     _getCommentList(id.value);
// };
</script>

<style lang="less" scoped>
.comment-option {
    background: #f8f9f9;
    height: 40px;
    margin-bottom: 10px;
    padding: 0;

    .comment-option-sel {
        color: #ffffff;
        list-style: none outside none;

        li {
            background: #f8f9f9;
            cursor: pointer;
            height: 38px;
            line-height: 38px;
            margin: 0;
            text-align: center;
            min-width: 100px;

            a {
                color: #6e6e6e;
                display: block;
                font-weight: 400;
                display: block;
                height: 40px;
                margin-left: 1px;
                padding: 0 10px;
            }
        }

        .o-over {
            a {
                background-color: #ffffff;
                background-image: none;
                border-bottom: 0 none;
                border-top: 2px solid var(--general);
                border-bottom: 1px solid #fff;
                line-height: 34px;
                height: 38px;
            }
        }

        .last {
            cursor: default;
            padding-right: 20px;
            text-align: right;
            width: 384px;
        }

        a:hover {
            text-decoration: none;
        }
    }
}

.pagination {
    padding: 10px 0;
    display: flex;
    justify-content: right;
}
.empty {
    text-align: center;
    padding: 70px 0;
}
.compare-view-box {
    background-color: #fdfdfd;
    border: 5px solid #f1f1f1;
    min-height: 90px;
    margin-bottom: 10px;
    overflow: hidden;
    padding: 10px;

    .compare-view {
        height: 100%;

        .centage-praise {
            border-right: 1px solid #d5d5d5;
            padding-right: 10px;
            text-align: center;
            width: 20%;

            span {
                display: block;
                text-align: center;
            }

            .s-centage {
                font-size: 36px;
                text-align: center;
                color: var(--general);
            }

            .s-praise {
                font-family: 微软雅黑, 微软雅黑;
                font-size: 14px;
                text-align: center;
                color: rgb(136, 136, 136);
                padding: 2px;
            }
        }

        .centage-all {
            border-right: 1px solid #d5d5d5;
            width: 50%;
            text-align: center;
            display: flex;
            justify-content: center;

            .centage-b {
                height: 20px;
                line-height: 20px;
                align-items: center;
                .centageText {
                    width: 119px;
                    // text-align: right;
                }
                .centage-enough {
                    background: #eee;
                    display: inline-block;
                    height: 12px;
                    line-height: 12px;
                    overflow: hidden;
                    padding: 0;
                    vertical-align: middle;
                    width: 200px;
                    text-align: left;

                    .centage-show {
                        background: #ff4040;
                        display: inline-block;
                        height: 12px;
                        line-height: 12px;
                    }
                }

                .centage-num {
                    padding-left: 8px;
                    color: #5f5f5f;
                    display: inline-block;
                    text-align: left;
                    width: 50px;
                }
            }
        }

        .comment-go {
            border-right: 0 none;
            padding-left: 0;
            width: 26%;
            text-align: center;

            .go-comm-txt {
                padding: 5px 5px 5px 30px;
                text-align: left;
                line-height: 30px;

                .btn-css3 {
                    background: #ff6c6c;
                    border: 1px solid #ff5e5e;
                    color: #fff;
                    cursor: pointer;
                    border-radius: 2px;
                    display: inline-block;
                    height: 16px;
                    line-height: 16px;
                    padding: 5px 12px;
                    text-align: center;
                    text-decoration: none;
                }
            }
        }
    }
}

.comm_Wrap {
    border-bottom: 1px solid #e8e8e8;
    padding: 15px 10px 7px;
    .eachInfo {
        .head {
            width: 77px;
            text-align: center;
            padding-right: 42px;

            img {
                width: 60px;
                height: 60px;
            }

            .name {
                color: #888;
            }
        }

        .info {
            width: 100%;

            .starLi {
                margin-bottom: 10px;

                .fl {
                    .tit {
                        color: #888;
                        width: 60px;
                    }

                    .msg {
                        width: 520px;
                    }

                    .pic-list {
                        flex: 1;

                        .pic {
                            display: inline-block;
                            border: 1px solid #eee;
                            margin-right: 8px;
                            margin-bottom: 8px;
                            .image {
                                width: 100px;
                                height: 100px;
                                transition: opacity 0.2s linear;
                            }
                        }
                    }
                }

                .fr {
                    color: #888;

                    .time {
                        margin-right: 10px;
                    }

                    .btn {
                        background: #cfcfcf;
                        display: inline-block;
                        border-radius: 1px;
                        color: #fff;
                        margin-left: 10px;
                        padding: 0 15px;
                        cursor: pointer;
                    }
                }
            }

            .g-comm-tag {
                span {
                    display: inline-block;
                    background: #eee;
                    padding: 3px 6px;
                    margin-right: 4px;
                    margin-bottom: 3px;
                    color: #888888;
                    cursor: pointer;
                }
            }

            .reply {
                margin: 5px 0;
                padding: 10px;
                background: #f8f8f8;
                border-radius: 4px;
                clear: both;
                color: #666;
                line-height: 1.75;

                .name {
                    color: var(--general);
                }

                .text {
                    color: #333;
                    display: block;
                    margin: 4px 0;
                }

                .time {
                    color: #999;
                    font-size: 12px;
                }
            }
        }
    }
}
.comnWid {
    height: 496px;
    overflow-y: auto;
}
</style>
