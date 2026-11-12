<template>
    <PageHeaderNew
        :isWhite="true"
        :showService="showService"
        :phone="formState.shop ? formState.shop.kefuPhone : ''"
        :orderId="formState.orderId"
        :shopId="formState.shopId"
    ></PageHeaderNew>
    <CommonHeader title="拼团详情"></CommonHeader>
    <TopJumpBar tltle="拼团详情"></TopJumpBar>
    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row" v-loading="loading">
            <div class="title-or-tabs">
                <span>{{ $t("拼团详情") }}</span>
            </div>
            <div class="order-info">
                <div class="page-info-body lr">
                    <div class="order-left">
                        <div>
                            <span
                                >{{ $t("订单号") }}：<strong>{{ orderFormState.orderSn ? orderFormState.orderSn : "--" }}</strong></span
                            >

                            <span style="margin-left: 40px"
                                >{{ $t("拼团状态") }}：
                                <strong>
                                    {{ formState.joinStatusDesc || "" }}
                                    <span v-if="formState.joinStatus === 1"
                                        >,{{ $t("还差") }}{{ (formState.minGroupNum || 0) - (formState.joinUsers?.length || 0) > 0 ? (formState.minGroupNum || 0) - (formState.joinUsers?.length || 0) : 0 }}{{ $t("人成团") }}</span
                                    >
                                </strong>
                            </span>
                        </div>
                        <div>{{ $t(oStatusInfo[orderFormState.orderStatus] || "") }}</div>
                    </div>
                    <div class="order-right">
                        <el-button @click="onInviteFriend" type="primary" :disabled="formState.joinStatus !== 1">{{ $t("邀请好友参团") }}</el-button>
                    </div>
                </div>

                <div class="page-info-body">
                    <div class="sub-title group-sub-title">
                        <div class="sub-title-l">
                            <span> {{ formState.joinStatusDesc || "" }}</span>
                            <span v-if="formState.joinStatus === 1">
                                &nbsp;{{ $t("还差") }}
                                <span style="color: var(--main-bg)">{{
                                    (formState.minGroupNum || 0) - (formState.joinUsers?.length || 0) > 0
                                        ? (formState.minGroupNum || 0) - (formState.joinUsers?.length || 0)
                                        : 0
                                }}</span>
                                {{ $t("人成团") }}
                            </span>
                        </div>
                        <div class="sub-title-r" v-if="formState.joinStatus === 1">
                            <CountdownNew countdown-type="specifyTime" :specify-time="new Date(formState?.endTime)?.getTime()"> </CountdownNew>
                        </div>
                    </div>
                    <div class="group-content">
                        <div class="group-content-item" v-for="item in joinUserList" :key="item.userId">
                            <div class="group-content-item-image">
                                <div v-if="item.classIcon" :class="item.classIcon" class="iconBox" @click="onInviteFriend" style="cursor: pointer"></div>
                                <div v-else class="group-content-item-image-img">
                                    <img
                                        :src="item.avatar ? item.avatar : '/assets/avatar/default.gif'"
                                        :alt="item.username"
                                    />
                                </div>
                                <div class="group-content-item-image-mask" :style="[{whiteSpace: isEn ? 'wrap' : 'nowrap'}]" v-if="item.leader">{{ $t("团长") }}</div>
                            </div>
                            <div class="group-content-item-bottom" :style="[{marginTop: isEn ? '10px' : ''}]">
                                <span :style="[{overflow: isEn && item.username === '待加入' ? 'visible' : 'hidden'}]">{{ item.username === "待加入" ? $t("待加入") : item.username }}</span>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="page-info-body">
                    <div class="sub-title">{{ $t("拼团流程") }}</div>
                    <div class="group-steps">
                        <div class="group-steps-content">
                            <div class="group-steps-item">
                                <img src="/assets/images/group/group_open.png" />
                                <span>{{ $t("1.开团/参加") }}</span>
                            </div>
                            <span style="display: inline-flex; align-items: center; margin: 0 12px">
                                <span
                                    v-for="i in 3"
                                    :key="i"
                                    style="display: inline-block; width: 8px; height: 8px; border-radius: 50%; background: #ccc; margin: 0 3px"
                                >
                                </span>
                            </span>
                            <div class="group-steps-item">
                                <img src="/assets/images/group/group_invite.png" />
                                <span>{{ $t("2.邀友参团") }}</span>
                            </div>
                            <span style="display: inline-flex; align-items: center; margin: 0 12px">
                                <span
                                    v-for="i in 3"
                                    :key="i"
                                    style="display: inline-block; width: 8px; height: 8px; border-radius: 50%; background: #ccc; margin: 0 3px"
                                >
                                </span>
                            </span>
                            <div class="group-steps-item">
                                <img src="/assets/images/group/group_done.png" />
                                <span>{{ $t("3.人满成团") }}</span>
                            </div>
                        </div>
                    </div>
                </div>

                <div class="page-info-body">
                    <div class="sub-title">{{ $t("订单商品") }}</div>
                    <table class="order-table-info">
                        <thead>
                            <tr class="receipt-header">
                                <th>{{ $t("商品名称") }}</th>
                                <th>{{ $t("商品价格") }}</th>
                                <th>{{ $t("购买数量") }}</th>
                                <th>{{ $t("小计") }}</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-for="item in orderFormState.items" class="receipt-body">
                                <td class="receipt-item product-name">
                                    <div class="product-info">
                                        <div>
                                            <img :alt="item.productName" :src="productImage(item)" class="img-border" />
                                        </div>
                                        <div>
                                            <p class="product-info__name">{{ item.productName }}</p>
                                            <div class="sku-style" v-if="item.skuData">
                                                <div v-for="sku in item.skuData">{{ sku.name }}：{{ sku.value }}</div>
                                            </div>
                                            <template v-if="item.extraSkuData && item.extraSkuData.length">
                                                <div class="extraskudata">
                                                    <template v-for="attr in item.extraSkuData" :key="attr">
                                                        <div class="extraskudata-item">
                                                            <div class="title">{{ attr.attrName }}</div>
                                                            <div class="line-box">
                                                                <div class="line"></div>
                                                            </div>
                                                            <div class="content">
                                                                <span class="content-text">{{ attr.attrValue }}</span>
                                                                <FormatPrice class="price" v-model="attr.attrPrice"></FormatPrice>
                                                                <span class="num">x{{ item.quantity }}</span>
                                                            </div>
                                                        </div>
                                                    </template>
                                                </div>
                                            </template>
                                        </div>
                                    </div>
                                </td>
                                <td class="receipt-item price">
                                    <FormatPrice v-model="item.price"></FormatPrice>
                                </td>
                                <td class="receipt-item price">{{ item.quantity }}</td>
                                <td class="receipt-item price">
                                    <FormatPrice v-model="item.subtotal"></FormatPrice>
                                </td>
                            </tr>
                        </tbody>
                        <tfoot>
                            <tr class="receipt-footer">
                                <th :colspan="orderFormState.availableActions.toAftersales ? 3 : 2"></th>
                                <th class="receipt-footer-it one" colspan="2" :style="[{minWidth: isEn ? '200px' : '150px'}]">
                                    <div class="item">
                                        <div class="label">{{ $t("商品总价") }}：</div>
                                        <div class="text">
                                            <FormatPrice v-model="orderFormState.productAmount"></FormatPrice>
                                        </div>
                                    </div>
                                    <div class="item" v-if="orderFormState.serviceFee && orderFormState.serviceFee > 0">
                                        <div class="label">{{ $t("附加费用") }}：</div>
                                        <div class="text">
                                            <FormatPrice v-model="orderFormState.serviceFee"></FormatPrice>
                                        </div>
                                    </div>
                                    <div class="item" v-if="orderFormState.shippingFee! > 0">
                                        <div class="label">{{ $t("运费") }}：</div>
                                        <div class="text">
                                            <FormatPrice v-model="orderFormState.shippingFee"></FormatPrice>
                                        </div>
                                    </div>
                                    <div class="item" v-if="orderFormState.balance! > 0">
                                        <div class="label">{{ $t("使用余额") }}：</div>
                                        <div class="text">
                                            <FormatPrice v-model="orderFormState.balance"></FormatPrice>
                                        </div>
                                    </div>
                                    <div class="item" v-if="orderFormState.pointsAmount! > 0">
                                        <div class="label">{{ $t("{0}减免", [commonStore.integralName]) }}：</div>
                                        <div class="text">
                                            <FormatPrice v-model="orderFormState.pointsAmount"></FormatPrice>
                                        </div>
                                    </div>
                                    <div class="item" v-if="orderFormState.discountAmount! > 0">
                                        <div class="label">{{ $t("其他优惠") }}：</div>
                                        <div class="text">
                                            -
                                            <FormatPrice v-model="orderFormState.discountAmount"></FormatPrice>
                                        </div>
                                    </div>
                                    <div class="item" v-if="orderFormState.couponAmount! > 0">
                                        <div class="label">{{ $t("优惠券优惠") }}：</div>
                                        <div class="text">
                                            -
                                            <FormatPrice v-model="orderFormState.couponAmount"></FormatPrice>
                                        </div>
                                    </div>
                                    <div class="item" v-if="orderFormState.unpaidAmount && orderFormState.unpaidAmount > 0">
                                        <div class="label">{{ $t("待支付金额") }}：</div>
                                        <div class="text">
                                            <FormatPrice v-model="orderFormState.unpaidAmount"></FormatPrice>
                                        </div>
                                    </div>
                                    <div class="item">
                                        <span v-if="orderFormState.payStatus == 2">{{ $t("实付款") }}：</span>
                                        <span v-else>{{ $t("应付款") }}：</span>
                                        <FormatPrice v-model="orderFormState.totalAmount" :fontStyle="{ fontSize: '20px', color: 'var(--price)' }">
                                        </FormatPrice>
                                    </div>
                                </th>
                            </tr>
                        </tfoot>
                    </table>
                </div>
            </div>
        </div>
        <!-- 邀请弹框 -->
        <el-dialog v-model="showInviteDialog" :title="$t('通过链接邀请')" width="600">
            <div class="invite-dialog-content">
                <el-input v-model="inviteLink" :disabled="true" size="large" class="invite-dialog-input" />
                <el-button type="primary" @click="copyInviteLink">{{ $t("复制链接") }}</el-button>
            </div>
            <template #footer>
                <div class="dialog-footer">
                    <el-button @click="showInviteDialog = false">{{ $t("关闭") }}</el-button>
                </div>
            </template>
        </el-dialog>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 拼团订单详情页（会员中心）
 * - 展示拼团状态、参团成员、剩余时间
 * - 支持分享邀请好友参团
 * - 显示关联的订单商品信息
 */
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import type { JoiningDetail, JoinUser } from "~/types/joining/joining";
import type { OrderFormState } from "~/types/user/order";
import { getOrder } from "~/api/user/order";
import { getJoiningDetail } from "~/api/joining/joining";
import { message, Modal } from "ant-design-vue";
import { useCommonStore } from "~/store/common";
import { CountdownNew } from "@/components/product";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";

