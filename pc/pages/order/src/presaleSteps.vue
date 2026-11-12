<template>
    <div class="presale-steps-box">
        <!-- 定金预售步骤 -->
        <div class="presale-steps-container" v-if="isDepositPresale">
            <!-- 步骤1：定金 -->
            <div class="presale-step-item">
                <div class="presale-step-content">
                    <div
                        class="presale-step-dot"
                        :class="{
                            active: currentStatus >= 0,
                            completed: currentStatus >= 0
                        }"
                    ></div>
                    <div
                        class="presale-step-text"
                        :class="{
                            active: currentStatus === 0
                        }"
                    >
                        {{ $t("定金") }}({{ $t("预售商品，定金不退") }})
                        <el-popover
                            placement="bottom"
                            :width="500"
                            trigger="hover"
                            :show-after="200"
                            :hide-after="200"
                            popper-style="box-shadow: rgb(14 18 22 / 35%) 0px 10px 38px -10px, rgb(14 18 22 / 20%) 0px 10px 20px -15px; padding: 20px;"
                            :teleported="false"
                        >
                            <template #reference>
                                <span class="presale-text"
                                    ><span style="color: #999">|</span>
                                    <span style="color: #989898; padding: 0">{{ $t("规则") }}</span>
                                    <el-icon style="margin-left: 5px; margin-top: 2px" size="14" color="#999"> <QuestionFilled /> </el-icon
                                ></span>
                            </template>
                            <template #default>
                                <div class="presale-rule-content">
                                    <div class="rule-section">
                                        <div class="rule-title">{{ $t("下单规则") }}</div>
                                        <div class="rule-item">
                                            <span class="rule-number">1、</span>
                                            <span>{{
                                                $t(
                                                    "预售商品，有营销（会员价、优惠券等）时自动推荐使用，若为定金预售则在营销（会员价、优惠券等）的基础上计算尾款，付尾款时以定金的价格为基础计算尾款金额；"
                                                )
                                            }}</span>
                                        </div>
                                        <div class="rule-item">
                                            <span class="rule-number">2、</span>
                                            <span>{{ $t("订单优惠信息将在尾款时计算") }}</span>
                                        </div>
                                    </div>
                                    <div class="rule-section">
                                        <div class="rule-title">{{ $t("售后规则") }}</div>
                                        <div class="rule-item">
                                            <span class="rule-number">1、</span>
                                            <span>{{ $t("全款预售商品，支付预售款后可以正常发起售后流程；") }}</span>
                                        </div>
                                        <div class="rule-item">
                                            <span class="rule-number">2、</span>
                                            <span>{{ $t("定金预售商品，未付尾款的订单到期也可支付尾款；付尾款后，可正常申请售后流程") }}</span>
                                        </div>
                                    </div>
                                </div>
                            </template>
                        </el-popover>
                    </div>
                </div>
                <div class="presale-step-line" :class="{ active: currentStatus > 0 }"></div>
            </div>
            <!-- 步骤2：尾款 -->
            <div class="presale-step-item">
                <div class="presale-step-content">
                    <div
                        class="presale-step-dot"
                        :class="{
                            active: currentStatus >= 1,
                            completed: currentStatus >= 1
                        }"
                    ></div>
                    <div
                        class="presale-step-text"
                        :class="{
                            active: currentStatus === 1
                        }"
                    >
                        {{ $t("尾款") }}({{ paymentTimeText }})
                    </div>
                </div>
                <div class="presale-step-line" :class="{ active: currentStatus > 1 }"></div>
            </div>
            <!-- 步骤3：发货 -->
            <div class="presale-step-item">
                <div class="presale-step-content">
                    <div
                        class="presale-step-dot"
                        :class="{
                            active: currentStatus >= 2,
                            completed: currentStatus >= 2
                        }"
                    ></div>
                    <div
                        class="presale-step-text"
                        :class="{
                            active: currentStatus === 2
                        }"
                    >
                        {{ depositPresaleStepThreeText }}
                    </div>
                </div>
            </div>
        </div>

        <!-- 全款预售步骤 -->
        <div class="presale-steps-container" v-if="!isDepositPresale">
            <!-- 步骤1：全款预售 -->
            <div class="presale-step-item">
                <div class="presale-step-content">
                    <div
                        class="presale-step-dot"
                        :class="{
                            active: currentStatus >= 0,
                            completed: currentStatus >= 0
                        }"
                    ></div>
                    <div
                        class="presale-step-text"
                        :class="{
                            active: currentStatus === 0
                        }"
                    >
                        <span style="display: flex; align-items: flex-end; gap: 4px">
                            {{ $t("全款预售") }}
                            <FormatPrice class="presale-step-amount" :model-value="fullPaymentAmount" :showText="false" :fontStyle="{ fontSize: '15px' }"></FormatPrice
                        ></span>

                        <el-popover
                            placement="bottom"
                            :width="500"
                            trigger="hover"
                            :show-after="200"
                            :hide-after="200"
                            popper-style="box-shadow: rgb(14 18 22 / 35%) 0px 10px 38px -10px, rgb(14 18 22 / 20%) 0px 10px 20px -15px; padding: 20px;"
                            :teleported="false"
                        >
                            <template #reference>
                                <span class="presale-text"
                                    ><span style="color: #999">|</span>
                                    <span style="color: #989898; padding: 0">{{ $t("规则") }}</span>
                                    <el-icon style="margin-left: 5px; margin-top: 2px" size="14" color="#999"> <QuestionFilled /> </el-icon
                                ></span>
                            </template>
                            <template #default>
                                <div class="presale-rule-content">
                                    <div class="rule-section">
                                        <div class="rule-title">{{ $t("下单规则") }}</div>
                                        <div class="rule-item">
                                            <span class="rule-number">1、</span>
                                            <span>{{
                                                $t(
                                                    "预售商品，有营销（会员价、优惠券等）时自动推荐使用，若为定金预售则在营销（会员价、优惠券等）的基础上计算尾款，付尾款时以定金的价格为基础计算尾款金额；"
                                                )
                                            }}</span>
                                        </div>
                                        <div class="rule-item">
                                            <span class="rule-number">2、</span>
                                            <span>{{ $t("订单优惠信息将在尾款时计算") }}</span>
                                        </div>
                                    </div>
                                    <div class="rule-section">
                                        <div class="rule-title">{{ $t("售后规则") }}</div>
                                        <div class="rule-item">
                                            <span class="rule-number">1、</span>
                                            <span>{{ $t("全款预售商品，支付预售款后可以正常发起售后流程；") }}</span>
                                        </div>
                                        <div class="rule-item">
                                            <span class="rule-number">2、</span>
                                            <span>{{ $t("定金预售商品，未付尾款的订单到期也可支付尾款；付尾款后，可正常申请售后流程") }}</span>
                                        </div>
                                    </div>
                                </div>
                            </template>
                        </el-popover>
                    </div>
                </div>
                <div class="presale-step-line" :class="{ active: currentStatus > 0 }"></div>
            </div>
            <!-- 步骤2：发货 -->
            <div class="presale-step-item">
                <div class="presale-step-content">
                    <div
                        class="presale-step-dot"
                        :class="{
                            active: currentStatus >= 1 && currentStatus !== 3,
                            completed: currentStatus >= 1 && currentStatus !== 3
                        }"
                    ></div>
                    <div
                        class="presale-step-text"
                        :class="{
                            active: currentStatus >= 1 && currentStatus !== 3
                        }"
                    >
                        {{ fullPaymentStepTwoText }}
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * @description 预售订单步骤条组件
 * - 支持定金预售和全款预售两种模式
 * - 定金预售：定金 -> 尾款 -> 发货 三步
 * - 全款预售：全款预售 -> 发货 两步
 */
