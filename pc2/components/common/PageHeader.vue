<template>
    <!-- 新版头部：后台配置了 PC 头部装修时全站启用；未配置回退下方旧版头部 -->
    <CommonPcHeaderRuntime
        v-if="useNewHeader"
        :is-white="true"
        :page="page"
        :shop-search="shopSearch"
        :ready="decorateRuntimeStore.pcHeaderReady"
        :config="decorateRuntimeStore.pcHeaderConfig"
    />
    <template v-else>
        <CommonTopBar :colorType="colorType" :pageType="pageType" :is-new-search="isNewSearch"></CommonTopBar>
        <div class="header header-shell-MjI4MnNlbm8" :style="{ borderBottom: pageType != 1 ? '1px none #ddd' : '1px solid #ddd' }">
            <div class="top-area container" :class="{ 'container-new': isNewSearch }">
                <div class="wrapper">
                    <div class="logo">
                        <NuxtLink exact-active-class="''" to="/" class="alv1">
                            <img :style="{ width: pageType != 1 ? '140px' : '190px' }" :src="imageFormat(commonStore.shopLogo)" alt="" />
                        </NuxtLink>
                    </div>

                    <div v-if="shopSearch">
                        <ShopSearchInput></ShopSearchInput>
                    </div>
                    <div v-else>
                        <template v-if="pageType != 1">
                            <SimNav :page="page"></SimNav>
                        </template>
                        <template v-if="pageType == 1">
                            <DefSearch :indexSearchShop="hideIndexSearchShop ? 0 : commonStore.indexSearchShop" :defaultSearchType="defaultSearchType"></DefSearch>
                        </template>
                        <template v-else>
                            <SimSearch></SimSearch>
                        </template>
                    </div>

                    <ShoppingCart v-if="pageType == 1"></ShoppingCart>
                </div>
                <template v-if="showDefNav && pageType == 1">
                    <DefNav :page="page" :is-new-search="isNewSearch"></DefNav>
                </template>
            </div>
        </div>
    </template>
    <CommonService
        v-if="showService !== null"
        :phone="phone"
        :shopId="shopId"
        :orderId="orderId"
        :productId="productId"
        :showService="showService"
    ></CommonService>
    <ClientOnly>
        <CommonStatisticLog
            ref="statisticLogRef"
            :immediateLog="immediateLog"
            :shopCategoryId="shopCategoryId"
            :shopId="shopId"
            :productId="productId"
        ></CommonStatisticLog>
    </ClientOnly>
</template>
<script lang="ts" setup>
import { useCommonStore } from "@/store/common";
import SimSearch from "~/components/common/src/SimSearch.vue";
import DefSearch from "~/components/common/src/DefSearch.vue";
import DefNav from "~/components/common/src/DefNav.vue";
import SimNav from "~/components/common/src/SimNav.vue";
import ShopSearchInput from "~/pages/shop/src/ShopSearchInput.vue";
import CommonPcHeaderRuntime from "~/decorate/components/pc/pcHeaderRuntime/index.vue";
import { useDecorateRuntimeStore } from "@/store/decorateRuntime";

const commonStore: any = useCommonStore();
const decorateRuntimeStore = useDecorateRuntimeStore();
// 新版头部分流：后台配置了 PC 头部装修则全站启用新版头部，否则回退旧版
const useNewHeader = computed(() => decorateRuntimeStore.pcHeaderReady && !!decorateRuntimeStore.pcHeaderConfig);
const statisticLogRef = ref();
commonStore.loadNav();
const pageType = computed(() => {
    return commonStore.decoratePageConfig?.headerStyle;
});
const colorType = computed(() => {
    return commonStore.decoratePageConfig?.colorStyle;
});

defineProps({
    page: { type: String, default: "" },
    showService: {
        type: [Boolean, null],
        default: null
    },
    shopId: {
        type: [Number, String],
        default: 0
    },
    productId: {
        type: [Number, String],
        default: 0
    },
    orderId: {
        type: [Number, String],
        default: 0
    },
    shopCategoryId: {
        type: [Number, String],
        default: ""
    },
    immediateLog: {
        type: Boolean,
        default: true
    },
    phone: {
        type: String,
        default: ""
    },
    showDefNav: {
        type: Boolean,
        default: true
    },
    shopSearch: {
        type: Boolean,
        default: false
    },
    hideIndexSearchShop: {
        type: Boolean,
        default: false
    },
    defaultSearchType: {
        type: String,
        default: "product"
    },
    isNewSearch: {
        type: Boolean,
        default: false
    }
});

const handleLog = () => {
    statisticLogRef.value?.__commonLog();
};
defineExpose({
    handleLog
});
</script>
<style lang="less" scoped>
.container {
    width: 100%;
    max-width: var(--pc-home-max-width, 1190px);
    margin: 0 auto;
    padding: 0 var(--pc-home-padding-x, 0px);
    box-sizing: border-box;
    min-width: var(--pc-home-min-width, auto);

    &.container-new {
        max-width: var(--pc-home-max-width, 1600px);
        width: 100%;
        margin: 0 auto;
        padding: 0 var(--pc-home-padding-x, 38px);
        box-sizing: border-box;
        min-width: var(--pc-home-min-width, auto);
    }
}

.acea-row {
    display: flex;
    flex-wrap: wrap;
}

.acea-row.row-middle {
    align-items: center;
}

.acea-row.row-right {
    justify-content: flex-end;
}

.acea-row.row-center-wrapper {
    align-items: center;
    justify-content: center;
}

.acea-row.row-between-wrapper {
    align-items: center;
    justify-content: space-between;
}

.header {
    background: #fff;

    position: relative;
    min-width: var(--pc-home-min-width, auto);
}

.top-area {
    position: relative;

    .wrapper {
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 16px;

        .logo {
            min-height: 100px;
            display: flex;
            align-items: center;
            flex-shrink: 0;

            img {
                padding: 10px 0;
            }
        }

        /* 保证中间的搜索组件是自适应伸缩的，不挤压两边 */
        > div:not(.logo):not(.cart) {
            flex: 1;
            display: flex;
            justify-content: center;
            min-width: 0;
        }
    }
}
</style>
