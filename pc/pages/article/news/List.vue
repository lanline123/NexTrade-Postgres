<template>
    <CommonHeader :title="articleName ? articleName : $t('资讯中心')"></CommonHeader>
    <CommonPageHeader></CommonPageHeader>
    <CommonPageContainer class="news-content">
        <div class="breadcrumb-menu">
            <el-icon color="#b9b9b9">
                <HomeFilled />
            </el-icon>
            <el-breadcrumb :separator-icon="ArrowRight">
                <el-breadcrumb-item :to="{ path: '/' }">{{ $t("首页") }}</el-breadcrumb-item>
                <el-breadcrumb-item :to="{ path: '/article/news/list' }">{{ $t("资讯中心") }}</el-breadcrumb-item>
                <el-breadcrumb-item v-if="articleName"
                    ><span class="breadcrumb-title">{{ articleName }}</span></el-breadcrumb-item
                >
            </el-breadcrumb>
        </div>
        <div class="content">
            <div class="box left">
                <Info v-model:articleName="articleName" v-model:productList="productList" v-model:seoMeta="seoMeta"></Info>
            </div>
            <div class="box right">
                <div class="title">{{ $t("文章分类") }}</div>
                <ul>
                    <li>
                        <div class="min-title">
                            <NuxtLink to="/article/issue/index">
                                <div class="orange">- {{ $t("帮助中心") }}</div>
                            </NuxtLink>
                        </div>
                    </li>
                    <li>
                        <div class="min-title">
                            <NuxtLink :to="'/article/news/list?categoryId=zxzx'">
                                <div class="orange">- {{ $t("资讯中心") }}</div>
                            </NuxtLink>
                            <div :class="xwzx ? 'left_ma' : ''" class="left_more" @click="xwzx = !xwzx"></div>
                        </div>
                        <div v-if="xwzx" class="min-list">
                            <ul>
                                <li v-for="item in filterzxzxState">
                                    <NuxtLink :to="'/article/news/list?articleId=' + item.articleId">
                                        <div class="orange">{{ item.articleTitle }}</div>
                                    </NuxtLink>
                                </li>
                            </ul>
                        </div>
                    </li>
                    <li>
                        <div class="min-title">
                            <NuxtLink :to="'/article/news/list?categoryId=bzgg'">
                                <div class="orange">- {{ $t("本站公告") }}</div>
                            </NuxtLink>
                            <div :class="zxgg ? 'left_ma' : ''" class="left_more" @click="zxgg = !zxgg"></div>
                        </div>
                        <div v-if="zxgg" class="min-list">
                            <ul>
                                <li v-for="item in filterzxggState">
                                    <NuxtLink :to="'/article/news/list?articleId=' + item.articleId">
                                        <div class="orange">{{ item.articleTitle }}</div>
                                    </NuxtLink>
                                </li>
                            </ul>
                        </div>
                    </li>
                    <li>
                        <div class="min-title">
                            <NuxtLink :to="'/article/news/list?categoryId=kfzx'">
                                <div class="orange">- {{ $t("客服中心") }}</div>
                            </NuxtLink>
                            <div :class="kfzx ? 'left_ma' : ''" class="left_more" @click="kfzx = !kfzx"></div>
                        </div>
                        <div v-if="kfzx" class="min-list">
                            <ul>
                                <li v-for="item in filterkfzxState">
                                    <NuxtLink :to="'/article/issue/info?articleId=' + item.articleId">
                                        <div class="orange">{{ item.articleTitle }}</div>
                                    </NuxtLink>
                                </li>
                            </ul>
                        </div>
                    </li>
                </ul>
                <template v-if="productList && productList.length > 0">
                    <div class="title">{{ $t("相关商品") }}</div>
                    <div class="product-list">
                        <div v-for="item in productList" class="product-col">
                            <NuxtLink
                                :title="item.productName"
                                :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                class="nuxt"
                                target="_blank"
                            >
                                <el-image
                                    :src="imageFormat(item.picThumb)"
                                    class="pro_pic"
                                    loading="lazy"
                                    style="transition: opacity 0.2s linear; width: 100px" :alt="item.productName || ''" 
                                />
                                <div>
                                    <FormatPrice v-model="item.productPrice" :fontStyle="{ fontSize: '20px', color: 'var(--general)' }"></FormatPrice>
                                </div>
                                <div>{{ item.productName }}</div>
                            </NuxtLink>
                        </div>
                    </div>
                </template>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 资讯中心页面
 * - 展示文章分类导航（帮助中心、资讯中心、本站公告、客服中心）
 * - 各分类支持展开/折叠显示文章列表
 * - 右侧显示关联商品列表
 * - 左侧内容区域通过 Info 组件展示文章详情或列表
 */
