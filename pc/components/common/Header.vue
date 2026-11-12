<template>
    <div>
        <Title>{{ titleStr }}</Title>
    </div>
</template>
<script lang="ts" setup>
const { t } = useI18n();
import { useCommonStore } from "@/store/common";
const commonStore = useCommonStore();
const props = defineProps({
    title: {
        type: String,
        default: ""
    }
});
const titleStr = computed(() => {
    let title = t(props.title);
    if (commonStore.poweredByStatus != 1) {
        title += "-powered by nextrade";
    }
    return title;
});
</script>
