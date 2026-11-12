<template>
    <div class="navigation-row flex align-center">
        <template v-for="(nav, index) in categoryNavigation" v-if="categoryNavigation.length > 0">
            <div class="nav-tit flex align-center" v-if="index === 0">
                <div class="item" @click="onLinkClick(nav)">{{ nav.categoryName }}</div>
                <i class="crumbs-arrow">/</i>
            </div>
            <div v-else>
                <div class="nav-select flex align-center">
                    <p @click="onLinkClick(nav)" class="navTitle">{{ nav.categoryName }}</p>
                    <i class="crumbs-arrow">/</i>
                </div>
            </div>
        </template>
        <div class="nav-txt" v-if="props.productName">
            <p class="navTitle">{{ props.productName }}</p>
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
    padding: 32px 0px;
    word-break: break-all;

    i {
        user-select: none;
        margin-left: 8px;
        margin-right: 8px;
    }
    .nav-tit {
        .item {
            font-weight: 400;
            font-size: 16px;
            line-height: 1.5;
            color: #000;
            cursor: pointer;
        }

        &:hover {
            .item {
                color: var(--general);
            }
        }
    }

    .nav-select {
        i {
            user-select: none;
            margin-left: 8px;
            margin-right: 8px;
        }
        .navTitle {
            cursor: pointer;
            font-weight: 400;
            font-size: 16px;
            line-height: 1.5;
            color: #000;
            display: inline-block;
            max-width: 100%;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            &:hover {
                color: var(--general);
            }
        }
        .select-item {
            border: 1px solid #dddddd;
            height: 22px;
            line-height: 22px;
            padding: 0 4px 0 8px;
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
        flex: 1;
        overflow: hidden;

        .navTitle {
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }
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
.navTitle {
    font-weight: 400;
    font-size: 16px;
    line-height: 1.5;
    color: rgba(0, 0, 0, 0.87);
}
</style>
