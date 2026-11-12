<template>
    <div>
        <CommonHeader title="PayPal支付"></CommonHeader>
        <div ref="formContainer"></div>
    </div>
</template>
<script lang="ts" setup>
/**
 * @description PayPal充值支付页面
 * - 调用PayPal支付接口，自动跳转到PayPal支付页面
 */
import { creatPay, orderPayInfo, checkPayStatus } from "~/api/order/pay";
import { updateRechargeOrderCreate } from "~/api/user/account";
const route = useRoute();
const id = ref(route.query.id);
const formContainer = ref(null);
const htmlContent = ref();
const _creatPay = async () => {
    try {
        const result = await updateRechargeOrderCreate({
            id: id.value,
            type: "paypal"
        });
        window.location.href = result.payInfo.url;
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
_creatPay();
</script>
<style lang="less" scoped></style>
