<template>
    <!-- <CommonPageHeaderNew></CommonPageHeaderNew> -->
    <PageHeaderNew :isWhite="true"></PageHeaderNew>

    <CommonHeader title="充值"></CommonHeader>
    <TopJumpBar tltle="充值"></TopJumpBar>

    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="title-or-tabs">
                <div class="tag-and-input">
                    <div class="tig-tabs">
                        <MemberTopMenu :menuList="menuList"></MemberTopMenu>
                    </div>
                </div>
            </div>
            <div v-if="flow" class="account-deposit-content">
                <div v-loading="loading" class="card-list">
                    <template v-if="filterState.length > 0">
                        <template v-for="(item, index) in filterState">
                            <div v-if="item.rechargeId != 0" :class="{ selected: item.selected }" class="card" @click="selectCard(index)">
                                <div>
                                    <FormatPrice
                                        v-model="item.money"
                                        :currencyStyle="{ fontSize: '14px', lineHeight: '24px' }"
                                        :fontStyle="{ fontSize: '20px', fontWeight: 700 }"
                                        :showText="false"
                                    ></FormatPrice>
                                </div>
                                <div v-if="Number(item.discountMoney) > 0" class="tips">
                                    {{ $t("赠送") }}
                                    <FormatPrice v-model="item.discountMoney" :showText="false"></FormatPrice>
                                </div>
                            </div>
                            <div v-else :class="{ selected: item.selected }" class="card" @click="selectCard(index)">
                                <div>
                                    <el-input-number v-model="payInfo.amount" @change="onAmountChange" :controls="false" :precision="2"></el-input-number>
                                </div>
                                <div class="tips" style="margin-top: 5px">
                                    {{ $t("自定义金额") }}
                                </div>
                            </div>
                        </template>
                    </template>
                    <template v-else>
                        <div class="no-result">
                            <div v-if="!loading"></div>
                        </div>
                    </template>
                </div>
                <div class="bom-div">
                    <el-button v-if="!loading" class="big-button tj-submit" type="primary" @click="onSubmit">{{ $t("提交申请") }}</el-button>
                </div>
            </div>
            <div v-else class="account-deposit-content">
                <el-form :model="payInfo">
                    <el-form-item>
                        {{ $t("充值金额") }}：
                        <FormatPrice :showText="false" v-model="payInfo.money" :fontStyle="{ fontSize: '18px', color: 'var(--price)' }"></FormatPrice>
                    </el-form-item>
                    <el-form-item>
                        <span class="title">{{ $t("平台支付") }}</span> <span class="info">{{ $t("支持所有银行卡或信用卡，更迅速、安全") }}</span>
                    </el-form-item>
                    <el-form-item>
                        <div class="pay-list">
                            <el-radio-group v-model="payInfo.payType">
                                <el-radio v-if="payList.includes('wechat')" class="tab flex align-center" value="wechat">
                                    <div class="cont"><img src="/assets/images/extra/payment_method_wechat.jpg" /></div>
                                </el-radio>
                                <el-radio v-if="payList.includes('alipay')" class="tab flex align-center" value="alipay">
                                    <div class="cont"><img src="/assets/images/extra/payment_method_alipay.jpg" /></div>
                                </el-radio>
                                <el-radio v-if="payList.includes('yabanpayWechat')" class="tab flex align-center" value="yabanpayWechat">
                                    <div class="cont"><img src="/assets/images/extra/payment_method_wechat.jpg" /></div>
                                </el-radio>
                                <el-radio v-if="payList.includes('yabanpayAlipay')" class="tab flex align-center" value="yabanpayAlipay">
                                    <div class="cont"><img src="/assets/images/extra/payment_method_alipay.jpg" /></div>
                                </el-radio>
                                <el-radio v-if="payList.includes('yunpayWechat')" class="tab flex align-center" value="yunpayWechat">
                                    <div class="cont"><img src="/assets/images/extra/payment_method_wechat.jpg" /></div>
                                </el-radio>
                                <el-radio v-if="payList.includes('yunpayAlipay')" class="tab flex align-center" value="yunpayAlipay">
                                    <div class="cont"><img src="/assets/images/extra/payment_method_alipay.jpg" /></div>
                                </el-radio>
                                <el-radio v-if="payList.includes('lakalaWechat')" class="tab flex align-center" value="lakalaWechat">
                                    <div class="cont"><img src="/assets/images/extra/payment_method_wechat.jpg" /></div>
                                </el-radio>
                                <el-radio v-if="payList.includes('lakalaAlipay')" class="tab flex align-center" value="lakalaAlipay">
                                    <div class="cont"><img src="/assets/images/extra/payment_method_alipay.jpg" /></div>
                                </el-radio>
                            </el-radio-group>
                        </div>
                    </el-form-item>
                    <el-form-item>
                        <el-button @click="flow = true">{{ $t("返回") }}</el-button>
                        <el-button class="ml10" type="primary" @click="toPage">{{ $t("去付款") }}</el-button>
                    </el-form-item>
                    <el-form-item>
                        <el-checkbox v-model="checkbox"
                            >{{ $t("本人此操作将视为同意并接受") }}
                            <!-- <NuxtLink class="font-color" target="_blank" to='/member/userInvoice/confirmInfo'>《NexTrade{{$t('服务协议')}}》</NuxtLink> -->
                            <NuxtLink class="font-color" target="_blank" to="/member/account/src/serviceAgreement"
                                >《{{ $t("{0}服务协议", [commonStore.shopName]) }}》</NuxtLink
                            >
                        </el-checkbox>
                    </el-form-item>
                </el-form>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * @description 余额充值页面
 * - 支持选择预设金额或自定义金额进行充值
 * - 提供多种支付方式选择（支付宝、微信、银行卡等）
 * - 创建充值订单并跳转到支付页面
 */
