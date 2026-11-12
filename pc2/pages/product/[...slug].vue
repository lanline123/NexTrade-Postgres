<template>
    <Product v-if="sn" :sn="sn" ui-version="overseas" />
</template>

<script setup lang="ts">
import Product from "@/pages/item/src/Product.vue";
import { extractProductSnFromSlug } from "@/utils/seo-url";

const route = useRoute();
const sn = computed(() => extractProductSnFromSlug(route.params.slug as string | string[] | undefined));

if (!sn.value) {
    await navigateTo("/404", { replace: true });
}
</script>
