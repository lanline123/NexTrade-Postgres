<template>
    <div class="menu-list">
        <div class="ico">
            <div class="sub-title">CLASSIFICATION</div>
            <div class="title">▪︎ {{ $t("商品分类") }} ▪︎</div>
        </div>
        <div class="menu">
            <el-collapse v-model="activeName">
                <el-collapse-item v-for="item in leftMenuList" :name="item.categoryId">
                    <template #title>
                        <NuxtLink :to="'/ShopCategory/' + shopId + '?categoryId=' + item.categoryId" class="title">
                            {{ item.categoryName }}
                        </NuxtLink>
                    </template>
                    <div class="min-menu">
                        <NuxtLink
                            v-for="items in item?.children"
                            :class="activeNameId === items.categoryId ? 'pitch-on' : ''"
                            :to="'/ShopCategory/' + shopId + '?categoryId=' + items.categoryId"
                            class="item-menu"
                        >
                            {{ items.categoryName }}
                        </NuxtLink>
                    </div>
                </el-collapse-item>
            </el-collapse>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref } from "vue";
import { useRouter } from "vue-router";

const activeName: any = ref();
const activeNameId: any = ref();

const props = defineProps({
    shopId: { type: Number, default: -1 },
    leftMenuList: {
        type: Array,
        default: []
    }
});
const over = ref<Boolean>(false);
const emit = defineEmits(["update:leftMenuList", "update:dataOver"]);

const router = useRouter();
watch(
    [() => router.currentRoute.value.query, () => over.value],
    () => {
        let id: any = router.currentRoute.value.query.categoryId;

        props.leftMenuList?.forEach((item: any, index: number) => {
            if (item.categoryId === id) {
                activeNameId.value = item.articleCategoryId;
            }
        });
    },
    { deep: true }
);
</script>
<style lang="less" scoped>
.menu-list {
    width: 200px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    background-color: #fcfcfc;
    padding: 15px 15px 0;

    .ico {
        width: 200px;

        .sub-title {
            font-size: 12px;
            color: #999999;
            text-align: center;
        }

        .title {
            font-size: 18px;
            text-align: center;
            color: black;
            height: 30px;
            line-height: 30px;
        }
    }

    .menu {
        .title {
            width: 100%;
            font-weight: bold;
            margin-left: 21px;
        }

        .min-menu {
            font-size: 12px;
            display: flex;
            flex-direction: column;

            .item-menu {
                text-align: center;
                height: 40px;
                line-height: 40px;
                width: 100%;
                font-size: 14px;
                color: black;
            }

            .pitch-on {
                color: var(--general);
            }
        }
    }
}

:deep(.el-collapse-item__header.is-active) {
    border-bottom: dashed 1px #e1e1e1;
}

.loading-data {
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: white;
    min-height: 400px;
    color: #999999;
    font-size: 20px;
    border: 1px solid #e4e4e4;
    font-weight: normal;
}
</style>
