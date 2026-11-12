<template>
    <CommonPageHeader></CommonPageHeader>
    <CommonHeader title="支付订单"></CommonHeader>
    <CommonPageContainer v-if="!loading">
        <div class="success_txt">{{ $t("您的订单已成功提交，请付款") }}！</div>
        <div class="price_box">
            <div class="num_box">
                <div class="tit">{{ $t("需支付") }}：</div>
                <FormatPrice class="price" v-model="order.unpaidAmount" :showText="false"></FormatPrice>
            </div>
            <div v-if="!isPremiumOrder || premiumOrderCreated" class="order_num">
                <div class="tit">{{ $t("订单号") }}：</div>
                <div class="num" :class="{ 'hand-pointer': !isPremiumOrder }">
                    <NuxtLink v-if="!isPremiumOrder" :to="'/member/order/info?id=' + order.orderId" target="_blank">{{ order.orderSn }}</NuxtLink>
                    <span v-else>{{ order.orderSn }}</span>
                </div>
            </div>
        </div>
        <div
            class="pay_type_box payment-methods-box"
            v-if="
                Number(order.payTypeId) < 3 &&
                (!isPremiumOrder || !premiumPaymentStarted) &&
                (paymentMethods.length > 0 || (isPremiumOrder && ((premiumDraft && premiumBalanceEnabled) || Number(order.balanceAmount || 0) > 0)))
            "
        >
            <div v-if="paymentMethods.length > 0" class="payment-section platform-payment-box">
                <div class="tit_row">
                    <h3>{{ $t("平台支付") }}</h3>
                    <p>{{ $t("支持所有银行卡或信用卡，更迅速、安全") }}</p>
                </div>
                <div class="type_tab">
                    <el-radio-group v-model="payType">
                        <el-radio v-for="item in paymentMethods" :key="item.type" class="tab flex align-center" :value="item.type">
                            <div class="cont"><img :src="item.icon" /></div>
                        </el-radio>
                    </el-radio-group>
                </div>
            </div>
        </div>
        <div class="pay_type_box" v-if="!isPremiumOrder && order.payTypeId >= 3">
            <el-tabs type="border-card" v-model="activeName">
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
            <div class="offline-claim-action">
                <el-button type="primary" size="large" @click="openOfflineClaim">{{ offlineClaimButtonText }}</el-button>
            </div>
        </div>
        <el-dialog v-model="offlineClaimVisible" :title="$t('填写付款确认')" width="500px" destroy-on-close>
            <el-form ref="offlineClaimFormRef" :model="offlineClaimForm" label-width="110px" class="offline-claim-form">
                <el-form-item :label="$t('付款金额')">
                    <FormatPrice v-model="order.unpaidAmount" :showText="false" class="offline-claim-amount" />
                </el-form-item>
                <el-form-item :label="$t('转账时间')">
                    <el-date-picker v-model="offlineClaimForm.transferTime" type="datetime" format="YYYY-MM-DD HH:mm" value-format="YYYY-MM-DD HH:mm" :disabled-date="isFutureDate" :placeholder="$t('请选择')" style="width: 100%" />
                </el-form-item>
                <el-form-item :label="$t('付款人姓名')">
                    <el-input v-model="offlineClaimForm.payerName" :placeholder="$t('请输入')" />
                </el-form-item>
                <el-form-item :label="$t('转账流水号')">
                    <el-input v-model="offlineClaimForm.bankSerialNo" :placeholder="$t('请填写')" />
                </el-form-item>
                <el-form-item :label="$t('付款凭证')" prop="vouchers" :rules="[{ required: true, message: $t('付款凭证必传'), trigger: 'change' }]">
                    <div class="offline-claim-upload-wrap">
                        <input ref="offlineNativeFileInput" class="offline-native-file-input" type="file" accept="image/png,image/jpeg,image/gif,image/webp,image/tiff" multiple @change="handleOfflineNativeFileChange" />
                        <el-upload
                            ref="offlineUploadRef"
                            v-model:file-list="offlineClaimFiles"
                            :class="['offline-claim-uploader', { 'is-limit-reached': offlineClaimFiles.length >= 5 }]"
                            list-type="picture-card"
                            :action="requestPrefix + '/user/user/uploadImg'"
                            :headers="uploadHeaders"
                            :limit="5"
                            accept="image/png,image/jpeg,image/gif,image/webp,image/tiff"
                            :before-upload="beforeOfflineVoucherUpload"
                            :on-success="handleOfflineVoucherSuccess"
                            :on-remove="handleOfflineVoucherRemove"
                            :on-preview="handleOfflineVoucherPreview"
                        >
                            <template #trigger>
                                <span class="offline-claim-upload-trigger" @click.stop="openOfflineFilePicker"><el-icon><Plus /></el-icon></span>
                            </template>
                        </el-upload>
                        <el-image ref="offlinePreviewImageRef" :src="offlinePreviewUrl" :preview-src-list="offlinePreviewUrls" :initial-index="offlinePreviewIndex" preview-teleported style="display: none" />
                        <div class="offline-claim-upload-tip">{{ $t('最多上传5张') }}</div>
                    </div>
                </el-form-item>
            </el-form>
            <template #footer>
                <el-button @click="offlineClaimVisible = false">{{ $t('取消') }}</el-button>
                <el-button type="primary" :loading="offlineClaimSubmitting" @click="submitOfflineClaim">{{ $t('确认') }}</el-button>
            </template>
        </el-dialog>
        <div class="btn_box" v-if="Number(order.payTypeId) < 3 && (!isPremiumOrder || !premiumPaymentStarted)">
            <NuxtLink :to="payLink" target="_blank" @click="handlePayClick">
                <div class="btn" :class="{ 'btn-loading': paying || applePaying }">
                    <span v-if="!paying && !applePaying">{{ $t("去付款") }}</span>
                    <span v-else>{{ $t("支付中") }}...</span>
                    <i v-if="!paying && !applePaying" class="iconfont-pc icon-yuanjiaojiantouyou"></i>
                </div>
            </NuxtLink>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * @description 订单支付选择页面
 * - 展示待支付订单信息
 * - 提供多种支付方式选择（微信、支付宝、银行卡等）
 * - 选择支付方式后跳转到对应的支付页面
 */
