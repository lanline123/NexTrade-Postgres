<template>
    <el-dialog
        v-model="visible"
        :title="$t('结算账号')"
        width="1000px"
        :close-on-click-modal="false"
        @close="handleClose"
    >
        <div class="settlement-dialog-content" v-loading="loading">
            <el-tabs type="border-card" v-model="activeName" v-if="hasPaymentList">
                <el-tab-pane v-if="'thirdPartyPaymentTransfer' in offlinePaymentList" :label="$t('第三方支付转账')" :name="3">
                    <div v-html="offlinePaymentList.thirdPartyPaymentTransfer"></div>
                </el-tab-pane>
                <el-tab-pane v-if="'offlinePayBank' in offlinePaymentList" :label="$t('银行汇款')" :name="1">
                    <div v-html="offlinePaymentList.offlinePayBank"></div>
                </el-tab-pane>
                <el-tab-pane v-if="'offlinePayCompany' in offlinePaymentList" :label="$t('企业汇款')" :name="2">
                    <div v-html="offlinePaymentList.offlinePayCompany"></div>
                </el-tab-pane>
                <el-tab-pane v-if="'otherPaymentRemittance' in offlinePaymentList" :label="$t('其他汇款方式')" :name="4">
                    <div v-html="offlinePaymentList.otherPaymentRemittance"></div>
                </el-tab-pane>
            </el-tabs>
            <div v-else-if="!loading" class="no-payment-list">
                {{ $t("暂无结算账号信息") }}
            </div>
        </div>
    </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { orderPayInfoTerm } from "~/api/order/pay";

const props = defineProps<{
    modelValue: boolean;
}>();

const emit = defineEmits<{
    (e: "update:modelValue", value: boolean): void;
}>();

const visible = computed({
    get: () => props.modelValue,
    set: (val) => emit("update:modelValue", val)
});

const loading = ref(false);
const activeName = ref<number>(1);
const offlinePaymentList = ref<any>({});

const hasPaymentList = computed(() => {
    return (
        "thirdPartyPaymentTransfer" in offlinePaymentList.value ||
        "offlinePayBank" in offlinePaymentList.value ||
        "offlinePayCompany" in offlinePaymentList.value ||
        "otherPaymentRemittance" in offlinePaymentList.value
    );
});

const loadOfflinePaymentList = async () => {
    if (hasPaymentList.value) return;

    loading.value = true;
    try {
        const result = await orderPayInfoTerm();
        offlinePaymentList.value = result.offlinePaymentList || {};

        // 默认选中第一个可用的支付方式
        if ("thirdPartyPaymentTransfer" in offlinePaymentList.value) {
            activeName.value = 3;
        } else if ("offlinePayBank" in offlinePaymentList.value) {
            activeName.value = 1;
        } else if ("offlinePayCompany" in offlinePaymentList.value) {
            activeName.value = 2;
        } else if ("otherPaymentRemittance" in offlinePaymentList.value) {
            activeName.value = 4;
        }
    } catch (error: any) {
        console.error("加载结算账号失败:", error);
        offlinePaymentList.value = {};
    } finally {
        loading.value = false;
    }
};

const handleClose = () => {
    emit("update:modelValue", false);
};

watch(
    () => props.modelValue,
    (val) => {
        if (val) {
            loadOfflinePaymentList();
        }
    }
);
</script>

<style lang="less" scoped>
.settlement-dialog-content {
    min-height: 200px;

    :deep(.el-tabs--border-card > .el-tabs__header .el-tabs__item.is-active) {
        color: #333;
    }
    :deep(.el-tabs--border-card > .el-tabs__header .el-tabs__item:not(.is-disabled):hover) {
        color: #333;
    }

    .no-payment-list {
        text-align: center;
        color: #999;
        padding: 40px 0;
    }
}
</style>
