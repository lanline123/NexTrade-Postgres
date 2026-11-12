<template>
    <div>
        <div class="cart-top flex justify-center">
            <div class="cart-cont flex justify-center">
                <div class="steps">
                    <Steps :current="props.step - 1" :sudden="sudden"></Steps>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
/**
 * @description 支付流程头部组件
 * - 显示商城Logo和支付流程步骤条
 * - 支持显示当前步骤状态
 */
import { useCommonStore } from "@/store/common";
const commonStore = useCommonStore();
const props = defineProps({
    step: { type: Number, default: 0 },
    sudden: { type: Number, default: 0 },
});
</script>
<style lang="less" scoped>
.logo {
    min-height: 100px;
    display: flex;
    align-items: center;
    img {
        width: 190px;
        padding: 10px 0;
    }
}
.cart-top {
    position: relative;
    z-index: 79;
    margin-bottom: 32px;

    .cart-cont {
        width: 1190px;
        padding: 0 0 0 0;
    }
}
.steps {
    padding-top: 20px;
}
</style>
