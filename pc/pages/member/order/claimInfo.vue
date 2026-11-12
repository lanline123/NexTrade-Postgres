<template>
    <PageHeaderNew :isWhite="true" />
    <CommonHeader title="认款详情" />
    <TopJumpBar tltle="认款详情" />
    <CommonPageContainer flex>
        <div class="menu"><MemberNav /></div>
        <div class="info-row">
            <div class="title-or-tabs right-info">
                <span>{{ $t("认款详情") }}</span>
                <div class="header-actions">
                    <NuxtLink v-if="orderId" :to="`/member/order/info?id=${orderId}`">
                        <el-button>{{ $t("订单详情") }}</el-button>
                    </NuxtLink>
                    <el-button v-if="claim && Number(claim.status) === 2 && !isEditMode" type="primary" @click="goEdit">{{ $t("修改申请") }}</el-button>
                </div>
            </div>
            <div class="claim-page" v-loading="loading">
                <el-empty v-if="!loading && !claim" :description="$t('暂无认款信息')" />
                <template v-else-if="claim">
                    <div class="claim-summary">
                        <div class="summary-row">
                            <span><span class="summary-label">{{ $t("订单号") }}：</span><strong class="summary-value">{{ claim.orderSn || claim.orderId || "-" }}</strong></span>
                            <span><span class="summary-label">{{ $t("审核状态") }}：</span><strong class="summary-value">{{ claim.statusName || statusName }}</strong></span>
                        </div>
                        <div v-if="Number(claim.status) === 0" class="status-tip">{{ $t("您已提交汇款凭证，等待财务人员核对流水，我们尽快完成审核。") }}</div>
                        <div v-else-if="Number(claim.status) === 1 && claim.auditRemark" class="status-tip">{{ $t("审核备注") }}：{{ claim.auditRemark }}</div>
                        <div v-else-if="Number(claim.status) === 2 && claim.rejectReason" class="status-tip reject-reason">{{ $t("认款审核未通过") }}：{{ claim.rejectReason }}</div>
                    </div>

                    <section class="detail-section">
                        <div class="sub-title">{{ isEditMode ? $t("修改申请") : $t("付款信息") }}</div>
                        <div v-if="isEditMode" class="edit-form">
                            <el-form ref="formRef" :model="form" label-width="110px">
                                <el-form-item :label="$t('付款金额')"><FormatPrice v-model="claim.claimedAmount" :showText="false" /></el-form-item>
                                <el-form-item :label="$t('转账时间')"><el-date-picker v-model="form.transferTime" type="datetime" format="YYYY-MM-DD HH:mm" value-format="YYYY-MM-DD HH:mm" :disabled-date="isFutureDate" :placeholder="$t('请选择')" style="width: 100%" /></el-form-item>
                                <el-form-item :label="$t('付款人姓名')"><el-input v-model="form.payerName" :placeholder="$t('请输入')" /></el-form-item>
                                <el-form-item :label="$t('转账流水号')"><el-input v-model="form.bankSerialNo" :placeholder="$t('请填写')" /></el-form-item>
                                <el-form-item :label="$t('付款凭证')" required>
                                    <div class="claim-upload-wrap">
                                        <input ref="nativeFileInput" class="native-file-input" type="file" accept="image/png,image/jpeg,image/gif,image/webp,image/tiff" multiple @change="handleNativeFileChange" />
                                        <el-upload ref="uploadRef" v-model:file-list="files" :class="['claim-uploader', { 'is-limit-reached': files.length >= 5 } ]" list-type="picture-card" :action="requestPrefix + '/user/user/uploadImg'" :headers="uploadHeaders" :limit="5" accept="image/png,image/jpeg,image/gif,image/webp,image/tiff" :before-upload="beforeUpload" :on-success="handleUploadSuccess" :on-remove="handleUploadRemove" :on-preview="handleUploadPreview">
                                            <template #trigger><span class="claim-upload-trigger" @click.stop="openFilePicker"><el-icon><Plus /></el-icon></span></template>
                                        </el-upload>
                                        <el-image ref="previewImageRef" :src="previewUrl" :preview-src-list="previewUrls" :initial-index="previewIndex" preview-teleported style="display: none" />
                                        <div class="upload-tip">{{ $t("最多上传5张") }}</div>
                                    </div>
                                </el-form-item>
                            </el-form>
                            <div class="page-actions">
                                <el-button @click="goBack">{{ $t("取消") }}</el-button>
                                <el-button type="primary" :loading="submitting" @click="submitEdit">{{ $t("确认") }}</el-button>
                            </div>
                        </div>
                        <template v-else>
                            <div class="detail-grid">
                                <div class="claim-row"><span>{{ $t("付款金额") }}：</span><FormatPrice v-if="claim.claimedAmount !== undefined && claim.claimedAmount !== null" v-model="claim.claimedAmount" :showText="false" :fontStyle="{ color: 'var(--price)', fontSize: '16px' }" /><b v-else>-</b></div>
                                <div class="claim-row"><span>{{ $t("转账时间") }}：</span><b>{{ formatTransferTime(claim.transferTime) || "-" }}</b></div>
                                <div class="claim-row"><span>{{ $t("付款人") }}：</span><b>{{ claim.payerName || "-" }}</b></div>
                                <div class="claim-row"><span>{{ $t("转账流水号") }}：</span><b>{{ claim.bankSerialNo || "-" }}</b></div>
                            </div>
                            <div class="claim-row voucher-row"><span>{{ $t("付款凭证") }}：</span><div v-if="voucherImages.length" class="voucher-list"><el-image v-for="(url, index) in voucherImages" :key="url" :src="url" :preview-src-list="voucherImages" :initial-index="index" :preview-teleported="true" fit="cover" class="voucher-image" /></div><span v-else class="empty-text">{{ $t("暂无付款凭证") }}</span></div>
                        </template>
                    </section>
                </template>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter />
