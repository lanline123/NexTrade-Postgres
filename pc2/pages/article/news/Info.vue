<template>
    <div>
        <template v-if="pageType === 'list' || pageType === 'none'">
            <div v-if="!loading" class="article-list">
                <template v-if="formListState && formListState.length > 0">
                    <div v-for="item in formListState" class="col">
                        <div v-if="item.articleThumb">
                            <NuxtLink :to="'/article/news/list?articleId=' + item.articleId">
                                <el-image
                                    :src="imageFormat(item.articleThumb)"
                                    fit="cover"
                                    loading="lazy"
                                    style="transition: opacity 0.2s linear; width: 120px; height: 80px"
                                    :alt="item.articleTitle || ''"
                                ></el-image>
                            </NuxtLink>
                        </div>
                        <div class="right">
                            <div class="col1">
                                <div v-if="item.articleTag" class="tips">{{ item.articleTag }}</div>
                                <div class="col-title">
                                    <NuxtLink :to="'/article/news/list?articleId=' + item.articleId">
                                        {{ item.articleTitle }}
                                    </NuxtLink>
                                </div>
                            </div>
                            <div class="col2">
                                {{ item.description }}
                            </div>
                            <div class="col3">
                                <div v-if="item.addTime" class="time-div">{{ item.addTime }}</div>
                                <NuxtLink v-if="item.link" :to="item.link" class="link-div">{{ $t("相关链接") }}</NuxtLink>
                            </div>
                        </div>
                    </div>
                    <div class="el-page">
                        <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="fetchArticle" />
                    </div>
                </template>
                <template v-else>
                    <el-empty :description="$t('没有内容')" />
                </template>
            </div>
            <div v-else class="loading-data">
                <div v-for="item in 3" class="loading-bottom">
                    <el-skeleton :rows="2" animated />
                </div>
            </div>
        </template>
        <template v-else-if="pageType === 'article'">
            <div v-if="!loading" class="article-info">
                <div class="text-content">
                    <div class="title">{{ formArticleState.articleTitle }}</div>
                    <div class="time">{{ $t("时间") }}：{{ formArticleState.addTime }}</div>
                    <div v-if="formArticleState.content" class="article_content">
                        <el-image
                            v-if="formArticleState.articleThumb"
                            :src="imageFormat(formArticleState.articleThumb)"
                            fit="cover"
                            loading="lazy"
                            style="transition: opacity 0.2s linear; width: 300px"
                            :alt="formArticleState.articleTitle || ''"
                        />
                        <div class="v-html" v-html="formArticleState.content"></div>
                        <template v-if="productIds && productIds.length">
                            <productItem :product-ids="productIds" />
                        </template>
                        <!-- <img src="https://doc.jiangsu.gov.cn/picture/0/936292b823f44791a95cad8ce00c2452.jpg" alt=""> -->
                    </div>
                    <div v-else class="no-content-data">
                        <el-empty :description="$t('没有内容')" />
                    </div>
                </div>
                <div class="navigation">
                    <div class="navigation-btn navigation-btn-left">
                        <NuxtLink v-if="formArticleState.prev" :to="'/article/news/list?articleId=' + formArticleState.prev.articleId"
                            >«{{ $t("上一篇") }}：
                            <div class="text">{{ formArticleState.prev.articleTitle }}</div>
                        </NuxtLink>
                    </div>
                    <div class="navigation-btn navigation-btn-right">
                        <NuxtLink v-if="formArticleState.next?.articleId" :to="'/article/news/list?articleId=' + formArticleState.next.articleId"
                            >{{ $t("下一篇") }}：
                            <div class="text">{{ formArticleState.next.articleTitle }}</div>
                            »
                        </NuxtLink>
                    </div>
                </div>
            </div>
            <div v-else class="loading-data">
                <el-skeleton :rows="5" animated />
            </div>
        </template>
    </div>
</template>
<script lang="ts" setup>
/**
 * 资讯文章详情/列表组件
 * - 根据 URL 参数自动切换显示模式：文章详情(articleId)、文章列表(categoryId)、默认列表(none)
 * - 文章详情模式：显示文章标题、时间、内容、关联商品、上下篇导航
 * - 文章列表模式：显示带缩略图的文章卡片列表，支持分页
 * - 通过 emit 向父组件传递文章名称和关联商品数据
 */
import { useRouter } from "vue-router";
import { reactive, ref } from "vue";
import type { ArticleFilterParams, ArticleFilterState, ArticleFormState } from "~/types/article/article.d";
import { getArticle, getArticleList } from "~/api/article/article";
import { buildSeoKeywords, truncateSeoText } from "@/utils/seo";
import { message } from "ant-design-vue";

import { Pagination } from "~/components/list";
import productItem from "../src/productItem.vue";

const router = useRouter();
const pageType = computed(() => {
    if (router.currentRoute.value.query.hasOwnProperty("articleId")) {
        return "article";
    } else if (router.currentRoute.value.query.hasOwnProperty("categoryId")) {
        return "list";
    } else {
        return "none";
    }
});
const props = defineProps({
    articleName: {
        type: String,
        default: ""
    },
    productList: {
        type: Array,
        default: []
    }
});

