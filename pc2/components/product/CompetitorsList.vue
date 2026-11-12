<template>
    <div>
        <!-- 相关分类 -->
        <ProductCompetitorsItem class="mb10" :title="$t('相关分类')" v-if="cateInfo.length" :list="cateInfo" :type="'cate'"></ProductCompetitorsItem>
        <!-- 同类其他品牌 -->
        <ProductCompetitorsItem class="mb10" :title="$t('同类其他品牌')" v-if="brandInfo.length" :list="brandInfo" :type="'brand'"></ProductCompetitorsItem>
        <!-- 相关文章 -->
        <ProductCompetitorsItem class="mb10" :title="$t('相关文章')" :type="'article'" v-if="articleList.length" :list="articleList"></ProductCompetitorsItem>
        <!-- 同类排行榜 -->
        <ProductCompetitorsItem
            class="mb10"
            :title="$t('同类排行榜')"
            :type="'rank'"
            v-if="cateRank && checkEmpty(cateRank)"
            :cateRanke="cateRank"
        ></ProductCompetitorsItem>
        <!-- 关联商品 -->
        <ProductCompetitorsItem
            class="mb10"
            :title="$t('相关商品')"
            :type="'relatedList'"
            v-if="relatedList.length"
            :list="relatedList"
        ></ProductCompetitorsItem>
        <!-- 查看更多 -->
        <ProductCompetitorsItem
            class="mb10"
            :title="$t('看了还看')"
            :type="'lookAlso'"
            isMore
            v-if="lookAlso.length"
            :list="lookAlso"
        ></ProductCompetitorsItem>
    </div>
</template>

<script setup>
import { reactive, ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import {
    getProductRelateArticle,
    getProductRelateBrand,
    getProductRelateCategory,
    getProductRelateLookAlso,
    getProductRelateProductList,
    getProductRelateRank
} from "~/api/product/category";

const props = defineProps({
    productId: {
        type: Number,
        default: 0
    }
});
const route = useRoute();
const filterParams = reactive({
    productId: props.productId,
    intro: "hot"
});
const articleList = ref([]);
const brandInfo = ref([]);
const cateInfo = ref([]);
const lookAlso = ref([]);
const cateRank = ref({});
const relatedList = ref({});
const _getProductRelateArticle = async () => {
    try {
        const result = await getProductRelateArticle({ ...filterParams });
        articleList.value = result;
    } catch (error) {
        message.error(error.message);
    }
};
_getProductRelateArticle();
const _getProductRelateCategory = async () => {
    try {
        const result = await getProductRelateCategory({ ...filterParams });
        cateInfo.value = result;
    } catch (error) {
        message.error(error.message);
    }
};
_getProductRelateCategory();
const _getProductRelateBrand = async () => {
    try {
        const result = await getProductRelateBrand({ ...filterParams });
        brandInfo.value = result;
    } catch (error) {
        message.error(error.message);
    }
};
_getProductRelateBrand();
const _getProductRelateRank = async () => {
    try {
        const result = await getProductRelateRank({ ...filterParams });
        cateRank.value = result;
    } catch (error) {
        message.error(error.message);
    }
};
_getProductRelateRank();
const _getProductRelateLookAlso = async () => {
    try {
        const result = await getProductRelateLookAlso({ ...filterParams });
        lookAlso.value = result;
    } catch (error) {
        message.error(error.message);
    }
};
_getProductRelateLookAlso();

//关联商品
const _getProductRElatedList = async () => {
    try {
        const result = await getProductRelateProductList({ ...filterParams });
        relatedList.value = result;
    } catch (error) {
        message.error(error.message);
    }
};
_getProductRElatedList();

const checkEmpty = (obj) => {
    for (const key in obj) {
        if (obj[key].length > 0) {
            return true;
        }
    }
    return false;
};
</script>
<style lang="less" scoped></style>