import { computed } from "vue";
import { QuestionFilled } from "@element-plus/icons-vue";

const { t, locale } = useI18n();

const props = defineProps({
    productPresale: { type: Object, default: () => ({}) },
    presaleStatus: { type: Number, default: 0 },
    total: { type: Object, default: () => ({}) },
    deliveryType: { type: Number, default: undefined }
});

const formatDurationText = (days?: number, hours?: number) => {
    const parts: string[] = [];
    if (days) parts.push(`${days}${t("天")}`);
    if (hours) parts.push(`${hours}${t("小时")}`);
    return parts.join("").trim();
};

const joinLocalized = (left: string, right: string) => {
    if (!left) return right;
    const loc = String(locale.value || "").toLowerCase();
    const needSpace = loc && !loc.startsWith("zh");
    return needSpace ? `${left} ${right}` : `${left}${right}`;
};

const fullPaymentAmount = computed(() => {
    const t: any = props.total;
    return t?.totalAmount ?? t?.unpaidAmount ?? t?.productAmount ?? 0;
});

// 判断是否为定金预售（saleMode === 2 为定金预售）
const isDepositPresale = computed(() => {
    return (props.productPresale as any)?.saleMode === 2;
});

// 当前预售状态：定金预售取 presaleStatus；全款预售取 total.orderStatus
const currentStatus = computed(() => {
    if (isDepositPresale.value) {
        return props.presaleStatus ?? 0;
    }
    return (props.total as any)?.orderStatus ?? 0;
});

