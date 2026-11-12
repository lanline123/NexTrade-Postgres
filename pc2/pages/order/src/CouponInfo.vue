<template>
    <div v-if="useCoupon">
        <div class="check-box">
            <div class="tit">{{ $t("优惠券") }}</div>
            <div>
                <div class="radio-group">
                    <div class="item" :class="{ active: coupons == 0 }" @click="coupons = 0">{{ $t("可用") }}</div>
                    <div class="item" :class="{ active: coupons == 1 }" @click="coupons = 1">{{ $t("不可用") }}</div>
                </div>
            </div>
        </div>
        <div class="from_list">
            <div class="arrow"></div>
            <div class="coupon-box">
                <div class="tab_list flex" v-if="false">
                    <div class="tab" v-for="(item, index) in tabList" @click="current = index" :class="current == index ? 'current' : ''">{{ $t(item) }}</div>
                </div>
                <div v-loading="couponLoading">
                    <div class="coupon_list" v-if="current == 0">
                        <template v-if="couponList.enableCoupons.length > 0 && coupons == 0">
                            <div class="coupon-container" v-for="coupon in couponList.enableCoupons" @click="toggleCouponSelection(coupon)">
                                <el-checkbox v-model="coupon.selected"> </el-checkbox>
                                <div>
                                    <div class="coupon-info" :class="{ border: coupon?.couponDesc }">
                                        <div class="coupon-off-text">
                                            <div class="top">
                                                <div class="price" v-if="coupon?.couponType == 1">
                                                    <FormatPrice
                                                        :showText="false"
                                                        v-model="coupon.couponMoney"
                                                        :currencyStyle="{ fontSize: '24px', fontWeight: '400' }"
                                                        :fontStyle="{ fontSize: '24px', fontWeight: '400' }"
                                                    ></FormatPrice>
                                                </div>
                                                <div class="discount" v-else>
                                                    {{ coupon?.couponDiscount }}<span>{{ $t("折") }}</span>
                                                </div>
                                                <div class="desc">
                                                    {{ $t("优惠") }}
                                                    <div class="name">{{ coupon?.couponName }}</div>
                                                </div>
                                            </div>
                                            <div class="time-wrap" v-if="coupon?.startDate">{{ coupon?.startDate }} {{ $t("至") }} {{ coupon?.endDate }}</div>
                                            <div class="time-wrap" v-else>{{ coupon?.endDate }} {{ $t("前有效") }}</div>
                                        </div>
                                    </div>
                                    <div class="coupon-desc" v-if="coupon?.couponDesc">
                                        {{ coupon?.couponDesc }}
                                    </div>
                                </div>
                            </div>
                        </template>
                        <template v-else-if="couponList.disableCoupons.length > 0 && coupons == 1">
                            <div class="coupon-containerOne" v-for="coupon in couponList.disableCoupons">
                                <el-checkbox :disabled="true" v-model="coupon.selected"> </el-checkbox>
                                <div>
                                    <div class="coupon-info" :class="{ border: coupon?.couponDesc }">
                                        <div class="coupon-off-text">
                                            <div class="top">
                                                <div class="price" v-if="coupon?.couponType == 1">
                                                    <FormatPrice
                                                        :showText="false"
                                                        v-model="coupon.couponMoney"
                                                        :currencyStyle="{ fontSize: '24px', fontWeight: '400' }"
                                                        :fontStyle="{ fontSize: '24px', fontWeight: '400' }"
                                                    ></FormatPrice>
                                                </div>
                                                <div class="discount" v-else>
                                                    {{ coupon?.couponDiscount }}<span>{{ $t("折") }}</span>
                                                </div>
                                                <div class="desc">
                                                    {{ $t("优惠") }}
                                                    <div class="name">{{ coupon?.couponName }}</div>
                                                </div>
                                            </div>
                                            <div class="time-wrap" v-if="coupon?.startDate">
                                                {{ coupon?.startDate }} <span v-if="coupon?.startDate">{{ $t("至") }}</span> {{ coupon?.endDate }}
                                            </div>
                                            <div class="time-wrap" v-else>{{ coupon?.endDate }} {{ $t("前有效") }}</div>
                                        </div>
                                    </div>
                                    <div class="coupon-desc" v-if="coupon?.couponDesc">
                                        {{ coupon?.couponDesc }}
                                    </div>
                                </div>
                            </div>
                        </template>
                        <div class="couponShop" v-else>
                            <SvgIcon name="shop" width="50" height="50"></SvgIcon>
                            <span>{{ $t("没有可用的优惠券") }}</span>
                        </div>
                    </div>
                </div>

                <div class="discountCode flex" v-if="false">
                    <div class="tit">{{ $t("输入券号") }}：</div>
                    <div class="inp_box">
                        <div class="inp">
                            <input type="text" />
                        </div>
                        <div class="btn">{{ $t("添加优惠券") }}</div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
