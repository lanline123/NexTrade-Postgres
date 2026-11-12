<template>
    <CommonPageHeader />
    <Search />
    <CommonPageFooter :mt30="false" />
    <!-- 跨境：搜索页挂整套 ElevatorList（与 GST 互斥，见 app.vue showSideTools） -->
    <div v-if="isOverseas()" class="home-elevator">
        <HomeElevatorList class="home-elevator-list"></HomeElevatorList>
    </div>
</template>

<script lang="ts" setup>
/**
 * 商品搜索页入口
 * - 统一走新版 Search 组件；经典版由 /search.v1 承载
 * - 跨境站额外挂 ElevatorList；标准/o2o 走 GlobalSideTools 兜底
 */
import { computed } from "vue";
import Search from "~/pages/search/components/search.vue";
import { useCommonStore } from "~/store/common";
import { isOverseas } from "@/utils/util";

const route = useRoute();
const { t } = useI18n();
const commonStore = useCommonStore();

const keyword = computed(() => {
    const raw = route.query.keyword;
    return typeof raw === "string" ? raw.trim() : Array.isArray(raw) ? String(raw[0] || "").trim() : "";
});

useSeo({
    title: computed(() => (keyword.value ? `${t("搜索")} "${keyword.value}"` : t("商品搜索"))),
    description: computed(() =>
        keyword.value
            ? `${t("在")} ${commonStore.shopTitle || ""} ${t("搜索")} "${keyword.value}" ${t("相关商品")}`
            : `${commonStore.shopTitle || ""} ${t("商品搜索")}`
    ),
    robots: "noindex,follow",
    canonical: "/search"
});
</script>

<style lang="less" scoped>
.home-elevator {
    position: fixed;
    bottom: 100px;
    right: 0;
    z-index: 1100;
    height: fit-content;
}

.home-elevator-list {
    width: 60px;
    z-index: 9999;
}

.not-zh {
    .home-elevator-list {
        width: 68px;
    }
}
</style>