import { reactive, ref, shallowRef, onMounted, onUnmounted, computed, watch, nextTick } from "vue";
import { orderPayInfo, creatPay, createDepositPayment } from "~/api/order/pay";
import { Plus } from "@element-plus/icons-vue";
import { submitOfflinePaymentClaim, getOfflinePaymentClaimDetail } from "~/api/user/order";
import { requestPrefix, requestHeader } from "~/utils/request";
import type { UploadProps, UploadUserFile } from "element-plus";
import { isOverseas } from "@/utils/util";
import { formatTransferTime, parseTransferTime } from "~/utils/transferTime";
import iconWechat from "~/assets/images/extra/payment_method_wechat.jpg";
import iconAlipay from "~/assets/images/extra/payment_method_alipay.jpg";
import iconPaypal from "~/assets/images/extra/payment_method_paypal.jpg";
import iconYunshanfu from "~/assets/images/extra/payment_method_yunpay_yunshanfu.jpg";
import iconApplePay from "~/assets/images/extra/payment_method_applepay.svg";
import iconStripe from "~/assets/images/extra/payment_method_stripe.png";
import iconAntom from "~/assets/images/extra/payment_method_antom.svg";
import iconXendit from "~/assets/images/extra/payment_method_xendit.svg";
const activeName = ref<number>(1);
const route = useRoute();
const id = ref(route.query.id);
const payType = ref("wechat");
const premiumOrderCreated = ref(false);
const premiumBalanceEnabled = ref(false);
const premiumDraftToken = ref("");
const premiumDraft = ref<any>();
const loading = ref(false);
const paying = ref(false); // 支付中状态
const order = ref<any>({});
const payment = ref<string[]>([]);
const offlinePaymentList = ref<any>({});
const offlineClaimVisible = ref(false);
const offlineClaimSubmitting = ref(false);
const offlineClaimFormRef = ref();
/** 转账时间不能晚于今天 */
const isFutureDate = (date: Date) => date.getTime() > Date.now();
const offlineClaimForm = reactive({ transferTime: "", payerName: "", bankSerialNo: "", vouchers: [] as any[] });
const offlineClaimFiles = ref<UploadUserFile[]>([]);
const offlineUploadRef = ref<any>();
const offlineNativeFileInput = ref<HTMLInputElement>();
const offlinePreviewImageRef = ref<any>();
const offlinePreviewUrl = ref("");
const offlinePreviewIndex = ref(0);
const uploadHeaders = requestHeader();
const type = ref<string>("");
const { t } = useI18n();
const isPremiumOrder = computed(() => route.query.orderType === "premiumMember" || route.query.premium === "1");
const premiumPaymentStarted = ref(false);
const premiumQrCode = ref("");
const premiumFormContainer = ref<HTMLDivElement | null>(null);