</template>

<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from "vue";
import { Plus } from "@element-plus/icons-vue";
import { message } from "ant-design-vue";
import type { UploadProps, UploadUserFile } from "element-plus";
import { getOfflinePaymentClaimDetail, submitOfflinePaymentClaim } from "~/api/user/order";
import type { OfflinePaymentClaim } from "~/types/user/order.d";
import { imageFormat } from "~/utils/format";
import { formatTransferTime, parseTransferTime } from "~/utils/transferTime";
import { requestHeader, requestPrefix } from "~/utils/request";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";

definePageMeta({ middleware: "auth" });
const route = useRoute();
const router = useRouter();
const { t } = useI18n();
const orderId = computed(() => Number(route.query.orderId || route.query.id || 0));
const editRequested = computed(() => String(route.query.edit || "") === "1");
const loading = ref(false);
const claim = ref<OfflinePaymentClaim | null>(null);
const formRef = ref();
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
const isEditMode = computed(() => editRequested.value && Number(claim.value?.status) === 2);
const statusName = computed(() => ({ 0: "待审核", 1: "审核通过", 2: "审核驳回" } as Record<number, string>)[Number(claim.value?.status)] || "-");
const voucherImages = computed(() => (claim.value?.voucherUrls || []).map((url) => imageFormat(url)));
const previewUrls = computed(() => files.value.map((file) => String(file.url || file.response?.data?.picUrl || "")).filter(Boolean));

