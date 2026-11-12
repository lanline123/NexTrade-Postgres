<template>
    <div @mouseleave="hideSubView">
        <div class="cate_menu">
            <div class="cate_menu_item" v-for="(item, index) in commonStore.allCate" @mouseenter="showSubView(index)"
                :class="{ active: activeSubView === index }">
                <span :class="'floor-ico ' + item.floorIcoFont"></span>
                <template v-for="(cat, _index) in item.catList">
                    <NuxtLink target="_blank" class="cate_menu_lk" :to="urlFormat({ path: 'list', seoLink: cat.seoLink, id: cat.id })"
                        v-if="cat.name != '-'">
                        {{ cat.name }}</NuxtLink>
                    <span class="cate_menu_line" v-if="cat.name != '-'">/</span>
                </template>
            </div>
        </div>
        <div class="subCata">
            <template v-for="(item, index) in commonStore.allCate" :key="index">
                <div class="subView" :class="{ active: activeSubView === index }">
                    <div class="left">
                        <div class="nav_cata_bd">
                            <template v-for="(cat, _index) in item.catList">
                                <div class="item" v-for="child in cat.children">
                                    <div class="title-list">
                                        <NuxtLink :to="urlFormat({ path: 'list', seoLink: child.seoLink, id: child.categoryId })">{{
                                            child.categoryName }}</NuxtLink>
                                        <i v-if="child.children?.length"> &gt;</i>
                                    </div>
                                    <div class="name-list" v-if="child.children?.length">
                                        <template v-for="childer in child.children">
                                            <NuxtLink :to="urlFormat({ path: 'list', seoLink: childer.seoLink, id: childer.categoryId })">{{
                                                childer.categoryName }}</NuxtLink>
                                        </template>
                                    </div>
                                </div>
                            </template>
                        </div>
                    </div>
                </div>
            </template>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, watch } from "vue";
import { getCatFloor } from "@/api/common";
import { urlFormat } from "@/utils/format";
import { useLoadCss } from "@/utils/domUtils";
import { useCommonStore } from "@/store/common";
const props = defineProps({
    isCateVisible: { type: Boolean, default: false }
});
const commonStore = useCommonStore();
const icoFontCss = ref("");

// 使用 useLazyAsyncData 在服务端和客户端都能获取数据
const { data: catFloorData } = await useLazyAsyncData(
    "catFloor",
    async () => {
        if (commonStore.allCate !== null) {
            return { catFloor: commonStore.allCate, icoDefinedCss: "" };
        }
        try {
            const result = await getCatFloor();
            return result;
        } catch (error) {
            return { catFloor: [], icoDefinedCss: "" };
        }
    },
    {
        server: true
    }
);

// 监听数据变化，更新 store 和加载 CSS
watch(
    catFloorData,
    (newData) => {
        if (newData) {
            commonStore.allCate = newData.catFloor ?? null;
            // useLoadCss 使用了 document API，只能在客户端执行
            if (import.meta.client && newData.icoDefinedCss && newData.icoDefinedCss != "") {
                useLoadCss(newData.icoDefinedCss);
            }
        }
    },
    { immediate: true }
);
const activeSubView = ref<number | null>(null);
const showSubView = (index: number) => {
    activeSubView.value = index;
};
const hideSubView = () => {
    activeSubView.value = null;
};


const height = computed(() => {
    return `ca`;
});
</script>

<style lang="less" scoped>
.cate_menu {
    overflow: hidden;
    padding: 10px 0;
    height: 450px;
    background-color: #fefefe;
    color: #636363;
    display: flex;
    flex-direction: column;

    .cate_menu_item {
        overflow: hidden;
        padding-left: 16px;
        height: 36px;
        line-height: 36px;
        flex-wrap: wrap;
        font-size: 0;
        transition: background-color 0.2s ease;
        display: flex;
        align-items: center;
        flex: 1 1 0%;
        max-height: 45px;

        .cate_menu_lk {
            font-size: 14px;
            color: #333;
            transition: color 0.2s ease;

            &:hover {
                color: var(--general);
            }
        }

        .cate_menu_line {
            padding: 0 4px;
            font-size: 12px;

            &:last-child {
                display: none;
            }
        }
    }

    .cate_menu_item.active {
        background-color: #f2f2f2;
    }
}

.floor-ico {
    padding-right: 10px;
    font-size: 18px;
}

.subCata {
    position: absolute;
    top: 0;
    left: 190px;
    z-index: 1998;
    transition: all 0.2s ease 0s;
    opacity: 1;
    box-shadow: 2px 0 6px rgba(0, 0, 0, 0.2);
    max-height: 700px;
    overflow-y: scroll;
    overflow: auto;
    -ms-overflow-style: none;
    /* IE 和 Edge */
    scrollbar-width: none;
    /* Firefox */
    &::-webkit-scrollbar {
        display: none;
        /* Chrome, Safari 和 Opera */
    }
}

.subView {
    background-color: #fff;
    border: 0 none;
    box-shadow: 2px 0 6px rgba(0, 0, 0, 0.2);
    display: none;
    min-height: 480px;
    overflow: hidden;
    position: relative;
    width: 778px;
    z-index: 10;
}

.subView.active {
    display: block;
}

.subView .left {
    min-height: 470px;
    position: relative;
    width: 777px;
    background: #fff;
    border-right: 1px solid #f2f2f2;
}

.subView .left .nav_cata_bd {
    padding: 12px 14px 10px;
}

.subView .item {
    overflow: hidden;
    clear: both;
    position: relative;
    font-size: 12px;
}

.subView .item .title-list {
    float: left;
    font-weight: bold;
    height: 39px;
    line-height: 38px;
    overflow: hidden;
    padding-right: 9px;
    text-align: right;
    width: 85px;
}

.subView .item .title-list a {
    color: #666;

    &:hover {
        color: var(--general);
    }
}

.subView .item .title-list i {
    font-family: "宋体", serif;
    font-style: normal;
    color: #666;
}

.subView .item .name-list {
    border-bottom: 1px solid #f2f2f2;
    display: block;
    float: left;
    line-height: 24px;
    padding: 7px 0;
    width: 647px;

    a:hover {
        color: var(--general);
    }
}

.subView .item .name-list a {
    display: inline-block;
    color: #666;
    margin: 0 9px;
}
</style>