const offlineClaimButtonText = computed(() => {
    const status = order.value?.offlinePaymentClaimStatus;
    return status === 2 ? t("修改申请") : status === 0 ? t("认款详情") : t("付款完成");
});

const resetOfflineClaimForm = () => {
    offlineClaimForm.transferTime = "";
    offlineClaimForm.payerName = "";
    offlineClaimForm.bankSerialNo = "";
    offlineClaimForm.vouchers = [];
    offlineClaimFiles.value = [];
};

const beforeOfflineVoucherUpload: UploadProps["beforeUpload"] = (rawFile) => {
    if (!/\/(png|jpe?g|gif|webp|tiff)$/i.test(rawFile.type)) {
        message.error(t("不支持上传此文件格式"));
        return false;
    }
    if (rawFile.size / 1024 / 1024 > 2) {
        message.error(t("只能上传2M以内的图片"));
        return false;
    }
    return true;
};
const openOfflineFilePicker = () => {
    if (offlineClaimFiles.value.length >= 5) return;
    offlineNativeFileInput.value && (offlineNativeFileInput.value.value = "");
    offlineNativeFileInput.value?.click();
};
const offlinePreviewUrls = computed(() => offlineClaimFiles.value.map((file) => String(file.url || file.response?.data?.picUrl || "")).filter(Boolean));
const handleOfflineVoucherPreview: UploadProps["onPreview"] = (uploadFile) => {
    const urls = offlinePreviewUrls.value;
    if (!urls.length) return;
    const index = Math.max(0, urls.indexOf(String(uploadFile.url || "")));
    offlinePreviewIndex.value = index;
    offlinePreviewUrl.value = urls[index];
    nextTick(() => offlinePreviewImageRef.value?.showPreview?.());
};
const handleOfflineNativeFileChange = (event: Event) => {
    const input = event.target as HTMLInputElement;
    const selectedFiles = Array.from(input.files || []).slice(0, Math.max(0, 5 - offlineClaimFiles.value.length));
    selectedFiles.forEach((file) => offlineUploadRef.value?.handleStart(file));
    if (selectedFiles.length) offlineUploadRef.value?.submit();
};

const handleOfflineVoucherSuccess: UploadProps["onSuccess"] = (response: any, uploadFile) => {
    const picUrl = response?.data?.picUrl;
    if (!picUrl) return;
    offlineClaimForm.vouchers.push({
        uid: uploadFile.uid,
        picUrl,
        url: picUrl
    });
};

const handleOfflineVoucherRemove: UploadProps["onRemove"] = (uploadFile) => {
    offlineClaimForm.vouchers = offlineClaimForm.vouchers.filter((item) => item.uid !== uploadFile.uid && item.picUrl !== uploadFile.url);
};

const openOfflineClaim = async () => {
    if (!order.value?.orderId) return;
    if (order.value.offlinePaymentClaimStatus === 0) {
        message.info(t("您已提交汇款凭证，等待财务人员核对流水，我们尽快完成审核。"));
        return;
    }
    resetOfflineClaimForm();
    try {
        const claim = await getOfflinePaymentClaimDetail({ orderId: Number(order.value.orderId) });
        if (claim && claim.status === 2) {
            offlineClaimForm.transferTime = formatTransferTime(claim.transferTime);
            offlineClaimForm.payerName = claim.payerName || "";
            offlineClaimForm.bankSerialNo = claim.bankSerialNo || "";
            offlineClaimForm.vouchers = (claim.voucherUrls || []).map((picUrl) => ({ picUrl }));
            offlineClaimFiles.value = (claim.voucherUrls || []).map((url, index) => ({ uid: `claim-${index}`, name: `voucher-${index + 1}`, url, status: "success" }));
        }
    } catch {
        // 未提交认款时接口返回空值，继续展示空表单
    }
    offlineClaimVisible.value = true;
};

