<template>
    <div @click="show">
        <slot></slot>
    </div>
    <el-dialog v-model="dialogVisible" :title="$t('商户入驻协议')" width="60%">
        <div class="xy-html" v-html="xyHtml"></div>
    </el-dialog>
</template>
<script lang="ts" setup>
import { ref } from "vue";
import { getApplyShopAgreement } from "~/api/merchant/merchant";
import { message } from "ant-design-vue";

const xyHtml = ref("");
const dialogVisible = ref(false);
const show = async () => {
    try {
        if (!xyHtml.value) {
            const result = await getApplyShopAgreement();
            xyHtml.value = result;
        }
        dialogVisible.value = true;
    } catch (error: any) {
        message.error(error.message);
    }
};
</script>
<style lang="less" scoped>
.xy-html {
    padding: 10px 40px;
    height: 400px;
    overflow: auto;
}
</style>
