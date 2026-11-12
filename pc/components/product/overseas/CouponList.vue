<template>
    <div v-if="promotionList.length > 0" class="flex w_100 coupon-list" @click="visible = true">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" fill="none">
            <path
                fill="var(--general)"
                d="M10.882 3.472a1.98 1.98 0 0 0-2.565 0l-.888.758a.66.66 0 0 1-.375.155l-1.163.092a1.98 1.98 0 0 0-1.814 1.814l-.092 1.164a.66.66 0 0 1-.156.375l-.757.888a1.98 1.98 0 0 0 0 2.565l.757.888a.66.66 0 0 1 .155.375l.093 1.163a1.98 1.98 0 0 0 1.814 1.814l1.163.092a.66.66 0 0 1 .376.156l.887.757a1.98 1.98 0 0 0 2.565 0l.889-.757a.66.66 0 0 1 .375-.155l1.163-.093a1.98 1.98 0 0 0 1.814-1.814l.092-1.163a.66.66 0 0 1 .156-.376l.757-.887a1.98 1.98 0 0 0 0-2.565l-.757-.888a.66.66 0 0 1-.156-.376l-.092-1.163a1.98 1.98 0 0 0-1.814-1.814l-1.163-.092a.66.66 0 0 1-.376-.155zm.581 3.732.933.932-4.66 4.66-.932-.932zM8.435 8.835a.988.988 0 1 1-1.422-1.373.988.988 0 0 1 1.422 1.373m2.33 3.728a.987.987 0 0 1 1.076-1.623 1 1 0 0 1 .32.225.989.989 0 0 1-1.396 1.398"
            ></path>
        </svg>
        <div
            class="promotion-box"
            :class="{ 'promotion-disabled': promotionList[0].canJoin === false }"
            :title="promotionList[0].canJoin === false ? promotionList[0].restrictTip || $t('该活动仅限指定会员参与') : ''"
        >
            {{ promotionList[0].promotionName }}
        </div>
        <span class="promotion-arrow">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="none" class="arrow-icon">
                <path stroke="#888B94" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M11.5 6.25 8 9.75l-3.5-3.5"></path>
            </svg>
        </span>
    </div>

    <div class="modal_region_box">
        <el-dialog
            v-model="visible"
            :title="$t('优惠')"
            :footer="null"
            top="15vh"
            width="880px"
            append-to-body
            :z-index="10000"
            :style="themeStore.themeColor"
        >
            <template v-if="activityList.length > 0 || giftList.length > 0">
                <div class="coupon-card-box">
                    <div class="coupon-card-title">{{ $t("促销") }}</div>
                    <div class="coupon-card-list">
                        <template v-if="activityList.length > 0" v-for="item in activityList" :key="item.promotionId">
                            <div class="card activity promotion-card">
                                <div class="activity-left">
                                    <div class="activity-left-tag">
                                        {{ $t(activityTextMap[item.type]) }}
                                    </div>
                                </div>
                                <div class="activity-right">
                                    <div class="activity-right-name">{{ item.promotionName }}</div>
                                    <div class="activity-right-time">{{ item.data.startTime }}~{{ item.data.endTime }}</div>
                                    <div class="activity-restrict-tip" v-if="item.canJoin === false">
                                        {{ item.restrictTip || $t("该活动仅限指定会员参与") }}
                                    </div>
                                </div>
                            </div>
                        </template>
                        <template v-if="giftList.length > 0" v-for="giftItem in giftList" :key="giftItem.promotionId">
                            <template v-for="item in giftItem.data.promotionDesc" :key="item.giftId">
                                <NuxtLink
                                    class="promotion-card-link"
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
                                    <div class="card activity promotion-card gift-card">
                                        <div class="activity-left">
                                            <div class="activity-right-name">
                                                {{ giftThresholdText(giftItem, item) }}
                                            </div>
                                        </div>
                                        <div class="activity-right">
                                            <div class="activity-product-item">
                                                <div class="activity-product-item-box">
                                                    <el-image class="activity-product-item-img" lazy :src="productImage(item.gift.productInfo)">
                                                    </el-image>
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
                                </NuxtLink>
                            </template>
                        </template>
                    </div>
                </div>
            </template>
            <template v-if="!loading && couponList.length > 0">
                <div class="coupon-card-box">
                    <div class="coupon-card-title">{{ $t("优惠券") }}</div>
                    <div class="coupon-card-list pointer">
                        <template v-for="(item, index) in couponList" :key="index">
                            <div class="coupon-container" :class="{ disable: couponStatus(item.isReceive!, item.limitNum!, item.receiveNum!) === t('已领取') }">
                                <div class="coupon-info border">
                                    <div class="coupon-off-text">
                                        <div class="top">
                                            <div class="price" v-if="item?.couponType == 1">
                                                <FormatPrice
                                                    :showText="false"
                                                    v-model="item.couponMoney"
                                                    :currencyStyle="{ fontSize: '24px', fontWeight: '400' }"
                                                    :fontStyle="{ fontSize: '24px', fontWeight: '400' }"
                                                ></FormatPrice>
                                            </div>
                                            <div class="discount" v-else>
                                                {{ item?.couponDiscount }}<span>{{ $t("折") }}</span>
                                            </div>
                                            <div class="desc">
                                                {{ $t("优惠") }}
                                                <div class="name">{{ item?.couponName || "" }}</div>
                                            </div>
                                        </div>
                                    </div>

                                    <div
                                        class="use-btn"
                                        :class="{ 'disable-btn': couponStatus(item.isReceive!, item.limitNum!, item.receiveNum!) === t('已领取') }"
                                        @click="onSubmit(item)"
                                    >
                                        {{ $t(couponStatus(item.isReceive!, item.limitNum!, item.receiveNum!)) }}
                                    </div>
                                </div>
                                <div class="coupon-desc" v-if="item?.couponDesc">
                                    {{ item?.couponDesc }}
                                </div>
                            </div>
                        </template>
                    </div>
                </div>
            </template>
        </el-dialog>
    </div>
