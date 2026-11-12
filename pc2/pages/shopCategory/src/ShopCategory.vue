<template>
    <CommonPageHeader :shopId="shopId" :shopCategoryId="filter.categoryId"></CommonPageHeader>
    <div style="background: white" v-if="loading">
        <template v-for="(item, index) in moduleList" :key="index">
            <ShopInfo
                @treeList="treeListData"
                :pageModule="pageModule"
                :shopId="Number(shopId)"
                :module="item.module"
                v-if="item.type == 'shopInfo'"
            ></ShopInfo>
        </template>
        <CommonPageContainer>
            <div class="page-info">
                <LeftMenu :shopId="Number(shopId)" :leftMenuList="leftMenuList"></LeftMenu>
                <div class="product-list">
                    <CatGoodsList v-model:queryParams="queryParams" v-model:filter="filter"></CatGoodsList>
                </div>
            </div>
        </CommonPageContainer>
    </div>
    <CommonPageFooter :mt30="false" :showFriendly="false"></CommonPageFooter>
</template>
<script setup lang="ts">
import { ref } from "vue";
import { getShopDecorate } from "~/api/shop/shop";
import { getShopDetail } from "~/api/product/product";
import LeftMenu from "~/components/shopCategory/LeftMenu.vue";
import CatGoodsList from "~/components/common/CatGoodsList.vue";
import { isOverseas, isO2o } from "~/utils/util";
import type { QueryParams } from "~/types/search/search";
import { useRouter } from "vue-router";
import { useCommonStore } from "@/store/common";
import { buildSeoKeywords } from "@/utils/seo";
const commonStore = useCommonStore();
const { t } = useI18n();
const title = ref(t("店铺商品"));
const description = ref("");
const keywords = ref("");
const leftMenuList = ref([]);
const treeListData = (items: any) => {
    leftMenuList.value = items;
};
const loading = ref(false);
const route = useRoute();
const shopId = ref(route.path.split("/").pop());
const shopInfo = ref<any>({});
const canonical = computed(() => {
    const params = new URLSearchParams();
    if (route.query.categoryId) {
        params.set("categoryId", String(route.query.categoryId));
    }
    const query = params.toString();
    return query ? `${route.path}?${query}` : route.path;
});
useSeo({
    title: computed(() => shopInfo.value.shopTitle || title.value || t("店铺商品")),
    description: computed(() => shopInfo.value.description || description.value || commonStore.shopDesc),
    keywords: computed(() => keywords.value || buildSeoKeywords([shopInfo.value.shopTitle, shopInfo.value.description])),
    image: computed(() => shopInfo.value.shopLogo || ""),
    canonical
});
const queryParams = computed(() => {
    let query = <QueryParams>{};
    if (filter.page > 1) {
        query.page = filter.page;
    }
    if (filter.categoryId > 0) {
        query.shopCategoryId = filter.categoryId;
    }
    if (filter.shopId > 0) {
        query.shopId = filter.shopId;
    }
    if (filter.keyword != "") {
        query.keyword = filter.keyword;
    }
    return query;
});

const filter: any = reactive({
    page: 1,
    categoryId: 0,
    shopId: -1,
    sortField: "",
    sortOrder: "",
    minPrice: 0,
    maxPrice: 0
});
const router = useRouter();
watch(
    [() => router.currentRoute.value.query],
    () => {
        filter.categoryId = router.currentRoute.value.query.categoryId;
        filter.shopId = shopId.value;
    },
    { deep: true, immediate: true }
);

const moduleList: any = ref([]);
const pageModule: any = ref({});
const decorateId = ref();
const getShopSeoInfo = async () => {
    try {
        const result = await getShopDetail(Number(shopId.value));
        shopInfo.value = result;
    } catch (error) {
    }
};
const getModuleList = async () => {
    loading.value = false;
    try {
        const [result]: any = await Promise.all([getShopDecorate({ shopId: shopId.value }), getShopSeoInfo()]);
        decorateId.value = result.decorateId;
        moduleList.value = result.moduleList;
        Object.assign(pageModule.value, result.pageModule);
        if (result.pageModule?.title) title.value = result.pageModule.title;
        if (result.pageModule?.description) description.value = result.pageModule.description;
        if (result.pageModule?.keywords) keywords.value = result.pageModule.keywords;
    } catch (error) {
    } finally {
        loading.value = true;
    }
};
getModuleList();
</script>
<style scoped lang="less">
.page-info {
    display: flex;

    .product-list {
        flex: 1;
        padding: 10px;

        .product-list {
            display: flex;
            flex-direction: column;
            box-sizing: border-box;
            width: 100%;
        }
    }
}
</style>
