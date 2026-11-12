<!--
  o2o 原版组件存档（v1）
  切回方法：删除当前 ChatMessageList.vue，将本文件重命名为 ChatMessageList.vue
-->
<template>
    <div class="message-list-container" ref="messageContainerRef">
        <!-- 加载更多提示 -->
        <div v-if="loadingMore" class="loading-more">
            <span>{{ $t("加载中...") }}</span>
        </div>
        <div v-if="loadEnd && messageList.length > 0" class="load-end">
            <span>{{ $t("已经没有更多消息了") }}</span>
        </div>

        <!-- 消息列表 -->
        <div class="message-item" v-for="(msg, index) in messageList" :key="msg.id || index">
            <!-- 系统消息 -->
            <div v-if="msg.type === 3" class="system-msg">
                <span>{{ msg.content?.content }}</span>
            </div>

            <!-- 客服消息 (左侧) -->
            <div v-else-if="msg.type === 2" class="msg-row left">
                <img :src="imageFormat(shopAvatar)" class="avatar" />
                <div class="msg-content-wrapper">
                    <div class="msg-header">
                        <span class="name">{{ shopName }}</span>
                        <span class="time" style="margin-left: 8px">{{ msg.sendTime }}</span>
                    </div>
                    <!-- 文本消息 -->
                    <div v-if="msg.messageType === 'text'" class="bubble white">
                        <EmojiHtml v-if="hasEmoji(msg.content?.content)" :content="msg.content?.content" />
                        <span v-else v-text="msg.content?.content"></span>
                    </div>
                    <!-- 图片消息 -->
                    <div v-else-if="msg.messageType === 'image'" class="bubble white image-bubble">
                        <img :src="imageFormat(msg.content?.pic)" class="msg-image" @click="handlePreviewImage(msg.content?.pic)" />
                    </div>
                    <!-- 商品卡片 -->
                    <div v-else-if="msg.messageType === 'custom' && msg.content?.product" class="product-card" @click="handleProductClick(msg.content.product)">
                        <div class="product-img">
                            <img :src="productImage(msg.content.product)" :alt="msg.content.product.productName || ''" />
                        </div>
                        <div class="product-info">
                            <div class="product-name">{{ msg.content.product.productName }}</div>
                            <div class="product-price">
                                <FormatPrice class="product-price" v-model="msg.content.product.productPrice" :showText="false"></FormatPrice>
                            </div>
                        </div>
                    </div>
                    <!-- 订单卡片 -->
                    <div v-else-if="msg.messageType === 'custom' && msg.content?.order" class="order-card" @click="handleOrderClick(msg.content.order)">
                        <div class="order-header">{{ msg.content.order.orderStatusName }}</div>
                        <div class="order-body">
                            <img :src="productImage(msg.content.order)" class="order-img" :alt="msg.content.order.productName || ''" />
                            <div class="order-info">
                                <div class="order-name">{{ msg.content.order.productName }}</div>
                                <div class="order-detail">
                                    {{ $t("共") }} {{ msg.content.order.productNum }} {{ $t("件商品") }}，{{ $t("合计") }}
                                    <FormatPrice v-model="msg.content.order.totalAmount" :showText="false"></FormatPrice>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- 用户消息 (右侧) -->
            <div v-else-if="msg.type === 1" class="msg-row right">
                <img :src="imageFormat(msg.user.avatar || userAvatar)" class="avatar" />
                <div class="msg-content-wrapper">
                    <div class="msg-header">
                        <span class="time" style="margin-right: 8px">{{ msg.sendTime }}</span>
                        <span class="name">{{ msg.user.nickname || msg.user.username }}</span>
                    </div>
                    <!-- 文本消息 -->
                    <div v-if="msg.messageType === 'text'" class="bubble blue">
                        <EmojiHtml v-if="hasEmoji(msg.content?.content)" :content="msg.content?.content" />
                        <span v-else v-text="msg.content?.content"></span>
                    </div>
                    <!-- 图片消息 -->
                    <div v-else-if="msg.messageType === 'image'" class="bubble blue image-bubble">
                        <img :src="imageFormat(msg.content?.pic)" class="msg-image" @click="handlePreviewImage(msg.content?.pic)" />
                    </div>
                    <!-- 商品卡片 -->
                    <div v-else-if="msg.messageType === 'custom' && msg.content?.product" class="product-card" @click="handleProductClick(msg.content.product)">
                        <div class="product-img">
                            <img :src="productImage(msg.content.product)" :alt="msg.content.product.productName || ''" />
                        </div>
                        <div class="product-info">
                            <div class="product-name">{{ msg.content.product.productName }}</div>
                            <div class="product-price">
                                <FormatPrice class="product-price" v-model="msg.content.product.productPrice" :showText="false"></FormatPrice>
                            </div>
                        </div>
                    </div>
                    <!-- 订单卡片 -->
                    <div v-else-if="msg.messageType === 'custom' && msg.content?.order" class="order-card" @click="handleOrderClick(msg.content.order)">
                        <div class="order-header">{{ msg.content.order.orderStatusName }}</div>
                        <div class="order-body">
                            <img :src="productImage(msg.content.order)" class="order-img" :alt="msg.content.order.productName || ''" />
                            <div class="order-info">
                                <div class="order-name">{{ msg.content.order.productName }}</div>
                                <div class="order-detail">
                                    {{ $t("共") }} {{ msg.content.order.productNum }} {{ $t("件商品") }}，{{ $t("合计") }}
                                    <FormatPrice v-model="msg.content.order.totalAmount" :showText="false"></FormatPrice>
                                </div>
                            </div>
                        </div>
                    </div>
                    <span class="read-status" :class="msg.isRead ? 'read' : 'unread'">
                              {{ msg.isRead ? $t("已读") : $t("未读") }}
                        </span>
                </div>
            </div>
        </div>

        <!-- 商品预览卡片 (从商品详情页跳转过来时显示) -->
        <div v-if="pendingProduct" class="pending-product-card">
            <div class="card-content">
                <div class="product-img">
                    <img :src="productImage(pendingProduct)" :alt="pendingProduct.productName || ''" />
                </div>
                <div class="product-info">
                    <div class="product-name">{{ pendingProduct.productName }}</div>
                    <div class="product-price">
                        <FormatPrice v-model="pendingProduct.productPrice" :showText="false"></FormatPrice>
                    </div>
                </div>
            </div>
            <div class="card-divider"></div>
            <div class="card-action" @click="handleSendProduct">{{ $t("发送宝贝") }}</div>
        </div>

        <!-- 订单预览卡片 (从订单详情页跳转过来时显示) -->
        <div v-if="pendingOrder" class="pending-order-card">
            <div class="card-header">
                <span class="order-sn">{{ pendingOrder.orderSn }}</span>
                <span class="order-status">{{ pendingOrder.orderStatusName }}</span>
            </div>
            <div class="card-content">
                <div class="order-img">
                    <img :src="productImage(pendingOrder)" :alt="pendingOrder.productName || ''" />
                </div>
                <div class="order-info">
                    <div class="order-name">{{ pendingOrder.productName }}</div>
                    <div class="order-sku" v-if="pendingOrder.skuData && pendingOrder.skuData.length">
                        <span v-for="(sku, idx) in pendingOrder.skuData" :key="idx">{{ sku.name }}：{{ sku.value }}</span>
                    </div>
                    <div class="order-bottom">
                        <div class="order-price">
                            <FormatPrice v-model="pendingOrder.totalAmount" :showText="false"></FormatPrice>
                        </div>
                        <div class="order-quantity" v-if="pendingOrder.quantity">
                            <span>x{{ pendingOrder.quantity }}</span>
                        </div>
                    </div>
                </div>
            </div>
            <div class="card-divider"></div>
            <div class="card-action" @click="handleSendOrder">{{ $t("发送订单") }}</div>
        </div>

        <!-- 空状态 -->
        <div v-if="messageList.length === 0 && !loading" class="empty-state">
            <span>{{ $t("暂无消息，发送消息开始聊天吧~") }}</span>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, nextTick, watch, onMounted } from "vue";
