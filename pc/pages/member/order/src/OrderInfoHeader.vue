<!--
  订单详情 · 头部信息栏
  - 订单号、下单门店（o2o）、订单状态
  - 订单状态描述提示（含自提/预约门店文案）
  - 右侧操作按钮：取消订单 / 付款 / 支付尾款 / 确认收货 / 整单售后
-->
<template>
    <div class="page-info-body lr">
        <div class="order-left">
            <div>
                <span
                    >{{ $t("订单号") }}：<strong>{{ formState.orderSn }}</strong></span
                >
                <!-- 门店 start：下单门店链接（o2o，融合后此处整体切割） -->
                <span style="margin-left: 40px" v-if="formState.shop && formState.shop?.shopTitle"
                    >{{ $t("下单门店") }}：<NuxtLink :to="{ path: '/store/info', query: { shopId: formState.shopId } }" target="_blank"
                        ><span class="font-color">{{ formState.shop?.shopTitle }}</span>
                    </NuxtLink></span
                >
                <!-- 门店 end -->
                <span style="margin-left: 40px"
                    >{{ $t("订单状态") }}：
                    <template v-if="[1, 3].includes(formState?.joinStatus)">
                        <strong>
                            {{ formState.joinStatusDesc || "" }}
                        </strong>
                    </template>
                    <template v-else>
                        <strong>
                            {{ formState.orderStatusName || "" }}
                            <span v-if="formState.joinStatusDesc && formState.joinStatus !== 1"> ({{ formState.joinStatusDesc }}) </span>
                        </strong>
                    </template>
                </span>
            </div>
            <div v-if="formState.cancelRemark?.trim()">
                {{ $t("取消原因") }}：{{ formState.cancelRemark }}
            </div>
            <template v-if="isGiftCardProduct">
                <div>{{ $t("为了您的财产安全，请妥善保管卡密信息，切勿将卡号、卡密、二维码截图等任何信息发送他人，谨防上当受骗!") }}</div>
            </template>
            <!-- 认款待审核/驳回优先于支付倒计时展示，与移动端保持一致 -->
            <template v-if="isOfflineOrder && offlineClaimStatus === 0">
                <div>{{ $t("您已提交汇款凭证，等待财务人员核对流水，我们尽快完成审核。") }}</div>
            </template>
            <template v-else-if="isOfflineClaimRejected">
                <div class="offline-claim-rejected">
                    {{ $t("认款审核未通过") }}<span v-if="formState.offlinePaymentClaimRejectReason">：{{ formState.offlinePaymentClaimRejectReason }}</span>
                </div>
            </template>
            <template v-else-if="showPayCountdown">
                <div class="order-status-sub">
                    {{ $t("还剩") }} <span class="countdown-text">{{ payCountdownText }}</span> {{ $t("订单自动取消") }}
                </div>
            </template>
            <template v-else-if="isOfflineOrder && offlineClaimStatus === 1 && offlineClaimAuditRemark">
                <div>{{ $t("审核备注") }}：{{ offlineClaimAuditRemark }}</div>
            </template>
            <template v-else-if="isPendingPay">
                <div>{{ $t("请尽快完成支付，以免订单被自动取消") }}</div>
            </template>
            <template v-else-if="formState.cycleCardInfo">
                <div v-if="formState.cycleCardInfo.expireTimeStr">
                    {{
                        formState.cycleCardInfo.expireTimeStr === "长期有效"
                            ? $t("长期有效")
                            : $t("有效期至：{0}", [formState.cycleCardInfo.expireTimeStr])
                    }}
                </div>
            </template>
            <template v-else-if="[1, 2].includes(formState.orderStatus) && formState.autoDeliveryDays > 0">
                <div>{{ $t("订单将在") }}{{ formState.autoDeliveryDays }}{{ $t("天后自动收货") }}</div>
            </template>
            <!-- 预售订单待付尾款提醒 -->
            <template v-else-if="formState.isPresale && formState.presaleInfo?.presaleStatus === 1 && formState.orderStatus === 1">
                <div>{{ $t("请在") }}{{ formState.presaleInfo?.finalPaymentStartTimeText }}{{ $t("前支付尾款，超时将自动取消订单") }}</div>
            </template>
            <!-- 门店 start：自提订单对应的订单状态描述（o2o） -->
            <template v-else-if="formState.isPickup && formState.orderType === 1">
                <div>{{ $t(pickupOrderStatusInfo[formState.orderStatus] || "") }}</div>
            </template>
            <!-- 预约订单对应的订单状态描述（o2o） -->
            <template v-else-if="(formState.orderType === 10 || formState.orderType === 9) && formState.orderStatus === 1">
                <div>{{ desc }}</div>
            </template>
            <!-- 门店 end -->
            <template v-else>
                <div>{{ $t(oStatusInfo[formState.orderStatus] || "") }}</div>
            </template>
        </div>
        <div class="order-right">
            <el-button v-if="formState.availableActions.cancelOrder" @click="emit('cancel-order')">{{ $t("取消订单") }}</el-button>
            <el-button v-if="formState.availableActions.offlinePaymentClaimDetail && !isOfflineClaimRejected" @click="emit('claim-detail')">{{ $t("认款详情") }}</el-button>
            <!-- 付款按钮：预售订单待付定金时显示，待付尾款时不显示 -->
            <div
                v-if="
                    formState.availableActions.toPay &&
                    canShowToPay &&
                    (!formState.isPresale || (formState.isPresale && formState?.presaleInfo?.presaleStatus === 0))
                "
            >
                <el-button v-if="isOfflineClaimRejected" type="primary" @click="emit('claim-edit')">{{ $t("修改申请") }}</el-button>
                <NuxtLink v-else :to="'/order/pay/?id=' + formState.orderId" target="_balnk">
                    <el-button type="primary">{{ $t("付款") }}</el-button>
                </NuxtLink>
            </div>
            <!-- 支付尾款按钮：预售订单待付尾款时显示 -->
            <NuxtLink
                v-if="formState.availableActions.toPay && canShowToPay && formState.presaleInfo?.saleMode === 2 && formState.presaleInfo?.presaleStatus === 1"
                target="_blank"
                :to="'/order/pay/?id=' + formState.orderId"
            >
                <el-button type="primary">{{ $t("支付尾款") }}</el-button>
            </NuxtLink>
            <el-button
                type="primary"
                v-if="formState.availableActions.confirmReceipt && formState.orderType !== 9 && formState.orderType !== 10"
                @click="emit('confirm-receipt')"
                >{{ $t("确认收货") }}</el-button
            >
            <NuxtLink
                v-if="
                    formState.availableActions.toAftersales &&
                    !formState.afterSalesIng &&
                    !hasItemAftersales &&
                    !giftCardAftersalesBlocked &&
                    !isGiftCardExchangeOrder &&
                    formState.orderSource !== 'pos' &&
                    formState.orderType !== 14 &&
                    formState.isCycleOrder !== 1 &&
                    !isPickupCompleted &&
                    formState.cycleCardInfo?.statusName !== '已过期'
                "
                target="_blank"
                :to="'/member/return/Info?orderId=' + formState.orderId"
            >
                <el-button>{{ $t("整单售后") }}</el-button>
            </NuxtLink>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { onMounted, onUnmounted, ref, computed } from "vue";