const submitOfflineClaim = async () => {
    if (!offlineClaimFormRef.value || !order.value?.orderId) return;
    try {
        await offlineClaimFormRef.value.validate();
        const voucherUrls = offlineClaimForm.vouchers.filter((item) => item?.picUrl).map((item) => item.picUrl);
        if (!voucherUrls.length) {
            message.error(t("付款凭证必传"));
            return;
        }
        offlineClaimSubmitting.value = true;
        await submitOfflinePaymentClaim({
            orderId: Number(order.value.orderId),
            transferTime: parseTransferTime(offlineClaimForm.transferTime),
            payerName: offlineClaimForm.payerName.trim(),
            bankSerialNo: offlineClaimForm.bankSerialNo.trim(),
            voucherUrls
        });
        message.success(t("提交成功，等待财务审核"));
        offlineClaimVisible.value = false;
        await navigateTo(`/member/order/info?id=${order.value.orderId}`);
    } catch (error: any) {
        if (error?.message) message.error(error.message);
    } finally {
        offlineClaimSubmitting.value = false;
    }
};

// 支付方式标识对应的图标
const PAYMENT_ICON_MAP: Record<string, string> = {
    wechat: iconWechat,
    alipay: iconAlipay,
    paypal: iconPaypal,
    yabanpayWechat: iconWechat,
    yabanpayAlipay: iconAlipay,
    yunpayWechat: iconWechat,
    yunpayAlipay: iconAlipay,
    yunpayYunshanfu: iconYunshanfu,
    stripe: iconStripe,
    antom: iconAntom,
    xendit: iconXendit,
    huifuWechat: iconWechat,
    huifuAlipay: iconAlipay,
    lakalaWechat: iconWechat,
    lakalaAlipay: iconAlipay,
    yeepayWechat: iconWechat,
    yeepayAlipay: iconAlipay,
    applepay: iconApplePay
};

// Apple Pay：内联流程封装 + 能力检测（仅苹果 Safari 可见可用）
const { launchApplePay, paying: applePayingSource } = useApplePay();
const applePayAvailable = ref(false);
onMounted(() => {
    applePayAvailable.value = canUseApplePay();
});
// applePaying：启用 Apple Pay 时同步内联支付状态，否则恒为 false
const applePaying = ref(false);
watch(applePayingSource, (v) => (applePaying.value = v), { immediate: true });

const getPaymentIcon = (type: string) => {
    if (PAYMENT_ICON_MAP[type]) return PAYMENT_ICON_MAP[type];
    const normalized = type.toLowerCase();
    if (normalized.includes("alipay")) return iconAlipay;
    if (normalized.includes("wechat")) return iconWechat;
    return iconWechat;
};

// 按接口返回的 paymentList 顺序生成可展示的支付方式列表
const paymentMethods = computed(() =>
    payment.value
        .filter((key) => {
            if (isPremiumOrder.value) {
                return !["offline", "balance", "cash", "transfer"].includes(key.toLowerCase());
            }
            if (!PAYMENT_ICON_MAP[key]) return false;
            if ((key === "stripe" || key === "antom" || key === "xendit") && !isOverseas()) return false;
            // Apple Pay 仅在支持的浏览器（苹果 Safari）中展示
            if (key === "applepay") return applePayAvailable.value;
            return true;
        })
        .map((key) => ({ type: key, icon: getPaymentIcon(key) }))
);

const checkType = (item: any) => {
    payType.value = item.type;
};

/**
 * @description 加载订单支付信息，包括订单详情、可用支付方式和线下支付说明
 * @returns 无返回值，设置order、payment、offlinePaymentList等状态
 */
const loadOrderPayInfo = async () => {
    loading.value = true;
    try {
        const result = await orderPayInfo({
            id: id.value
        });
        order.value = result.order;
        payment.value = result.paymentList;
        offlinePaymentList.value = result.offlinePaymentList;
        // 默认选中接口返回的第一个可用支付方式
        if (paymentMethods.value.length > 0 && !paymentMethods.value.some((item) => item.type === payType.value)) {
            payType.value = paymentMethods.value[0].type;
        }
        // 定金预售且处于“非尾款阶段”时，标记为定金支付
        if ((result.order as any)?.presaleInfo?.saleMode === 2 && (result.order as any)?.presaleInfo?.presaleStatus !== 1) {
            type.value = "saleMode";
        } else {
            type.value = "";
        }
        // 根据已启用的线下支付方式，默认选中第一个
        if ("thirdPartyPaymentTransfer" in offlinePaymentList.value) {
            activeName.value = 3;
        } else if ("offlinePayBank" in offlinePaymentList.value) {
            activeName.value = 1;
        } else if ("offlinePayCompany" in offlinePaymentList.value) {
            activeName.value = 2;
        } else if ("otherPaymentRemittance" in offlinePaymentList.value) {
            activeName.value = 4;
        }
        loading.value = false;
    } catch (error: any) {
        message.error(error.message);
        navigateTo("/member/order/list/");
    } finally {
    }
};
loadOrderPayInfo();
// 收银台重定向类支付：调 creatPay 拿 payInfo.url 后前端直接跳转，不经 /order/xxx 中间页
const REDIRECT_PAY_TYPES = ["stripe", "antom", "xendit"];