import EmojiHtml from "./EmojiHtml.vue";
import { hasEmoji } from "./emojiElements";
import { useUserStore } from "~/store/user";
import { useCommonStore } from "~/store/common";
import { imageFormat, productImage } from "~/utils/format";
const { t } = useI18n();

const props = defineProps<{
    messageList: any[];
    shopInfo: any;
    loading?: boolean;
    loadingMore?: boolean;
    loadEnd?: boolean;
    pendingProduct?: any;
    pendingOrder?: any;
}>();

const emit = defineEmits<{
    (e: "loadMore"): void;
    (e: "sendProduct", product: any): void;
    (e: "sendOrder", order: any): void;
    (e: "setRead"): void;
}>();

const userStore = useUserStore();
const commonStore = useCommonStore();
const messageContainerRef = ref<HTMLElement | null>(null);

// 用于记录加载更多前的滚动状态
const previousScrollHeight = ref(0);
const isLoadingMore = ref(false);

// 店铺头像
const shopAvatar = computed(() => {
    // 如果本地图片请使用 require 或 import 语法导入图片资源
    if (props.shopInfo?.shopLogo) {
        return props.shopInfo.shopLogo;
    }
    if (commonStore.icoImg) {
        return commonStore.icoImg;
    }
    // 本地图片请使用 import 方式，否则静态字符串路径不会被打包处理
    return new URL("/assets/images/im/self_shop.png", import.meta.url).href;
});

