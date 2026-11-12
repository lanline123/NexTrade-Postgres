<template>
    <CommonPageHeader></CommonPageHeader>
    <CommonHeader :title="$t('集券中心')"></CommonHeader>
    <CommonPageContainer class="coupon-list">
        <div class="title">
            <div class="title-image-left"></div>
            <div class="com-pc-title">{{ $t("集券中心") }}</div>
            <div class="title-image-right"></div>
        </div>
        <div v-loading="loading" class="coupon-card-list">
            <template v-if="total > 0">
                <div v-for="item in filterState" class="card" @click="onSubmit(item)">
                    <div class="left-card">
                        <div>
                            <template v-if="item.couponType == 1">
                                <FormatPrice
                                    v-model="item.couponMoney"
                                    :showText="false"
                                    :currencyStyle="{ fontSize: '24px', fontFamily: 'Verdana, serif', lineHeight: '41px' }"
                                    :fontStyle="{ fontSize: '40px', fontWeight: 'bold' }"
                                    class="card-price"
                                ></FormatPrice>
                            </template>
                            <template v-else>
                                <!--折扣券-->
                                <div class="coupon_discount">
                                    {{ item.couponDiscount }}<span>{{ $t("折") }}</span>
                                </div>
                            </template>
                        </div>
                        <div class="name">{{ item.couponName ? "【" + item.couponName + "】" : "" }}</div>
                        <div class="time" v-if="item.sendType == 1">{{ item.useStartDate }} {{ $t("至") }} {{ item.useEndDate }}</div>
                        <div class="time" v-if="item.sendType == 0 && item.delayDay > 0">
                            {{ $t("领券{0}天后生效，有效期{1}天", [item.delayDay, item.useDay]) }}
                        </div>
                        <div class="time" v-if="item.sendType == 0 && item.delayDay == 0">
                            {{ $t("领券当日起{0}天内可用", [item.useDay]) }}
                        </div>
                        <view class="name desc" v-if="item.couponDesc">
                            {{ item.couponDesc }}
                        </view>
                    </div>
                    <div class="right-card">
                        <span class="division_line"></span>
                        <span class="text">{{ couponStatus(item.isReceive!, item.limitNum, item.receiveNum!) }}</span>
                    </div>
                </div>
            </template>
            <template v-else>
                <div class="no-result">
                    <span v-if="!loading">{{ $t("暂无可领取的优惠券") }}～</span>
                </div>
            </template>
        </div>
    </CommonPageContainer>

    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 集券中心页面
 * - 展示所有可领取的优惠券列表
 * - 支持满减券和折扣券两种类型
 * - 根据领取状态显示不同按钮（马上领/再次领取/已领取）
 * - 点击卡片即可领取优惠券
 */
import type { CouponFilterParams, CouponFilterState } from "~/types/user/coupon.d";
import { getCouponList, addCoupon } from "~/api/user/coupon";
import { useRouter } from "vue-router";

const filterState = ref(<CouponFilterState[]>[]);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<CouponFilterParams>({
    //初使化用于查询的参数
    page: 1,
    isShow: 1,
    shopId: -1
});
const router = useRouter();
const loadFilter = async () => {
    loading.value = true;
    try {
        if (router.currentRoute.value.query.shopId) {
            filterParams.shopId = Number(router.currentRoute.value.query.shopId);
        }
        const result = await getCouponList({ ...filterParams });
        total.value = result.total;
        filterState.value = result.records;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const { t } = useI18n();
onMounted(() => {
    loadFilter();
});
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
    if (couponStatus(value.isReceive, value.limitNum, value.receiveNum) === t("已领取")) {
        message.warning(t("您已超出领取上限"));
        return;
    }
    try {
        const result = await addCoupon({ couponId: value.couponId });
        message.success(t("领取成功"));
        await loadFilter();
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.coupon-list {
    .title {
        display: flex;
        justify-content: center;
        margin: 20px;
        align-items: center;

        .com-pc-title {
            height: 45px;
            font-size: 28px;
            font-weight: 700;
            text-align: center;
            line-height: 45px;
            padding: 0 30px;
            overflow: hidden;
            color: #333;
        }

        .title-image-left {
            width: 25px; /* 原图的 50% */
            height: 20px; /* 原图的 50% */
            background-image: url("/assets/images/common/sprite@2x.png");
            background-position: 0 0;
            background-repeat: no-repeat;
            background-size: 200%; /* 2倍于容器的大小，使图片缩小到 50% */
        }

        .title-image-right {
            width: 25px; /* 原图的 50% */
            height: 20px; /* 原图的 50% */
            background-image: url("/assets/images/common/sprite@2x.png");
            background-position: -25px 0;
            background-repeat: no-repeat;
            background-size: 200%; /* 2倍于容器的大小，使图片缩小到 50% */
        }
    }

    .coupon-card-list {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
        margin: 50px 0;
        gap: 20px;

        .no-result {
            grid-column: 1 / -1;
        }

        .card {
            cursor: pointer;
            display: flex;
            width: auto;
            height: 190px;
            background-color: #fff;
            border-radius: 5px;

            .left-card {
                display: flex;
                flex: 1;
                flex-direction: column;
                border-top-left-radius: 5px;
                border-bottom-left-radius: 5px;
                box-sizing: border-box;
                padding: 20px;
                gap: 10px;
                color: var(--general);

                .card-price {
                    color: var(--general);
                }

                .name {
                    color: #666666;
                }
                .desc {
                    word-break: break-all;
                    display: -webkit-box;
                    -webkit-line-clamp: 2;
                    -webkit-box-orient: vertical;
                    overflow: hidden;
                }

                .time {
                    height: 18px;
                    color: #999;
                    line-height: 18px;
                }

                .coupon_discount {
                    font-size: 40px;
                    // color: #74d2d4;
                    font-weight: bold;

                    & > span {
                        font-size: 24px;
                    }
                }
            }

            .right-card {
                width: 70px;
                display: flex;
                background-color: var(--general);
                border-top-right-radius: 5px;
                border-bottom-right-radius: 5px;

                .division_line {
                    top: 0;
                    width: 9px;
                    height: 190px;
                    overflow: hidden;
                    background: url("/assets/images/coupon/semi-circle.png") repeat-y;
                    left: 0;
                }

                .text {
                    display: flex;
                    font-size: 16px;
                    margin-left: 0;
                    padding: 0 21px;
                    text-align: center;
                    color: white;
                    align-items: center;
                }
            }
        }
    }
}
</style>
