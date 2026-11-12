<template>
    <CommonPageHeader page="index" :showService="false"></CommonPageHeader>
    <CommonPageContainer v-if="!loading">
        <div class="info-row">
            <div class="gift-card-container">
                <div v-if="giftCardDetail" class="card-info">
                    <div class="card-wrapper">
                        <giftCard :card-info="giftCardDetail" card-size="large" :show-actions="isClaimGiftCardSuccess" @copy-all="handleGiftCardCopyAll" />
                    </div>
                </div>
                <div class="use-step group">
                    <div class="use-step-header group-header">
                        <div class="line left"></div>
                        <div class="group-title">{{ $t("使用方法") }}</div>
                        <div class="line right"></div>
                    </div>
                    <div class="use-step-content">
                        <div class="use-step-item step-1">
                            <div class="icon">
                                <img class="img" src="~/assets/images/giftCard/cart.png" />
                            </div>
                            <div class="text">{{ $t("礼品卡下单") }}</div>
                        </div>
                        <div class="next-icon">
                            <div class="iconfont-pc icon-a-gengduo20"></div>
                        </div>
                        <div class="use-step-item">
                            <div class="icon">
                                <img class="img" src="~/assets/images/giftCard/exchange.png" />
                            </div>
                            <div class="text">{{ $t("礼品卡兑换") }}</div>
                        </div>
                        <div class="next-icon">
                            <div class="iconfont-pc icon-a-gengduo20"></div>
                        </div>
                        <div class="use-step-item">
                            <div class="icon">
                                <img class="img" src="~/assets/images/giftCard/card.png" />
                            </div>
                            <div class="text">{{ $t("输入卡号卡密") }}</div>
                        </div>
                        <div class="next-icon">
                            <div class="iconfont-pc icon-a-gengduo20"></div>
                        </div>
                        <div class="use-step-item">
                            <div class="icon">
                                <img class="img" src="~/assets/images/giftCard/product.png" />
                            </div>
                            <div class="text">{{ $t("立即兑换") }}</div>
                        </div>
                    </div>
                </div>
                <div v-if="giftCardGroupDetail?.giftCardProducts.length" class="products group">
                    <div class="group-header">
                        <div class="line left"></div>
                        <div class="group-title">{{ $t("兑换商品") }}</div>
                        <div class="line right"></div>
                    </div>
                    <div class="group-sub-header">{{ $t("该兑换卡可兑换商品") }}</div>
                    <div class="product-box">
                        <div v-for="(item, index) in giftCardGroupDetail.giftCardProducts" :key="index">
                            <productItem :product="item" :product-style="giftCardProductStyleMap[item.productId]" />
                        </div>
                    </div>
                </div>
                <div class="exchange-flow group">
                    <div class="group-header">
                        <div class="line left"></div>
                        <div class="group-title">{{ $t("兑换流程") }}</div>
                        <div class="line right"></div>
                    </div>
                    <div class="group-sub-header">{{ $t("该兑换卡可兑换商品") }}</div>
                    <div class="flows">
                        <div class="flow">
                            <div class="flow-title">
                                <div class="flow-no">1</div>
                                <div class="flow-title-text">{{ $t("购买礼品卡") }}</div>
                            </div>
                            <div class="flow-content">
                                <div class="flow-tips">
                                    {{ $t("在商城选择所需购买的礼品卡，点击立即购买，并支付订单。 支付成功后，礼品卡需在礼品卡兑换中兑换才可使用") }}
                                </div>
                            </div>
                        </div>
                        <div class="flow-second">
                            <div class="flow flow2">
                                <div class="flow-title">
                                    <div class="flow-no">2</div>
                                    <div class="flow-title-text">{{ $t("礼品卡兑换") }}</div>
                                </div>
                                <div class="flow-content">
                                    <div><img class="img" src="~/assets/images/giftCard/flow2.png" /></div>
                                </div>
                            </div>
                            <div class="flow flow3">
                                <div class="flow-title">
                                    <div class="flow-no">3</div>
                                    <div class="flow-title-text">{{ $t("使用礼品卡") }}</div>
                                </div>
                                <div class="flow-content">
                                    <div><img class="img" src="~/assets/images/giftCard/flow3.png" /></div>
                                </div>
                            </div>
                        </div>
                        <div class="flow-third">
                            <div class="flow flow4">
                                <div class="flow-title">
                                    <div class="flow-no">4</div>
                                    <div class="flow-title-text">{{ $t("礼品卡支付") }}</div>
                                </div>
                                <div class="flow-content">
                                    <div><img class="img" src="~/assets/images/giftCard/flow4.png" /></div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        <div class="flex align-center justify-center actions-container">
            <div class="actions">
                <giftCardActions
                    :show-transfer="false"
                    :exchange-class="'exchange-btn'"
                    :disabled="!isClaimGiftCardSuccess"
                    :default-card-number="copiedCardNumber"
                    :default-card-password="copiedCardPassword"
                />
            </div>
        </div>
    </CommonPageContainer>
