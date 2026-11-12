<template>
    <component :is="layoutComponent" />
</template>

<script setup lang="ts">
/**
 * @description 店铺内搜索页容器
 * - 依据 uiVersion 在新旧两套 UI 布局间切换
 * - URL 携带 ?ui=new 走新版，否则走旧版；也可通过 props 覆盖
 */
import { computed } from "vue";
import { useRoute } from "vue-router";
import ShopSearchLayoutClassic from "./ShopSearchLayoutClassic.vue";
import ShopSearchLayoutModern from "./ShopSearchLayoutModern.vue";

const props = defineProps({
    uiVersion: {
        type: String,
        default: ""
    }
});

const route = useRoute();

const resolvedUiVersion = computed(() => {
    if (props.uiVersion === "new" || props.uiVersion === "old") {
        return props.uiVersion;
    }
    return route.query.ui === "new" ? "new" : "old";
});

const layoutComponent = computed(() => (resolvedUiVersion.value === "new" ? ShopSearchLayoutModern : ShopSearchLayoutClassic));
</script>