/**
 * @description 根据选择的支付方式生成对应的支付页面链接（重定向类支付除外）
 * @returns 支付页面URL，包含订单ID参数
 */
const payLink = computed(() => {
    if (payment.value.length === 0) {
        return "";
    }

    // 重定向类支付（stripe/antom/xendit）不走链接跳转，改由 handleRedirectPay 处理
    if (isOverseas() && REDIRECT_PAY_TYPES.includes(payType.value)) {
        return "";
    }

    // 与多商户版本一致：用 type 传递预售定金场景标记（普通订单会带 &type= 为空）
    const orderTypeParam = `&type=${type.value}`;

    switch (payType.value) {
        case "wechat":
            return "/order/wechat/?id=" + id.value + "&type=" + type.value;
        case "alipay":
            return "/order/alipay/?id=" + id.value + "&type=" + type.value;
        case "paypal":
            return "/order/paypal/?id=" + id.value + "&type=" + type.value;
        case "yabanpayWechat":
            return "/order/yabanpay/?type=wechat&id=" + id.value;
        case "yabanpayAlipay":
            return "/order/yabanpay/?type=alipay&id=" + id.value;
        case "yunpayWechat":
            return "/order/yunpay/?type=wechat&id=" + id.value;
        case "yunpayAlipay":
            return "/order/yunpay/?type=alipay&id=" + id.value;
        case "yunpayYunshanfu":
            return "/order/yunpay/?type=yunshanfu&id=" + id.value;
        case "huifuWechat":
            return "/order/huifu/?payCode=huifuWechat&id=" + id.value + orderTypeParam;
        case "huifuAlipay":
            return "/order/huifu/?payCode=huifuAlipay&id=" + id.value + orderTypeParam;
        case "lakalaWechat":
            return "/order/lakala/?payCode=lakalaWechat&id=" + id.value + orderTypeParam;
        case "lakalaAlipay":
            return "/order/lakala/?payCode=lakalaAlipay&id=" + id.value + orderTypeParam;
        case "yeepayWechat":
            return "/order/yeepay/?payCode=yeepayWechat&id=" + id.value + orderTypeParam;
        case "yeepayAlipay":
            return "/order/yeepay/?payCode=yeepayAlipay&id=" + id.value + orderTypeParam;
        default:
            return "";
    }
});


/**
 * @description 收银台重定向类支付处理（stripe / antom / xendit 共用）
 * 创建支付会话后：优先按后端返回的 url 直接重定向；stripe 另兼容 html 表单自动提交
 * @param payTypeValue 支付方式标识，作为 creatPay/createDepositPayment 的 type 参数
 */
const handleRedirectPay = async (payTypeValue: string) => {
    if (paying.value) return;
    paying.value = true;

    try {
        const result =
            type.value === "saleMode"
                ? await createDepositPayment({ id: id.value, type: payTypeValue })
                : await creatPay({
                      id: id.value,
                      type: payTypeValue
                  });

        // antom/xendit 收银台与 stripe checkout 均返回 url
        if (result.payInfo?.url) {
            window.location.href = result.payInfo.url;
            return;
        }

        // html 表单自动提交（stripe 兜底）
        if (result.payInfo?.html) {
            const formContainer = document.createElement("div");
            document.body.appendChild(formContainer);
            formContainer.innerHTML = result.payInfo.html;
            const forms = document.forms;
            forms && forms[0]?.submit();
            return;
        }

        message.error(t("支付信息获取失败，请重试"));
    } catch (error: any) {
        message.error(error.message || t("该订单金额未满足该支付方式的最低金额要求,请选择其他支付"));
    } finally {
        paying.value = false;
    }
};

