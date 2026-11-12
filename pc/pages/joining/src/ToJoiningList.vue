<template>
    <div :class="{ 'to-joining-list': !isMore, 'new-ui': uiVersion === 'new' }" v-if="joiningList.length > 0">
        <slot name="header" :joiningList="joiningList"></slot>
        <div class="itemList" :class="{ isMore: isMore }">
            <el-row :gutter="rowGutter">
                <el-col :span="colSpan" v-for="(item, index) in moreList" :key="index" v-loading="loading">
                    <div class="item">
                        <div class="user">
                            <div class="item-img">
                                <img :src="imageFormat(item.joinUser?.avatar || commonStore.defaultAvatar)" />
                            </div>
                            <div class="item-name">
                                {{ hideUsername(item.joinUser.username) }}
                            </div>
                        </div>
                        <div class="item-content">
                            <div class="item-info">
                                <div class="item-count">
                                    {{ $t("还差")
                                    }}<span class="num">{{ item.minGroupNum - item.joinNumCount > 0 ? item.minGroupNum - item.joinNumCount : 0 }}</span
                                    >{{ $t("人") }}
                                </div>
                                <div class="countdown">
                                    <div class="title" v-if="item.countdownText">{{ $t("剩余") }} {{ item.countdownText }}{{ $t("结束") }}</div>
                                    <div class="title" v-else>{{ $t("已结束") }}</div>
                                </div>
                            </div>

                            <ProductBuy
                                :extraAttrIds="extraAttrIds"
                                :id="productId"
                                :shop-id="product.shopId"
                                :skuId="skuId"
                                :number="productNumber"
                                :isQuick="true"
                                :grouponDetail="{ ...grouponDetail, grouponPromotionRecordId: item.grouponPromotionRecordId }"
                            >
                                <el-button
                                    link
                                    :disabled="isOverseasUi ? isMyGroup(item) || !item.countdownText : !item.countdownText || userStore.userInfo.userId === item.joinUser.userId"
                                    :class="{ 'no-hover': isOverseasUi ? isMyGroup(item) || !item.countdownText : !item.countdownText || userStore.userInfo.userId === item.joinUser.userId }"
                                    >{{ $t("去参团") }} <el-icon style="margin-left: 2px"> <ArrowRight /> </el-icon
                                ></el-button>
                            </ProductBuy>
                        </div>
                    </div>
                </el-col>
            </el-row>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, unref, inject, onMounted, onUnmounted } from "vue";
import { imageFormat, getCountdownFormat, hideUsername } from "~/utils/format";
import { getProductRecordList } from "~/api/joining/joining";
import { message } from "ant-design-vue";
import { Vue3SeamlessScroll } from "vue3-seamless-scroll";
import { ArrowRight } from "@element-plus/icons-vue";
import { useCommonStore } from "~/store/common";
import { useUserStore } from "~/store/user";
const { t } = useI18n();
const userStore = useUserStore();
const commonStore = useCommonStore();
// 跨境版 UI 开关（由商详页 Product.vue provide 下发；非跨境时默认原逻辑）
const detailUiVersion = inject("detailUiVersion", null);
const isOverseasUi = computed(() => unref(detailUiVersion) === "overseas");

const props = defineProps({
    productId: {
        type: Number,
        default: 0
    },
    productNumber: {
        type: Number,
        default: 1
    },
    skuId: {
        type: Number || null,
        default: null
    },
    grouponDetail: {
        type: Object,
        default: () => {
            return {};
        }
    },
    price: {
        type: Number,
        default: 0
    },
    extraAttrIds: {
        type: String,
        default: ""
    },
    isMore: {
        type: Boolean,
        default: false
    },
    // 跨境版列宽控制（span===24 单列）
    span: {
        type: Number,
        default: 12
    },
    product: {
        type: Object,
        default: () => {
            return {};
        }
    },
    // UI 版本：old 老版，new 新版
    uiVersion: {
        type: String,
        default: "old"
    }
});

const colSpan = computed(() => {
    if (props.isMore) {
        return 24;
    }
    if (isOverseasUi.value) {
        return props.span === 24 ? 24 : 12;
    }
    return props.uiVersion === "new" ? 24 : 12;
});

const rowGutter = computed(() => (!isOverseasUi.value && props.uiVersion === "new" ? 0 : 20));

