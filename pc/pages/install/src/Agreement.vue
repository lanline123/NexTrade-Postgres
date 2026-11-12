<template>
    <div class="agreement-container">
        <h2 class="tit">软件许可协议</h2>
        <div class="content" v-html="formattedText"></div>
    </div>
</template>
<script lang="ts" setup>
import { ref, onMounted } from "vue";
import { getAgreement } from "@/api/install/install";
import { message } from "ant-design-vue"; // 获取当前的 URL
const emit = defineEmits(["onDisabled"]);
const formattedText = ref("");
const loading = ref(false);
const loadFilter = async () => {
    loading.value = true;
    try {
        const result = await getAgreement();
        formattedText.value = "&nbsp;&nbsp;&nbsp;&nbsp;" + result.text.replace(/\n/g, "</br>");
    } catch (error: any) {
        if (error.code === -1) {
            navigateTo("/404");
        }
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
onMounted(() => {
    emit("onDisabled", false);
    loadFilter();
});
</script>
<style lang="less" scoped>
.agreement-container {
    width: 1190px;
    .tit {
        padding: 30px 0;
        color: #333;
        text-align: center;
        font-size: 18px;
    }
    .content {
        font-size: 14px;
        padding: 0 15%;
        line-height: 28px;
    }
}
</style>