// 店铺名称
const shopName = computed(() => {
    return t(props.shopInfo?.shopTitle || commonStore.shopName || "平台自营");
});

// 用户头像
const userAvatar = computed(() => {
    if (userStore.userInfo?.avatar) {
        return userStore.userInfo.avatar;
    }
    if (commonStore.defaultAvatar) {
        return commonStore.defaultAvatar;
    }
    return new URL("/assets/images/im/self_shop.png", import.meta.url).href;
});

// 滚动到底部
const scrollToBottom = () => {
    nextTick(() => {
        if (messageContainerRef.value) {
            messageContainerRef.value.scrollTop = messageContainerRef.value.scrollHeight;
        }
    });
};

// 监听滚动事件
const handleScroll = () => {
    if (!messageContainerRef.value) return;
    const { scrollTop } = messageContainerRef.value;

    // 滚动到顶部加载更多
    if (scrollTop === 0 && !props.loadEnd && !props.loadingMore && props.messageList.length > 0) {
        // 记录加载前的滚动高度，用于加载完成后恢复位置
        previousScrollHeight.value = messageContainerRef.value.scrollHeight;
        isLoadingMore.value = true;
        emit("loadMore");
    }

    // 设置已读
    emit("setRead");
};

// 预览图片
const handlePreviewImage = (url: string) => {
    window.open(url, "_blank");
};

// 点击商品
const handleProductClick = (product: any) => {
    window.open(`/item/${product.productSn || product.productId}`, "_blank");
};

// 点击订单
const handleOrderClick = (order: any) => {
    window.open(`/member/order/info?id=${order.orderId}`, "_blank");
};

// 发送商品
const handleSendProduct = () => {
    if (props.pendingProduct) {
        emit("sendProduct", props.pendingProduct);
    }
};

// 发送订单
const handleSendOrder = () => {
    if (props.pendingOrder) {
        emit("sendOrder", props.pendingOrder);
    }
};