</template>

<script lang="ts" setup>
import { ref, computed } from "vue";
import type { PropType } from "vue";
import { getProductCouponList } from "~/api/product/product";
import type { ProductCouponItem, SkuPromotion } from "~/types/product/product.d";
import { addCoupon } from "~/api/user/coupon";
import { useThemeStore } from "~/store/theme";

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
    }
});

interface ActivityTextMap {
    [key: number]: string;
}

const activityTextMap: ActivityTextMap = {
    3: "满减",
    4: "满折",
    5: "满赠"
};
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
        await addCoupon({ couponId: value.couponId });
        await getProductCoupon();
        message.success(t("领取成功"));
        visible.value = false;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
watch(
    () => visible.value,
    () => {
        if (visible.value) {
            disableLarge();
        } else {
            enableLarge();
        }
    }
);
const disableLarge = () => {
    document.querySelector(".large")?.classList.add("disabled");
    document.querySelector(".tab-main")?.classList.add("tab-main-disabled");
};
const enableLarge = () => {
    document.querySelector(".large")?.classList.remove("disabled");
    document.querySelector(".tab-main")?.classList.add("tab-main-disabled");
};
</script>

<style lang="less" scoped>
:global(body .ant-message) {
    z-index: 10001 !important;
}

.coupon-list {
    align-items: center;
    cursor: pointer;
}

.promotion-box {
    margin-left: 4px;
    font-size: 14px;
    margin-right: 8px;
    color: var(--general);
    font-weight: 700;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;

    // 二期会员标签：不在人群范围的活动
    &.promotion-disabled {
        opacity: 0.55;
    }
}

