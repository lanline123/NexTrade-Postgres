<template>
    <el-dialog v-model="visible" :title="isEditMode ? $t('修改申请') : $t('认款详情')" :width="isEditMode ? '500px' : '620px'" destroy-on-close>
        <el-skeleton v-if="loading" :rows="5" animated />
        <el-empty v-else-if="!claim" :description="$t('暂无认款信息')" />
        <el-form v-else-if="isEditMode" ref="formRef" :model="form" label-width="100px" class="claim-edit-form">
            <el-form-item :label="$t('付款金额')"><FormatPrice v-model="claim.claimedAmount" :showText="false" /></el-form-item>
            <el-form-item :label="$t('转账时间')"><el-date-picker v-model="form.transferTime" type="datetime" format="YYYY-MM-DD HH:mm" value-format="YYYY-MM-DD HH:mm" :disabled-date="isFutureDate" :placeholder="$t('请选择')" style="width: 100%" /></el-form-item>
            <el-form-item :label="$t('付款人姓名')"><el-input v-model="form.payerName" :placeholder="$t('请输入')" /></el-form-item>
            <el-form-item :label="$t('转账流水号')"><el-input v-model="form.bankSerialNo" :placeholder="$t('请填写')" /></el-form-item>
            <el-form-item :label="$t('付款凭证')" required>
                <div class="claim-upload-wrap">
                    <input ref="nativeFileInput" class="native-file-input" type="file" accept="image/png,image/jpeg,image/gif,image/webp,image/tiff" multiple @change="handleNativeFileChange" />
                    <el-upload ref="uploadRef" v-model:file-list="files" :class="['claim-uploader', { 'is-limit-reached': files.length >= 5 }]" list-type="picture-card" :action="requestPrefix + '/user/user/uploadImg'" :headers="uploadHeaders" :limit="5" accept="image/png,image/jpeg,image/gif,image/webp,image/tiff" :before-upload="beforeUpload" :on-success="handleUploadSuccess" :on-remove="handleUploadRemove" :on-preview="handleUploadPreview">
                        <template #trigger>
                            <span class="claim-upload-trigger" @click.stop="openFilePicker"><el-icon><Plus /></el-icon></span>
                        </template>
                    </el-upload>
                    <el-image ref="previewImageRef" :src="previewUrl" :preview-src-list="previewUrls" :initial-index="previewIndex" preview-teleported style="display: none" />
                    <div class="upload-tip">{{ $t('最多上传5张') }}</div>
                </div>
            </el-form-item>
        </el-form>
        <div v-else class="claim-detail-page">
            <div class="claim-summary">
                <div class="summary-row">
                    <div><span class="summary-label">{{ $t("订单号") }}：</span><strong>{{ claim.orderSn || claim.orderId || "-" }}</strong></div>
                    <div><span class="summary-label">{{ $t("认款状态") }}：</span><strong class="status-value" :class="statusClass">{{ claim.statusName || statusName }}</strong></div>
                </div>
                <div v-if="Number(claim.status) === 0" class="status-tip">{{ $t("您已提交汇款凭证，等待财务人员核对流水，我们尽快完成审核。") }}</div>
                <div v-else-if="Number(claim.status) === 1 && claim.auditRemark" class="status-tip">{{ $t("审核备注") }}：{{ claim.auditRemark }}</div>
                <div v-else-if="Number(claim.status) === 2 && claim.rejectReason" class="status-tip reject-reason">{{ $t("认款审核未通过") }}：{{ claim.rejectReason }}</div>
            </div>
            <section class="detail-section">
                <h3>{{ $t("付款信息") }}</h3>
                <div class="detail-grid">
                    <div class="claim-row"><span>{{ $t("付款金额") }}：</span><FormatPrice v-if="claim.claimedAmount !== undefined && claim.claimedAmount !== null" v-model="claim.claimedAmount" :showText="false" :fontStyle="{ color: 'var(--price)', fontSize: '16px' }" /><b v-else>-</b></div>
                    <div class="claim-row"><span>{{ $t("转账时间") }}：</span><b>{{ formatTransferTime(claim.transferTime) || "-" }}</b></div>
                    <div class="claim-row"><span>{{ $t("付款人") }}：</span><b>{{ claim.payerName || "-" }}</b></div>
                    <div class="claim-row"><span>{{ $t("转账流水号") }}：</span><b>{{ claim.bankSerialNo || "-" }}</b></div>
                </div>
                <div class="claim-row voucher-row"><span>{{ $t("付款凭证") }}：</span><div v-if="voucherImages.length" class="voucher-list"><el-image v-for="(url, index) in voucherImages" :key="url" :src="url" :preview-src-list="voucherImages" :initial-index="index" :preview-teleported="true" fit="cover" class="voucher-image" /></div><span v-else class="empty-text">{{ $t("暂无付款凭证") }}</span></div>
            </section>
        </div>
        <template v-if="isEditMode" #footer>
            <el-button @click="visible = false">{{ $t('取消') }}</el-button>
            <el-button type="primary" :loading="submitting" @click="submitEdit">{{ $t('确认') }}</el-button>
        </template>
    </el-dialog>