import { useI18nStore } from "~/store/i18n";

const i18nStore = useI18nStore();
const isEn = computed(() => i18nStore.XLocaleCode?.includes("en"));

definePageMeta({
    middleware: "auth"
});

const commonStore = useCommonStore();
const { t } = useI18n();
const router = useRouter();
const formState = ref<JoiningDetail>({
    joinStatus: 0,
    grouponPromotionRecordId: 0,
    grouponPromotionItemId: 0,
    productName: "",
    joinType: 0,
    minGroupNum: 0,
    shopId: 0,
    shopTitle: "",
    grouponImage: "",
    minGrouponPrice: 0,
    productPrice: 0,
    picThumb: "",
    joinNumCount: 0,
    joinUsers: [],
    endTime: "",
    orderId: 0,
    productSn: "",
    joinStatusDesc: "",
    shop: {}
} as JoiningDetail);

const orderFormState = ref<OrderFormState>({
    orderSn: "",
    orderId: 0,
    items: [],
    availableActions: {},
    shop: {}
} as OrderFormState);

const loading = ref<boolean>(true);
const joinUserList = ref<JoinUser[]>([]);
const showInviteDialog = ref<boolean>(false);
const inviteLink = ref<string>("");

const oStatusInfo = ref([
    "您的订单已提交成功，请尽快完成付款哦！",
    "感谢您在本店的购物，您的订单已提交成功，我们会尽快为您发货！",
    "您的订单已经发货，请注意查收，祝您购物愉快！",
    '您的订单已被取消，您可以通过"再次购买"，重新购买订单中的商品。',
    '您的订单被认定为无效订单，订单已被关闭。您可以通过"再次购买"，重新购买订单中的商品。给您造成的不便请多多谅解！',
    "您的订单已经确认收货，感谢您在本店购物，祝您生活愉快！"
]);

