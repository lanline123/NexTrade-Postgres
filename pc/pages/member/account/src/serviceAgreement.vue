<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader :title="articleTitle"></CommonHeader>
    <CommonPageContainer flex v-loading="loading">
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <template v-if="!loading">
            <div class="article">
                <div class="title">{{ formArticleState.articleTitle }}</div>
                <div class="text-content">
                    <div v-if="formArticleState.content" class="article_content">
                        <el-image
                            v-if="formArticleState.articleThumb"
                            :src="imageFormat(formArticleState.articleThumb)"
                            fit="cover"
                            loading="lazy"
                            style="transition: opacity 0.2s linear; width: 300px"
                        />
                        <div class="v-html" v-html="formArticleState.content"></div>
                    </div>
                    <div v-else class="no-content-data">
                        <el-empty :description="$t('没有内容')" />
                    </div>
                </div>
            </div>
        </template>
        <div v-else class="loading-data">
            <LoadingOutlined />
        </div>
    </CommonPageContainer>

    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * @description 服务协议页面
 * - 展示平台的服务协议内容
 * - 通过文章SN获取协议文章内容
 */
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import { ref } from "vue";
import type { ArticleFormState } from "~/types/article/article";
import { getArticle } from "~/api/article/article";
import { message } from "ant-design-vue";
import { LoadingOutlined } from "@ant-design/icons-vue";
const loading = ref<boolean>(true);
const formArticleState = ref<ArticleFormState>({});
const articleTitle = ref<any>("");


const fetchArticle = async () => {
    try {
        loading.value = true;
        const result = await getArticle({ articleSn: 'fwxy' }, "issueInfo");
        articleTitle.value = result?.item?.articleTitle;
        Object.assign(formArticleState.value, result?.item);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

fetchArticle()
</script>
<style lang="less" scoped>
.container{
    margin-top: 32px;
}

.article {
    background-color: #ffffff;
    flex: 1;
    padding: 20px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;

    .title {
        font-size: 32px;
        font-weight: 700;
        color: #000;
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
            color: #000;

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
            color: #000;
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
</style>
