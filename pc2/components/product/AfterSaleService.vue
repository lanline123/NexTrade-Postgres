<template>
    <div>
        <div class="tab_body">
            <p v-html="formState.templateContent"></p>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { reactive, ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { getAfterSaleService } from "~/api/product/product";
import type { ProductConsultationItem, ProductConsultationFormState } from "~/types/product/product.d";
const route = useRoute();
const id = ref(Number(route.params.id));
const loading = ref(false);
const formState = ref<any>({});
const fetchAfterSaleService = async () => {
    try {
        const result = await getAfterSaleService(id.value);
        Object.assign(formState.value, result);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
fetchAfterSaleService();
</script>

<style lang="less" scoped>
.tab_body {
    text-align: start;
    color: #333;
    font-family: 微软雅黑, 微软雅黑;
}
</style>