const presaleDeliveryType = computed(() => {
    if (props.deliveryType !== undefined && props.deliveryType !== null) return Number(props.deliveryType);
    const total: any = props.total || {};
    const shippingTypeName = String(total?.shippingTypeName || "");
    if (total?.isPickup || shippingTypeName.includes("自提")) return 2;
    if (total?.deliveryOrderInfo?.type === 1 || shippingTypeName.includes("同城")) return 3;
    return 1;
});

const pendingDeliveryNodeText = computed(() => {
    if (presaleDeliveryType.value === 2) return t("自提");
    if (presaleDeliveryType.value === 3) return t("配送");
    return t("发货");
});

const deliveredNodeText = computed(() => {
    if (presaleDeliveryType.value === 2 || presaleDeliveryType.value === 3) return t("已备货");
    return t("已发货");
});

const paymentTimeText = computed(() => {
    const info: any = props.productPresale;
    if (!info) return "";
    if (info.finalPaymentType === 1) {
        const start = info.finalPaymentRelativeDays
            ? `${t("付定金")}${info.finalPaymentRelativeDays}${t("天")}${t("后")}${t("开始")}`
            : "";
        const duration = formatDurationText(info.finalPaymentDurationDays, info.finalPaymentDurationHours);
        const end = duration ? joinLocalized(duration, t("结束")) : "";
        if (start && end) return `${start}, ${end}`;
        return start || end || "";
    }
    return info.finalPaymentStartTime ? joinLocalized(info.finalPaymentStartTime, t("开始")) : "";
});

const deliverTimeText = computed(() => {
    const info: any = props.productPresale;
    if (!info) return "";
    if (info.deliveryType === 1) {
        const duration = formatDurationText(info.deliveryRelativeDays, info.deliveryRelativeHours);
        return `${t("付尾款")}${duration}${t("后")}${pendingDeliveryNodeText.value}`.trim();
    }
    return info.deliveryDate ? joinLocalized(String(info.deliveryDate), t("开始")) : "";
});

const fullPaymentDeliverTimeText = computed(() => {
    const info: any = props.productPresale;
    if (!info) return "";
    if (info.deliveryType === 1) {
        const duration = formatDurationText(info.deliveryRelativeDays, info.deliveryRelativeHours);
        return `${t("付全款")}${duration}${t("后")}`.trim();
    }
    return info.deliveryDate ? joinLocalized(String(info.deliveryDate), t("开始")) : "";
});

