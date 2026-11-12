<template>
    <div :class="'buy ' + (disabled == true ? 'disabled' : '')" @click="_addToCart">
        <slot></slot>
    </div>
</template>
<script lang="ts" setup>
import { ref, toRefs, onMounted } from "vue";
import { addToCart } from "~/api/product/product";
import { message } from "ant-design-vue";
import { useRouter } from "vue-router";
const router = useRouter();
const { t } = useI18n();
const props = defineProps({
    id: Number,
    number: { type: Number, default: 1 },
    skuId: { type: Number, default: 0 },
    disabled: { type: Boolean, default: false },
    isQuick: { type: Boolean, default: false }
});
const loading = ref(false);
const { disabled, number } = toRefs(props);
const _addToCart = async () => {
    if (disabled.value) return false;
    try {
        if (loading.value == true) {
            return;
        }
        loading.value = true;
        const result = await addToCart({
            id: props.id,
            number: number.value,
            skuId: props.skuId,
            isQuick: props.isQuick ? 1 : 0
        });
        if (props.isQuick == true) {
            router.push("/order/check/");
        } else {
            message.success(t("加入购物车成功"));
        }
    } catch (error) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
    return;
};
</script>
<style lang="less" scoped>
.buy {
    display: inline-flex;
}
</style>
