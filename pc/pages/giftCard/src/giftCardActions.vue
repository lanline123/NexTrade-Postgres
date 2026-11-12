<template>
    <div v-if="showTransfer || showExchange" class="page-info-body flex justify-end" style="padding: 20px 0;">
        <el-button v-if="showTransfer" type="primary" plain :disabled="disabled" @click="showTransferDialog = true">{{ $t(transferBtnText) }}</el-button>
        <el-button v-if="showExchange" type="primary" :disabled="disabled" :class="exchangeClass" @click="showExchangeDialog = true">{{
            $t(exchangeBtnText)
        }}</el-button>
        <!-- 赠送弹窗 -->
        <el-dialog v-model="showTransferDialog" :title="$t('礼品卡赠送')" width="600">
            <div class="invite-dialog-content">
                <el-radio-group v-model="selectedCardSn">
                    <div class="gift-card-list">
                        <template v-for="item in cards" :key="item.giftCardSn">
                            <div>
                                <el-radio :value="item.giftCardSn">
                                    <div class="gift-card-item">
                                        <giftCard :card-info="item" />
                                    </div>
                                </el-radio>
                            </div>
                        </template>
                    </div>
                </el-radio-group>
            </div>
            <template #footer>
                <div class="dialog-footer">
                    <el-button @click="showTransferDialog = false">{{ $t("取消") }}</el-button>
                    <el-button type="primary" :loading="transferLoading" @click="handleGiftCardSend">{{ $t("赠送") }}</el-button>
                </div>
            </template>
        </el-dialog>
        <!-- 赠送链接复制弹窗 -->
        <el-dialog v-model="showTransferLinkDialog" :title="$t('礼品卡赠送')" width="600">
            <div class="link-dialog-body">
                <div class="link-dialog-content">
                    <el-input v-model="receiptLink" :disabled="true" size="large" class="invite-dialog-input" />
                    <el-button type="primary" @click="handleLinkCopy">{{ $t("复制链接") }}</el-button>
                </div>
                <div class="tips">
                    {{ $t("点击上方复制链接，发送给好友进行赠送") }}
                </div>
            </div>
            <template #footer>
                <div class="dialog-footer">
                    <el-button @click="showTransferLinkDialog = false">{{ $t("关闭") }}</el-button>
                </div>
            </template>
        </el-dialog>
        <!-- 兑换弹窗 -->
        <el-dialog v-model="showExchangeDialog" :title="$t('礼品卡兑换')" width="600" @close="resetExchangeForm">
            <div class="invite-dialog-content">
                <el-form ref="formRef" :model="form" label-width="auto" :rules="rules">
                    <el-form-item :label="$t('卡号')" prop="cardNumber">
                        <el-input v-model="form.cardNumber" :placeholder="$t('请输入卡号')" />
                    </el-form-item>
                    <el-form-item :label="$t('卡密')" prop="cardPassword">
                        <el-input v-model="form.cardPassword" :placeholder="$t('请输入卡密')" />
                    </el-form-item>
                </el-form>
            </div>
            <template #footer>
                <div class="dialog-footer">
                    <el-button @click="showExchangeDialog = false">{{ $t("取消") }}</el-button>
                    <el-button type="primary" :loading="exchangeLoading" @click="handleExchange">{{ $t("兑换") }}</el-button>
                </div>
            </template>
        </el-dialog>
    </div>
</template>

<script lang="ts" setup>
/**
 * @description 礼品卡操作组件
 * - 提供礼品卡赠送功能：选择卡片 -> 生成分享链接 -> 复制发送
 * - 提供礼品卡兑换功能：输入卡号卡密进行兑换
 * - 支持自动读取剪切板解析礼品卡信息（HTTPS环境）
 * - 兑换成功后根据礼品卡类型跳转到结算页或账户余额页
 */