import type { PropType } from "vue";
import type { OrderFormState } from "~/types/user/order.d";
import { isGiftCardAftersalesBlocked } from "~/utils/giftCard";
import { canShowOrderPayment } from "~/utils/orderPayment";
const props = defineProps({
    formState: {
        type: Object as PropType<OrderFormState>,
        required: true
    }
});

const emit = defineEmits(["cancel-order", "confirm-receipt", "claim-detail", "claim-edit", "refresh"]);

const { t } = useI18n();

/** 判断是否为礼品卡商品（实体卡type=7/电子卡type=8） */
const isGiftCardProduct = computed(() => {
    return props.formState.items?.some((item) => item.productType === 7 || item.productType === 8);
});
const giftCardAftersalesBlocked = computed(() => isGiftCardAftersalesBlocked(props.formState));
/** 判断是否为礼品卡兑换订单（orderType=12） */
const isGiftCardExchangeOrder = computed(() => {
    return props.formState.orderType === 12;
});
/** 任一商品已有售后记录时，不再展示整单售后入口 */
const hasItemAftersales = computed(() => {
    return props.formState.items?.some((item) => Boolean(item.aftersalesItem)) ?? false;
});
// 自提订单已完成时不允许申请售后
const isPickupCompleted = computed(() => {
    return props.formState.isPickup && props.formState.orderStatus === 5;
});
const isOfflineOrder = computed(() => Number(props.formState.payTypeId) === 3);
const canShowToPay = computed(() => canShowOrderPayment(props.formState));
/** 认款状态统一为数字或 null，避免 Number(null) === 0 把未提交认款误判为待审核 */
const offlineClaimStatus = computed(() => {
    const status = props.formState.offlinePaymentClaimStatus;
    return status === null || status === undefined ? null : Number(status);
});
const offlineClaimAuditRemark = computed(() => String((props.formState as any).offlinePaymentClaim?.auditRemark || "").trim());
/** 认款被驳回且订单未取消时才允许修改申请 */
const isOfflineClaimRejected = computed(() => isOfflineOrder.value && offlineClaimStatus.value === 2 && Number(props.formState.orderStatus) !== 3);

