<template>
    <div v-if="!loading">
        <ShopHomepage v-if="hasDecorate" />
        <ShopDefaultPage v-else />
    </div>
</template>
<script lang="ts" setup>
/**
 * @description 店铺主页路由页面
 * - 根据店铺配置决定展示装修首页还是默认样式首页
 * - pcDefaultStyle=1强制使用默认样式，否则检查是否有装修配置
 */
import { ref, onMounted } from "vue";
import { useRoute } from "vue-router";
import { getShopDecorate } from "~/api/shop/shop";
import { getShopHead } from "~/api/product/product";
import ShopHomepage from "~/pages/shop/src/ShopHomepage.vue";
import ShopDefaultPage from "~/pages/shop/src/ShopDefaultPage.vue";

const route = useRoute();
const shopId = ref(route.params.id || route.path.split("/").pop());
const useDefaultStyle = ref(route.query.useDefaultStyle ? Number(route.query.useDefaultStyle) : 0);
const hasDecorate = ref(false);
const loading = ref(true);

/**
 * @description 检查店铺是否有装修配置
 * 有moduleList则使用装修首页，否则使用默认样式
 */
const checkDecorate = async () => {
    try {
        const result: any = await getShopDecorate({ shopId: shopId.value });
        hasDecorate.value = result.moduleList && result.moduleList.length > 0;
    } catch (error) {
        hasDecorate.value = false;
    }
};

const initPage = async () => {
    try {
        const headResult: any = await getShopHead(Number(shopId.value));
        const pcDefaultStyle = headResult?.data?.pcDefaultStyle;
        // pcDefaultStyle 为 "1" 时使用默认样式，不检查装修配置
        if (pcDefaultStyle === "1") {
            hasDecorate.value = false;
            return;
        }
        // pcDefaultStyle 为 "0" 或其他值时，检查装修配置
        await checkDecorate();
    } catch (error) {
        // 获取配置失败时，默认检查装修配置
        await checkDecorate();
    } finally {
        loading.value = false;
    }
};

onMounted(() => {
    initPage();
});
</script>
