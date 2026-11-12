<template>
    <div class="gift-card-box" :class="`size-${props.cardSize}`">
        <div class="gift-card">
            <div class="gift-shop-info">
                <!-- <el-image v-if="cardInfo.shopLogo" class="shop-img" :src="imageFormat(cardInfo.shopLogo || '')" />
                <div v-else class="shop-img">{{ cardInfo.shopTitle?.at(0) || "" }}</div> -->
                <div class="shop-title">{{ cardInfo.shopTitle }}</div>
            </div>
            <div class="gift-card-no">
                <div class="gift-card-label">{{ $t("卡号") }}</div>
                <div v-if="!showGiftCardNo" class="gift-card-no-box">
                    <span class="card-no-item">{{ hideCardNumber }}</span>
                </div>
                <div v-else class="gift-card-no-box">
                    <span class="card-no-item">{{ cardNumber }}</span>
                </div>
                <div v-if="showActions" class="copy-btn" @click="handleCardNumberCopy">{{ $t("复制") }}</div>
            </div>
            <div class="gift-card-pwd">
                <div class="gift-card-label">{{ $t("密码") }}</div>
                <div v-if="!showGiftCardNo" class="card-no-item">
                    <span>{{ hideCardPassword }}</span>
                </div>
                <div v-else class="card-no-item">
                    <span>{{ cardPassword }}</span>
                </div>
                <div v-if="showActions" class="copy-btn" @click="handlePasswordCopy">{{ $t("复制") }}</div>
            </div>
            <div class="gift-card-info">
                <div class="gift-card-name">{{ $t(cardInfo.cardGroupName) }}</div>
                <div class="flex align-end gift-card-date">
                    <div v-if="cardInfo.expiredTime == 0" class="gift-card-date-text">
                        {{ $t("长期有效") }}
                    </div>
                    <div v-else-if="cardInfo.expiredTime && cardInfo.expiredTime != 0" class="gift-card-date-text">
                        {{ $t("有效期至") }} {{ formatTimestamp(cardInfo.expiredTime * 1000) }}
                    </div>
                </div>
            </div>
            <div v-if="showActions" class="gift-card-actions-box">
                <div class="gift-card-copy-all" @click="handleCopyAll">
                    <div class="iconfont-pc icon-fuzhi" />
                    <span>{{ $t("复制券码") }}</span>
                </div>
                <div class="gift-card-divider"></div>
                <div class="gift-card-show" @click="handleShowGiftCardNo">
                    <div class="iconfont-pc" :class="showGiftCardNo ? 'icon-xianshi' : 'icon-buxianshi'" />
                    <span v-if="!showGiftCardNo">{{ $t("显示券码") }}</span>
                    <span v-else>{{ $t("隐藏券码") }}</span>
                </div>
            </div>
        </div>

        <div class="gift-card-status">
            <div class="gift-card-status-item">
                {{ displayStatus }}
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
/**
 * @description 礼品卡展示组件
 * - 展示礼品卡信息（卡号、密码、有效期、状态）
 * - 支持显示/隐藏卡号密码
 * - 支持复制卡号、密码、一键复制券码
 * - 根据过期时间自动判断状态（待兑换/已兑换/已过期）
 */
import { ref, computed } from "vue";
import { message } from "ant-design-vue";
import dayjs from "dayjs";
import type { GiftCardInfo } from "~/types/giftCard/giftCard";
import cardBg from "~/assets/images/giftCard/card_bg.png";
import { getGiftCardPassword } from "~/api/giftCard/giftCard";

const props = defineProps({
    cardSize: {
        type: String as PropType<"large" | "small">,
        default: "small"
    },
    cardInfo: {
        type: Object as PropType<GiftCardInfo>,
        default: (): GiftCardInfo => ({}) as GiftCardInfo
    },
    showActions: {
        type: Boolean,
        default: true
    }
});

const emit = defineEmits<{
    (e: "copyAll", cardNumber: string, cardPassword: string): void;
}>();

const { t } = useI18n();

