<!--
  订单详情页面（会员中心）
  - 展示订单完整信息：订单号、状态、商品列表、收货地址、支付方式、配送方式
  - 显示订单状态流程图（下单、付款、发货、收货、完成）
  - 提供订单操作：取消订单、付款、确认收货、申请售后、查看物流
  - 支持自提订单和预约订单的特殊状态展示
  - 显示订单金额明细：商品金额、运费、优惠、实付金额
  - 页面按区块拆分为 src/ 下的子组件（Header/Steps/DeliveryInfo/GiftCard/Ecard/ProductTable）
-->
<template>
    <CommonPageHeader
        :showService="showService"
        :phone="formState.shop ? formState.shop.kefuPhone : ''"
        :orderId="formState.orderId"
        :shopId="formState.shopId"
    ></CommonPageHeader>
    <CommonHeader title="订单详情"></CommonHeader>
    <TopJumpBar tltle="订单详情"></TopJumpBar>
    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="title-or-tabs right-info">
                <span>{{ $t("订单详情") }}</span>
                <div v-if="!formState.availableActions.splitOrder">
                    <NuxtLink :to="'/member/feedback/askList?orderId=' + formState.orderId + '&orderSn=' + formState.orderSn">
                        <el-button>{{ $t("订单留言/咨询") }}</el-button>
                    </NuxtLink>
                </div>
            </div>
            <div class="order-info">
                <OrderInfoHeader
                    :formState="formState"
                    @cancel-order="onCancelOrder"
                    @confirm-receipt="onConfirmReceipt"
                    @claim-detail="openClaimDetail"
                    @claim-edit="openClaimEdit"
                    @refresh="fetchOrder"
                ></OrderInfoHeader>
                <OrderStatusSteps :formState="formState"></OrderStatusSteps>
                <OrderDeliveryInfo :formState="formState"></OrderDeliveryInfo>
                <OrderGiftCard v-if="showGiftCard" :formState="formState" @copy-all="handleGiftCardCopyAll"></OrderGiftCard>
                <OrderEcard v-if="showEcard" :formState="formState"></OrderEcard>
                <OrderProductTable
                    :formState="formState"
                    :copiedCardNumber="copiedCardNumber"
                    :copiedCardPassword="copiedCardPassword"
                ></OrderProductTable>
            </div>
        </div>
    </CommonPageContainer>

    <OfflinePaymentClaimDetail v-model="claimDetailVisible" :order-id="Number(formState.orderId || 0)" :editable="claimDetailEditable" @submitted="fetchOrder" />

    <CommonPageFooter></CommonPageFooter>
    <CancelOrderReasonDialog ref="cancelOrderReasonDialogRef" @success="fetchOrder" />
</template>
<script lang="ts" setup>
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import type { OrderFormState } from "~/types/user/order.d";
import { getOrder, confirmReceipt } from "~/api/user/order";
import { message, Modal } from "ant-design-vue";
import { getShopDetail } from "~/api/product/product";
import OrderInfoHeader from "~/pages/member/order/src/OrderInfoHeader.vue";
import OrderStatusSteps from "~/pages/member/order/src/OrderStatusSteps.vue";
import OrderDeliveryInfo from "~/pages/member/order/src/OrderDeliveryInfo.vue";
import OrderGiftCard from "~/pages/member/order/src/OrderGiftCard.vue";
import OrderEcard from "~/pages/member/order/src/OrderEcard.vue";
import OrderProductTable from "~/pages/member/order/src/OrderProductTable.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
import OfflinePaymentClaimDetail from "~/components/member/order/OfflinePaymentClaimDetail.vue";

import CancelOrderReasonDialog from "~/components/member/order/CancelOrderReasonDialog.vue";

definePageMeta({
    middleware: "auth"
});

const { t } = useI18n();
const router = useRouter();
const formState = ref<OrderFormState>({
    orderStatus: 0,
    availableActions: {}
} as OrderFormState);
const loading = ref<boolean>(true);
const claimDetailVisible = ref(false);
const claimDetailEditable = ref(false);
const showEcard = computed(() => {
    let value = false;
    formState.value.items?.forEach((item) => {
        if (item.eCard && item.eCard.length > 0) {
            value = true;
        }
    });

    return value;
});

/** 判断是否显示礼品卡信息区块（电子礼品卡且已生成卡信息） */
const showGiftCard = computed(() => {
    return formState.value.items?.some((item) => item.productType === 8) && (formState.value.giftCardOrder?.num || 0) > 0;
});

// 存储一键复制的卡号和卡密
const copiedCardNumber = ref("");
const copiedCardPassword = ref("");
/** 礼品卡一键复制回调，保存卡号卡密供后续操作使用 */
const handleGiftCardCopyAll = (cardNumber: string, cardPassword: string) => {
    copiedCardNumber.value = cardNumber;
    copiedCardPassword.value = cardPassword;
};

const fetchOrder = async () => {
    try {
        const result = await getOrder({ id: router.currentRoute.value.query.id });
        Object.assign(formState.value, result);
        await getShop(result.shopId);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
fetchOrder();

const shopInfo = ref();
const getShop = async (id: number) => {
    try {
        const result = await getShopDetail(id);
        shopInfo.value = result;
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};

const showService = computed(() => {
    const info = shopInfo.value || {};
    if (typeof info === "object" && Object.keys(info).length > 0) {
        if (Number(info.shopId) === 0) return true;
        return Array.isArray(info.kefuInlet) && info.kefuInlet.includes(2);
    } else {
        return null;
    }
});

const cancelOrderReasonDialogRef = ref<{ open: (id: number) => void }>();
const onCancelOrder = () => {
    cancelOrderReasonDialogRef.value?.open(Number(formState.value.orderId));
};
/**
 * 确认收货
 */
const onConfirmReceipt = async () => {
    Modal.confirm({
        title: t("确认要收货吗"),
        cancelText: t("取消"),
        okText: t("确定"),
        maskClosable: true,
        centered: true,
        onOk: async () => {
            try {
                const result = await confirmReceipt({ id: formState.value.orderId });
                await fetchOrder();
            } catch (error: any) {
                message.error(error.message);
            } finally {
            }
        }
    });
};
const openClaimDetail = () => {
    navigateTo(`/member/order/claimInfo?orderId=${Number(formState.value.orderId || 0)}`);
};
const openClaimEdit = () => {
    claimDetailEditable.value = true;
    claimDetailVisible.value = true;
};
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.right-info {
    justify-content: space-between;
    align-items: center;
}

.order-info {
    display: flex;
    flex-direction: column;
    gap: 10px;
}
</style>