</template>

<script lang="ts" setup>
/**
 * 礼品卡转赠领取页
 * - 通过分享链接进入，自动领取礼品卡
 * - 展示礼品卡信息、使用方法、可兑换商品
 * - 领取成功后可一键复制卡号卡密进行兑换
 */
import { getGiftCardDetail, getGiftCardGroupDetail, giftCardClaim } from "~/api/giftCard/giftCard";
import giftCardActions from "../src/giftCardActions.vue";
import productItem from "../src/productItem.vue";
import giftCard from "../src/giftCard.vue";
import type { GiftCardGroupDetail, GiftCardInfo } from "~/types/giftCard/giftCard";
import { computed } from "vue";
import { useProductStyle } from "~/hooks";

const loading = ref(true);
const route = useRoute();
const giftCardSn = ref<string>("");
const giftCardDetail = ref<GiftCardInfo>();
const giftCardGroupDetail = ref<GiftCardGroupDetail>();
const giftCardProductsForStyle = computed(() => giftCardGroupDetail.value?.giftCardProducts ?? []);
const { styleMap: giftCardProductStyleMap } = useProductStyle(giftCardProductsForStyle);
const isClaimGiftCardSuccess = ref(false);

/** 存储用户点击"一键复制"后的卡号卡密，用于自动填充兑换表单 */
const copiedCardNumber = ref("");
const copiedCardPassword = ref("");

/** 处理一键复制回调，保存卡号卡密 */
const handleGiftCardCopyAll = (cardNumber: string, cardPassword: string) => {
    copiedCardNumber.value = cardNumber;
    copiedCardPassword.value = cardPassword;
};

const _getGiftCardDetail = async (giftCardSn: string) => {
    const result = await getGiftCardDetail(giftCardSn);
    giftCardDetail.value = result;
};

/**
 * 领取礼品卡
 */
const _claimGiftCard = async (giftCardSn: string) => {
    try {
        await giftCardClaim({ giftCardSn });
        isClaimGiftCardSuccess.value = true;
    } catch (error: any) {
        message.error(error.message);
        isClaimGiftCardSuccess.value = false;
    }
};

/**
 * 初始化页面：领取礼品卡并获取详情
 * - 先调用领取接口（若已领取会提示）
 * - 再获取礼品卡详细信息展示
 */