import { reactive, ref, computed } from "vue";
import { useRouter } from "vue-router";
import type { DepositFilterState } from "~/types/user/account.d";
import { getDepositList, getPaymentList, updateRechargeOrder } from "~/api/user/account";
import { useCommonStore } from "~/store/common";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from '~/components/common/TopJumpBar.vue'
const commonStore = useCommonStore();
const filterState = ref<DepositFilterState[]>([]);
const loading = ref<boolean>(true);
const flow = ref<boolean>(true);
const checkbox = ref<boolean>(false);
definePageMeta({
    middleware: "auth"
});
const { t } = useI18n();
const loadFilter = async () => {
    loading.value = true;
    try {
        const result = await getDepositList();
        filterState.value = result;
        //自定义金额
        let temp: any = {
            rechargeId: 0
        };
        filterState.value.push(temp);
        filterState.value.forEach((item: any) => {
            item.selected = false;
        });
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
onMounted(() => {
    loadFilter();
    loadPayList();
});
const payList = ref<string[]>([]);

const loadPayList = async () => {
    loading.value = true;
    try {
        const result = await getPaymentList();
        // Apple Pay 充值为二期，一期充值页过滤隐藏 applepay（后端充值 create 不处理）
        payList.value = (result || []).filter((item: string) => item !== "applepay");
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

const payInfo = ref<any>({
    id: 0,
    amount: null,
    payType: "wechat"
});

const onAmountChange = (value: any) => {
    if (value && value < 0) {
        payInfo.value.amount = Math.abs(value);
    }
};

const onSubmit = async () => {
    let isSelectedItem = false;
    const selectedItem = filterState.value.find((item) => item.selected);
    if (selectedItem) {
        isSelectedItem = true;
        const { rechargeId } = selectedItem;
        payInfo.value.id = rechargeId;
        if (rechargeId !== 0) {
            Object.assign(payInfo.value, {
                amount: selectedItem.money,
                money: selectedItem.money
            });
        } else {
            payInfo.value.money = payInfo.value.amount;
        }
    }
    if (isSelectedItem) {
        if (payInfo.value.amount === null || payInfo.value.amount === "") {
            return message.error(t("请输入充值金额"));
        }
        if (payInfo.value.amount === 0) {
            return message.error(t("请输入正确的充值金额"));
        }
        try {
            const result: any = await updateRechargeOrder({ ...payInfo.value });
            orderId.value = result?.orderId;
        } catch (error: any) {
            message.error(error.message);
        }
        flow.value = false;
    } else {
        message.error(t("请选择充值金额"));
    }
};
const orderId = ref(0);

const toPage = async () => {
    try {
        if (!checkbox.value) {
            message.error(t("请先阅读并勾选服务协议"));
            return;
        }
        if (payInfo.value.payType === "wechat") {
            navigateTo("/member/account/src/wechat/?id=" + orderId.value, { open: { target: "_blank" } });
        }
        if (payInfo.value.payType === "alipay") {
            navigateTo("/member/account/src/alipay/?id=" + orderId.value, { open: { target: "_blank" } });
        }
        if (payInfo.value.payType === "paypal") {
            navigateTo("/member/account/src/paypal/?id=" + orderId.value, { open: { target: "_blank" } });
        }
        if (payInfo.value.payType === "yabanpayWechat") {
            navigateTo("/member/account/src/yabanpay/?type=wechat&id=" + orderId.value, { open: { target: "_blank" } });
        }
        if (payInfo.value.payType === "yabanpayAlipay") {
            navigateTo("/member/account/src/yabanpay/?type=alipay&id=" + orderId.value, { open: { target: "_blank" } });
        }
        if (payInfo.value.payType === "yunpayWechat") {
            navigateTo("/member/account/src/yunpay/?type=wechat&id=" + orderId.value, { open: { target: "_blank" } });
        }
        if (payInfo.value.payType === "yunpayAlipay") {
            navigateTo("/member/account/src/yunpay/?type=alipay&id=" + orderId.value, { open: { target: "_blank" } });
        }
        if (payInfo.value.payType === "lakalaWechat") {
            navigateTo("/member/account/src/lakala/?payCode=lakalaWechat&id=" + orderId.value, { open: { target: "_blank" } });
        }
        if (payInfo.value.payType === "lakalaAlipay") {
            navigateTo("/member/account/src/lakala/?payCode=lakalaAlipay&id=" + orderId.value, { open: { target: "_blank" } });
        }
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

const selectCard = (index: number) => {
    filterState.value.forEach((item, idx) => {
        item.selected = idx === index;
    });
};

const router = useRouter();
const splitStr = (str: string) => {
    return str.split("?")[0];
};
// 会员余额提现开关（D1107）：关闭时隐藏“提现”入口，不影响分销佣金提现
const menuList = computed<any>(() =>
    [
        { value: "账户明细", url: "/member/account/detail", size: 0 },
        { value: "申请记录", url: "/member/account/log", size: 0 },
        { value: "充值", url: "/member/account/deposit", size: 0 },
        { value: "提现", url: "/member/account/raplytocard", size: 0 },
        { value: "卡管理", url: "/member/account/cardManagement", size: 0 }
    ].filter((item) => item.url !== "/member/account/raplytocard" || commonStore.memberWithdrawalEnabled === 1)
);
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.account-deposit-content {
    background: #fff;
    border: 0;
    padding: 20px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;

    .title {
        color: #333;
        font-size: 18px;
        font-weight: 400;
        margin-right: 10px;
    }

    .info {
        color: #999;
        font-size: 12px;
    }

    .go-pay {
        width: 90px;
        height: 30px;
        line-height: 30px;
    }

    .pay-list {
        display: flex;
        gap: 10px;
        width: 800px;

        .pay {
            width: 170px;
            height: 50px;
            display: flex;
            border: 1px solid #d9d9d9;
        }
    }

    .card-list {
        display: flex;
        flex-direction: row;
        flex-wrap: wrap;
        align-content: flex-start;
        gap: 10px;

        .card {
            width: 32%;
            height: 60px;
            position: relative; /* 为了绝对定位内部的三角形 */
            cursor: pointer; /* 可选，鼠标悬停时显示为指针 */
            color: #1a1a1a;
            text-align: center;
            padding: 15px 0;
            border: 1px solid #ddd;
            font-weight: 700;
            font-size: 20px;
            display: flex;
            flex-direction: column;
            line-height: 1.5;
            justify-content: center;

            :deep(.el-input__inner){
                font-size: 14px;
            }

            span {
                font-size: 14px;
            }

            .tips {
                font-size: 14px;
                font-weight: 300;
            }
        }

        .selected {
            border-color: var(--general); /* 选中状态下边框变红 */
        }

        .selected::after {
            content: "";
            position: absolute;
            bottom: 0; /* 三角形在右下角，但留有一点间距 */
            right: 0;
            width: 0;
            height: 0;
            border-left: 20px solid transparent;
            border-top: 20px solid transparent;
            border-bottom: 20px solid var(--general); /* 三角形大小和颜色 */
            font-size: 20px; /* 勾的大小 */
            line-height: 40px; /* 根据三角形大小调整，确保勾垂直居中 */
            text-align: center;
            transform: rotate(-0deg); /* 调整勾的位置和角度 */
        }

        .selected::before {
            content: "✔";
            color: var(--main-text); /* 勾的颜色 */
            font-size: 10px; /* 勾的大小 */
            position: absolute;
            bottom: 5px; /* 调整勾在三角形内的垂直位置 */
            right: 0; /* 调整勾在三角形内的水平位置 */
            transform: translateX(0%) translateY(50%); /* 微调勾的位置 */
            z-index: 100;
        }
    }

    .tj-submit {
        margin-top: 30px;
    }
}

.bom-div {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
}
.cont {
    width: 136px;
    display: flex;
    align-items: center;
    justify-content: center;
    & > img {
        max-width: 100%;
        max-height: 35px;
        object-fit: contain;
    }
}
</style>