import type { FormInstance } from "element-plus";
import { giftCardExchange, giftCardTransfer } from "~/api/giftCard/giftCard";
import type { GiftCardInfo } from "~/types/giftCard/giftCard";
import giftCard from "./giftCard.vue";
import { useUserStore } from "~/store/user";

const { t } = useI18n();
interface IProps {
    showTransfer?: boolean;
    showExchange?: boolean;
    transferBtnText?: string;
    exchangeBtnText?: string;
    exchangeClass?: string;
    cards?: GiftCardInfo[];
    card?: GiftCardInfo;
    disabled?: boolean;
    // 默认卡号和卡密（用于一键复制后自动填充）
    defaultCardNumber?: string;
    defaultCardPassword?: string;
}
const props = withDefaults(defineProps<IProps>(), {
    showTransfer: true,
    showExchange: true,
    transferBtnText: "赠送",
    exchangeBtnText: "立即兑换",
    defaultCardNumber: "",
    defaultCardPassword: ""
});

const userStore = useUserStore();
const transferLoading = ref(false);
const exchangeLoading = ref(false);
const showTransferDialog = ref(false);
const showTransferLinkDialog = ref(false);
const showExchangeDialog = ref(false);
const receiptLink = ref("");
const formRef = ref<FormInstance>();
const rules = {
    cardNumber: [{ required: true, message: t("请输入卡号"), trigger: "blur" }],
    cardPassword: [{ required: true, message: t("请输入卡密"), trigger: "blur" }]
};
const form = reactive({
    cardNumber: "",
    cardPassword: ""
});
const selectedCardSn = ref<string>("");
// const selectedCard = ref<GiftCardInfo>();
if (props.cards && props.cards?.length > 0) {
    selectedCardSn.value = props.cards[0].giftCardSn;
} else if (props.card) {
    selectedCardSn.value = props.card.giftCardSn;
}
const selectedCard = computed(() => {
    if (props.cards && props.cards?.length > 0) {
        return props.cards.find((item) => item.giftCardSn === selectedCardSn.value);
    } else if (props.card) {
        return props.card;
    }
    return undefined;
});

// 礼品卡赠送按钮点击事件
const handleGiftCardSend = async () => {
    if (transferLoading.value) {
        return;
    }
    transferLoading.value = true;
    try {
        if (!selectedCard.value) {
            showTransferDialog.value = false;
            message.error(t("请选择礼品卡"));
            return;
        }
        const result = await giftCardTransfer({ giftCardSn: selectedCard.value.giftCardSn, isFrom: "2" });
        // receiptLink.value = `${selectedCard.value.shopTitle} ${location.origin}/giftCard/transfer/${selectedCard.value.giftCardSn} ${selectedCard.value.cardGroupName}`;
        receiptLink.value = result as string;
        showTransferDialog.value = false;
        setTimeout(() => {
            showTransferLinkDialog.value = true;
        }, 120);
    } catch (error: any) {
        showTransferDialog.value = false;
        message.error(error.message);
    } finally {
        transferLoading.value = false;
    }
};

const handleLinkCopy = async () => {
    showTransferLinkDialog.value = false;
    copyText(receiptLink.value, () => {
        message.success(t("复制成功"));
    });
};

const handleExchange = async () => {
    if (exchangeLoading.value) {
        return;
    }
    exchangeLoading.value = true;
    if (!formRef.value) return;
    await formRef.value.validate(async (valid, fields) => {
        if (valid) {
            try {
                const res: any = await giftCardExchange(form);
                showExchangeDialog.value = false;
                message.success(t("兑换成功"));
                if (res.exchangeProductToCart?.flowType) {
                    navigateTo(`/order/check?flowType=${res.exchangeProductToCart.flowType}`);
                } else {
                    // 刷新余额
                    userStore.userInfoLoaded = false;
                    await userStore.getUserInfo();
                    setTimeout(() => {
                        navigateTo("/member/account/detail");
                    }, 200);
                }
            } catch (error: any) {
                showExchangeDialog.value = false;
                message.error(error.message);
            } finally {
                exchangeLoading.value = false;
            }
        } else {
            exchangeLoading.value = false;
            console.error("error submit!", fields);
        }
    });
};

