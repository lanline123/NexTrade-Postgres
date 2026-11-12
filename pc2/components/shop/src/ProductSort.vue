<template>
    <div class="type-style">
        <div class="item" v-for="item in treeList?.slice(0, 5)">
            <NuxtLink :to="'/ShopCategory/' + shopId + '?categoryId=' + item.categoryId" class="title"
                >{{ item.categoryName }} <span class="iconfont-pc icon-jiantou_you"></span
            ></NuxtLink>
            <div v-if="showSublevel == 1" class="info-list">
                <NuxtLink :to="'/ShopCategory/' + shopId + '?categoryId=' + info.categoryId" class="info" v-for="info in item.children">
                    {{ info.categoryName }}</NuxtLink
                >
            </div>
        </div>
    </div>
</template>
<script setup lang="ts">
const props = defineProps({
    treeList: { type: Array, default: [] },
    showSublevel: { type: Number, default: 1 },
    shopId: { type: Number, default: -1 }
});
</script>
<style scoped lang="less">
.type-style {
    display: flex;
    flex-direction: row;

    .item {
        display: flex;
        flex-direction: column;
        padding: 10px 20px;
        box-sizing: border-box;
        flex: 1;

        .title {
            cursor: pointer;
            width: 116px;
            color: black;
            height: 36px;
            line-height: 32px;
            font-weight: bold;
            display: flex;
            align-items: center;
            font-size: 14px;
        }

        .info-list {
            margin-top: 15px;
            display: flex;
            flex-direction: column;
            gap: 8px;

            .info {
                color: #999;
                font-size: 12px;
                cursor: pointer;
            }
        }
    }
}
</style>