const formArticleState = ref<ArticleFormState>({});
const formListState = ref<ArticleFilterState[]>();
const productIds = ref([]);

const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<ArticleFilterParams>({
    //初使化用于查询的参数
    page: 1,
    size: 10,
    categorySn: ""
});
const emit = defineEmits(["update:articleName", "update:productList", "update:seoMeta"]);
const fetchArticle = async () => {
    try {
        loading.value = true;
        emit("update:articleName", "");
        emit("update:productList", []);
        emit("update:seoMeta", {
            description: "",
            keywords: "",
            image: ""
        });
        if (pageType.value === "article") {
            const result: any = await getArticle({ id: router.currentRoute.value.query.articleId }, "newsInfo");
            productIds.value = (result.item as any).productIds || [];
            Object.assign(formArticleState.value, result.item);
            formArticleState.value.prev = result.prev;
            formArticleState.value.next = result.next;
            emit("update:articleName", formArticleState.value.articleTitle);
            emit("update:productList", (result as any).productList || []);
            emit("update:seoMeta", {
                description: result.item.description || truncateSeoText(result.item.content, 160),
                keywords: result.item.keywords || buildSeoKeywords([result.item.articleTitle]),
                image: result.item.articleThumb || ""
            });
        } else if (pageType.value === "list") {
            let temp = router.currentRoute.value.query.categoryId;
            typeof temp === "string" ? (filterParams.categorySn = temp) : "";
            const result = await getArticleList({ ...filterParams });
            formListState.value = result.records;
            total.value = result.total;
        } else if (pageType.value === "none") {
            filterParams.categorySn = "zxzx";
            const result = await getArticleList({ ...filterParams });
            formListState.value = result.records;
            total.value = result.total;
        }
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

watch(
    () => router.currentRoute.value.query,
    (val) => {
        fetchArticle();
    },
    { deep: true, immediate: true }
);
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.article-list {
    display: flex;
    flex-direction: column;
    width: 100%;

    .col {
        border-bottom: 1px solid #e6e6e6;
        padding: 10px;
        display: flex;
        gap: 16px;

        .right {
            display: flex;
            flex-direction: column;
            gap: 10px;

            .col1 {
                display: flex;
                gap: 10px;
                align-items: center;

                .tips {
                    background: var(--tag-bg) none repeat scroll 0 0;
                    color: var(--tag-text);
                    font-size: 12px;
                    font-weight: 400;
                    height: 20px;
                    line-height: 20px;
                    display: inline-block;
                    padding: 0 8px;
                    box-sizing: border-box;
                }

                .col-title {
                    color: #333;
                    width: 500px;
                    font-size: 16px;
                    font-weight: bold;
                    overflow: hidden;
                    display: block;
                    text-overflow: ellipsis;
                    white-space: nowrap;
                }
            }

            .col2 {
                line-height: 1.5;
            }

            .col3 {
                display: flex;
                align-items: center;

                .time-div {
                    border: 1px solid #e3e3e3;
                    border-radius: 5px;
                    float: left;
                    height: 23px;
                    line-height: 23px;
                    margin-right: 19px;
                    padding: 0 10px;
                    color: #9b9b9b;
                }

                .link-div {
                    border-radius: 5px;
                    display: inline-block;
                    width: 80px;
                    height: 23px;
                    background: #e03737;
                    line-height: 23px;
                    text-align: center;
                    color: #fff;
                }
            }
        }
    }
}

.article-info {
    // display: flex;
    // flex-direction: column;
    // flex: 1;

    .text-content {
        flex: 1;

        .title {
            font-size: 20px;
            font-weight: bold;
            display: flex;
            flex-wrap: wrap;
        }

        .time {
            margin: 10px 0;
            color: #888888;
        }

        .article_content {
            .v-html {
                height: 100%;
                overflow: hidden;
                display: flex;
                flex-direction: column;
                :deep(p) {
                    font-size: 14px;
                    line-height: 2;
                    max-width: 100%;
                    width: 100%;
                    height: auto;
                }
                :deep(img) {
                    max-width: 100%;
                    width: 100%;
                    display: block;
                    height: auto;
                }
            }
        }

        .no-content-data {
            display: flex;
            align-items: center;
            justify-content: center;
            color: #999999;
            font-size: 20px;
            font-weight: normal;
        }
    }

    .navigation {
        margin: 20px 0;
        display: flex;
        justify-content: space-between;
        align-items: center;

        .navigation-btn {
            cursor: pointer;
            flex: 1;

            :deep(a) {
                color: #333333;
                display: flex;
                justify-content: center;
                align-items: center;
            }

            .text {
                white-space: nowrap;
                max-width: 80%;
                overflow: hidden;
                text-overflow: ellipsis;
            }
        }

        .navigation-btn-left {
            display: flex;
        }

        .navigation-btn-right {
            display: flex;
            justify-content: flex-end;
        }
    }
}

.loading-data {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    color: #999999;
    font-size: 20px;
    font-weight: normal;
    .loading-bottom {
        border-bottom: 1px solid #e6e6e6;
        width: 100%;
        padding: 10px;
    }
}
</style>