const oStatusInfo = ref([
    "您的订单已提交成功，请尽快完成付款哦！",
    "感谢您在本店的购物，您的订单已提交成功，我们会尽快为您发货！",
    "您的订单已经发货，请注意查收，祝您购物愉快！",
    '您的订单已被取消，您可以通过"再次购买"，重新购买订单中的商品。',
    '您的订单被认定为无效订单，订单已被关闭。您可以通过"再次购买"，重新购买订单中的商品。给您造成的不便请多多谅解！',
    "您的订单已经确认收货，感谢您在本店购物，祝您生活愉快！"
]);

const pickupOrderStatusInfo = ref(["您的订单已提交成功，请尽快完成支付哦！", "已完成支付，商家正在备货", "商家备货已完成，请按照商家营业时间进行提货"]);

// ============ 顶部状态头：待支付倒计时 ============
const nowTs = ref(Date.now());
const payEndTime = computed(() => String((props.formState as any)?.payEndTime || ""));
const isPendingPay = computed(() => !!(props.formState as any)?.availableActions?.toPay && canShowToPay.value);
const payEndTimeMs = computed(() => {
    const raw = payEndTime.value;
    if (!raw) return 0;
    const ms = Date.parse(raw.replace(/-/g, "/"));
    return Number.isFinite(ms) ? ms : 0;
});
const showPayCountdown = computed(() => isPendingPay.value && payEndTimeMs.value > nowTs.value);
const payCountdownText = computed(() => {
    const diff = payEndTimeMs.value - nowTs.value;
    if (diff <= 0) return "";
    const totalSec = Math.floor(diff / 1000);
    const h = Math.floor(totalSec / 3600);
    const m = Math.floor((totalSec % 3600) / 60);
    const s = totalSec % 60;
    const pad = (n: number) => (n < 10 ? `0${n}` : `${n}`);
    return `${pad(h)}:${pad(m)}:${pad(s)}`;
});
let countdownTimer: ReturnType<typeof setInterval> | null = null;
let refetchedOnZero = false;
onMounted(() => {
    countdownTimer = setInterval(() => {
        if (!isPendingPay.value || payEndTimeMs.value <= 0) return;
        nowTs.value = Date.now();
        if (payEndTimeMs.value - nowTs.value <= 0 && !refetchedOnZero) {
            refetchedOnZero = true;
            emit("refresh");
        }
    }, 1000);
});
onUnmounted(() => {
    if (countdownTimer) {
        clearInterval(countdownTimer);
        countdownTimer = null;
    }
});

const desc = computed(() => {
    let str = "";
    if (props.formState.afterSalesIng) {
        str = t("请前往售后详情查看售后进度");
    } else if (props.formState.orderType === 9) {
        str = t("请在 {0} {1}~{2} 准时到店消费", [
            props.formState.orderReservation.reservationDate,
            props.formState.orderReservation.startTime,
            props.formState.orderReservation.endTime
        ]);
    } else if (props.formState.orderType === 10) {
        str = t("您的服务预约在 {0} {1}~{2}，如需更改上门服务改时间请提前联系商家进行沟通", [
            props.formState.orderReservation.reservationDate,
            props.formState.orderReservation.startTime,
            props.formState.orderReservation.endTime
        ]);
    }

    return str;
});
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.order-status-sub {
    .countdown-text {
        color: #cc0c1c;
        font-weight: 600;
    }
}
.offline-claim-rejected { color: #666; }

.lr {
    display: flex;

    .order-left {
        flex: 1;
        line-height: 3;

        & > div {
            display: flex;
        }
    }

    .order-right {
        display: flex;
        gap: 10px;
        flex-wrap: wrap;
        width: 200px;
        justify-content: flex-end;
    }
}
</style>
