<template>
    <div :class="['buy', { disabled: disabled == true, 'buy--overseas': isOverseasUi }]" @click="_addToCart">
        <slot></slot>
    </div>
</template>
<script lang="ts" setup>
import { ref, computed, unref, inject } from "vue";
import { addExchangeToCart, addToCart } from "~/api/product/product";
import { message } from "ant-design-vue";
import { useRouter } from "vue-router";
import { useCartStore } from "~/store/cart";
import { useCommonStore } from "~/store/common";

// 跨境版 UI 开关（由商详页 Product.vue provide 下发；非跨境时默认原逻辑）
const detailUiVersion = inject<any>("detailUiVersion", null);
const isOverseasUi = computed(() => unref(detailUiVersion) === "overseas");

const cartStore = useCartStore();

const commonStore = useCommonStore();

const router = useRouter();

const props = defineProps({
    id: Number,
    number: { type: Number },
    skuId: { type: Number },
    extraAttrIds: { type: String, default: "" },
    disabled: { type: Boolean, default: false },
    isQuick: { type: Boolean, default: false },
    productType: { type: String, default: "product" },
    skuItem: { type: Array },
    grouponDetail: { // 拼团信息
        type: Object,
        default: () => {
            return {};
        }
    },
    shopId: {
        type: Number,
        default: null
    },
    // 是否周期购
    isCycleInfo: {
        type: Boolean,
        default: false
    }
});
const route = useRoute();
const { t } = useI18n();
const emit = defineEmits(["callback"]);
const loading = ref(false);

const handleToMobileProduct = (item: any) => {
    let url = `/pcMobile?path=/pages/product/index&id=${item.id}`;
    window.open(url, "_blank");
};

const _addToCart = async (e?: Event) => {
    try {
        // 跨境版按钮点击阻止冒泡（原 overseas 版为 @click.stop）
        if (isOverseasUi.value) e?.stopPropagation();
        if (props.disabled) return;

        if (loading.value) return;

        if(props.isCycleInfo) {
            handleToMobileProduct(props)
            return;
        }

        loading.value = true;
        let result: any = {};
        const filterParams: any = {
            id: props.id ?? 0,
            number: props.number,
            skuId: props.skuId,
            isQuick: props.isQuick ? 1 : 0
        };
        if (props.extraAttrIds) {
            filterParams.extraAttrIds = props.extraAttrIds;
        }
        if (isB2B()) {
            filterParams.skuItem = props.skuItem;
        }
        let grouponData = {}
        if (props.grouponDetail && props.grouponDetail.productId) {
            grouponData = {
                grouponPromotionId: props.grouponDetail.grouponPromotionId,
                grouponPromotionItemId: props.grouponDetail.grouponPromotionItemId,
                groupNumKey: props.grouponDetail.groupNumKey,
                grouponDetailId: props.grouponDetail.grouponPromotionDetails?.find((item: any) => item.skuId === props.extraAttrIds)?.grouponDetailId || props.grouponDetail.grouponPromotionDetails[0]?.grouponDetailId,
                price: props.grouponDetail.price,
                grouponPromotionRecordId: props.grouponDetail.grouponPromotionRecordId,
            }
        }
        if (props.productType == "product") {
            result = await addToCart({ ...filterParams, grouponData });
        } else if (props.productType == "exchange") {
            result = await addExchangeToCart(filterParams);
        }
        if (props.isQuick == true) {
            if (commonStore.closeOrder === 1) return message.error(t("商城已关闭下单"));

            router.push({
                path: "/order/check",
                query: {
                    flowType: result.flowType,
                    ...(props.skuId ? { skuId: props.skuId } : {})
                }
            });
        } else {
            cartStore.changeCartList();
            message.success(t("加入购物车成功"));
            emit("callback");
        }
    } catch (error: any) {
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
// 跨境版：买按钮占位 50%（两按钮并排）
.buy.buy--overseas {
    width: 50%;
}
</style>