/**
 * @description 拉起 Apple Pay 内联支付（不跳中间页）
 * 支付成功后弹窗提示并跳转订单列表；取消静默、失败提示
 */
const startApplePay = () => {
    launchApplePay({
        id: id.value,
        isDeposit: type.value === "saleMode",
        amount: order.value.unpaidAmount,
        label: order.value.shopName || order.value.orderSn || "Apple Pay",
        onSuccess: () => {
            ElMessageBox.confirm(t("支付完成"), t("提示"), {
                confirmButtonText: t("查看订单"),
                showClose: false,
                showCancelButton: false,
                closeOnClickModal: false,
                type: "success"
            })
                .then(() => {
                    navigateTo("/member/order/list");
                })
                .catch(() => {});
        },
        onFail: (msg?: string) => message.error(msg || t("支付失败"))
    });
};

// 处理支付按钮点击
const handlePayClick = (e: Event) => {
    // applepay：走内联支付流程，不做 NuxtLink 跳转
    if (payType.value === "applepay") {
        e.preventDefault();
        startApplePay();
        return;
    }
    if (!payType.value) {
        e.preventDefault();
        message.error(t("请选择支付方式"));
        return;
    }

    // 重定向类支付（stripe/antom/xendit）：阻止 NuxtLink 空跳转，改为创建支付会话后重定向
    if (isOverseas() && REDIRECT_PAY_TYPES.includes(payType.value)) {
        e.preventDefault();
        handleRedirectPay(payType.value);
    }
};