import { ArrowRight } from "@element-plus/icons-vue";
import Info from "~/pages/article/news/Info.vue";
import { reactive, ref } from "vue";
import type { ArticleFilterParams, ArticleFilterState } from "~/types/article/article.d";
import { getArticleList } from "~/api/article/article";
import type { ProductItem } from "~/types/product/product";
import { buildSeoKeywords } from "@/utils/seo";

const productList = ref<ProductItem[]>([]);

const articleName = ref<string>("");
const seoMeta = ref({
    description: "",
    keywords: "",
    image: ""
});
const route = useRoute();
const canonical = computed(() => {
    if (route.query.articleId) {
        return `/article/news/list?articleId=${String(route.query.articleId)}`;
    }
    if (route.query.categoryId) {
        return `/article/news/list?categoryId=${String(route.query.categoryId)}`;
    }
    return "/article/news/list";
});
const { t } = useI18n();
const currentCategoryLabel = computed(() => {
    const categoryId = String(route.query.categoryId || "");
    if (categoryId === "bzgg") return t("本站公告");
    if (categoryId === "kfzx") return t("客服中心");
    return t("资讯中心");
});
useSeo({
    title: computed(() => articleName.value || currentCategoryLabel.value),
    description: computed(() => seoMeta.value.description || t("浏览{0}文章", [currentCategoryLabel.value])),
    keywords: computed(() => seoMeta.value.keywords || buildSeoKeywords([articleName.value, currentCategoryLabel.value])),
    image: computed(() => seoMeta.value.image || ""),
    ogType: computed(() => (route.query.articleId ? "article" : "website")),
    canonical
});

const xwzx = ref(true);
const zxgg = ref(true);
const kfzx = ref(true);

const filterzxzxState = ref(<ArticleFilterState[]>[]);
const filterzxggState = ref(<ArticleFilterState[]>[]);
const filterkfzxState = ref(<ArticleFilterState[]>[]);
const loading = ref<boolean>(true);
const filterParams = reactive<ArticleFilterParams>({
    //初使化用于查询的参数
    page: 1,
    size: 10,
    categorySn: ""
});

const loadFilter = async (value: string) => {
    loading.value = true;
    try {
        filterParams.categorySn = value;
        const result = await getArticleList({ ...filterParams });
        if (value === "zxzx") {
            filterzxzxState.value = result.records;
        } else if (value === "bzgg") {
            filterzxggState.value = result.records;
        } else if (value === "kfzx") {
            filterkfzxState.value = result.records;
        }
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
loadFilter("zxzx");
loadFilter("bzgg");
loadFilter("kfzx");
</script>
<style lang="less" scoped>
.news-content {
    padding-top: 20px;
    padding-bottom: 20px;

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
            background-color: #ffffff;
            padding: 30px;
            box-sizing: border-box;
            height: fit-content;
        }

        .left {
            flex: 1;
        }

        .right {
            width: 300px;

            .title {
                color: #333;
                display: block;
                font-size: 14px;
                font-weight: 700;
                height: 24px;
                line-height: 24px;
            }

            ul li {
                cursor: pointer;
                list-style: none;

                line-height: 30px;
                color: #888888;
                font-size: 14px;
                display: flex;
                flex-direction: column;

                .min-title {
                    display: flex;
                    width: 100%;
                    align-items: center;
                    justify-content: space-between;

                    :deep(a) {
                        color: #888888;
                    }
                }

                .min-list {
                    margin-left: 10px;

                    .orange {
                        color: #bbbbbb;
                        font-size: 12px;
                        white-space: nowrap;
                        overflow: hidden;
                        text-overflow: ellipsis;
                        display: inline-block;
                        width: 214px;
                        line-height: 2;
                    }
                }

                .left_more {
                    background: rgba(0, 0, 0, 0) url("/assets/images/index/click.jpg") no-repeat scroll 15px -17px;
                    cursor: pointer;
                    height: 30px;
                    width: 30px;
                }

                .left_ma {
                    background: rgba(0, 0, 0, 0) url("/assets/images/index/click.jpg") no-repeat scroll 15px 11px;
                }
            }
            .orange:hover {
                color: var(--general) !important;
            }

            .product-list {
                margin-top: 10px;
                display: flex;
                flex-direction: column;
                gap: 10px;

                .product-col {
                    .nuxt {
                        display: flex;
                        flex-direction: column;
                        justify-content: center;
                        align-items: center;
                        gap: 8px;

                        .text {
                            width: 100px;
                            display: inline-block;
                            white-space: nowrap;
                            overflow: hidden;
                            text-overflow: ellipsis;
                        }
                    }
                }
            }
        }
    }
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