const resetExchangeForm = () => {
    form.cardNumber = "";
    form.cardPassword = "";
};

// 解析礼品卡剪切板内容（兼容新旧两种格式）
const parseGiftCardClipboard = (text: string) => {
    if (!text) return false;
    // 格式：NEXTRADE_GIFTCARD|卡号|卡密
    if (text.startsWith("NEXTRADE_GIFTCARD|")) {
        const parts = text.split("|");
        if (parts.length === 3) {
            form.cardNumber = parts[1];
            form.cardPassword = parts[2];
            return true;
        }
    }
    // 新格式：卡号：xxx\n卡密：xxx
    const lines = text.split("\n").map((l) => l.trim()).filter(Boolean);
    if (lines.length === 2) {
        const numberMatch = lines[0].match(/^(?:卡号|Card No\.?|Card Number)(?:：|:)\s*(.+)$/i);
        const pwdMatch = lines[1].match(/^(?:卡密|Card Pwd|Card Password)(?:：|:)\s*(.+)$/i);
        if (numberMatch && pwdMatch) {
            form.cardNumber = numberMatch[1].trim();
            form.cardPassword = pwdMatch[1].trim();
            return true;
        }
    }
    return false;
};

// 使用 props 中的默认值填充表单
const fillFormFromProps = () => {
    if (props.defaultCardNumber && props.defaultCardPassword) {
        form.cardNumber = props.defaultCardNumber;
        form.cardPassword = props.defaultCardPassword;
        return true;
    }
    return false;
};

// 读取剪切板并解析礼品卡信息（HTTPS 环境）
const readClipboardForGiftCard = async () => {
    // 确保在客户端环境且支持 Clipboard API（仅 HTTPS 或 localhost）
    if (typeof window === "undefined" || !navigator?.clipboard?.readText) {
        // 剪切板不可用，尝试从 props 获取
        fillFormFromProps();
        return;
    }
    try {
        const clipboardText = await navigator.clipboard.readText();
        // 尝试从剪切板解析，如果失败则从 props 获取
        if (!parseGiftCardClipboard(clipboardText)) {
            fillFormFromProps();
        }
    } catch (error) {
        // 剪切板读取失败，尝试从 props 获取
        console.log("无法读取剪切板:", error);
        fillFormFromProps();
    }
};

// 粘贴事件处理（支持 HTTP 环境）
const handlePaste = (event: ClipboardEvent) => {
    const clipboardText = event.clipboardData?.getData("text");
    if (clipboardText && parseGiftCardClipboard(clipboardText)) {
        event.preventDefault();
    }
};

// 监听兑换弹窗打开
watch(showExchangeDialog, (newVal) => {
    if (newVal) {
        // 尝试自动读取剪切板（HTTPS 环境）
        readClipboardForGiftCard();
        // 添加粘贴事件监听（支持 HTTP 环境，用户按 Ctrl+V 时自动解析）
        nextTick(() => {
            document.addEventListener("paste", handlePaste);
        });
    } else {
        // 弹窗关闭时移除监听
        document.removeEventListener("paste", handlePaste);
    }
});
</script>

<style scoped lang="less">
.gift-card-item {
    width: 300px;
    height: 185px;
}

/* 添加以下样式使el-radio占据足够高度 */
.el-radio {
    display: flex;
    align-items: center;
    margin-bottom: 15px;
    height: auto;
}

.el-radio :deep(.el-radio__input) {
    margin-top: 8px;
}

.el-radio :deep(.el-radio__label) {
    flex: 1;
    height: 100%;
}
.link-dialog-body {
    min-height: 100px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    justify-content: center;
    .link-dialog-content {
        display: flex;
        align-items: center;
        gap: 10px;
    }
    .tips {
        color: #a4adb3;
        font-size: 12px;
        // padding-left: 6px;
    }
}
</style>