</template>

<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from "vue";
import { Plus } from "@element-plus/icons-vue";
import { message } from "ant-design-vue";
import { getOfflinePaymentClaimDetail, submitOfflinePaymentClaim } from "~/api/user/order";
import type { OfflinePaymentClaim } from "~/types/user/order.d";
import { imageFormat } from "~/utils/format";
import { formatTransferTime, parseTransferTime } from "~/utils/transferTime";
import { requestHeader, requestPrefix } from "~/utils/request";
import type { UploadProps, UploadUserFile } from "element-plus";
const { t } = useI18n();

const props = withDefaults(defineProps<{ modelValue: boolean; orderId: number; editable?: boolean }>(), { editable: false });
const emit = defineEmits<{ (event: "update:modelValue", value: boolean): void; (event: "submitted"): void }>();
const visible = computed({ get: () => props.modelValue, set: (value) => emit("update:modelValue", value) });
const loading = ref(false);
const claim = ref<OfflinePaymentClaim | null>(null);
/** 转账时间不能晚于今天 */
const isFutureDate = (date: Date) => date.getTime() > Date.now();
const form = reactive({ transferTime: "", payerName: "", bankSerialNo: "" });
const files = ref<UploadUserFile[]>([]);
const uploadRef = ref<any>();
const nativeFileInput = ref<HTMLInputElement>();
const previewImageRef = ref<any>();
const previewUrl = ref("");
const previewIndex = ref(0);
const submitting = ref(false);
const uploadHeaders = requestHeader();
const isEditMode = computed(() => props.editable && Number(claim.value?.status) === 2);
const statusName = computed(() => ({ 0: "待审核", 1: "审核通过", 2: "审核驳回" } as Record<number, string>)[Number(claim.value?.status)] || "-");
const statusClass = computed(() => `status-${Number(claim.value?.status)}`);
const voucherImages = computed(() => (claim.value?.voucherUrls || []).map((url) => imageFormat(url)));
const previewUrls = computed(() => files.value.map((file) => String(file.url || file.response?.data?.picUrl || "")).filter(Boolean));


watch(() => [props.modelValue, props.orderId], async ([isVisible, orderId]) => {
    if (!isVisible || !orderId) return;
    loading.value = true;
    claim.value = null;
    files.value = [];
    try {
        claim.value = await getOfflinePaymentClaimDetail({ orderId: Number(orderId) });
        if (claim.value) {
            form.transferTime = formatTransferTime(claim.value.transferTime);
            form.payerName = claim.value.payerName || "";
            form.bankSerialNo = claim.value.bankSerialNo || "";
            files.value = (claim.value.voucherUrls || []).map((url, index) => ({ uid: `claim-${index}`, name: `voucher-${index + 1}`, url: imageFormat(url), status: "success", picUrl: url } as UploadUserFile & { picUrl: string }));
        }
    } finally {
        loading.value = false;
    }
}, { immediate: true });

const handleUploadSuccess: UploadProps["onSuccess"] = (response: any, uploadFile) => {
    const picUrl = response?.data?.picUrl;
    if (picUrl) {
        uploadFile.url = imageFormat(picUrl);
        (uploadFile as UploadUserFile & { picUrl?: string }).picUrl = picUrl;
        return;
    }
    uploadFile.status = "fail";
    message.error(response?.message || t("上传失败"));
};
const handleUploadPreview: UploadProps["onPreview"] = (uploadFile) => {
    const urls = previewUrls.value;
    if (!urls.length) return;
    const index = Math.max(0, urls.indexOf(String(uploadFile.url || "")));
    previewIndex.value = index;
    previewUrl.value = urls[index];
    nextTick(() => previewImageRef.value?.showPreview?.());
};