const getDetail = async () => {
    try {
        giftCardSn.value = String(route.params.id);
        if (!giftCardSn.value) {
            navigateTo("/404");
        }
        await _claimGiftCard(giftCardSn.value);
        await _getGiftCardDetail(giftCardSn.value);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

getDetail();
</script>

<style scoped lang="less">
.container {
    // position: relative;
    // z-index: 1;
}
.gift-card-container {
    padding: 0 30px 100px;
    width: 550px;
    margin: 0 auto;
    background-color: #fbeede;
    position: relative;
    z-index: 1;
    box-sizing: border-box;
}

.card-info {
    padding: 40px 0;
    display: flex;
    justify-content: center;
    border-bottom: 1px solid #a9781b;
    .card-wrapper {
        width: 400px;
        height: 232px;
        .gift-card-box {
            box-shadow: 0px 12px 9px 0px rgba(0, 14, 43, 0.17);
        }
    }
}

.group {
    .group-header {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 20px;
        .group-title {
            font-size: 20px;
            color: #a9781b;
        }
        .line {
            height: 2px;
            width: 100px;
            &.left {
                background: linear-gradient(90deg, #fbeede, #bd983d);
            }
            &.right {
                background: linear-gradient(90deg, #bd983d, #fbeede);
            }
        }
    }
    .group-sub-header {
        color: #a9781b;
        font-size: 16px;
        padding-top: 15px;
        text-align: center;
    }
}

.use-step {
    padding-top: 50px;
    .use-step-content {
        padding-top: 25px;
        display: flex;
        align-items: center;
        justify-content: center;
        .use-step-item {
            display: flex;
            // flex-shrink: 0;
            // flex-grow: 0;
            box-sizing: border-box;
            align-items: center;
            gap: 8px;
            border: 1px solid #a9781b;
            background-color: #fff;
            padding: 10px 7px;
            border-radius: 12px;
            width: 150px;
            height: 66px;
            .icon {
                width: 28px;
                height: 28px;
                flex-shrink: 0;
                .img {
                    width: 100%;
                    height: 100%;
                }
            }
            .text {
                font-size: 16px;
                color: #a9781b;
            }
        }
        .next-icon {
            .icon-a-gengduo20 {
                font-size: 16px;
                padding: 0 4px;
                color: #a9781b;
            }
        }
    }
}

.products {
    margin-top: 80px;
    .product-box {
        border: 1px solid #a9781b;
        border-radius: 12px;
        padding: 25px 16px;
        background: #fff;
        margin-top: 55px;
        display: flex;
        flex-direction: column;
        row-gap: 30px;
    }
}
.exchange-flow {
    margin-top: 50px;
    .flows {
        margin-top: 30px;
        .flow-title {
            display: flex;
            align-items: baseline;
            gap: 15px;
            .flow-no {
                font-size: 40px;
                width: 30px;
                color: #a9781b;
                font-weight: 400;
                font-family: DengXian;
                // 倾斜
                transform: skewX(-6deg);
            }
            .flow-title-text {
                color: #000000;
                font-size: 20px;
                font-weight: 400;
            }
        }
        .flow-content {
            padding-left: 44px;
            .img {
                object-fit: contain;
            }
        }
        .flow-tips {
            color: #6a6a6a;
            font-size: 16px;
            font-weight: 400;
            line-height: 36px;
            margin-top: 10px;
        }
        .flow-second {
            display: flex;
            gap: 25px;
            margin-top: 50px;
        }
        .flow-third {
            display: flex;
            gap: 25px;
            margin-top: 26px;
        }
        .flow2 {
            .flow-content {
                padding-left: 30px;
            }
            .img {
                width: 200px;
                // height: 154.54px;
            }
        }
        .flow3 {
            .flow-content {
                padding-left: 30px;
            }
            .img {
                width: 180px;
                // height: 152.55px;
            }
        }
        .flow4 {
            .flow-content {
                padding-left: 30px;
            }
            .img {
                width: 200px;
                // height: 252.86px;
            }
        }
    }
}
.order-info-btn {
    height: 100%;
    width: 100%;
    display: flex;
    justify-content: flex-end;
    padding: 11px 25px;
    .order-info-btn-box {
        display: flex;
        align-items: center;
        flex: 1;
        .btn {
            margin-right: 10px;
            flex: 1;
            height: 100%;
            &:last-child {
                margin-right: 0;
            }
        }
    }
}
.actions-container {
    height: 80px;
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    background-color: #fff;
    z-index: 99;
    .actions {
        width: 550px;
        :deep(.exchange-btn) {
            padding: 20px 30px;
            font-size: 16px;
        }
    }
}
</style>