// 判断当前记录是否是当前用户开的团（跨境版逻辑，兼容 joinUser 为数组）
const isMyGroup = (item: any) => {
    if (Array.isArray(item.joinUser)) {
        return item.joinUser.some((user: any) => user.leader && user.userId === userStore.userInfo?.userId);
    }
    return item.joinUser.leader && item.joinUser.userId === userStore.userInfo?.userId;
};

const joiningList = ref<any>([]);
const loading = ref(false);

const moreList = computed(() => {
    return props.isMore ? joiningList.value : joiningList.value.slice(0, 2);
});

// 倒计时更新相关
let countdownTimer: number | null = null;

// 更新倒计时显示
const updateCountdown = () => {
    joiningList.value.forEach((item: any) => {
        if (item.endTime) {
            item.countdownText = getCountdownFormat(new Date(item.endTime).getTime(), t("天"));
        }
    });
};

const filterParams = ref({
    page: 1,
    size: 10,
    isMine: false,
    joinStatus: 1,
    productId: props.productId,
    excludeEnded: true
});

// 获取可以拼团的列表
const loadFilter = async () => {
    loading.value = true;
    try {
        const result: any = await getProductRecordList({ ...filterParams.value });
        joiningList.value = result.records;
        updateCountdown();
        countdownTimer = window.setInterval(() => {
            updateCountdown();
        }, 1000);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

// // 启动倒计时定时器
onMounted(() => {
    loadFilter();
    // 跨境版：确保 message 提示在遮罩层之上
    if (isOverseasUi.value) {
        const style = document.createElement("style");
        style.id = "to-joining-list-message-style";
        style.textContent = ".ant-message { z-index: 10000 !important; }";
        document.head.appendChild(style);
    }
});

// 清理定时器
onUnmounted(() => {
    if (countdownTimer) {
        clearInterval(countdownTimer);
    }
    const style = document.getElementById("to-joining-list-message-style");
    if (style) {
        document.head.removeChild(style);
    }
});
</script>

<style scoped lang="less">
.scroll-list {
    .itemList {
        .item {
            &:last-child {
                margin-bottom: 20px;
            }
        }
    }
}

.to-joining-list {
    margin-bottom: 24px;
}

.itemList {
    display: flex;
    flex-direction: column;
    gap: 20px;

    .item {
        width: 100%;
        display: flex;
        align-items: center;
        justify-content: space-between;
        background-color: #f2f4fa;
        border-radius: 4px;
        padding: 5px 10px;
        box-sizing: border-box;

        .user {
            display: flex;
            align-items: center;
            gap: 10px;

            .item-img {
                width: 50px;
                height: 50px;
                border-radius: 50%;
                overflow: hidden;

                img {
                    width: 100%;
                    height: 100%;
                    object-fit: contain;
                }
            }

            .item-name {
                font-size: 14px;
                color: #1a1a1a;
                line-height: 28px;
            }
        }

        .item-content {
            display: flex;
            align-items: center;
            gap: 25px;

            .item-info {
                display: flex;
                flex-direction: column;
                align-items: end;
                gap: 3px;

                .item-count {
                    font-weight: 500;
                    color: #1a1a1a;
                    white-space: nowrap;

                    .num {
                        color: var(--general);
                        margin: 0 5px;
                    }
                }

                .countdown {
                    font-weight: 500;
                    color: #9a9a9a;
                    .title {
                        white-space: nowrap;
                    }
                }
            }
        }
    }
}

.isMore {
    height: 240px;
    overflow-y: auto;
    &::-webkit-scrollbar {
        display: none;
    }
    .item {
        box-sizing: border-box;
    }
}

.no-hover {
    cursor: not-allowed !important;
    &:hover {
        color: var(--el-button-disabled-text-color) !important;
    }
}

.new-ui {
    .itemList {
        gap: 12px;
        overflow-x: hidden;

        :deep(.el-row) {
            margin-left: 0 !important;
            margin-right: 0 !important;
            row-gap: 12px;
        }

        :deep(.el-col) {
            padding-left: 0 !important;
            padding-right: 0 !important;
        }

        .item {
            border-radius: 12px;
            padding: 12px;
            flex-wrap: wrap;
            gap: 8px;
            min-width: 0;

            .user {
                min-width: 0;
                flex: 1;

                .item-name {
                    word-break: break-all;
                }
            }

            .item-content {
                flex-wrap: wrap;
                gap: 8px;
                min-width: 0;
            }
        }
    }

    &.to-joining-list {
        // max-width: 490px;
    }
}
</style>
