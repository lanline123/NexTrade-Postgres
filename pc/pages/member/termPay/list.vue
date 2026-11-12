<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader title="账期支付"></CommonHeader>
    <TopJumpBar tltle="账期支付"></TopJumpBar>
    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
<!--            <div class="title-or-tabs">-->
<!--                <span>{{ $t("账期支付") }}</span>-->
<!--            </div>-->

            <div class="page-info-body" v-loading="loading">
                <!-- 额度信息 -->
                <div class="termPay-summary" v-if="creditDetail">
                    <div class="termPay-summary__left">
                        <div class="summary-item">
                            <div class="label">{{ $t("信用额度") }}</div>
                            <div class="value">
                                <FormatPrice v-model="creditDetail.creditLimit" :showText="false" :fontStyle="moneyFontStyle"></FormatPrice>
                            </div>
                        </div>
                        <div class="summary-item active-item">
                            <div class="label">{{ $t("剩余额度") }}</div>
                            <div class="value">
                                <FormatPrice v-model="creditDetail.availableAmount" :showText="false" :fontStyle="moneyFontStyleV2"></FormatPrice>
                            </div>
                        </div>
                    </div>
                    <div class="termPay-summary__right">
                        <div class="days-item">
                            <div class="label">{{ $t("账期") }}</div>
                            <div class="value">{{ settlementDaysText }}</div>
                        </div>
                        <div class="days-item active-item">
                            <div class="label">{{ $t("账期剩余") }}</div>
                            <div class="value">{{ remainingDaysText }}</div>
                        </div>
                    </div>
                </div>
                <div class="termPay-info">
                    <!-- tabs -->
                    <div class="termPay-tabs" v-if="creditDetail">
                        <div class="tig-tabs" style="justify-content: flex-start; flex: 1">
                            <div
                                v-for="tab in tabList"
                                :key="tab.status"
                                class="tab"
                                :class="currentStatus === tab.status ? 'active' : ''"
                                @click="currentStatus = tab.status"
                            >
                                <p>
                                    {{ $t(tab.label) }}
                                </p>
                            </div>
                        </div>
                        <el-button type="primary" @click="showSettlementDialog = true">{{ $t("结算账号") }}</el-button>
                    </div>

                    <!-- 订单列表 -->
                    <div class="termPay-list" v-if="creditDetail">
                        <template v-if="currentOrders.length">
                            <div v-for="order in currentOrders" :key="order.orderId">
                                <template v-if="order.items && order.items.length">
                                    <div class="termPay-card">
                                        <div class="memberOrder_top">
                                            <div class="memberOrder_top_left">
                                                <div class="status" :class="{ overdue: order.riskStatus === 1 }">
                                                    {{ order.riskStatusText || order.settlementStatusText }}
                                                </div>
                                            </div>
                                        </div>
                                        <div class="termPay-card__header">
                                            <div class="time fs_14">
                                                {{ $t("订单号") }}：<NuxtLink :to="'/member/order/info?id=' + order.orderId" target="_blank"
                                                    ><span>{{ order.orderSn }}</span></NuxtLink
                                                >
                                            </div>
                                            <div>{{ $t("共") + order.totalQuantity + $t("件") }}:
                                                <FormatPrice v-model="order.creditAmount" :showText="false" :fontStyle="amountFontStyle"></FormatPrice>
                                            </div>
                                        </div>
                                        <div class="termPay-card__goods" v-if="order.items && order.items.length">
                                            <div
                                                v-for="good in order.items"
                                                :key="good.productId"
                                                class="good-item"
                                                @click="goOrderDetail(order.orderId)"
                                            >
                                                <div class="good-img">
                                                    <el-image
                                                        :src="productImage(good)"
                                                        loading="lazy"
                                                        style="width: 60px; height: 60px; border-radius: 4px" :alt="good.productName || ''" 
                                                    />
                                                </div>
                                                <div class="good-info">
                                                    <div class="good-name text-ellipsis-2">{{ good.productName }}</div>
                                                    <div class="sku-value">{{ good.skuValue }}</div>
                                                </div>
                                                <div class="good-price">
                                                    <FormatPrice v-model="good.price" :showText="false" :fontStyle="amountFontStyle"></FormatPrice>
                                                </div>
                                                <div class="good-qty">x{{ good.quantity }}</div>
                                            </div>
                                        </div>
                                    </div>
                                </template>

                            </div>
                        </template>
                        <template v-else>
                            <div class="no-result" v-if="!loading">
                                <div>
                                    <span class="sign sign1"></span>
                                </div>
                                <div class="tip-message">
                                    <div>{{ $t("暂无订单") }}</div>
                                </div>
                            </div>
                        </template>
                    </div>
                </div>

                <!-- 错误/无授信兜底 -->
                <div class="no-result" v-if="!loading && !creditDetail">
                    <div>
                        <span class="sign sign1"></span>
                    </div>
                    <div class="tip-message">
                        <div>{{ errorText || $t("暂无账期信息") }}</div>
                    </div>
                </div>
            </div>
        </div>
    </CommonPageContainer>

    <CommonPageFooter></CommonPageFooter>

    <!-- 结算账号弹窗 -->
    <SettlementDialog v-model="showSettlementDialog" />
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
import { getUserCreditDetail } from "~/api/user/termPay";
import type { UserCreditDetailVO, UserCreditOrderItem } from "~/types/user/termPay";
import SettlementDialog from "./src/SettlementDialog.vue";

definePageMeta({
    middleware: "auth"
});

const loading = ref(false);
const errorText = ref("");
const creditDetail = ref<UserCreditDetailVO | null>(null);
const showSettlementDialog = ref(false);
const { t } = useI18n();