.promotion-arrow {
    display: flex;
    align-items: center;
    color: rgb(136, 139, 148);
    transform: rotate(-90deg);
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
        color: #1a1a1a;
    }
}
.coupon-card-list {
    display: flex;
    flex-wrap: wrap;
    gap: 20px;
    width: 100%;

    .coupon-container {
        flex: 1;
        max-width: calc(50% - 10px);
        box-sizing: border-box;
        height: fit-content;
        min-height: 86px;
        min-width: calc((100% - 20px) / 2);
        border-radius: 6px;
        padding: 11px;
        border: 1px solid var(--main-bg);
        border-left-width: 4px;
        cursor: pointer;

        .coupon-info {
            display: flex;
            justify-content: space-between;
            align-items: center;
            width: 100%;
            padding-bottom: 8px;
            &.border {
                border-bottom: 0.5px dashed var(--main-bg);
                margin-bottom: 8px;
            }
            .coupon-off-text {
                flex: 1;
                .top {
                    display: flex;
                    align-items: baseline;
                    .price {
                        :deep(.util) {
                            color: var(--main-bg);
                        }
                        :deep(.num) {
                            color: var(--main-bg);
                        }
                    }
                    .discount {
                        font-size: 24px;
                        color: var(--main-bg);
                        font-weight: 400;
                    }
                    .desc {
                        display: flex;
                        align-items: baseline;
                        margin-left: 3px;
                        font-size: 14px;
                        color: var(--main-bg);
                        width: 0;
                        flex: 1;
                        white-space: nowrap;
                        overflow: hidden;
                        text-overflow: ellipsis;
                        .name {
                            margin-left: 10px;
                            white-space: nowrap;
                            overflow: hidden;
                            text-overflow: ellipsis;
                        }
                    }
                }
                .time-wrap {
                    height: auto;
                    font-size: 12px;
                    line-height: 16px;
                    border-radius: 4px;
                    padding: 4px;
                    color: #c99227;
                    background-color: #fff8ea;
                }
            }
            .use-btn {
                min-width: 48px !important;
                height: 20px;
                padding: 0 4px;
                border-radius: 4px;
                border: 1px solid var(--main-bg);
                color: var(--main-bg);
                cursor: pointer;
                text-align: center;
                line-height: 20px;
                font-weight: 500;
                &:hover {
                    background: var(--primary-light-9);
                }
            }
            .disable-btn {
                border-color: #888b94;
                color: #888b94;
                cursor: not-allowed;
                &:hover {
                    background: #f0f0f0;
                }
            }
        }

        .coupon-desc {
            height: auto;
            font-size: 12px;
            line-height: 16px;
            color: #888b94;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }

        &.disable {
            border-color: #c3c3c3;
            .coupon-info {
                &.border {
                    border-color: #c3c3c3;
                }
            }
            .coupon-off-text {
                .top {
                    .price {
                        :deep(.util) {
                            color: #c3c3c3;
                        }
                        :deep(.num) {
                            color: #c3c3c3;
                        }
                    }
                    .discount {
                        color: #c3c3c3;
                    }
                    .desc {
                        color: #c3c3c3;
                    }
                }
                .time-wrap {
                    color: #c3c3c3;
                    background-color: #f0f0f0;
                }
            }
        }
    }

    .promotion-card-link,
    .promotion-card {
        flex: 0 0 calc((100% - 20px) / 2);
        max-width: calc((100% - 20px) / 2);
    }

    .promotion-card-link {
        display: block;
    }

    .promotion-card-link .promotion-card {
        width: 100%;
        max-width: none;
    }

    .card.gift-card {
        align-items: center;
        .activity-left {
            padding-top: 0;
            .activity-right-name {
                color: var(--general);
            }
        }
    }

    .card {
        display: flex;
        width: 100%;
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
                width: 10px;
                display: flex;
                font-size: 12px;
                margin-left: 0;
                margin-right: 14px;
                text-align: center;
                color: white;
                align-items: center;
                word-break: break-all;
            }
        }
        .on-card {
            background-color: #666;
        }
        &.activity {
            padding: 0 10px;
        }

        .activity-left {
            padding-top: 13px;

            .activity-left-tag {
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
            }
        }
        .activity-right {
            padding-left: 10px;
            padding: 13px 0 10px 10px;
            display: flex;
            flex-direction: column;
            flex: 1;

            .activity-right-name {
                padding-bottom: 10px;
                color: #333;
            }

            .activity-right-time {
                height: 18px;
                color: #999;
                line-height: 18px;
                font-size: 12px;
            }
            // 二期会员标签：不可参与提示
            .activity-restrict-tip {
                margin-top: 4px;
                color: var(--tag-text);
                font-size: 12px;
            }
            .activity-name {
                &.isHasContent {
                    padding-bottom: 10px;
                    padding-top: 3px;
                }
            }
            .activity-product-item-box {
                flex-shrink: 0;
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
.w_100 {
    width: 100%;
    z-index: 999;
}
.flexCsb {
    width: 496px;
    align-items: center;
    justify-content: space-between;
}
</style>
