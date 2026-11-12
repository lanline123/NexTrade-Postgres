<template>
    <CommonPageHeader :shopSearch="true"></CommonPageHeader>
    <div style="background: white" v-if="loading">
        <ShopHeader v-if="useDefaultStyle" :shopInfo="shopInfo" @collect="handleCollect" @kefu="handleKefu"></ShopHeader>
        <template v-else v-for="(item, index) in moduleList" :key="index">
            <ShopInfo
                @treeList="treeListData"
                :pageModule="pageModule"
                :shopId="Number(shopId)"
                :module="item.module"
                v-if="item.type == 'shopInfo'"
            ></ShopInfo>
        </template>
        <div class="container">
            <div class="page-info">
                <CatGoodsList
                    v-model:queryParams="queryParams"
                    v-model:filter="filter"
                    @filter-undo="onCatGoodsFilterUndo"
                ></CatGoodsList>
            </div>
        </div>
    </div>
    <CommonPageFooter :mt30="false" :showFriendly="false"></CommonPageFooter>
</template>

<script setup lang="ts">
/**
 * @description 店铺内搜索页 - 旧版 UI
 * - 顶部 CommonPageHeader + 装修/默认店铺头部 + 商品列表
 * - 装修态渲染 ShopInfo 模块，pcDefaultStyle=1 时走默认样式店头
 */
import { ref, computed, reactive, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { getShopDecorate } from "~/api/shop/shop";
import { getShopHead, getShopDetail, getShopCollect } from "~/api/product/product";
import CatGoodsList from "~/components/common/CatGoodsList.vue";
import ShopHeader from "~/pages/shop/src/ShopHeader.vue";
import type { QueryParams } from "~/types/search/search";
import { useCommonStore } from "@/store/common";
import { useUserStore } from "~/store/user";
import { safeOpen } from "@/utils/util";

const commonStore = useCommonStore();
const userStore = useUserStore();
const { t } = useI18n();

const title = ref(t("店铺商品搜索"));
const description = ref("");
const keywords = ref("");

const leftMenuList = ref([]);
const treeListData = (items: any) => {
    leftMenuList.value = items;
};

const loading = ref(false);
const useDefaultStyle = ref(false);
const shopInfo: any = ref({});
const route = useRoute();
const router = useRouter();
const shopId = ref(route.path.split("/").pop());

const queryParams = computed(() => {
    const query = <QueryParams>{};
    if (filter.page > 1) {
        query.page = filter.page;
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
    keyword: "",
    shopId: -1
});

const onCatGoodsFilterUndo = () => {
    if (typeof window !== "undefined" && window.history.length > 1) {
        router.back();
        return;
    }
    router.replace({
        path: router.currentRoute.value.path,
        query: {}
    });
};

watch(
    [() => router.currentRoute.value.query],
    () => {
        filter.keyword = router.currentRoute.value.query.keyword;
        filter.shopId = shopId.value;
    },
    { deep: true, immediate: true }
);

const moduleList: any = ref([]);
const pageModule: any = ref({});
const decorateId = ref();
const canonical = computed(() => {
    const params = new URLSearchParams();
    if (filter.keyword) {
        params.set("keyword", String(filter.keyword));
    }
    return params.size > 0 ? `${route.path}?${params.toString()}` : route.path;
});

useSeo({
    title: computed(() => title.value || shopInfo.value?.shopTitle || t("店铺商品搜索")),
    description: computed(() => description.value || shopInfo.value?.shopDesc || commonStore.shopDesc || ""),
    keywords: computed(() => keywords.value || shopInfo.value?.shopKeywords || shopInfo.value?.shopTitle || ""),
    image: computed(() => shopInfo.value?.shopLogo || pageModule.value?.shopLogo || ""),
    canonical,
    robots: "noindex,follow"
});

const getShopInfo = async () => {
    try {
        const result = await getShopDetail(Number(shopId.value));
        Object.assign(shopInfo.value, result);
    } catch (error: any) {
        console.error(error);
    }
};

const handleCollect = async () => {
    if (!userStore.userInfo?.userId || userStore.userInfo.userId <= 0) {
        message.warning(t("请先登录"));
        return;
    }
    try {
        await getShopCollect(Number(shopId.value));
        await getShopInfo();
        message.success(t("操作成功"));
    } catch (error: any) {
        message.error(error.message);
    }
};

const handleKefu = () => {
    if (shopInfo.value.kefuLink) {
        safeOpen(shopInfo.value.kefuLink, "newWindow", "width=800,height=600,scrollbars=yes,resizable=yes");
    } else {
        safeOpen(`/chat?shopId=${shopId.value}`, "newWindow", "width=800,height=600,scrollbars=yes,resizable=yes");
    }
};

const getModuleList = async () => {
    loading.value = false;
    try {
        const headResult: any = await getShopHead(Number(shopId.value));
        const pcDefaultStyle = headResult?.data?.pcDefaultStyle;

        if (pcDefaultStyle === "1") {
            useDefaultStyle.value = true;
            await getShopInfo();
        } else {
            useDefaultStyle.value = false;
            const result: any = await getShopDecorate({ shopId: shopId.value });
            decorateId.value = result.decorateId;
            moduleList.value = result.moduleList;
            Object.assign(pageModule.value, result.pageModule);
            if (result.pageModule?.title) title.value = result.pageModule.title;
            if (result.pageModule?.description) description.value = result.pageModule.description;
            if (result.pageModule?.keywords) keywords.value = result.pageModule.keywords;
        }
    } catch (error) {
        try {
            const result: any = await getShopDecorate({ shopId: shopId.value });
            decorateId.value = result.decorateId;
            moduleList.value = result.moduleList;
            Object.assign(pageModule.value, result.pageModule);
            if (result.pageModule?.title) title.value = result.pageModule.title;
            if (result.pageModule?.description) description.value = result.pageModule.description;
            if (result.pageModule?.keywords) keywords.value = result.pageModule.keywords;
        } catch (e) {
            console.error(e);
        }
    } finally {
        loading.value = true;
    }
};

getModuleList();
</script>

<style scoped lang="less">
.page-info {
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
}
</style>
