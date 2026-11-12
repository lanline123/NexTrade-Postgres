<template>
    <component :is="layoutComponent" />
</template>
<script lang="ts" setup>
/**
 * @description 商品搜索/列表页容器
 * - 业务逻辑统一下沉到 useSearchPage（唯一真理源），通过 provide 注入给布局组件
 * - 依据 uiVersion 在新旧两套 UI 布局间切换，二者共用同一份逻辑，不维护两套业务实现
 */
import { computed, provide } from "vue";
import { useSearchPage, SearchPageInjectionKey } from "@/composables/useSearchPage";
import SearchLayoutClassic from "./SearchLayoutClassic.vue";
import SearchLayoutModern from "./SearchLayoutModern.vue";

const props = defineProps({
    pageType: {
        type: String,
        default: "search"
    },
    uiVersion: {
        type: String,
        default: "old"
    }
});

const ctx = useSearchPage(() => props.pageType);
provide(SearchPageInjectionKey, ctx);

const layoutComponent = computed(() => (props.uiVersion === "new" ? SearchLayoutModern : SearchLayoutClassic));
</script>
