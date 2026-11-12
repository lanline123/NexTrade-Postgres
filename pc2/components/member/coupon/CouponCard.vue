<template>
    <div class="coupon-container" :class="[coupon?.status == 2 || coupon?.status == 1 || coupon?.status == 3 ? '' : 'disable']">
        <div class="coupon-info" :class="{border:coupon?.couponDesc}">
            <div class="coupon-off-text">
                <div class="top">
                    <div class="price" v-if="coupon?.couponType == 1">
                        <FormatPrice
                            :showText="false"
                            v-model="coupon.couponMoney"
                            :currencyStyle="{ fontSize: '24px', fontWeight: '400'}"
                            :fontStyle="{ fontSize: '24px', fontWeight: '400'}"
                        ></FormatPrice>
                    </div>
                    <div class="discount" v-else>
                        {{ coupon?.couponDiscount }}<span>{{ $t("折") }}</span>
                    </div>
                    <div class="desc">
                        {{ $t('优惠') }}
                        <div class="name">{{ coupon?.couponName }}</div>
                    </div>
                </div>
                <div class="time-wrap">
                    {{ coupon?.startDate }} {{ $t("-") }} {{ coupon?.endDate }}
                </div>
            </div>
            <div v-if="coupon?.status === 2 || coupon?.status === 1" class="use-btn enabel" @click.stop="openLink('/search/?couponId=' + coupon?.couponId, coupon?.status)">
                {{$t('去使用')}}
            </div>
            <div v-if="coupon?.status === 4" class="use-btn disable-btn" @click.stop="openLink('/search/?couponId=' + coupon?.couponId, coupon?.status)">
                {{$t('已使用')}}
            </div>
            <div v-if="coupon?.status === 5" class="use-btn disable-btn" @click.stop="openLink('/search/?couponId=' + coupon?.couponId, coupon?.status)">
                {{$t('已过期')}}
            </div>
        </div>
        <div class="coupon-desc" v-if="coupon?.couponDesc">
            {{ coupon?.couponDesc }}
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, type PropType } from "vue";
import type { CouponFilterParams, MyCouponFilterState } from "~/types/user/coupon.d";

const props = defineProps({
    coupon: {
        type: Object as PropType<MyCouponFilterState>
    }
});

const openLink = (url: string, status: number) => {
    if (status === 2 || status === 1) {
        window.open(url);
    }
};
</script>

<style lang="less" scoped>
.coupon-container {
    box-sizing: border-box;
    width: 475px;
    height: fit-content;
    min-height: 108px;
    border-radius: 6px;
    padding: 11px;
    border: 1px solid var(--main-bg);
    border-left-width: 4px;
    .coupon-info {
        display: flex;
        justify-content: space-between;
        align-items: center;
        width: 100%;
        padding-bottom: 8px;
        &.border {
            border-bottom: .5px dashed var(--main-bg);
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
                    :deep(.num){
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
            text-align: center;
            line-height: 20px;
            font-weight:500;
            cursor: not-allowed;

            &.enabel {
                cursor: pointer;
                &:hover {
                    background: var(--primary-light-9);
                }
            }
        }
        .disable-btn{
            border-color: #888b94;
            color: #888b94;
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
                    :deep(.num){
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
</style>
