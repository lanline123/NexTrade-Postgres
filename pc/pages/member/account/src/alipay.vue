<template>
    <div>
        <CommonHeader title="支付宝支付"></CommonHeader>
        <div ref="formContainer"></div>
    </div>
</template>
<script lang="ts" setup>
/**
 * @description 支付宝充值支付页面
 * - 调用支付宝支付接口，自动提交表单跳转到支付宝支付页面
 */
import { updateRechargeOrderCreate } from "~/api/user/account";
const route = useRoute();
const id = ref(route.query.id);
const formContainer = ref<HTMLDivElement | null>(null);
const htmlContent = ref();
const _creatPay = async () => {
    try {
        const result = await updateRechargeOrderCreate({
            id: id.value,
            type: "alipay"
        });
        htmlContent.value = result.payInfo.html;
        if (formContainer.value !== null) {
            formContainer.value.innerHTML = htmlContent.value;
            const forms = document.forms;
            forms && forms[0].submit();
        }
    } catch (error: any) {
    } finally {
    }
};
_creatPay();
</script>
<style lang="less" scoped></style>
