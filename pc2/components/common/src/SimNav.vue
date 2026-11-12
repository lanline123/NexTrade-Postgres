<template>
    <div class="header-nav">
        <NuxtLink class="nav-item" :to="'/'">{{ $t("首页") }}</NuxtLink>
        <template v-for="(item, index) in commonStore.mainNav">
            <div class="nav-item">
                <NuxtLink :to="item.link ? urlFormat(item.link) : ''" class="alv1">{{ $t(item.title) }}</NuxtLink>
            </div>
        </template>
        <template v-if="isOverseas()">
            <NuxtLink class="nav-item" to="/join/intro">{{ $t("商家入驻") }} </NuxtLink>
        </template>
    </div>
</template>
<script setup lang="ts">
import { urlFormat } from "~/utils/format";
import { useCommonStore } from "~/store/common";
import { isOverseas } from "@/utils/util";

const commonStore: any = useCommonStore();

commonStore.loadNav();
</script>
<style scoped lang="less">
.header-nav {
    width: 680px;
    display: flex;
    align-items: center;
    flex-direction: row;
    margin-left: 30px;

    .nav-item {
        display: flex;
        box-sizing: border-box;
        align-items: center;
        justify-content: center;
        padding: 26px 10px;
        color: #333333;
        font-size: 16px;
        font-weight: 400;
    }

    .nav-item:hover {
        color: var(--general);
    }
}
</style>