const joinStatusInfo = ref(["待成团", "拼团成功", "拼团失败"]);

//获取订单信息
const fetchOrder = async (id: number) => {
    try {
        const result = await getOrder({ id });
        Object.assign(orderFormState.value, result);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
//获取拼团信息
const fetchJoiningDetail = async () => {
    loading.value = true;
    try {
        const result = await getJoiningDetail({ grouponPromotionRecordId: router.currentRoute.value.query.id });
        console.log(result, 11);
        formState.value = result;
        joinUserList.value = [...result.joinUsers];
        //如果参团人数不够，显示占位
        if ((result.joinUsers?.length || 0) < formState.value.minGroupNum && formState.value.joinStatus === 1) {
            joinUserList.value.push({
                userId: 0,
                // i18n-ignore-next-line 占位哨兵值，模板里按 username === "待加入" 判定后再 $t
                username: "待加入",
                avatar: "/assets/avatar/default.gif",
                classIcon: "iconfont-pc icon-zengjia"
            });
        }
        //将joinUserList.value中的leader（团长）为true的放在最前面
        joinUserList.value.sort((a, b) => {
            return a.leader === true ? -1 : 1;
        });
        fetchOrder(result.orderId);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

const onInviteFriend = () => {
    showInviteDialog.value = true;
    inviteLink.value = location.origin + "/joining/groupByInvited?" + "id=" + router.currentRoute.value.query.id;
};

const copyInviteLink = async () => {
    copyText(
        inviteLink.value,
        () => {
            message.success(t("复制成功"));
        },
        () => {
            message.error(t("复制失败，请手动复制"));
        }
    );
    showInviteDialog.value = false;
};

const showService = computed(() => {
    if (formState.value.shop && Object.keys(formState.value.shop).length > 0) {
        if (formState.value.shop.kefuInlet && formState.value.shop.kefuInlet.length > 0 && formState.value.shop.kefuInlet.includes(2)) {
            return true;
        }
        return false;
    }
    return true;
});

// onMounted(() => {
fetchJoiningDetail();
// });
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.order-info {
    display: flex;
    flex-direction: column;
    gap: 10px;

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

    .sub-title {
        color: #333333;
        font-size: 16px;
        font-weight: 400;
        letter-spacing: 0;
        display: flex;
        justify-content: space-between;
        align-items: center;
        width: 100%;

        .sub-title-l {
            color: #333333;
        }
    }

    .order-ul {
        li {
            line-height: 3;
            color: #666;

            span {
                color: #333333;
            }
        }

        &.virtualsample-box {
            .virtualsample-item {
                display: flex;

                .item {
                    padding-right: 10px;

                    &:last-child {
                        padding-right: 0;
                    }
                }
            }
        }
    }

    .order-table-info {
        width: 100%;
        margin: 10px auto;
        background-color: #fcfcfc;
        border-collapse: collapse;
        border: 1px solid #eee;

        .receipt-header {
            line-height: 2;
            border-bottom: 1px solid #eeeeee;

            & > th {
                padding: 10px;
            }
        }

        .receipt-body {
            background-color: white;
            border: 1px solid #eeeeee;

            .receipt-item {
                padding: 10px;
                border: 1px solid #f0f1f2;
                color: #1a1a1a;
                font-size: 14px;
                .product-info {
                    display: flex;
                    align-items: flex-start;
                    gap: 10px;
                    flex-wrap: nowrap;
                    img {
                        height: 60px;
                        width: 60px;
                        border-radius: 4px;
                        margin-right: 10px;
                    }
                }

                /* 商品信息区域可收缩，长文本在右侧区域内换行 */
                .product-info > div:last-child {
                    flex: 1;
                    min-width: 0;
                    overflow-wrap: anywhere;
                }

                /* 商品名称最多两行，超出显示省略号 */
                .product-info__name {
                    display: -webkit-box;
                    -webkit-box-orient: vertical;
                    -webkit-line-clamp: 2;
                    line-clamp: 2;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    word-break: break-word;
                    line-height: 1.4;
                    max-height: calc(1.4em * 2);
                    margin: 0;
                }
                :deep(.el-button) {
                    font-size: 14px !important;
                }
            }


            .sku-style {
                width: 100%;
                display: flex;
                flex-direction: column;

                & > div {
                    width: 100%;
                }
            }

            & > td:first-child > div {
                & > img {
                    height: 60px;
                    width: 60px;
                }

                display: flex;
                align-items: flex-start;
                gap: 20px;
                flex-wrap: wrap;
            }

            .operation {
                width: 80px;
            }
        }

        .receipt-footer {
            border-bottom: 1px dashed #e8e8e8;

            .receipt-footer-it {
                text-align: right;
                padding: 10px;
                line-height: 2;

                .item {
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    line-height: 28px;
                }

                .two {
                    color: #888;
                    font-size: 14px;
                }
            }
        }
    }
}

.group-steps {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 30px 0;

    .group-steps-content {
        width: 80%;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;

        .group-steps-item {
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 10px;

            span {
                font-size: 16px;
            }
        }
    }
}

.group-sub-title {
    border-bottom: 1px solid #f4f4f4;
    padding-bottom: 20px;
}

.group-content {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 20px 0;
    overflow-x: auto;

    &::-webkit-scrollbar {
        display: none;
    }

    .group-content-item {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 15px;
        padding: 0 20px;

        .group-content-item-image {
            background-color: #f4f4f4;
            position: relative;
            width: 68px;
            height: 68px;
            border-radius: 50%;

            .iconBox {
                position: absolute;
                left: 50%;
                top: 50%;
                transform: translate(-50%, -50%);
                font-size: 28px;
                color: #c3c3c3;
            }

            .group-content-item-image-img {
                width: 68px;
                height: 68px;
                border-radius: 50%;
                overflow: hidden;

                img {
                    width: 100%;
                    height: 100%;
                    object-fit: contain;
                }
            }

            .group-content-item-image-mask {
                position: absolute;
                bottom: 0;
                right: 50%;
                transform: translateX(50%) translateY(70%);
                background-color: var(--main-bg);
                color: #fff;
                border-radius: 2px;
                padding: 2.5px 4px;
                z-index: 999;
                white-space: nowrap;
            }
        }

        span {
            display: inline-block;
            font-size: 14px;
            font-weight: bold;
            color: #9a9a9a;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            max-width: 5em;
        }

        &:first-child .group-content-item-image {
            border: 2px solid var(--main-bg);
        }
    }
}

.invite-dialog-content {
    min-height: 100px;
    display: flex;
    align-items: center;
    gap: 10px;

    .invite-dialog-input {
        width: 100%;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
}
</style>
