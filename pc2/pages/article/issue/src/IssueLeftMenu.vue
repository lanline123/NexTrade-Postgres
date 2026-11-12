<template>
    <div class="menu-list">
        <div v-if="!loading" class="menu">
            <CustomCollapse v-model="activeName" accordion>
                <CustomCollapseItem
                    v-for="item in articleState"
                    :key="item.articleCategoryId"
                    :name="item.articleCategoryId"
                    :title="item.articleCategoryName"
                    :disabled="item.articles.length === 0"
                >
                    <template #icon="{ isActive }">
                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="none" class="collapse-icon" :class="{ 'is-active': isActive }">
                            <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M11.5 6.25 8 9.75l-3.5-3.5"></path>
                        </svg>
                    </template>
                    <div class="min-menu">
                        <NuxtLink
                            v-for="items in item?.articles"
                            :key="items.articleId"
                            :class="activeNameId === items.articleId ? 'pitch-on' : ''"
                            :to="'/article/issue/info?articleId=' + items.articleId"
                            class="item-menu"
                        >
                            {{ items.articleTitle }}
                        </NuxtLink>
                    </div>
                </CustomCollapseItem>
            </CustomCollapse>
        </div>
        <div v-else class="loading-data">
            {{ $t("加载中") }}
        </div>
    </div>
</template>
<script lang="ts" setup>
/**
 * 帮助中心左侧分类菜单组件
 * - 使用 el-collapse 折叠面板展示文章分类及其子文章
 * - 根据 URL 参数(articleId/categoryId)自动展开对应分类并高亮当前文章
 * - 通过 emit 向父组件传递分类列表数据
 */
import { reactive, ref } from "vue";
import { getBzzxCategoryList } from "~/api/article/category";
import CustomCollapse from "~/components/common/CustomCollapse.vue";
import CustomCollapseItem from "~/components/common/CustomCollapseItem.vue";
import type { CategoryFilterState } from "~/types/article/category";

const articleState = ref<CategoryFilterState[]>([]);
const activeName: any = ref();
const activeNameId: any = ref();

const loading = ref<boolean>(true);

const props = defineProps({
    dataOver: {
        type: Boolean,
        default: false
    },
    leftMenuList: {
        type: Array,
        default: []
    }
});
const over = ref<Boolean>(false);
const emit = defineEmits(["update:leftMenuList", "update:dataOver"]);

const loadBzzxList = async () => {
    loading.value = true;
    emit("update:leftMenuList", []);
    try {
        const result = await getBzzxCategoryList({ categorySize: 99, articleSize: 99 });
        Object.assign(articleState.value, result);
        emit("update:leftMenuList", articleState.value);
        emit("update:dataOver", true);
        over.value = true;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
loadBzzxList();
const router = useRouter();
watch(
    [() => router.currentRoute.value.query, () => over.value],
    () => {
        if (router.currentRoute.value.query.hasOwnProperty("articleId")) {
            let id: any = router.currentRoute.value.query.articleId;
            articleState.value?.forEach((item: any, index: number) => {
                item.articles.forEach((item2: any, index2: number) => {
                    if (item2.articleId === Number(id)) {
                        activeName.value = item.articleCategoryId;
                        activeNameId.value = item2.articleId;
                    }
                });
            });
        } else if (router.currentRoute.value.query.hasOwnProperty("categoryId")) {
            let id: any = router.currentRoute.value.query.categoryId;
            articleState.value?.forEach((item: any, index: number) => {
                if (item.categorySn === id) {
                    activeName.value = item.articleCategoryId;
                }
            });
        }
    },
    { deep: true }
);
</script>
<style lang="less" scoped>
.menu-list {
    box-sizing: border-box;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 20px;
    overflow: hidden;
    border-radius: 8px;
    padding: 8px 24px;
    background: #fff;

    .menu {
        border-radius: 8px;

        // 共享 CustomCollapse / CustomCollapseItem 的默认样式带顶部/底部 1px 横线，
        // 这里通过作用域内 :deep() 覆盖，仅作用于本菜单实例，不影响其他调用方
        :deep(.cc-collapse) {
            border-top: none;
        }
        :deep(.cc-collapse-item:last-child) {
            border-bottom: none;
        }
        :deep(.cc-collapse-item__title) {
            font-size: 18px;
            font-weight: 600;
        }

        .collapse-icon {
            color: #999;
            flex-shrink: 0;
            transition: transform 0.3s ease-in-out;

            &.is-active {
                transform: rotate(180deg);
            }
        }

        .min-menu {
            padding-left: 40px;
            font-size: 14px;
            display: flex;
            flex-direction: column;
            a {
                width: 100%;
                white-space: nowrap;
                overflow: hidden;
                text-overflow: ellipsis;
            }

            .item-menu {
                height: 55px;
                line-height: 55px;
                width: 100%;
            }

            .pitch-on {
                color: var(--general);
            }
        }
    }
}

.loading-data {
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: white;
    min-height: 400px;
    color: #999999;
    font-size: 20px;
    font-weight: normal;
}
</style>
