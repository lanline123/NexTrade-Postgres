<template>
    <div style="background-color: #f6f6f6">
        <CommonHeader :title="articleName ? articleName : $t('帮助中心')"></CommonHeader>
        <CommonPageHeader></CommonPageHeader>
        <CommonPageContainer class="news-content">
            <div class="breadcrumb-menu">
                <el-icon color="#b9b9b9">
                    <HomeFilled />
                </el-icon>
                <el-breadcrumb :separator-icon="ArrowRight">
                    <el-breadcrumb-item :to="{ path: '/' }">{{ $t("首页") }}</el-breadcrumb-item>
                    <el-breadcrumb-item :to="{ path: '/article/issue/index' }">{{ $t("帮助中心") }}</el-breadcrumb-item>
                    <el-breadcrumb-item v-if="articleName"
                        ><span class="breadcrumb-title">{{ articleName }}</span></el-breadcrumb-item
                    >
                </el-breadcrumb>
            </div>
            <div class="content">
                <div class="box left">
                    <IssueLeftMenu v-model:leftMenuList="leftMenuList"></IssueLeftMenu>
                </div>
                <div class="box right">
                    <template v-if="!loading">
                        <template v-if="pageType === 'article'">
                            <div class="title">{{ formArticleState.articleTitle }}</div>
                            <div class="text-content">
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
                                </div>
                                <div v-else class="no-content-data">
                                    <el-empty :description="$t('没有内容')" />
                                </div>
                            </div>
                            <div class="navigation">
                                <div class="navigation-btn navigation-btn-left">
                                    <NuxtLink v-if="formArticleState.prev?.articleId" :to="'/article/issue/info?articleId=' + formArticleState.prev.articleId"
                                        >«{{ $t("上一篇") }}：
                                        <div class="text">{{ formArticleState.prev.articleTitle }}</div>
                                    </NuxtLink>
                                </div>
                                <div class="navigation-btn navigation-btn-right">
                                    <NuxtLink
                                        v-if="formArticleState.next?.articleId"
                                        :to="'/article/issue/info?articleId=' + formArticleState.next.articleId"
                                        >{{ $t("下一篇") }}：
                                        <div class="text">{{ formArticleState.next.articleTitle }}</div>
                                        »
                                    </NuxtLink>
                                </div>
                            </div>
                        </template>
                        <template v-else>
                            <div class="article-list">
                                <NuxtLink :to="'/article/issue/info?articleId=' + item.articleId" class="article-list-item" v-for="item in formListState">{{
                                    item.articleTitle
                                }}</NuxtLink>
                            </div>
                            <div class="el-page">
                                <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="fetchArticle" />
                            </div>
                        </template>
                    </template>
                    <div v-else class="loading-data">
                        <!-- <el-skeleton :rows="5" animated /> -->
                        <LoadingOutlined />
                    </div>
                </div>
            </div>
        </CommonPageContainer>
        <CommonPageFooter></CommonPageFooter>
    </div>
</template>
<script lang="ts" setup>
/**
 * 帮助文章详情/列表页
 * - 根据 URL 参数自动判断展示类型：文章详情(articleId) 或 文章列表(categoryId)
 * - 文章详情页支持上一篇/下一篇导航、关联商品展示
 * - 文章列表页支持分页浏览
 * - 监听路由变化自动刷新内容
 */
import { ArrowRight } from "@element-plus/icons-vue";
import { reactive, ref } from "vue";
import IssueLeftMenu from "~/pages/article/issue/src/IssueLeftMenu.vue";
import { LoadingOutlined } from '@ant-design/icons-vue';
import type { CategoryFilterState } from "~/types/article/category";
import type { ArticleFilterParams, ArticleFilterState, ArticleFormState } from "~/types/article/article";
import { getArticle, getArticleList } from "~/api/article/article";
import { buildSeoKeywords, truncateSeoText } from "@/utils/seo";
import { message } from "ant-design-vue";
import { useRouter } from "vue-router";
import { Pagination } from "~/components/list";
import productItem from "../src/productItem.vue";

