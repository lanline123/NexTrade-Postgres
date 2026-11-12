<template>
    <div class="navigation-row flex align-center category-path-MjI4MnV0bmg">
        <template v-for="(nav, index) in categoryNavigation" v-if="categoryNavigation.length > 0">
            <div class="nav-tit flex align-center" v-if="index === 0">
                <div class="item" @click="onLinkClick(nav)">{{ nav.categoryName }}</div>
                <i class="crumbs-arrow">></i>
            </div>
            <div class="nav-select-row" v-else>
                <div class="nav-select flex align-center">
                    <div class="select-item flex align-center">
                        <p>{{ nav.categoryName }}</p>
                        <i class="iconfont-pc icon-xiajiantou"></i>
                    </div>
                    <i class="crumbs-arrow">></i>
                </div>
                <div class="pop-up-conter">
                    <div class="pop-up flex justify-between">
                        <div class="item" @click="onLinkClick(item)" v-for="item in nav.catList">
                            {{ item.categoryName }}
                        </div>
                    </div>
                </div>
            </div>
        </template>
        <div class="nav-txt" v-if="props.productName">
            <p class="ellipsis">{{ props.productName }}</p>
        </div>
        <slot></slot>
    </div>
</template>
<script lang="ts" setup>
import { ref, shallowRef, onMounted, reactive } from "vue";
import { getParentCategoryTree } from "@/api/product/category";
import type { CategoryNavigation } from "@/types/product/category.d";
import { urlFormat } from "@/utils/format";
const emit = defineEmits(["change"]);
const props = defineProps({
    isSearchPage: {
        type: Boolean,
        default: false
    },
    productName: {
        type: String,
        default: ""
    }
});
const id = defineModel<number>("id");
const categoryNavigation = ref<CategoryNavigation[]>([]);
const _getParentCategoryTree = async () => {
    try {
        if (id.value && id.value > 0) {
            const result = await getParentCategoryTree(id.value);
            categoryNavigation.value = result;
        } else {
            categoryNavigation.value = [];
        }
    } catch (error) {
    } finally {
    }
};
_getParentCategoryTree();
const onLinkClick = (category: { categoryId: number; seoLink?: string }) => {
    if (props.isSearchPage) {
        id.value = category.categoryId;
        emit("change", 1);
    } else {
        navigateTo(urlFormat({ path: "list", seoLink: category.seoLink, id: category.categoryId }));
    }
};

watch(
    () => id.value,
    () => {
        _getParentCategoryTree();
    },
    { immediate: true }
);
</script>
<style lang="less" scoped>
.navigation-row {
    padding-top: 15px;
    cursor: pointer;
    i {
        font-family: simsun;
        font-style: normal;
        color: #333;
        font-size: 12px;
        margin: 0 5px;
    }
    .nav-tit {
        .item {
            font-family: "微软雅黑";
            font-size: 18px;
            color: #666666;
            font-weight: 700;
            outline: none;
        }
        .item:hover {
            color: #666 !important;
        }
    }
    .nav-select-row {
        position: relative;
        .nav-select {
            i {
                font-family: simsun;
                font-style: normal;
                color: #666;
                font-size: 12px;
            }
            .select-item {
                border: 1px solid #dddddd;
                height: 22px;
                line-height: 22px;
                padding: 0 4px 0 8px;
            }
        }
        .pop-up-conter {
            background-color: #ffffff;
            border: 1px solid #dddddd;
            display: none;
            left: 0;
            padding: 10px;
            position: absolute;
            top: 23px;
            width: 350px;
            .pop-up {
                flex-wrap: wrap;
                justify-content: flex-start;

                .item {
                    height: 24px;
                    line-height: 24px;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    white-space: nowrap;
                    width: 65px;
                }
            }
        }
        &:hover {
            .nav-select {
                .select-item {
                    border-color: var(--general);
                    border-bottom-color: #ffffff;
                    position: relative;
                    z-index: 99;

                    .icon-xiajiantou {
                        transform: rotate(180deg);
                    }
                }
            }
            .pop-up-conter {
                display: block;
                border-color: var(--general);
                z-index: 88;
            }
        }
    }
    .nav-txt {
        color: #888;
        width: 260px;
    }
    :deep(.nav-selected) {
        .item {
            display: inline-block;
            white-space: nowrap;
            border: 1px solid #dddddd;
            color: var(--general);
            padding: 0 24px 0 8px;
            margin-right: 8px;
            background: url("/assets/images/common/icons.png") no-repeat scroll right -247px #fff;
            height: 22px;
            line-height: 22px;
            &:hover {
                background-color: #f2f2f2;
                background-position: right -219px;
                text-decoration: none;
            }
        }
        span {
            color: #666;
        }
    }
}
</style>
