<!-- 商品详情相关文章。 -->
<template>
    <div class="product-article">
        <div class="item" v-for="item in articleList">
            <NuxtLink target="_blank" :to="`/article/news/list?articleId=${item.articleId}`">
                <a class="ellipsis2">{{ item.articleTitle }}</a>
            </NuxtLink>
        </div>
        <div class="empty" v-if="articleList.length == 0 && !loading">「{{ $t("暂无文章") }}」</div>
    </div>
</template>
<script lang="ts" setup>
import { ref, reactive } from "vue";
import { message } from "ant-design-vue";
import { getProductRelateArticle } from "~/api/product/category";

const props = defineProps({
    productId: {
        type: Number,
        default: 0
    }
});
const filterParams = reactive({
    productId: props.productId,
    intro: "hot"
});
const loading = ref(false);
const articleList = ref<any[]>([]);

const _getProductRelateArticle = async () => {
    loading.value = true;
    try {
        const result: any = await getProductRelateArticle({ ...filterParams });
        articleList.value = result;
    } catch (error: any) {
        message.error(error?.message);
    } finally {
        loading.value = false;
    }
};
_getProductRelateArticle();
</script>

<style lang="less" scoped>
.product-article {
    .item {
        margin: 10px 0;
        a {
            display: block;
            line-height: 17px;
            overflow: hidden;
            height: 16px;
            text-indent: 10px;
        }
    }
    .empty {
        text-align: center;
        padding: 70px 0;
    }
}
</style>