/**
 * @description 优惠券选择组件
 * - 展示可用优惠券列表，支持选择和取消选择
 * - 支持优惠码兑换功能
 * - 处理优惠券叠加规则（全场通用券与商品券的互斥）
 */
import { ref, reactive, watch } from "vue";
import type { CheckCouponList, EnableCoupon } from "@/types/order/check";
interface Coupon {
    id: number;
    shopId: number;
    enabledOverlay: boolean; // 可叠加使用：true 是，false 否
    isGlobal: boolean; // 全场通用券：true 是，false 否
    userCouponId: number;
}
const emit = defineEmits(["change"]);
const useCoupon = defineModel<boolean>("useCoupon");
const couponList = defineModel<CheckCouponList>("couponList");

const couponLoading = defineModel("couponLoading");
const current = ref(0);
const tabList = reactive(["商品优惠券", "优惠码兑换"]);

const selectedCouponIds = defineModel<number[]>("selectedIds");
const selectedUserCouponIds = defineModel<number[]>("selectedUserCouponIds");
const selectedDatas = ref<EnableCoupon[]>([]);

const isSelectedUserCoupon = (UserCouponId: number): boolean => {
    return selectedUserCouponIds.value ? selectedUserCouponIds.value.includes(UserCouponId) : false;
};
const toggleCouponSelection = (coupon: any) => {
    // return
    const userCouponId = Number(coupon.userCouponId);
    if (isSelectedUserCoupon(userCouponId)) {
        selectedDatas.value = selectedDatas.value.filter((item) => item.userCouponId !== userCouponId);
    } else {
        // 同店铺优惠券
        const sameShopCouponIndex = selectedDatas.value.findIndex((data) => data.shopId === coupon.shopId);
        if (sameShopCouponIndex > -1) {
            selectedDatas.value = selectedDatas.value.filter((data) => data.shopId !== coupon.shopId);
        } else {
            // 不同店铺 同一种优惠券
            const sameCouponIndex = selectedDatas.value.findIndex((data) => data.id === coupon.id);
            if (sameCouponIndex > -1) {
                // 剔除同种优惠券
                selectedDatas.value = selectedDatas.value.filter((data) => data.id !== coupon.id);
            }
        }
        // 加入选中数组
        selectedDatas.value.push(coupon);
    }

    // return;
    selectedCouponIds.value = selectedDatas.value.map(({ id }) => Number(id));
    selectedUserCouponIds.value = selectedDatas.value.map(({ userCouponId }) => Number(userCouponId));
    emit("change");
};
const coupons = ref<number>(0);
</script>
<style lang="less" scoped>
.check-box {
    display: flex;
    justify-content: space-between;
    align-items: center;
    .tit {
        height: 28px;
        font-size: 18px;
        font-weight: 700;
        line-height: 28px;
        text-align: center;
    }
    span {
        margin-left: 10px;
        color: #333;
    }
}

.from_list {
    margin-top: 10px;
    .coupon-box {
        .tab_list {
            margin-bottom: 10px;

            .tab {
                height: 20px;
                color: #333;
                margin: 5px 55px 10px 0;
                cursor: pointer;
            }

            .current {
                color: #e43a3d;
                border-bottom: 2px solid #e43a3d;
            }
        }

        .coupon_list {
            display: flex;
            flex-wrap: wrap;
            justify-content: flex-start;
            column-gap: 16px;
            max-height: 500px;
            overflow: hidden;
            overflow-y: scroll;
            padding-top: 2px;

            /*控制滚动条宽度*/
            &::-webkit-scrollbar {
                width: 5px;
                height: 5px;
            }

            &::-webkit-scrollbar-thumb {
                border-radius: 10px;
                background-color: #999;
            }

            .coupon_item {
                height: 102px;
                width: 201px;
                border: 1px solid var(--vice-bg);
                background-color: var(--vice-bg);
                color: var(--vice-text);
                margin-right: 10px;
                margin-left: 10px;
                margin-bottom: 25px;
                position: relative;
                transition: opacity 0.3s;
                :deep(.icon-gou::after) {
                    border-bottom-color: var(--general);
                }
                &:hover {
                    opacity: 0.9;
                }
                .detail {
                    background-color: #fff;
                    color: #888;
                    position: absolute;
                    width: 191px;
                    height: 25px;
                    line-height: 25px;
                    overflow: hidden;
                    bottom: -0px;
                    padding-left: 10px;
                }

                .time {
                    margin-left: 10px;
                }
                .item-selected-cancel {
                    background-color: var(--main-bg);
                    color: var(--main-text);
                    padding: 2px 3px 2px 5px;
                    cursor: pointer;
                    z-index: 21;
                    position: absolute;
                    right: 0;
                    display: none;
                }

                :deep(.priceNum) {
                    font-size: 24px;
                    margin-left: 10px;
                    margin-top: 10px;
                    margin-bottom: 8px;

                    .price {
                        font-size: 24px;
                    }
                }

                .zk {
                    font-size: 24px;
                    margin-left: 10px;
                    margin-top: 10px;
                    margin-bottom: 8px;

                    span {
                        font-size: 12px;
                    }
                }
            }

            .disabled {
                border-color: #d1d1d1;
                background-color: #d1d1d1;
                cursor: not-allowed;
                color: #999;
                .time {
                    color: #999;
                }

                .detail {
                    color: #d1d1d1;
                }
            }
        }

        .discountCode {
            .tit {
                height: 28px;
                line-height: 28px;
                color: #666;
                margin-left: 10px;
            }

            .inp_box {
                input {
                    height: 20px;
                    line-height: 20px;
                }
            }

            .btn {
                margin-top: 10px;
                text-decoration: none;
                box-shadow: 0 1px 1px rgba(0, 1, 1, 0.08);
                cursor: pointer;
                color: #fff;
                background: #ff6c6c;
                border: 1px solid #ff5e5e;
                border-radius: 2px;
                display: inline-block;
                line-height: 16px;
                padding: 5px 12px;
                text-align: center;
            }
        }
    }
}