const formatShippingDate = (dateStr: string) => {
    if (!dateStr) return "";
    const [datePart, timePart] = dateStr.split(" ");
    const [year, month, day] = datePart.split("-");
    return `${year}${t("年")}${month}${t("月")}${day}${t("日")}  ${timePart || ""}`;
};

// 全款预售步骤二文本
const fullPaymentStepTwoText = computed(() => {
    const status = currentStatus.value;
    if (status >= 2 && status !== 3) {
        const shippingTime = (props.total as any)?.shippingTime;
        if (shippingTime) {
            return `${deliveredNodeText.value}(${formatShippingDate(shippingTime)})`;
        }
        return `${deliveredNodeText.value}`;
    }
    return `${pendingDeliveryNodeText.value}(${fullPaymentDeliverTimeText.value})`;
});

// 定金预售步骤三文本
const depositPresaleStepThreeText = computed(() => {
    const orderStatus = (props.total as any)?.orderStatus ?? 0;
    if (orderStatus >= 2 && orderStatus !== 3 && props.presaleStatus === 2) {
        const shippingTime = (props.total as any)?.shippingTime;
        if (shippingTime) {
            return `${deliveredNodeText.value}(${formatShippingDate(shippingTime)})`;
        }
        return `${deliveredNodeText.value}`;
    }
    return `${pendingDeliveryNodeText.value}(${deliverTimeText.value})`;
});
</script>

<style lang="less" scoped>
.presale-text {
    margin-right: 4px;
    font-weight: 500;
    cursor: pointer;
    display: flex;
    align-items: center;
    span {
        padding: 0 8px;
    }
}

.presale-steps-box {
    .presale-steps-container {
        display: flex;
        flex-direction: column;
        gap: 0;
    }

    .presale-step-item {
        display: flex;
        flex-direction: column;
        position: relative;
    }

    .presale-step-content {
        display: flex;
        align-items: center;
        gap: 12px;
        position: relative;
        min-height: 12px;
    }

    .presale-step-dot {
        width: 12px;
        height: 12px;
        min-width: 12px;
        min-height: 12px;
        border-radius: 50%;
        background-color: #c4c4c4;
        flex-shrink: 0;
        position: relative;
        z-index: 2;
        box-sizing: border-box;

        &.active,
        &.completed {
            background-color: var(--general);
        }
    }

    .presale-step-text {
        font-size: 14px;
        line-height: 1.5;
        flex: 1;
        padding-top: 0;
        padding-bottom: 0;
        border-bottom: none;
        text-decoration: none;
        margin-top: 0;
        margin-bottom: 0;
        display: flex;
        align-items: center;
        color: #c4c4c4;

        &.active {
            color: #333;
            border-bottom: none !important;
            text-decoration: none;
        }
    }

    :deep(.presale-step-amount) {
        font-size: 15px;
    }

    .presale-step-line {
        width: 2px;
        height: 30px;
        background-color: #eee;
        margin-left: 5px;
        margin-top: 4px;
        margin-bottom: 4px;
        flex-shrink: 0;
        position: relative;
        align-self: flex-start;

        &.active {
            background-color: var(--general);
        }
    }
}

.presale-rule-content {
    .rule-section {
        margin-bottom: 16px;
        &:last-child {
            margin-bottom: 0;
        }
        .rule-title {
            font-size: 16px;
            font-weight: 600;
            color: #333;
            margin-bottom: 12px;
        }
        .rule-item {
            font-size: 14px;
            color: #666;
            line-height: 1.8;
            margin-bottom: 8px;
            display: flex;
            &:last-child {
                margin-bottom: 0;
            }
            .rule-number {
                flex-shrink: 0;
                margin-right: 4px;
            }
        }
    }
}
</style>

