<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader title="礼品卡兑换"></CommonHeader>
    <TopJumpBar tltle="礼品卡兑换"></TopJumpBar>
    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="title-or-tabs">
                <span>{{ $t("礼品卡兑换") }}</span>
            </div>
            <div class="security-info">
                <div class="form-card">
                    <el-form ref="formRef" :model="form" :rules="rules" :label-width="labelWidth" class="gift-form">
                        <el-form-item :label="$t('卡号')" prop="cardNumber">
                            <el-input v-model="form.cardNumber" :placeholder="$t('请输入卡号')" />
                        </el-form-item>
                        <el-form-item :label="$t('卡密')" prop="cardPassword">
                            <el-input v-model="form.cardPassword" :placeholder="$t('请输入卡密')" />
                        </el-form-item>
                    </el-form>
                    <div class="form-actions" :style="{ paddingLeft: labelWidth }">
                        <el-button type="primary" :loading="exchangeLoading" @click="handleExchange">{{ $t("立即兑换") }}</el-button>
                    </div>
                </div>
            </div>
            <div class="security-info">
                <div class="bordered-div">
                    <div class="title">{{ $t("兑换规则") }}</div>
                    <p>{{ $t("1. 储值卡输入卡密进行兑换将自动充值到余额") }}。</p>
                    <p>{{ $t("2. 兑换卡输入卡密进行兑换后将跳转至该商品详情页进行兑换") }}。</p>
                </div>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 礼品卡兑换页
 * - 输入卡号卡密进行兑换
 * - 储值卡：兑换后自动充值到余额，跳转账户明细
 * - 兑换卡：兑换后跳转订单结算页
 */
import { computed, reactive, ref } from "vue";
import type { FormInstance } from "element-plus";
import { giftCardExchange } from "~/api/giftCard/giftCard";
import { useUserStore } from "~/store/user";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";

definePageMeta({
    middleware: "auth"
});

const { t, locale } = useI18n();
const userStore = useUserStore();

// 英文标签较长，需要更宽的 label-width 避免换行
const labelWidth = computed(() => (String(locale.value).toLowerCase().startsWith("zh") ? "60px" : "90px"));

const formRef = ref<FormInstance>();
const exchangeLoading = ref(false);
const form = reactive({
    cardNumber: "",
    cardPassword: ""
});
const rules = {
    cardNumber: [{ required: true, message: t("请输入卡号"), trigger: "blur" }],
    cardPassword: [{ required: true, message: t("请输入卡密"), trigger: "blur" }]
};

/**
 * 提交兑换
 * - 兑换卡：返回flowType，跳转订单确认页
 * - 储值卡：无flowType，刷新用户余额后跳转账户明细
 */
const handleExchange = async () => {
    if (exchangeLoading.value || !formRef.value) {
        return;
    }
    exchangeLoading.value = true;
    await formRef.value.validate(async (valid, fields) => {
        if (valid) {
            try {
                const res: any = await giftCardExchange(form);
                message.success(t("兑换成功"));
                if (res.exchangeProductToCart?.flowType) {
                    navigateTo(`/order/check?flowType=${res.exchangeProductToCart.flowType}`);
                } else {
                    userStore.userInfoLoaded = false;
                    await userStore.getUserInfo();
                    setTimeout(() => {
                        navigateTo("/member/account/detail");
                    }, 200);
                }
            } catch (error: any) {
                message.error(error.message);
            } finally {
                exchangeLoading.value = false;
            }
        } else {
            exchangeLoading.value = false;
            console.error("error submit!", fields);
        }
    });
};
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.security-info {
    background: #fff;
    border: 0;
    padding: 20px;
}

.form-card {
    padding: 12px 0 24px 0;
    max-width: 520px;
}

.form-title {
    font-size: 16px;
    font-weight: 600;
    margin-bottom: 18px;
    color: #333;
}

.form-actions {
    display: flex;
    width: 100%;
}

.gift-form {
    :deep(.el-form-item__label) {
        white-space: nowrap;
    }
}

.bordered-div {
    position: relative;
    border: 1px solid #ddd;
    width: 100%;
    padding: 20px;
    line-height: 3;
    box-sizing: border-box;
}

.bordered-div .title {
    position: absolute;
    top: -1.5em;
    left: 20px;
    background: white;
    padding: 0 15px;
    color: #666666;
    font-weight: bold;
    font-size: 12px;
}

.bordered-div::before {
    content: "";
    position: absolute;
    top: 0;
    left: 15px;
    width: calc(20px + 10px);
    height: 2px;
    background: white;
}
</style>