const beforeUpload: UploadProps["beforeUpload"] = (rawFile) => {
    if (!/\/(png|jpe?g|gif|webp|tiff)$/i.test(rawFile.type)) {
        message.error(t("不支持上传此文件格式"));
        return false;
    }
    return true;
};
const openFilePicker = () => {
    if (files.value.length >= 5) return;
    nativeFileInput.value && (nativeFileInput.value.value = "");
    nativeFileInput.value?.click();
};
const handleNativeFileChange = (event: Event) => {
    const input = event.target as HTMLInputElement;
    const selectedFiles = Array.from(input.files || []).slice(0, Math.max(0, 5 - files.value.length));
    selectedFiles.forEach((file) => uploadRef.value?.handleStart(file));
    if (selectedFiles.length) uploadRef.value?.submit();
};
const handleUploadRemove: UploadProps["onRemove"] = () => undefined;
const submitEdit = async () => {
    if (!claim.value || submitting.value) return;
    // 仅接受已上传成功且拿到服务端地址的凭证，避免提交 blob: 临时地址
    const getServerPicUrl = (file: UploadUserFile) => String((file as UploadUserFile & { picUrl?: string }).picUrl || file.response?.data?.picUrl || "");
    if (files.value.some((file) => file.status === "ready" || file.status === "uploading")) return message.error(t("凭证上传中，请稍后再提交"));
    if (files.value.some((file) => file.status !== "success" || !getServerPicUrl(file))) return message.error(t("存在上传失败的凭证，请删除后重新上传"));
    const voucherUrls = files.value.map(getServerPicUrl).filter(Boolean);
    if (!voucherUrls.length) return message.error(t("付款凭证必传"));
    submitting.value = true;
    try {
        await submitOfflinePaymentClaim({
            orderId: Number(props.orderId),
            transferTime: parseTransferTime(form.transferTime),
            payerName: form.payerName.trim(),
            bankSerialNo: form.bankSerialNo.trim(),
            voucherUrls
        });
        message.success(t("提交成功，等待财务审核"));
        emit("submitted");
        visible.value = false;
    } catch (error: any) {
        message.error(error?.message || t("提交失败，请稍后重试"));
    } finally {
        submitting.value = false;
    }
};
</script>

<style scoped lang="less">
.claim-detail-page { color: #333; font-size: 14px; }
.claim-summary { padding: 8px 0 20px; border-bottom: 1px solid #f0f0f0; }
.summary-row { display: flex; gap: 70px; align-items: center; line-height: 32px; }
.summary-row > div { min-width: 0; }
.summary-label { color: #666; }
.status-value { font-weight: 600; color: var(--general); }.status-1 { color: #67c23a; }.status-2 { color: #f56c6c; }
.status-tip { margin-top: 12px; color: #666; line-height: 24px; }.reject-reason { color: #666; }
.detail-section { padding-top: 20px; }.detail-section h3 { margin: 0 0 14px; color: #222; font-size: 17px; font-weight: 600; }
.detail-grid { display: grid; grid-template-columns: 1fr 1fr; column-gap: 56px; }
.claim-row { display: flex; min-height: 48px; align-items: center; line-height: 1.5; }.claim-row > span:first-child { flex: 0 0 100px; color: #666; }.claim-row b { flex: 1; color: #333; font-weight: 400; word-break: break-all; }
.voucher-row { align-items: flex-start; padding-top: 10px; }.voucher-list { display: flex; flex-wrap: wrap; gap: 10px; }.voucher-image { width: 88px; height: 88px; border: 1px solid #e8e8e8; border-radius: 4px; cursor: pointer; }.empty-text { color: #999; }
.claim-upload-wrap { width: 100%; }
.native-file-input { display: none; }
.upload-tip { margin-top: 6px; color: #999; font-size: 12px; line-height: 18px; }
.claim-upload-trigger { display: flex; width: 100%; height: 100%; align-items: center; justify-content: center; cursor: pointer; }
.claim-uploader.is-limit-reached :deep(.el-upload--picture-card) { display: none; }
.claim-uploader :deep(.el-upload-list__item .el-icon--close-tip) { display: none !important; }
.claim-uploader :deep(.el-upload-list__item-status-label) { display: none !important; }
.claim-uploader {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    :deep(.el-upload--picture-card),
    :deep(.el-upload-list--picture-card .el-upload-list__item) {
        width: 80px;
        height: 80px;
        margin: 0 8px 8px 0;
    }
    :deep(.el-upload--picture-card) { display: flex; align-items: center; justify-content: center; }
}
</style>