const router = useRouter();
const leftMenuList = ref<CategoryFilterState[]>([]);
const formArticleState = ref<ArticleFormState>({});
const loading = ref<boolean>(true);
const articleName = ref<string>("");
const formListState = ref<ArticleFilterState[]>();
const total = ref<number>(0);
const productIds = ref([]);
const filterParams = reactive<ArticleFilterParams>({
    //初使化用于查询的参数
    page: 1,
    categorySn: ""
});
const pageType = computed(() => {
    if (router.currentRoute.value.query.hasOwnProperty("articleId") || router.currentRoute.value.query.hasOwnProperty("articleSn")) {
        return "article";
    } else if (router.currentRoute.value.query.hasOwnProperty("categoryId")) {
        return "list";
    } else {
        return "none";
    }
});
const canonical = computed(() => {
    const currentQuery = router.currentRoute.value.query;
    const articleSn = currentQuery.articleSn;
    const articleId = currentQuery.articleId;
    const categoryId = currentQuery.categoryId;

    if (articleSn) {
        return `/article/issue/info?articleSn=${String(articleSn)}`;
    }
    if (articleId) {
        return `/article/issue/info?articleId=${String(articleId)}`;
    }
    if (categoryId) {
        return `/article/issue/info?categoryId=${String(categoryId)}`;
    }
    return "/article/issue/index";
});
const { t } = useI18n();
const seoDescription = computed(() => {
    if (pageType.value !== "article") {
        return filterParams.categorySn ? t("浏览{0}相关文章", [filterParams.categorySn]) : t("帮助中心");
    }

    return formArticleState.value.description || truncateSeoText(formArticleState.value.content, 160);
});
const seoKeywords = computed(() => {
    if (pageType.value !== "article") {
        return buildSeoKeywords([t("帮助中心"), filterParams.categorySn]);
    }

    return formArticleState.value.keywords || buildSeoKeywords([formArticleState.value.articleTitle, filterParams.categorySn]);
});
useSeo({
    title: computed(() => formArticleState.value?.articleTitle || t("帮助中心")),
    description: seoDescription,
    keywords: seoKeywords,
    image: computed(() => formArticleState.value?.articleThumb || ""),
    ogType: computed(() => (pageType.value === "article" ? "article" : "website")),
    canonical
});

const fetchArticle = async () => {
    try {
        loading.value = true;
        if (pageType.value === "article") {
            let temp;
            if (router.currentRoute.value.query.hasOwnProperty("articleSn")) {
                temp = {
                    articleSn: router.currentRoute.value.query.articleSn
                };
            } else {
                temp = {
                    id: router.currentRoute.value.query.articleId
                };
            }
            const result: any = await getArticle(temp, "issueInfo");
            productIds.value = (result.item as any).productIds || [];
            Object.assign(formArticleState.value, result?.item);
            formArticleState.value.prev = result.prev;
            formArticleState.value.next = result.next;
            articleName.value = String(formArticleState.value.articleTitle);
        } else if (pageType.value === "list" || pageType.value === "none") {
            let temp = router.currentRoute.value.query.categoryId;
            typeof temp === "string" ? (filterParams.categorySn = temp) : "";
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
    () => {
        fetchArticle();
    },
    { deep: true, immediate: true }
);
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";
.news-content {
    padding-top: 20px;
    padding-bottom: 20px;
    background-color: #f6f6f6;

    .breadcrumb-menu {
        display: flex;
        gap: 5px;

        .breadcrumb-title {
            display: inline-block;
            white-space: nowrap;
            width: 300px;
            overflow: hidden;
            text-overflow: ellipsis;
        }
    }

    .content {
        display: flex;
        margin-top: 20px;
        gap: 20px;

        .box {
            box-sizing: border-box;
        }

        .left {
            width: 200px;
        }

        .right {
            background-color: #ffffff;
            flex: 1;
            padding: 20px;
            box-sizing: border-box;
            display: flex;
            flex-direction: column;

            .title {
                font-size: 20px;
                font-weight: bold;
                height: 40px;
                line-height: 40px;
                margin-bottom: 20px;
            }

            .text-content {
                display: flex;
                flex: 1;
                flex-direction: column;

                .article_content {
                    display: flex;
                    gap: 10px;
                    flex-wrap: wrap;
                    flex-direction: column;

                    .v-html {
                        :deep(p) {
                            font-size: 14px;
                            line-height: 2;
                        }
                        :deep(img) {
                            max-width: 100%;
                            width: 100%;
                        }
                    }
                }

                .no-content-data {
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    height: 100%;
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
            .article-list {
                display: flex;
                flex-direction: column;
                gap: 10px;
                color: #333333;
                font-weight: bold;
                font-size: 14px;
                .article-list-item {
                    height: 30px;
                    line-height: 30px;
                    border-bottom: 1px dashed #dadada; /* 设置为1px宽度的黑色虚线边框 */
                }
            }
        }
    }

    .help_division {
        border: none;
        border-top: 1px dashed #e5e5e5; /* 设置虚线样式，可以根据需要调整颜色和粗细 */
    }
}

.loading-data {
    display: flex;
    justify-content: center;
    align-items: center;
    width: 100%;
    height: 100%;
    color: #999999;
    font-size: 20px;
    font-weight: normal;
}
:deep(.el-breadcrumb) {
    font-size: 12px;
    .el-breadcrumb__inner a,
    .el-breadcrumb__inner.is-link {
        font-weight: normal;
        color: #606266;
    }
}
</style>
