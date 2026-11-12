<template>
    <el-dialog v-model="visible" :title="$t('请选择取消订单原因')" width="480px" :close-on-click-modal="false">
        <el-radio-group v-model="selectedReason" class="reason-list">
            <el-radio v-for="reason in reasons" :key="reason" :value="reason">
                {{ reason }}
            </el-radio>
        </el-radio-group>
        <template #footer>
            <el-button @click="visible = false">{{ $t('取消') }}</el-button>
            <el-button type="primary" :loading="submitting" :disabled="!selectedReason" @click="submitWithReason">
                {{ $t('确定') }}
            </el-button>
        </template>
    </el-dialog>
</template>

<script lang="ts" setup>
import { ref } from "vue";
import { message, Modal } from "ant-design-vue";
import { cancelOrder, getOrderReasonConfig } from "~/api/user/order";
import { buildCancelOrderPayload, keepValidSelection } from "~/utils/orderReason.mjs";

const emit = defineEmits<{ success: [] }>();
const { t } = useI18n();
const visible = ref(false);
const submitting = ref(false);
const orderId = ref(0);
const selectedReason = ref("");
const reasons = ref<string[]>([]);

const loadConfig = async () => {
    const result = await getOrderReasonConfig();
    reasons.value = Array.isArray(result.cancelOrderReasonList) ? result.cancelOrderReasonList : [];
    selectedReason.value = keepValidSelection(selectedReason.value, reasons.value);
    return Number(result.afterSalesReasonEnabled) === 1;
};

const cancelWithoutReason = () => {
    Modal.confirm({
        title: t("确认要取消该订单吗"),
        cancelText: t("取消"),
        okText: t("确定"),
        maskClosable: true,
        centered: true,
        onOk: async () => {
            try {
                await cancelOrder(buildCancelOrderPayload(orderId.value));
                message.success(t("取消订单成功"));
                emit("success");
            } catch (error: any) {
                message.error(error.message);
            }
        }
    });
};

const open = async (id: number) => {
    orderId.value = id;
    selectedReason.value = "";
    try {
        const enabled = await loadConfig();
        if (enabled) {
            visible.value = true;
        } else {
            cancelWithoutReason();
        }
    } catch (error: any) {
        message.error(error.message);
    }
};

const submitWithReason = async () => {
    if (!selectedReason.value || submitting.value) return;
    submitting.value = true;
    try {
        await cancelOrder(buildCancelOrderPayload(orderId.value, selectedReason.value));
        message.success(t("取消订单成功"));
        visible.value = false;
        emit("success");
    } catch (error: any) {
        message.error(error.message);
        try {
            const enabled = await loadConfig();
            if (!enabled) visible.value = false;
        } catch {
            // 保留当前弹窗，让买家可再次操作或关闭。
        }
    } finally {
        submitting.value = false;
    }
};

defineExpose({ open });
</script>

<style lang="less" scoped>
.reason-list {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 14px;
    width: 100%;
}
</style>
