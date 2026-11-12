<template>
    <div v-if="displayPromotionList.length > 0" class="coupon-list-wrap" :class="{ 'new-ui': uiVersion === 'new' }">
        <slot></slot>
        <div class="lyecsquan flex">
            <div class="promotion-box">
                <div class="promotion-box-item" v-for="item in displayPromotionList.slice(0, 3)" :key="item.promotionId">
                    <div
                        class="promotion"
                        v-if="item.type != 2 && activitTextMap[item.type]"
                        :class="{ 'promotion-disabled': item.canJoin === false }"
                        :title="item.canJoin === false ? item.restrictTip || $t('该活动仅限指定会员参与') : ''"
                    >
                        <div class="ticket-content">
                            {{ $t(activitTextMap[item.type]) }}
                        </div>
                    </div>

                    <div class="promotion" v-if="item.type == 2">
                        <div class="ticket-type">{{ $t("券") }} <span class="ticket-line"></span></div>
                        <div class="ticket-content">{{ item.data.promotionDesc }}</div>
                    </div>
                </div>
            </div>
            <div class="lyecsquan-more hand-pointer" @click="visible = true">{{ $t("更多优惠") }} ></div>
        </div>
        <div class="modal_region_box">
            <el-dialog v-model="visible" :title="$t('优惠')" :footer="null" top="15vh" width="1090px" append-to-body :z-index="9000" :style="themeStore.themeColor">
                <template v-if="activityList.length > 0 || giftList.length > 0">
                    <div class="coupon-card-box">
                        <div class="coupon-card-title">{{ $t("促销") }}</div>
                        <div class="coupon-card-list">
                            <template v-if="activityList.length > 0" v-for="item in activityList" :key="item.promotionId">
                                <div class="card activity">
                                    <div class="activity-left">
                                        <div class="activity-left-name">
                                            {{ activitTextMap[item.type] ? $t(activitTextMap[item.type]) : "" }}
                                        </div>
                                    </div>
                                    <div class="activity-right">
                                        <div class="activity-right-name">{{ item.data.promotionName }}</div>
                                        <div class="activity-right-time">{{ item?.startTime }}~{{ item?.endTime }}</div>
                                        <div class="activity-restrict-tip" v-if="item.canJoin === false">
                                            {{ item.restrictTip || $t("该活动仅限指定会员参与") }}
                                        </div>
                                    </div>
                                </div>
                            </template>
                            <template v-if="giftList.length > 0" v-for="giftItem in giftList" :key="giftItem.promotionId">
                                <div class="card activity" v-for="item in giftItem.data.promotionDesc" :key="item.giftId">
                                    <div class="activity-left gift-card-left">
                                        <div class="activity-left-tag">
                                            {{ giftThresholdText(giftItem, item) }}
                                        </div>
                                    </div>
                                    <div class="activity-right">
                                        <div class="activity-name">
                                            {{ item.desc }}
                                        </div>
                                        <div class="activity-product-item">
                                            <div class="activity-product-item-box">
                                                <NuxtLink
                                                    :to="
                                                        urlFormat({
                                                            path: 'product',
                                                            seoLink: item.gift.productInfo.seoLink,
                                                            id: item.gift.productInfo.productId,
                                                            sn: item.gift.productInfo.productSn
                                                        })
                                                    "
                                                    target="_blank"
                                                >
                                                    <el-image
                                                        class="activity-product-item-img"
                                                        lazy
                                                        :src="productImage(item.gift.productInfo)"
                                                        :alt="item.gift.productInfo.productName || ''"
                                                    >
                                                    </el-image>
                                                </NuxtLink>
                                                <div class="activity-product-price">
                                                    <FormatPrice
                                                        :showText="false"
                                                        :fontStyle="{ fontSize: '10px', fontWeight: 'bold' }"
                                                        class=""
                                                        v-model="item.gift.productInfo.productPrice"
                                                    ></FormatPrice>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </template>
                        </div>
                    </div>
                </template>
                <template v-if="!loading && couponList.length > 0">
                    <div class="coupon-card-box">
                        <div class="coupon-card-title">{{ $t("优惠券") }}</div>
                        <div class="coupon-card-list">
                            <div v-for="item in couponList" class="card" @click="onSubmit(item)">
                                <div class="left-card">
                                    <div>
                                        <template v-if="item.couponType == 1">
                                            <FormatPrice
                                                :showText="false"
                                                v-model="item.couponMoney"
                                                :currencyStyle="{
                                                    color:
                                                        couponStatus(item.isReceive!, item.limitNum, item.receiveNum) === t('已领取') ? '#666' : 'var(--tag-text)',
                                                    fontSize: '12px',
                                                    lineHeight: '24px'
                                                }"
                                                :fontStyle="{
                                                    color:
                                                        couponStatus(item.isReceive!, item.limitNum, item.receiveNum) === t('已领取') ? '#666' : 'var(--tag-text)',
                                                    fontSize: '24px',
                                                    fontWeight: 'bold'
                                                }"
                                            ></FormatPrice>
                                        </template>
                                        <template v-else>
                                            <!--折扣券-->
                                            <div
                                                class="coupon_discount"
                                                :class="{ on: couponStatus(item.isReceive!, item.limitNum, item.receiveNum) === t('已领取') }"
                                            >
                                                {{ item.couponDiscount }}<span>{{ $t("折") }}</span>
                                            </div>
                                        </template>
                                    </div>
                                    <div class="name">{{ item.couponName ? "【" + item.couponName + "】" : "" }}</div>
                                    <div class="name" v-if="item.couponDesc">{{ item.couponDesc }}</div>
                                    <div class="time" v-if="item.sendType == 1">{{ item.useStartDate }} {{ $t("至") }} {{ item.useEndDate }}</div>
                                    <div
                                        class="time"
                                        v-if="item.sendType == 0 && item.delayDay > 0"
                                        :title="$t('领券{0}天后生效，有效期{1}天', [item.delayDay, item.useDay])"
                                    >
                                        {{ $t("领券{0}天后生效，有效期{1}天", [item.delayDay, item.useDay]) }}
                                    </div>
                                    <div
                                        class="time"
                                        v-if="item.sendType == 0 && item.delayDay == 0"
                                        :title="$t('领券当日起{0}天内可用', [item.useDay])"
                                    >
                                        {{ $t("领券当日起{0}天内可用", [item.useDay]) }}
                                    </div>
                                </div>
                                <div class="right-card" :class="{ 'on-card': couponStatus(item.isReceive!, item.limitNum, item.receiveNum) === t('已领取') }">
                                    <span class="division_line"></span>
                                    <span class="text">{{ $t(couponStatus(item.isReceive!, item.limitNum!, item.receiveNum!)) }}</span>
                                </div>
                            </div>
                        </div>
                    </div>
                </template>
            </el-dialog>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { ref, computed } from "vue";
