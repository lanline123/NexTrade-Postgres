<template>
    <div class="top-nav" :class="{ 'top-nav-index': props.page === 'index' }">
        <div class="nav-bd container flex align-end" :class="{ 'container-new': isNewSearch }">
            <div class="main-nav-link" @mouseenter="showHiddenCate" @mouseleave="hideHiddenCate">
                <div class="acea-row row-center-wrapper">
                    <div>{{ $t("全部商品分类") }}</div>
                    <div class="iconfont-pc icon-xiajiantou" :class="isIconVisible ? 'icon-xiajiantou-hover' : 'not-icon-xiajiantou-hover'"></div>
                </div>
                <div class="hidden-cate" v-if="props.page != 'index' && isCateVisible" :class="{ 'hidden-hover': isCateVisible }">
                    <CommonAllCate v-model:isCateVisible="isCateVisible" v-if="isCateVisible"></CommonAllCate>
                </div>
            </div>
            <ul class="sub-nav">
                <li>
                    <NuxtLink :to="'/'" class="alv1">{{ $t("首页") }}</NuxtLink>
                </li>
                <li v-for="(item, index) in commonStore.mainNav">
                    <CommonLink class="alv1" :item="item">{{ $t(item.title) }}</CommonLink>
                </li>
                <template v-if="isDemo()">
<!--                    <li>-->
<!--                        <NuxtLink :to="'/shop/list'" class="alv1">{{ $t("店铺街") }}</NuxtLink>-->
<!--                    </li>-->
                    <li>
                        <NuxtLink :to="'/join/intro'" class="alv1">{{ $t("商家入驻") }}</NuxtLink>
                    </li>
                </template>
            </ul>
        </div>
    </div>
</template>
<script setup lang="ts">
import { urlFormat } from "~/utils/format";
import { useCommonStore } from "~/store/common";
import { isDemo } from "@/utils/util";
const props = defineProps({
    page: { type: String, default: "" },
    isNewSearch: {
        type: Boolean,
        default: false
    }
});
const commonStore: any = useCommonStore();

commonStore.loadNav();
// 所有分类显示
const isCateVisible = ref(false);
const isIconVisible = ref(false);
const isShowVisible = ref(false);
let delayTimer = <any>null; // 延时定时器
const showHiddenCate = async () => {
    delayTimer = setTimeout(() => {
        isIconVisible.value = true;
        isCateVisible.value = true;
        // startSecondAnimation();
    }, 300);
};
const hideHiddenCate = () => {
    if (delayTimer) {
        clearTimeout(delayTimer);
    }
    isIconVisible.value = false;
    isCateVisible.value = false;
    isShowVisible.value = false;
};
</script>
<style scoped lang="less">
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
        padding: 0 var(--pc-home-padding-x, 0px);
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

.top-nav {
    position: relative;
    height: 40px;
    z-index: 1000;
}

.top-nav-index .nav-bd .main-nav-link:hover .icon-xiajiantou {
    transform: rotate(0);
    transition: transform ease 0.5s;
}

.nav-bd {
    .main-nav-link {
        width: 190px;
        height: 37px;
        line-height: 37px;
        background: var(--main-bg);
        font-size: 14px;
        color: #fff;
        font-weight: 700;
        position: relative;
        cursor: pointer;
        z-index: 1001;

        .icon-xiajiantou {
            font-size: 12px;
            margin-left: 10px;
        }

        .hidden-cate {
            font-weight: normal;
            position: relative;
            top: 0;
            height: 470px;
            z-index: 1002;
            /*transition: height 0.2s ease-in-out;
            box-shadow: 1px 0 6px rgba(0, 0, 0, 0.2);*/
        }

        .show {
            overflow: visible;
        }

        .hidden-hover {
            height: 470px;
        }

        .icon-xiajiantou-hover {
            transform: rotate(-180deg);
            transition: transform ease 0.5s;
        }

        .not-icon-xiajiantou-hover {
            transform: rotate(0deg);
            transition: transform ease 0.5s;
        }

        // &:hover {
        //     .hidden-cate {
        //         height: 470px;
        //     }

        //     .icon-xiajiantou {
        //         transform: rotate(-180deg);
        //         transition: transform ease 0.5s;
        //     }
        // }

        // &:not(:hover) {
        //     .icon-xiajiantou {
        //         transform: rotate(0deg);
        //         transition: transform ease 0.5s;
        //     }
        // }
    }

    .sub-nav {
        height: 40px;
        display: flex;
        flex-wrap: wrap;
        align-items: center;
    }

    li .alv1 {
        font-size: 15px;
        padding: 0 18px;
        color: #333;
        display: block;
        height: 40px;
        line-height: 40px;

        &:hover {
            color: var(--general);
        }
    }
}
</style>