const loadClaim = async () => {
    if (!orderId.value) return;
    loading.value = true;
    try {
        claim.value = await getOfflinePaymentClaimDetail({ orderId: orderId.value });
        if (claim.value) {
            form.transferTime = formatTransferTime(claim.value.transferTime);
            form.payerName = claim.value.payerName || "";
            form.bankSerialNo = claim.value.bankSerialNo || "";
            files.value = (claim.value.voucherUrls || []).map((url, index) => ({ uid: `claim-${index}`, name: `voucher-${index + 1}`, url: imageFormat(url), status: "success", picUrl: url } as UploadUserFile & { picUrl: string }));
        }
    } catch (error: any) {
        message.error(error?.message || t("获取认款信息失败"));
    } finally {
        loading.value = false;
    }
};
watch(orderId, loadClaim, { immediate: true });
const goEdit = () => navigateTo(`/member/order/claimInfo?orderId=${orderId.value}&edit=1`);
const goBack = () => router.back();
const handleUploadSuccess: UploadProps["onSuccess"] = (response: any, uploadFile) => {
    const picUrl = response?.data?.picUrl;
    if (picUrl) { uploadFile.url = imageFormat(picUrl); (uploadFile as UploadUserFile & { picUrl?: string }).picUrl = picUrl; return; }
    uploadFile.status = "fail";
    message.error(response?.message || t("上传失败"));
};
const handleUploadPreview: UploadProps["onPreview"] = (uploadFile) => { const urls = previewUrls.value; if (!urls.length) return; previewIndex.value = Math.max(0, urls.indexOf(String(uploadFile.url || ""))); previewUrl.value = urls[previewIndex.value]; nextTick(() => previewImageRef.value?.showPreview?.()); };
const beforeUpload: UploadProps["beforeUpload"] = (rawFile) => { if (!/\/(png|jpe?g|gif|webp|tiff)$/i.test(rawFile.type)) { message.error(t("不支持上传此文件格式")); return false; } return true; };
const openFilePicker = () => { if (files.value.length >= 5) return; if (nativeFileInput.value) nativeFileInput.value.value = ""; nativeFileInput.value?.click(); };
const handleNativeFileChange = (event: Event) => { const input = event.target as HTMLInputElement; const selectedFiles = Array.from(input.files || []).slice(0, Math.max(0, 5 - files.value.length)); selectedFiles.forEach((file) => uploadRef.value?.handleStart(file)); if (selectedFiles.length) uploadRef.value?.submit(); };
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
        await submitOfflinePaymentClaim({ orderId: orderId.value, transferTime: parseTransferTime(form.transferTime), payerName: form.payerName.trim(), bankSerialNo: form.bankSerialNo.trim(), voucherUrls });
        message.success(t("提交成功，等待财务审核"));
        await navigateTo(`/member/order/claimInfo?orderId=${orderId.value}`);
    } catch (error: any) { message.error(error?.message || t("提交失败，请稍后重试")); } finally { submitting.value = false; }
};
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";
.right-info { justify-content: space-between; align-items: center; }
.header-actions { display: flex; gap: 10px; }
.claim-page { min-height: 420px; padding: 0; background: #fff; color: #333; font-size: 14px; }
.claim-summary { padding-bottom: 0; }.summary-row { line-height: 3; }.summary-row > span + span { margin-left: 40px; }.summary-label { color: #666; }.summary-value { font-weight: 700; color: #666; }.status-tip { margin-top: 0; color: #666; line-height: 3; }.reject-reason { color: #666; }
.detail-section { padding-top: 10px; }.sub-title { color: #333; font-size: 16px; font-weight: 400; letter-spacing: 0; display: inline-block; }.detail-grid { display: block; margin-top: 20px; }.claim-row { display: flex; min-height: 0; align-items: baseline; line-height: 3; }.claim-row > span:first-child { flex: 0 0 90px; color: #666; }.claim-row > :not(:first-child) { margin-left: 10px; }.claim-row b { flex: 0 1 auto; color: #333; font-weight: 400; word-break: break-all; }.voucher-row { align-items: flex-start; padding-top: 20px; }.voucher-list { display: flex; flex-wrap: wrap; gap: 10px; }.voucher-image { width: 96px; height: 96px; border: 1px solid #e8e8e8; border-radius: 4px; cursor: pointer; }.empty-text { color: #999; }
.edit-form { max-width: 680px; margin-top: 20px; }.claim-upload-wrap { width: 100%; }.native-file-input { display: none; }.upload-tip { margin-top: 6px; color: #999; font-size: 12px; }.claim-upload-trigger { display: flex; width: 100%; height: 100%; align-items: center; justify-content: center; cursor: pointer; }.claim-uploader.is-limit-reached :deep(.el-upload--picture-card) { display: none; }.claim-uploader { display: flex; flex-wrap: wrap; align-items: flex-start; }.claim-uploader :deep(.el-upload--picture-card), .claim-uploader :deep(.el-upload-list--picture-card .el-upload-list__item) { width: 80px; height: 80px; margin: 0 8px 8px 0; }.page-actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 20px; }
.claim-uploader :deep(.el-upload-list__item-status-label) { display: none !important; }
</style>