import type { PropType } from "vue";
import { getProductCouponList } from "~/api/product/product";
import type { AttrList, ProductCouponItem, SkuPromotion } from "~/types/product/product.d";
import { addCoupon } from "~/api/user/coupon";
import { useUserStore } from "~/store/user";
import { useThemeStore } from "~/store/theme";
const route = useRoute();
const themeStore = useThemeStore();
const loading = ref(false);
const visible = ref(false);
const couponList = ref<ProductCouponItem[]>([]);
const { t } = useI18n();
const props = defineProps({
    productId: {
        type: Number,
        default: 0
    },
    shopId: {
        type: Number,
        default: 0
    },
    promotionList: {
        type: Array as PropType<SkuPromotion[]>,
        default: () => []
    },
    uiVersion: {
        type: String,
        default: "old"
    }
});

interface ActivitTextMap {
    [key: number]: string;
}

const activitTextMap: ActivitTextMap = {
    3: "满减",
    4: "满折",
    5: "满赠"
};
const displayPromotionList = computed(() => {
    // 顶部优惠标签与跨境版保持一致，直接展示商品可用性接口返回的完整活动列表
    return props.promotionList;
});
const activityList = computed(() => {
    return props.promotionList.filter((item) => item.type === 3 || item.type === 4);
});
const giftList = computed(() => {
    return props.promotionList.filter((item) => item.type === 5);
});
const giftThresholdText = (giftItem: any, item: any) => {
    const rulesType = giftItem?.data?.rulesType;
    const minAmount = item?.minAmount;
    if (minAmount === undefined || minAmount === null || minAmount === "") return t("满赠");
    return rulesType === 1 ? t("实付满{0}元赠", [minAmount]) : t("实付每满{0}元赠", [minAmount]);
};
const getProductCoupon = async () => {
    try {
        const result = await getProductCouponList({ id: Number(props.productId), shopId: Number(props.shopId) });
        couponList.value = result;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
getProductCoupon();

const couponStatus = (isReceive: number, limitNum: number, receiveNum: number) => {
    if (limitNum === 0) {
        if (isReceive === 1) {
            return t("已领取");
        } else if (receiveNum > 0) {
            return t("再次领取");
        } else {
            return t("马上领");
        }
    } else {
        if (isReceive === 0) {
            return t("马上领");
        } else if (receiveNum >= limitNum) {
            return t("已领取");
        } else {
            return t("再次领取");
        }
    }
};

const onSubmit = async (value: any) => {
    if (couponStatus(value.isReceive!, value.limitNum!, value.receiveNum!) === t("已领取")) {
        message.info(t("您已领取该优惠券"));
        return;
    }
    try {
        const result = await addCoupon({ couponId: value.couponId });
        await getProductCoupon();
        message.success(t("领取成功"));
        visible.value = false;
        // await navigateTo("/coupon?id="+value.couponId);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
</script>

<style lang="less" scoped>
:global(.ant-message) {
    z-index: 10000 !important;
}

.coupon-list-wrap {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    row-gap: 8px;
    min-width: 0;
}
.promotion-box {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    row-gap: 8px;
    min-width: 0;
    .promotion-box-item {
        margin-right: 10px;
    }
    .promotion-disabled {
        opacity: 0.55;
    }
    .activity {
        max-width: 100px;
        background-color: var(--general);
        color: var(--main-text);
        padding: 0 8px;
        border-radius: 2px;
        padding-bottom: 2px;
        margin-right: 10px;
    }

    .promotion {
        display: flex;
        background-color: var(--tag-bg);
        color: var(--tag-text);
        border-radius: 2px;
        padding-bottom: 2px;

        .ticket-type {
            padding: 0 6px;
            max-width: 50px;
            position: relative;

            .ticket-line {
                position: absolute;
                display: inline-block;
                height: 100%;
                opacity: 0.9;
                width: 2px;
                top: 0;
                right: 0px;
                transform: scaleY(0.7);
                background: linear-gradient(
                    to bottom,
                    var(--ump-border, #c9c9ff) 20%,
                    var(--ump-border, #c9c9ff) 20%,
                    var(--ump-border, #c9c9ff) 20%,
                    rgba(255, 255, 255, 0) 20%,
                    rgba(255, 255, 255, 0) 35%,
                    rgba(136, 76, 255, 1) 35%,
                    var(--ump-border, #c9c9ff) 35%,
                    var(--ump-border, #c9c9ff) 35%,
                    var(--ump-border, #c9c9ff) 60%,
                    rgba(255, 255, 255, 0) 60%,
                    rgba(255, 255, 255, 0) 55%,
                    rgba(255, 255, 255, 0) 75%,
                    var(--ump-border, #c9c9ff) 75%,
                    var(--ump-border, #c9c9ff) 75%,
                    var(--ump-border, #c9c9ff) 100%
                );
            }
        }
        .ticket-content {
            padding: 0 4px;
            max-width: 90px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
    }
}

.lyecsquan {
    align-items: center;
    flex-wrap: wrap;

    .lyecsquan-item {
        margin-right: 13px;

        .txt {
            height: 18px;
            line-height: 18px;
            text-align: center;
            background: var(--vice-bg);
            font-size: 14px;
            white-space: nowrap;
            cursor: pointer;
            color: var(--vice-text);
            font-size: 12px;
            padding: 0 10px;
            position: relative;
            &::before {
                content: "";
                position: absolute;
                left: -3px;
                top: 0;
                bottom: 0;
                width: 6px;
                height: 100%;
                color: var(--general);
                background: var(--vice-bg);
                -webkit-mask: radial-gradient(circle at 2px, #0000 2px, var(--vice-bg) 0);
                -webkit-mask-position: -3px;
                -webkit-mask-size: 100% 6px;
            }
            &::after {
                content: "";
                position: absolute;
                right: -3px;
                top: 0;
                bottom: 0;
                width: 6px;
                height: 100%;
                color: #fff;
                background: var(--vice-bg);
                -webkit-mask: radial-gradient(circle at 2px, #0000 2px, var(--vice-bg) 0);
                -webkit-mask-position: 5px;
                -webkit-mask-size: 100% 6px;
            }
        }
    }
    .lyecsquan-more {
        color: var(--general);
    }
}
.coupon-list-wrap.new-ui .lyecsquan {
    row-gap: 8px;
}
.modal_region_box :deep(.el-dialog) {
    height: 600px;
    overflow-y: auto;
}

.coupon-card-box {
    margin: 15px 0;
    .coupon-card-title {
        font-size: 15px;
        padding-bottom: 15px;
    }
}
.coupon-card-list {
    display: flex;
    flex-wrap: wrap;
    gap: 20px;
    .card {
        cursor: pointer;
        display: flex;
        width: 330px;
        min-height: 95px;
        background-color: #f2f3f5;
        border-radius: 5px;
        box-sizing: border-box;

        .left-card {
            display: flex;
            flex: 1;
            flex-direction: column;
            border-top-left-radius: 5px;
            border-bottom-left-radius: 5px;
            box-sizing: border-box;
            padding: 10px;
            gap: 5px;

            .name {
                color: #666666;
                font-size: 12px;
            }

            .time {
                width: 260px;
                height: 18px;
                color: #999;
                line-height: 18px;
                font-size: 12px;
                white-space: nowrap; /* 不允许换行 */
                overflow: hidden; /* 超出部分隐藏 */
                text-overflow: ellipsis; /* 显示省略号 */
            }

            .coupon_discount {
                display: flex;
                align-items: flex-end;
                font-size: 24px;
                color: var(--tag-text);
                font-weight: bold;

                & > span {
                    font-size: 14px;
                    margin-left: 5px;
                    margin-bottom: 3px;
                }
            }
            .on {
                color: #666;
            }
        }

        .right-card {
            width: 40px;
            display: flex;
            background-color: var(--tag-text);
            border-top-right-radius: 5px;
            border-bottom-right-radius: 5px;

            .division_line {
                top: 0;
                width: 40px;
                height: 95px;
                overflow: hidden;
                background: url("/assets/images/coupon/semi-circle.png") repeat-y;
                left: 0;
            }

            .text {
                display: flex;
                font-size: 12px;
                margin-left: 0;
                padding: 0 10px;
                text-align: center;
                color: white;
                align-items: center;
            }
        }
        .on-card {
            background-color: #666;
        }

        &.activity {
            padding: 0 10px;
        }

        .activity-left {
            font-size: 20px;
            max-width: 150px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: var(--tag-text);
            font-weight: bold;
            padding-top: 13px;
            .activity-left-name {
                font-size: 14px;
                font-weight: normal;
                padding: 0 4px;
                max-width: 150px;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
                display: flex;
                background-color: var(--tag-bg);
                color: var(--tag-text);
                border-radius: 2px;
                padding-bottom: 2px;
                align-self: flex-start;
            }
            .activity-left-tag {
                font-size: 14px;
                font-weight: 500;
            }
        }
        .gift-card-left {
            padding-top: 0 !important;
        }
        .activity-right {
            padding-left: 10px;
            padding: 13px 0 10px 10px;
            display: flex;
            flex-direction: column;

            .activity-right-name {
                padding-bottom: 10px;
            }

            .activity-right-time {
                height: 18px;
                color: #999;
                line-height: 18px;
                font-size: 12px;
                white-space: nowrap;
            }
            .activity-restrict-tip {
                margin-top: 4px;
                color: var(--tag-text);
                font-size: 12px;
            }
            .activity-name {
                padding-bottom: 10px;
                padding-top: 3px;
            }
            .activity-product-item-box {
                width: 50px;
                height: 50px;
                border-radius: 4px;
                overflow: hidden;
                position: relative;
                .activity-product-item-img {
                    width: 100%;
                    height: 100%;
                }

                .activity-product-price {
                    width: 100%;
                    position: absolute;
                    bottom: 0;
                    background-color: rgba(0, 0, 0, 0.24);
                    color: #fff;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    font-size: 12px;
                    height: 18px;
                }
            }
        }
    }
}
</style>