.coupon-notice {
    position: absolute;
    bottom: -20px;
    color: #a3a3a3;
    display: flex;
    align-items: center;

    i {
        padding-right: 2px;
        height: 15px;
    }
}

.coupon-globle {
    padding-left: 10px;
    font-size: 12px;
}
.no-cou {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    width: 100%;
}
.coupon-container {
    box-sizing: border-box;
    width: 400px;
    height: fit-content;
    min-height: 110px;
    border-radius: 6px;
    padding: 11px;
    border: 1px solid var(--main-bg);
    border-left-width: 4px;
    margin-bottom: 12px;
    display: flex;
    align-items: center;
    cursor: pointer;
    .coupon-info {
        margin-left: 15px;
        width: 335px;
        &.border {
            border-bottom: 0.5px dashed var(--main-bg);
            margin-bottom: 8px;
        }
        .coupon-off-text {
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
                    margin-left: 3px;
                    font-size: 14px;
                    color: var(--main-bg);
                    white-space: nowrap;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    flex: 1;
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
                margin-bottom: 8px;
                margin-top: 3px;
                display: inline-block;
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
            &:hover {
                background: var(--primary-light-9);
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
        margin-left: 15px;
    }
}
.coupon-containerOne {
    box-sizing: border-box;
    width: 400px;
    height: fit-content;
    min-height: 80px;
    border-radius: 6px;
    padding: 11px;
    border: 1px solid #c3c3c3;
    border-left-width: 4px;
    margin-bottom: 12px;
    display: flex;
    align-items: center;
    border-color: #c3c3c3;
    .coupon-info {
        margin-left: 15px;
        width: 335px;
        &.border {
            border-bottom: 0.5px dashed #c3c3c3;
            margin-bottom: 8px;
        }
    }
    .coupon-off-text {
        .top {
            display: flex;
            align-items: baseline;
            .price {
                :deep(.util) {
                    color: #888b94;
                }
                :deep(.num) {
                    color: #888b94;
                }
            }
            .discount {
                font-size: 24px;
                color: #888b94;

                font-weight: 400;
            }
            .desc {
                display: flex;
                margin-left: 3px;
                font-size: 14px;
                color: #888b94;

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
            color: #888b94;
            background-color: #f5f6fa;
            margin-bottom: 8px;
            margin-top: 3px;
            display: inline-block;
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
        margin-left: 15px;
    }
}
.active {
    position: relative;
    border: 1px solid var(--general);
    background-color: var(--general);
    color: var(--main-text);

    .c-top-dong {
        display: none;
    }

    .icon-gou {
        display: block;
    }

    &:hover {
        .item-selected-cancel {
            display: block;
        }
    }
}

.active::after {
    content: "";
    position: absolute;
}
.couponShop {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    padding: 10px 0 30px 0;
    span {
        font-size: 14px;
        font-weight: 400;
        line-height: 18px;
        text-align: center;
        color: #c2c4cc;
    }
}

.radio-group {
    display: flex;
    font-size: 14px;
    height: 30px;
    cursor: pointer;
    .item {
        border: 1px solid #e0e1e5;
        background: #fff;
        color: #505259;
        line-height: 28px;
        padding: 0 12px;
        &:first-child {
            border-right: none;
            border-radius: 4px 0 0 4px;
        }
        &:last-child {
            border-left: none;
            border-radius: 0 4px 4px 0;
        }
    }
    .active {
        border: 1px solid var(--general) !important;
        background: #fff3f4;
        border-color: var(--general);
        color: var(--general);
        font-weight: bold;
    }
}
</style>
