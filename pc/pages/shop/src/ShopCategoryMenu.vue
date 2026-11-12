<template>
    <div class="shop-category-menu">
        <div class="menu-item all-products" :class="{ active: !currentCategoryId }" @click="handleSelect(null)">
            {{ $t("全部商品") }}
        </div>

        <div class="category-list">
            <div v-for="item in menuList" :key="item.categoryId" class="category-group">
                <div
                    class="menu-item parent-item"
                    :class="{ active: currentCategoryId === item.categoryId || isChildActive(item) }"
                    @click="toggleExpand(item)"
                >
                    <span class="label">{{ item.categoryName }}</span>
                    <el-icon v-if="item.children && item.children.length" class="arrow-icon" :class="{ expanded: item.isExpanded }">
                        <ArrowDown />
                    </el-icon>
                </div>

                <div v-if="item.children && item.children.length" v-show="item.isExpanded" class="sub-menu">
                    <div
                        v-for="subItem in item.children"
                        :key="subItem.categoryId"
                        class="menu-item sub-item"
                        :class="{ active: currentCategoryId === subItem.categoryId }"
                        @click="handleSelect(subItem)"
                    >
                        {{ subItem.categoryName }}
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, watch, computed, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ArrowDown } from "@element-plus/icons-vue";
import { getShopTree } from "~/api/product/product";

const props = defineProps<{
    shopId: number;
}>();

const route = useRoute();
const router = useRouter();

const currentCategoryId = computed(() => Number(route.query.categoryId) || null);

const menuList = ref<any[]>([]);
const currentMainCateId = ref<number>(0);

const checkExpandStatus = () => {
    const currentId = currentCategoryId.value;
    if (!currentId) return;

    menuList.value.forEach((item) => {
        if (item.categoryId === currentId || (item.children && item.children.some((child: any) => child.categoryId === currentId))) {
            item.isExpanded = true;
        }
    });
};

const getMenuList = async () => {
    try {
        const result = await getShopTree(props.shopId);
        if (result && result.length > 0) {
            menuList.value = result.map((item: any) => ({
                ...item,
                isExpanded: true
            }));
            currentMainCateId.value = result[0]?.categoryId || 0;
            checkExpandStatus();
        }
    } catch (error) {
        console.error(error);
    }
};

watch(
    () => currentCategoryId.value,
    () => {
        checkExpandStatus();
    }
);

const isChildActive = (parent: any) => {
    if (!parent.children) return false;
    return parent.children.some((child: any) => child.categoryId === currentCategoryId.value);
};

const toggleExpand = (item: any) => {
    if (!item.children || item.children.length === 0) {
        handleSelect(item);
    } else {
        item.isExpanded = !item.isExpanded;
    }
};

const handleSelect = (item: any) => {
    const query: any = { ...route.query };
    if (item) {
        query.categoryId = item.categoryId;
    } else {
        delete query.categoryId;
    }
    query.page = 1;

    router.push({ query });
};

onMounted(() => {
    if (props.shopId) {
        getMenuList();
    }
});
</script>

<style scoped lang="less">
.shop-category-menu {
    width: 100%;
    background: #fff;
    padding: 10px;
    user-select: none;
    box-sizing: border-box;
    min-height: 400px;

    .menu-item {
        cursor: pointer;
        padding: 12px 20px;
        font-size: 14px;
        color: #333;
        transition: all 0.3s;
        display: flex;
        align-items: center;
        justify-content: space-between;
        border-radius: 8px;

        &:hover {
            color: var(--general);
        }

        &.active {
            background-color: var(--primary-light-9);
            color: var(--general);
            font-weight: bold;
            border-radius: 8px;
        }
    }

    .parent-item {
        .arrow-icon {
            font-size: 12px;
            color: #999;
            transition: transform 0.3s;

            &.expanded {
                transform: rotate(180deg);
            }
        }
    }

    .sub-menu {
        .sub-item {
            padding-left: 40px;
            font-size: 13px;
            color: #666;

            &:hover {
                color: var(--general);
            }

            &.active {
                color: var(--general);
                background-color: transparent;
            }
        }
    }
}
</style>