// 监听消息列表变化
watch(
    () => props.messageList.length,
    () => {
        if (isLoadingMore.value) {
            // 加载历史消息：保持用户当前查看的位置不变
            nextTick(() => {
                if (messageContainerRef.value) {
                    const newScrollHeight = messageContainerRef.value.scrollHeight;
                    const scrollDiff = newScrollHeight - previousScrollHeight.value;
                    messageContainerRef.value.scrollTop = scrollDiff;
                }
                isLoadingMore.value = false;
            });
        } else {
            // 新消息：滚动到底部
            scrollToBottom();
        }
    }
);

// 监听 loadingMore 状态变化，处理加载完成但没有新数据的情况
watch(
    () => props.loadingMore,
    (newVal, oldVal) => {
        if (oldVal === true && newVal === false) {
            // 加载完成，重置状态
            isLoadingMore.value = false;
        }
    }
);

onMounted(() => {
    if (messageContainerRef.value) {
        messageContainerRef.value.addEventListener("scroll", handleScroll);
    }
    scrollToBottom();
});

// 暴露方法
defineExpose({
    scrollToBottom
});
</script>

<style lang="less" scoped>
.message-list-container {
    flex: 1;
    overflow-y: auto;
    padding: 16px 12px;
    background-color: #f7f8fc;

    &::-webkit-scrollbar {
        width: 0px;
    }
    &::-webkit-scrollbar-thumb {
        background: #ccc;
        border-radius: 3px;
    }

    .loading-more,
    .load-end {
        text-align: center;
        padding: 10px 0;
        span {
            font-size: 12px;
            color: #888b94;
            //background-color: #e5e5e5;
            padding: 4px 12px;
            border-radius: 10px;
        }
    }

    .time-separator {
        text-align: center;
        margin: 15px 0;
        span {
            font-size: 12px;
            color: #999;
            background-color: #e5e5e5;
            padding: 4px 12px;
            border-radius: 10px;
        }
    }

    .system-msg {
        text-align: center;
        margin: 15px 0;
        span {
            font-size: 12px;
            color: #999;
        }
    }

    .msg-row {
        display: flex;
        margin-bottom: 20px;
        align-items: flex-start;

        .avatar {
            width: 32px;
            height: 32px;
            border-radius: 8px;
            flex-shrink: 0;
            object-fit: cover;
        }

        .msg-content-wrapper {
            display: flex;
            flex-direction: column;
            max-width: 60%;

            .msg-header {
                margin-bottom: 5px;
                font-size: 12px;
                color: #999;
                .time{
                    font-size: 12px;
                }

                .name {
                    color: #2a2a2a;
                    font-size: 12px;
                    //font-weight: 500;
                }
            }
            .read-status {
                margin-top: 5px;
                font-size: 12px;
                &.read {
                    color: #a1a6af;
                }
                &.unread {
                    color: #a1a6af;
                }
            }

            .bubble {
                border-radius: 8px;
                padding: 12px 9px;
                font-size: 14px;
                line-height: 1.2;
                position: relative;
                word-break: break-all;
                white-space: pre-wrap;

                &.image-bubble {
                    padding: 5px;
                    background-color: transparent !important;

                    .msg-image {
                        max-width: 200px;
                        max-height: 200px;
                        border-radius: 4px;
                        cursor: pointer;
                    }
                }
            }

            .product-card,
            .order-card {
                max-width: 200px;
                background-color: #fff;
                border-radius: 8px;
                padding: 12px;
                min-width: 240px;
                cursor: pointer;

                &:hover {
                    //box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
                }
            }

            .product-card {
                display: flex;

                .product-img {
                    width: 60px;
                    height: 60px;
                    margin-right: 10px;
                    flex-shrink: 0;

                    img {
                        width: 100%;
                        height: 100%;
                        object-fit: cover;
                        border-radius: 4px;
                    }
                }

                .product-info {
                    flex: 1;
                    display: flex;
                    flex-direction: column;
                    justify-content: space-between;

                    .product-name {
                        width: 100%;
                        font-size: 13px;
                        color: #333;
                        overflow: hidden;
                        text-overflow: ellipsis;
                        display: -webkit-box;
                        -webkit-line-clamp: 2;
                        line-clamp: 2;
                        -webkit-box-orient: vertical;
                    }

                    .product-price {
                        font-size: 14px;
                        color: #ff4d4f;
                        font-weight: 500;
                    }
                }
            }

            .order-card {
                .order-header {
                    font-size: 14px;
                    font-weight: 500;
                    color: #333;
                    margin-bottom: 10px;
                }

                .order-body {
                    display: flex;

                    .order-img {
                        width: 60px;
                        height: 60px;
                        object-fit: cover;
                        border-radius: 4px;
                        margin-right: 10px;
                    }

                    .order-info {
                        flex: 1;

                        .order-name {
                            width: 100%;
                            font-size: 13px;
                            color: #333;
                            overflow: hidden;
                            text-overflow: ellipsis;
                            display: -webkit-box;
                            -webkit-line-clamp: 2;
                            line-clamp: 2;
                            -webkit-box-orient: vertical;
                        }

                        .order-detail {
                            margin-top: 5px;
                            display: flex;
                            align-items: center;
                            font-size: 12px;
                            color: #999;
                        }
                    }
                }
            }
        }

        &.left {
            .avatar {
                margin-right: 10px;
            }
            .msg-content-wrapper {
                align-items: flex-start;
                .bubble.white {
                    background-color: #fff;
                    color: #333;
                    border-top-left-radius: 0;
                }
            }
        }

        &.right {
            flex-direction: row-reverse;
            .avatar {
                margin-left: 10px;
            }
            .msg-content-wrapper {
                align-items: flex-end;
                .msg-header {
                    text-align: right;
                }
                .bubble.blue {
                    background-color: #f8e3dd;
                    color: #661b08;
                    border-top-right-radius: 0;
                }
            }
        }
    }

    .pending-product-card,
    .pending-order-card {
        background-color: #fff;
        border-radius: 8px;
        margin: 20px 40px 0;
        overflow: hidden;
        box-shadow: 0 1px 4px rgba(0, 0, 0, 0.03);
        max-width: 600px;
        margin-left: auto;

        .card-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 12px 15px;
            border-bottom: 1px solid #f0f0f0;

            .order-sn {
                font-size: 13px;
                color: #333;
            }

            .order-status {
                font-size: 13px;
                color: #1890ff;
            }
        }

        .card-content {
            display: flex;
            padding: 15px;

            .product-img,
            .order-img {
                width: 80px;
                height: 80px;
                margin-right: 12px;
                flex-shrink: 0;

                img {
                    width: 100%;
                    height: 100%;
                    object-fit: cover;
                    border-radius: 4px;
                }
            }

            .product-info,
            .order-info {
                flex: 1;
                display: flex;
                flex-direction: column;
                justify-content: space-between;
                overflow: hidden;

                .product-name,
                .order-name {
                    font-size: 14px;
                    color: #333;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    display: -webkit-box;
                    -webkit-line-clamp: 2;
                    line-clamp: 2;
                    -webkit-box-orient: vertical;
                }

                .order-sku {
                    font-size: 12px;
                    color: #999;
                    margin-top: 4px;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    white-space: nowrap;

                    span {
                        margin-right: 8px;
                    }
                }

                .order-bottom {
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    margin-top: 4px;
                }

                .product-price,
                .order-price {
                    font-size: 16px;
                    color: #ff4d4f;
                    font-weight: 500;
                }

                .order-quantity {
                    font-size: 12px;
                    color: #999;
                }
            }
        }

        .card-divider {
            height: 1px;
            background-color: #f0f0f0;
        }

        .card-action {
            padding: 12px;
            text-align: center;
            color: #1890ff;
            font-size: 14px;
            font-weight: 500;
            cursor: pointer;

            &:hover {
                background-color: #f5f5f5;
            }
        }
    }

    .empty-state {
        display: flex;
        justify-content: center;
        align-items: center;
        height: 200px;
        color: #999;
        font-size: 14px;
    }
}
</style>
