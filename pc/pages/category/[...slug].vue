<template>
    <CommonPageHeader />
    <Search v-if="!loading" v-model:cat="cat" :page-type="route.path" />
    <CommonPageFooter />
</template>

<script setup lang="ts">
import { getCategoryIdBySeoLink } from "@/api/common";
import Search from "@/pages/search/components/search.vue";

const route = useRoute();
const { t } = useI18n();
const loading = ref(true);
const cat = ref("");

const getFirstQueryValue = (value: unknown) => (Array.isArray(value) ? value[0] : value);

const getSeoLink = () => {
    const slug = route.params.slug;
    const segments = Array.isArray(slug) ? slug : typeof slug === "string" ? [slug] : [];
    return segments
        .map((item) => String(item).trim())
        .filter(Boolean)
        .join("/");
};

const resolveCategory = async () => {
    loading.value = true;
    const queryCat = getFirstQueryValue(route.query.cat);
    const categoryIdFromQuery = Number(queryCat);

    if (Number.isInteger(categoryIdFromQuery) && categoryIdFromQuery > 0) {
        cat.value = String(categoryIdFromQuery);
        loading.value = false;
        return;
    }

    const seoLink = getSeoLink();
    if (!seoLink) {
        await navigateTo("/404", { replace: true });
        loading.value = false;
        return;
    }

    try {
        const categoryId = Number(await getCategoryIdBySeoLink({ seoLink }));
        if (!Number.isInteger(categoryId) || categoryId <= 0) {
            await navigateTo("/404", { replace: true });
            return;
        }

        cat.value = String(categoryId);
        await navigateTo(
            {
                path: route.path,
                query: {
                    ...route.query,
                    cat: String(categoryId)
                }
            },
            { replace: true }
        );
    } catch {
        await navigateTo("/404", { replace: true });
    } finally {
        loading.value = false;
    }
};

useSeo({
    title: computed(() => t("商品列表")),
    canonical: computed(() => route.path),
    robots: "index,follow"
});

await resolveCategory();

watch(
    () => `${getSeoLink()}|${String(getFirstQueryValue(route.query.cat) || "")}`,
    (newValue, oldValue) => {
        if (newValue !== oldValue) {
            void resolveCategory();
        }
    }
);
</script>