const showGiftCardNo = ref(false);
const isGetCardPassword = ref(false);
const hideCardNumber = ref(props.cardInfo.cardNumber);
const hideCardPassword = ref(props.cardInfo.cardPassword);
const cardNumber = ref("");
const cardPassword = ref("");
const cardBackgroundImg = computed(() => {
    return `url(${props.cardInfo.imageUrl || cardBg}) center no-repeat`;
});
const statusMap: Record<number, string> = {
    0: t("待兑换"),
    1: t("已兑换"),
    2: t("已过期")
};

// 判断是否已过期
const isExpired = computed(() => {
    const { expiredTime } = props.cardInfo;
    // 如果过期时间为0或不存在，表示长期有效，不会过期
    if (!expiredTime || expiredTime == 0) {
        return false;
    }
    // 将秒级时间戳转换为毫秒级，然后与当前时间比较
    const expiredTimestamp = expiredTime * 1000;
    const currentTimestamp = Date.now();
    return currentTimestamp > expiredTimestamp;
});

// 显示的状态文本
const displayStatus = computed(() => {
    const status = props.cardInfo.status || 0;
    // 如果是已兑换，显示已兑换
    if (status === 1) {
        return statusMap[1]; // 已兑换
    }
    // 如果是未兑换，根据过期时间判断
    if (isExpired.value) {
        return statusMap[2]; // 已过期
    }
    return statusMap[0]; // 待兑换
});

const _getGiftCardPassword = async () => {
    try {
        const res: any = await getGiftCardPassword(props.cardInfo.giftCardSn);
        isGetCardPassword.value = true;
        cardNumber.value = res.cardNumber;
        cardPassword.value = res.cardPassword;
    } catch (error: any) {
        console.error(error);
    }
};

const formatTimestamp = (timestamp: string | number) => {
    // 使用dayjs格式化时间
    return dayjs(timestamp).format("YYYY-MM-DD");
};

const handleShowGiftCardNo = async () => {
    if (!isGetCardPassword.value) {
        await _getGiftCardPassword();
    }
    if (isGetCardPassword.value) {
        showGiftCardNo.value = !showGiftCardNo.value;
    }
};

const handleCardNumberCopy = async () => {
    if (!isGetCardPassword.value) {
        await _getGiftCardPassword();
    }
    copyText(cardNumber.value, () => {
        message.success(t("复制成功"));
    });
};

const handlePasswordCopy = async () => {
    if (!isGetCardPassword.value) {
        await _getGiftCardPassword();
    }
    copyText(cardPassword.value, () => {
        message.success(t("复制成功"));
    });
};