// tabs：0-未结算 1-已结算
const currentStatus = ref<0 | 1>(0);

const tabList = computed(() => {
    const pendingNum = creditDetail.value?.pendingOrders?.length || 0;
    const settledNum = creditDetail.value?.settledOrders?.length || 0;
    return [
        { label: "未结算", status: 0 as const, num: pendingNum },
        { label: "已结算", status: 1 as const, num: settledNum }
    ];
});

const currentOrders = computed<UserCreditOrderItem[]>(() => {
    if (!creditDetail.value) return [];
    return currentStatus.value === 0 ? creditDetail.value.pendingOrders || [] : creditDetail.value.settledOrders || [];
});

const settlementDaysText = computed(() => {
    const days = creditDetail.value?.settlementDays;
    return typeof days === "number" ? `${days}${t("天")}` : days ? `${days}${t("天")}` : "-";
});
const remainingDaysText = computed(() => {
    const days = creditDetail.value?.remainingDays;
    return typeof days === "number" ? `${days}${t("天")}` : days ? `${days}${t("天")}` : "-";
});

const moneyFontStyle = { fontSize: "18px", color: "#1a1a1a", fontWeight: "700" };
const amountFontStyle = { fontSize: "16px", color: "#000", fontWeight: "700" };

const moneyFontStyleV2 = { fontSize: "18px", color: "var(--general)", fontWeight: "700" };

const loadData = async () => {
    loading.value = true;
    errorText.value = "";
    try {
        creditDetail.value = await getUserCreditDetail();
    } catch (e: any) {
        creditDetail.value = null;
        errorText.value = e?.message || e?.msg || "加载失败";
    } finally {
        loading.value = false;
    }
};

const formatSkuValue = (val?: string) => {
    if (!val) return "";
    return val.replaceAll("|", " ");
};

const goOrderDetail = (orderId: string | number) => {
    if (orderId === undefined || orderId === null || orderId === "") return;
    navigateTo(`/member/order/info?id=${orderId}`);
};

onMounted(() => {
    loadData();
});
</script>

<style scoped lang="less">
@import "/assets/css/member/member";

.page-info-body, .info-row{
    background: transparent !important;
}

.title-or-tabs{
    height: 40px !important;
}

.termPay-info{
    width: calc(100% + 32px);
    transform: translateX(-16px);
    border-radius: 8px;
    padding: 0 16px;
    box-sizing: border-box;
    overflow: hidden;
    background: #fff;
}

.termPay-summary {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    border-bottom: 1px solid #f2f2f2;
    width: calc(100% + 32px);
    transform: translateX(-16px);
    background: #f5f6fa !important;
    padding-bottom: 12px;

    &__left, &__right {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 20px;
        background: #fff;
        padding: 15px 50px;
        border-radius: 8px;
    }

    .summary-item,
    .days-item {
        display: flex;
        gap: 10px;
        align-items: center;
        color: #1a1a1a;
        .label {
            //color: #8c8c8c;
            font-size: 16px;
            font-weight: 600;
        }
        .value {
            font-size: 16px;
            font-weight: 600;
        }
    }
    .active-item{
        .label, .value{
            color: var(--general);
        }
    }
}

.termPay-tabs {
    padding: 12px 0 6px;
    display: flex;
    align-items: center;
    justify-content: space-between;
}

.termPay-card {
    border: 1px solid #f0f1f2;
    border-radius: 8px;
    margin-top: 12px;
    .memberOrder_top {
        display: flex;
        align-items: center;
        justify-content: space-between;
        height: 48px;
        background-color: #f5f6fa;
        padding: 0 16px;
        border-bottom: 1px solid #f0f1f2;
        font-size: 14px;
        .memberOrder_top_left {
            font-size: 16px;
            font-weight: 700;
            line-height: 20px;
            color: #1a1a1a;
        }
        .icon {
            margin-top: 2px;
            margin-left: 3px;
        }
        &:hover {
            a {
                color: var(--general);
            }
            .icon {
                color: var(--general);
            }
        }
    }

    &__header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        color: #8c8c8c;
        font-size: 12px;
        border-bottom: 1px dashed #f0f1f2;
        padding: 14px 14px 10px;
        .status {
            font-weight: 600;
            color: #1a1a1a;
        }
        .overdue {
            color: #ee0a24;
        }
    }

    &__goods {
        padding: 14px 14px 10px;
        .good-item {
            display: flex;
            //align-items: center;
            gap: 12px;
            padding: 10px 0;
            cursor: pointer;
            transition: background-color 0.15s ease;

            &:hover {
                background-color: #fafafa;
            }
            &:not(:last-child) {
                border-bottom: 1px solid #f7f7f7;
            }
            .good-info {
                flex: 1;
                min-width: 0;
                .good-name {
                    font-size: 14px;
                    color: #1a1a1a;
                    line-height: 18px;
                }
                .sku-value{
                    font-size: 14px;
                    line-height: 18px;
                    color: #888b94;
                    margin-top: 4px;
                }
            }
            .good-price{
                min-width: 120px;
                flex-shrink: 0;
            }
            .good-qty {
                color: #8c8c8c;
                font-size: 12px;
                flex-shrink: 0;
            }
        }
    }

    &__footer {
        display: flex;
        justify-content: space-between;
        align-items: center;
        color: #8c8c8c;
        font-size: 12px;
        padding: 14px 14px 10px;
        .amount {
            display: inline-flex;
            gap: 6px;
            align-items: baseline;
        }
    }
}

.fs_14 {
    font-size: 14px;
}

.text-ellipsis-2 {
    display: -webkit-box;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
}
</style>
