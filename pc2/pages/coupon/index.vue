<template>
    <CommonPageHeader></CommonPageHeader>
    <CommonPageContainer class="coupon-info">
        <div class="coupon-card-info">
            <div class="ticket-info">
                <div class="ti-detail">
                    <div class="ti-detail-cont">
                        <div class="ti-detail-name">
                            <h3>{{ formState.couponName }}</h3>
                            <p>
                                {{ formState.couponDesc }}
                            </p>
                        </div>
                        <div class="ti-detail-money">
                            <template v-if="formState.couponType == 1">
                                <FormatPrice
                                    :showText="false"
                                    v-model="formState.couponMoney"
                                    :currencyStyle="{ fontSize: '14px', color: 'var(--price)' }"
                                    :fontStyle="{ color: 'var(--price)', fontSize: '32px', fontWeight: '700' }"
                                ></FormatPrice>
                            </template>
                            <template v-else>
                                <!--折扣券-->
                                <div class="coupon_discount">{{ formState.couponDiscount }}{{ $t("折") }}</div>
                            </template>
                        </div>
                    </div>
                </div>
                <div class="ti-time">{{ formState.useStartDate }} {{ $t("至") }} {{ formState.useEndDate }}</div>
            </div>
            <div class="result-div">
                <div class="icon"></div>
                <div>{{ $t("已领取") }}</div>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 优惠券详情页面
 * - 展示已领取优惠券的详细信息
 * - 显示优惠券名称、描述、金额/折扣、有效期
 * - 根据 URL 参数 id 获取优惠券详情
 */
import { ref, computed } from "vue";
import type { CouponFilterState } from "~/types/user/coupon.d";
import { getMyCouponInfo } from "~/api/user/coupon";
import { useCommonStore } from "@/store/common";
const commonStore = useCommonStore();
const { t } = useI18n();
const title = ref(t("优惠券详情"));
const description = ref("");

useSeo({ title, description });
const loading = ref<boolean>(true);
const formState = ref<CouponFilterState>({});
const route = useRoute();
const fetchCouponDetail = async () => {
    try {
        const result = await getMyCouponInfo(route.query.id as string);
        Object.assign(formState.value, result);
        if (formState.value.couponName) title.value = formState.value.couponName;
        if (formState.value.couponDesc) description.value = formState.value.couponDesc;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
onMounted(() => {
    fetchCouponDetail();
});
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.coupon-info {
    background: #fff5ce url("/assets/images/coupon/getBg.jpg") no-repeat center top;
    display: flex;
    width: 100%;

    justify-content: center;
    background-size: cover;
    margin-bottom: -30px;

    .coupon-card-info {
        display: flex;
        margin: 50px 0;
        flex-wrap: wrap;
        gap: 20px;
        width: 500px;
        box-shadow: 0 0 10px #999;
        height: 400px;
        background-color: white;
        border-radius: 20px;
        box-sizing: border-box;
        padding: 70px 50px;
        flex-direction: column;

        .ticket-info {
            margin: 0 auto;
            width: 324px;

            .ti-detail {
                border-radius: 6px;
                border: 1px solid #dfdfdf;
                border-bottom: 1px none;
                border-top: 5px solid var(--general);

                .ti-detail-cont {
                    display: flex;
                    padding: 20px;

                    .ti-detail-name {
                        flex: 1;

                        & > h3 {
                            font-size: 14px;
                            line-height: 22px;
                            height: 22px;
                        }

                        & > p {
                            line-height: 20px;
                        }
                    }

                    .ti-detail-money {
                        width: auto;
                    }
                }
            }

            .ti-time {
                padding-top: 12px;
                height: 31px;
                background: #f8f8f8 url("/assets/images/coupon/borderWave.jpg") no-repeat 0 0;
                color: #999;
                text-align: center;
                font-size: 12px;
                line-height: 26px;
            }
        }

        .result-div {
            display: flex;
            margin: 0 auto;
            width: 324px;
            height: 100px;
            line-height: 100px;
            justify-content: center;
            font-size: 16px;
            color: #666666;
            align-items: center;
            gap: 10px;

            .icon {
                float: left;
                width: 32px;
                height: 32px;
                background: url("/assets/images/coupon/getSprit.png") no-repeat -26px 0px;
            }
        }
    }
}
</style>