const handleCopyAll = async () => {
    if (!isGetCardPassword.value) {
        await _getGiftCardPassword();
    }
    const copyContent = `${t("卡号")}：${cardNumber.value}\n${t("卡密")}：${cardPassword.value}`;
    copyText(copyContent, () => {
        message.success(t("复制成功"));
    });
    // 触发事件，传递卡号和卡密
    emit("copyAll", cardNumber.value, cardPassword.value);
};
</script>
<style lang="less" scoped>
.gift-card-box {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    border-radius: 10px;
    padding: 0 12.5px;
    overflow: hidden;
    position: relative;
    &::before {
        content: "";
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: v-bind(cardBackgroundImg);
        background-size: cover;
        border-radius: 11px;
        z-index: 0;
    }
    .gift-card {
        position: relative;
        z-index: 2;
        box-sizing: border-box;
        color: #fff;
        .gift-shop-info {
            padding: 12px 0;
            display: flex;
            align-items: center;
            .shop-img {
                width: 30px;
                height: 30px;
                border-radius: 50%;
                overflow: hidden;
                display: flex;
                justify-content: center;
                align-items: center;
                background-color: #fff;
                color: #000;
                font-weight: bold;
            }
            .shop-title {
                // padding-left: 9px;
                font-size: 16px;
                // line-height: 16px;
                display: -webkit-box;
                -webkit-line-clamp: 1;
                -webkit-box-orient: vertical;
                overflow: hidden;
            }
        }
        .gift-card-label {
            white-space: nowrap;
            font-size: 12px;
            line-height: 12px;
            // opacity: 0.8;
        }
        .gift-card-no {
            display: flex;
            align-items: center;
            gap: 12px;
            margin-top: 12px;
            height: 20px;
            .gift-card-no-box {
                display: flex;
                align-items: center;
                gap: 16px;
                line-height: 15px;
                .card-no-item {
                    display: flex;
                    align-items: center;
                    font-size: 16px;
                    line-height: 16px;
                    font-weight: bold;
                }
            }
        }
        .gift-card-pwd {
            display: flex;
            align-items: center;
            gap: 12px;
            margin-top: 11px;
            overflow: hidden;
            height: 20px;
            .card-no-item {
                font-size: 16px;
                line-height: 16px;
                // font-weight: bold;
            }
        }
        .gift-card-info {
            margin-top: 18px;
            .gift-card-name {
                font-size: 15px;
                line-height: 15px;
                display: block;
                overflow: hidden;
                white-space: nowrap;
                text-overflow: ellipsis;
                max-width: 100%;
            }
            .gift-card-date {
                position: relative;
                padding-top: 15px;
                .gift-card-date-text {
                    line-height: 12px;
                    font-size: 12px;
                    line-height: 12px;
                    opacity: 0.51;
                }
            }
        }
        .gift-card-actions-box {
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 0;
            margin-top: 15px;
            background-color: rgba(0, 0, 0, 0.65);
            border-radius: 10px;
            padding: 10px;
        }
        .gift-card-copy-all,
        .gift-card-show {
            cursor: pointer;
            font-size: 14px;
            line-height: 14px;
            display: flex;
            align-items: center;
            gap: 4px;
            flex: 1;
            justify-content: center;
            .iconfont-pc {
                padding-top: 1.75px;
                font-size: 14px;
            }
        }
        .gift-card-divider {
            width: 1px;
            height: 14px;
            background-color: rgba(255, 255, 255, 0.3);
            margin: 0 12px;
        }
        .copy-btn {
            font-size: 10px;
            line-height: 10px;
            background-color: #0000004d;
            border: 1px solid rgb(255 255 255 / 20%);
            border-radius: 10px;
            padding: 2.5px 7px;
            white-space: nowrap;
            cursor: pointer;
        }
        .hide-info {
            padding-top: 4px;
        }
    }
    .gift-card-status {
        position: absolute;
        right: 0px;
        top: 0px;
        overflow: hidden;
        z-index: 2;
        .gift-card-status-item {
            font-size: 12px;
            line-height: 12px;
            background-color: #f8e0c8;
            color: #996500;
            padding: 5px 10px;
            border-left: 1px solid #fff;
            border-bottom: 1px solid #fff;
            border-bottom-left-radius: 10px;
        }
    }
    &.size-large {
        padding: 0 15px;
        border-bottom-left-radius: 15px;
        border-bottom-right-radius: 15px;
        .gift-shop-info {
            padding: 15px 0;
            .shop-title {
                font-size: 20px;
                // line-height: 20px;
            }
        }
        .gift-card-label {
            font-size: 14px;
            line-height: 14px;
        }
        .gift-card-no {
            margin-top: 10px;
            gap: 12px;
            .gift-card-no-box {
                gap: 16px;
                .card-no-item {
                    font-size: 20px;
                    line-height: 20px;
                    font-weight: bold;
                }
            }
            .hide-info {
                padding-top: 6px;
            }
        }
        .gift-card-pwd {
            gap: 12px;
            margin-top: 16px;
            .card-no-item {
                font-size: 20px;
                line-height: 20px;
                // font-weight: bold;
            }
        }
        .gift-card-info {
            margin-top: 18px;
            .gift-card-name {
                font-size: 16px;
                line-height: 16px;
                display: block;
                overflow: hidden;
                white-space: nowrap;
                text-overflow: ellipsis;
                max-width: 100%;
            }
            .gift-card-date {
                font-size: 12px;
                line-height: 12px;
                // padding-top: 8px;
            }
        }
        .gift-card-actions-box {
            margin-top: 5px;
        }
        .gift-card-copy-all,
        .gift-card-show {
            font-size: 14px;
            line-height: 14px;
            .iconfont-pc {
                padding-top: 1.75px;
                font-size: 14px;
            }
        }
        .copy-btn {
            font-size: 13px;
            line-height: 13px;
        }
    }
}
</style>
