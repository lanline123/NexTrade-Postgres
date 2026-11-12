<template>
    <div>
        <CommonHeader title="Stripe支付"></CommonHeader>
        <div class="loading-container" v-if="loading">
            <div class="loading-spinner"></div>
            <p>{{ $t("正在跳转到支付页面") }}...</p>
        </div>
        <div class="error-container" v-if="errorMsg">
            <p class="error-text">{{ $t(errorMsg) }}</p>
            <NuxtLink :to="`/order/pay?id=${id}`" class="back-btn">{{ $t("返回上一步") }}</NuxtLink>
        </div>
        <div ref="formContainer"></div>
    </div>
</template>
<script lang="ts" setup>
/**
 * Stripe 支付页面（跨境版专用；O2O 融合版 pay.vue 已内联 handleRedirectPay，此页作为深链/历史书签兜底保留）
 * - 创建 Stripe 支付会话并重定向到 Stripe Checkout
 * - 支持 HTML 表单自动提交与 URL 重定向两种方式
 */
import { creatPay } from "~/api/order/pay";

const route = useRoute();
const id = ref(route.query.id);
const formContainer = ref<HTMLDivElement | null>(null);
const loading = ref(true);
const errorMsg = ref("");
const { t } = useI18n();

const _creatPay = async () => {
    try {
        const result = await creatPay({
            id: id.value,
            type: "stripe"
        });

        if (result.payInfo?.url) {
            window.location.href = result.payInfo.url;
            return;
        }

        if (result.payInfo?.html) {
            if (formContainer.value !== null) {
                formContainer.value.innerHTML = result.payInfo.html;
                const forms = document.forms;
                forms && forms[0]?.submit();
            }
            return;
        }

        loading.value = false;
        errorMsg.value = t("支付信息获取失败，请重试");
    } catch (error: any) {
        loading.value = false;
        errorMsg.value = error.message || t("支付创建失败");
        message.error(t("该订单金额未满足该支付方式的最低金额要求,请选择其他支付"));
    }
};

_creatPay();
</script>
<style lang="less" scoped>
.loading-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 300px;

    .loading-spinner {
        width: 40px;
        height: 40px;
        border: 3px solid #f3f3f3;
        border-top: 3px solid var(--general);
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }

    p {
        margin-top: 20px;
        color: #666;
        font-size: 14px;
    }
}

.error-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 300px;

    .error-text {
        color: #ff4d4f;
        font-size: 16px;
        margin-bottom: 20px;
    }

    .back-btn {
        padding: 10px 20px;
        background-color: var(--general);
        color: #fff;
        border-radius: 4px;
        text-decoration: none;

        &:hover {
            background-color: var(--general);
            color: #666 !important;
        }
    }
}

@keyframes spin {
    0% {
        transform: rotate(0deg);
    }
    100% {
        transform: rotate(360deg);
    }
}
</style>