</script>
<style lang="less" scoped>
.container {
    margin-top: 30px;
    :deep(.el-tabs--border-card > .el-tabs__header .el-tabs__item.is-active) {
        color: #333;
    }
    :deep(.el-tabs--border-card > .el-tabs__header .el-tabs__item:not(.is-disabled):hover) {
        color: #333;
    }
    .success_txt {
        color: #239700;
        font-size: 14px;
        margin-bottom: 15px;
    }

    .price_box {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 10px;
        .num_box {
            display: flex;
            align-items: baseline;
            .tit {
                color: #333;
                font-size: 18px;
                font-weight: bold;
                line-height: 26px;
            }
            :deep(.price) {
                align-items: baseline;
                vertical-align: baseline;
                .util {
                    color: var(--price);
                    font-size: 16px;
                    line-height: 28px;
                }
                .num,
                .decimals {
                    color: var(--price);
                    font-size: 28px;
                }
            }
        }
        .order_num {
            display: flex;
            align-items: center;
            a {
                color: var(--general);
            }
        }
    }
    .pay_type_box {
        // border: 1px solid #ccc;
        background: #fff;
        border-radius: 8px;
        padding: 30px 30px;
        .tit_row {
            display: flex;
            align-items: flex-end;
            h3 {
                color: #333;
                font-size: 18px;
                font-weight: 400;
                margin-right: 10px;
            }
            p {
                color: #999;
            }
        }
        .type_tab {
            display: flex;
            .tab {
                border: 1px solid #d9d9d9;
                cursor: pointer;
                height: 39px;
                padding: 4px 5px 4px 15px;
                background: #fff;
                overflow: hidden;
                width: 146px;
                margin-right: 20px;
                border-radius: 8px;
                margin-top: 25px;
                .cont {
                    height: 35px;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    img {
                        max-height: 35px;
                        max-width: 100%;
                        object-fit: contain;
                    }
                }
                .check {
                    width: 10px;
                    height: 10px;
                    border: 1px solid #333;
                    border-radius: 100px;
                    margin-right: 5px;
                    text-align: center;
                    line-height: 10px;
                    span {
                        display: inline-block;
                        width: 7px;
                        height: 7px;
                        border-radius: 100px;
                    }
                }
            }
            .active {
                border: 1px solid #ed3714;
                .check {
                    border: 1px solid #0075ff;
                    span {
                        background-color: #0075ff;
                    }
                }
            }

            .balance-tab {
                width: 180px;
                padding: 4px 12px;

                .balance-payment {
                    display: flex;
                    flex-direction: column;
                    align-items: flex-start;
                    justify-content: center;
                    gap: 2px;
                    line-height: 1.2;

                    &__title {
                        color: #333;
                        font-size: 14px;
                    }

                    &__amount {
                        display: inline-flex;
                        align-items: center;
                        gap: 4px;
                        color: #999;
                        font-size: 12px;

                        :deep(.util),
                        :deep(.num),
                        :deep(.decimals) {
                            color: #999;
                            font-size: 12px;
                        }
                    }
                }
            }
        }
        .offline-claim-action {
            display: flex;
            justify-content: flex-end;
            margin-top: 24px;
        }
    }

    .premium-balance-box {
        .tit_row h3 {
            margin: 0 10px 0 0;
            line-height: 26px;
        }

        .balance-tab {
            display: inline-flex;
            align-items: center;
            min-height: 39px;
            min-width: 270px;
            margin-top: 25px;
            padding: 4px 14px;
            box-sizing: border-box;
            white-space: nowrap;
            border: 1px solid #d9d9d9;
            border-radius: 8px;
            background: #fff;

            .balance-payment__title {
                color: #333;
                font-size: 14px;
            }

            .balance-payment__amount {
                display: inline-flex;
                align-items: center;
                gap: 4px;
                margin-left: 12px;
                color: #999;
                font-size: 12px;
            }
        }
    }

    .payment-methods-box {
        .payment-section + .payment-section {
            margin-top: 20px;
            padding-top: 20px;
            border-top: 1px solid #f0f0f0;
        }

        .platform-payment-box .tit_row {
            align-items: flex-end;
        }
    }

    .premium-payment-result {
        margin-top: 20px;
        padding: 24px;
        border-radius: 8px;
        background: #fff;
        text-align: center;

        .premium-payment-qrcode {
            h3 {
                margin: 0 0 12px;
                color: #333;
                font-size: 18px;
                font-weight: 400;
            }

            img {
                display: block;
                width: 300px;
                height: 300px;
                margin: 0 auto;
            }

            p {
                margin: 12px 0 0;
                color: #8a919f;
            }
        }
    }
    .btn_box {
        display: flex;
        flex-direction: column;
        align-items: flex-end;
        margin-top: 20px;
        .btn {
            display: flex;
            align-items: center;
            background-color: var(--general);
            border: 1px solid var(--general);
            color: var(--main-text);
            cursor: pointer;
            border-radius: 2px;
            height: 16px;
            line-height: 16px;
            text-align: center;
            margin-right: 20px;
            width: 80px;
            padding: 10px 12px;
            font-size: 16px;
            span {
                display: inline-block;
                margin-left: 10px;
            }
            i {
                font-size: 12px;
                margin-left: 5px;
            }
            &.btn-loading {
                opacity: 0.7;
                cursor: not-allowed;
            }
        }
        .tips {
            display: flex;
            margin-right: 16px;
            margin-top: 10px;
            cursor: pointer;
            p {
                margin-left: 5px;
                span {
                    color: #b2002e;
                    margin-left: 5px;
                }
            }
        }
    }
}
.offline-claim-form {
    .offline-claim-amount { color: var(--price); font-size: 20px; }
    .offline-claim-upload-wrap { width: 100%; }
    .offline-native-file-input { display: none; }
    .offline-claim-upload-tip { margin-top: 6px; color: #999; font-size: 12px; line-height: 18px; }
    .offline-claim-upload-trigger { display: flex; width: 100%; height: 100%; align-items: center; justify-content: center; cursor: pointer; }
    .offline-claim-uploader.is-limit-reached :deep(.el-upload--picture-card) { display: none; }
    .offline-claim-uploader :deep(.el-upload-list__item .el-icon--close-tip) { display: none !important; }
    .offline-claim-uploader :deep(.el-upload-list__item-status-label) { display: none !important; }
    .offline-claim-uploader {
        display: flex;
        flex-wrap: wrap;
        align-items: flex-start;
        :deep(.el-upload--picture-card),
        :deep(.el-upload-list--picture-card .el-upload-list__item) { width: 80px; height: 80px; margin: 0 8px 8px 0; }
        :deep(.el-upload--picture-card) { display: flex; align-items: center; justify-content: center; border: 1px dashed var(--el-border-color); }
        :deep(.el-upload--picture-card:empty)::after { content: "+"; color: #8c939d; font-size: 28px; line-height: 1; }
    }
}
.cont {
    width: 100px;
    display: flex;
    align-items: center;
    justify-content: center;
    & > img {
        max-width: 100%;
        max-height: 100%;
        object-fit: contain;
    }
}
</style>
