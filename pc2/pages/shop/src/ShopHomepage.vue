<!--
  店铺首页组件
  - 展示店铺装修页面，动态加载店铺各类装修模块
  - 支持店铺信息、公告、优惠券、图片广告、商品展示、热区图等模块
  - 根据模块 isFluxWidth 配置决定模块是否全屏宽度显示
  - 店铺搜索功能根据系统配置决定是否启用
-->
<template>
    <CommonPageHeader :shopId="shopId" :showDefNav="false" :shopSearch="commonStore.searchSelfStore === 1"></CommonPageHeader>
    <div style="background: white" v-if="loading">
        <template v-for="(item, index) in moduleList" :key="index">
            <ShopInfo :pageModule="pageModule" :shopId="Number(shopId)" :module="item.module" v-if="item.type == 'shopInfo' && item.isShow"></ShopInfo>
            <div class="container">
                <ShopNotice :module="item.module" v-if="item.type == 'notice' && item.isShow"></ShopNotice>
                <ShopCoupon :shopId="Number(shopId)" :module="item.module" v-if="item.type == 'coupon' && item.isShow"></ShopCoupon>
                <ShopImageSquareAd :module="item.module" v-if="item.type == 'imageSquareAd' && item.isShow"></ShopImageSquareAd>
                <ShopWhiteBlank :module="item.module" v-if="item.type == 'whiteBlank' && item.isShow"></ShopWhiteBlank>
                <ShopWhiteLine :module="item.module" v-if="item.type == 'whiteLine' && item.isShow"></ShopWhiteLine>
                <ShopTitleAd :module="item.module" v-if="item.type == 'titleAd' && item.isShow"></ShopTitleAd>
                <ShopImageNav :module="item.module" v-if="item.type == 'imageNav' && item.isShow"></ShopImageNav>
                <ShopImageAd :module="item.module" v-if="item.type == 'imageAd' && item.isShow && item.module.isFluxWidth == 0"></ShopImageAd>
                <ShopProduct
                    :shopId="Number(shopId)"
                    :decorateId="decorateId"
                    :moduleIndex="item.moduleIndex"
                    :module="item.module"
                    v-if="item.type == 'product' && item.isShow"
                ></ShopProduct>
                <ShopImageHotarea :module="item.module" v-if="item.type == 'imageHotarea' && item.isShow"></ShopImageHotarea>
            </div>
            <ShopImageAd v-if="item.type == 'imageAd' && item.isShow && item.module.isFluxWidth == 1" :module="item.module"></ShopImageAd>
        </template>
    </div>
    <CommonPageFooter :mt30="false" :showFriendly="false"></CommonPageFooter>
</template>
<script setup lang="ts">
/**
 * 店铺首页组件
 * - 通过店铺装修API获取店铺模块配置
 * - 动态渲染各类装修组件：店铺信息、公告、优惠券、图片导航、商品列表等
 * - 支持页面SEO配置（title、keywords、description）
 */
import { ref } from "vue";
import { getShopDecorate } from "~/api/shop/shop";
import { getShopDetail } from "~/api/product/product";
import ShopNotice from "~/components/shop/ShopNotice.vue";
import ShopWhiteBlank from "~/components/shop/ShopWhiteBlank.vue";
import ShopWhiteLine from "~/components/shop/ShopWhiteLine.vue";
import ShopImageHotarea from "~/components/shop/ShopImageHotarea.vue";
import ShopProduct from "~/components/shop/ShopProduct.vue";
import ShopCoupon from "~/components/shop/ShopCoupon.vue";
import { useCommonStore } from "@/store/common";
import { buildSeoKeywords } from "@/utils/seo";
import { buildShopPath } from "@/utils/seo-url";
const commonStore = useCommonStore();
const { t } = useI18n();
const title = ref(t("店铺详情"));
const description = ref("");
const keywords = ref("");
const loading = ref(false);
const route = useRoute();
const shopId = ref(route.path.split("/").pop());
const shopInfo = ref<any>({});
const canonical = computed(() => buildShopPath({ seoLink: shopInfo.value?.seoLink, id: shopId.value }) || route.path);
useSeo({
    title: computed(() => shopInfo.value.shopTitle || title.value || t("店铺详情")),
    description: computed(() => shopInfo.value.description || description.value || commonStore.shopDesc),
    keywords: computed(() => keywords.value || buildSeoKeywords([shopInfo.value.shopTitle, shopInfo.value.description])),
    image: computed(() => shopInfo.value.shopLogo || ""),
    canonical
});
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
<style scoped lang="less"></style>
