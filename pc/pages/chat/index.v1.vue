<!--
  o2o 原版页面存档（v1）
  切回方法：删除当前 index.vue，将本文件重命名为 index.vue
-->
<template>
    <div class="im-page" id="im-body">
        <div class="im-container">
            <!-- 左侧: 会话列表 -->
            <ChatList ref="chatListRef" :current-conversation-id="currentConversationId" @select="handleSelectConversation" />

            <!-- 中间: 聊天区域 -->
            <div class="chat-main" v-if="currentConversationId">
                <div class="chat-header">
                    <span class="title">{{ currentShopName }}</span>
                    <span v-if="!currentShopInfo?.shopId || currentShopInfo?.shopId === 0" class="official-tag">{{ $t("平台自营") }}</span>
                </div>
                <ChatMessageList
                    ref="messageListRef"
                    :message-list="messageList"
                    :shop-info="currentShopInfo"
                    :loading="loading"
                    :loading-more="loadingMore"
                    :load-end="loadEnd"
                    :pending-product="pendingProduct"
                    :pending-order="pendingOrder"
                    @load-more="handleLoadMore"
                    @send-product="handleSendProduct"
                    @send-order="handleSendOrderFromMessageList"
                    @set-read="handleSetRead"
                />
                <ChatInput @send="handleSendMessage" />
            </div>

            <!-- 右侧: 信息面板 -->
            <template v-if="currentConversationId">
                <ChatRight :shop-info="currentShopInfo" :product-info="productInfo" :order-info="orderInfo" @send-order="handleSendOrder" />
            </template>

            <template v-if="!currentConversationId">
                <div class="im-layout">
                    <div class="im-default-box">
                        <div class="im-default">
                            <div class="im-default-icon">
                                <el-icon color="#d5d5d5" size="50">
                                    <ChatDotSquare />
                                </el-icon>
                            </div>
                            <div class="im-default-text"><span>{{ $t("没有选中会话哦") }}</span></div>
                        </div>
                    </div>
                </div>
            </template>
        </div>

        <!-- 提示音 -->
        <audio ref="audioRef" src="/assets/im/notify.wav" preload="auto"></audio>
    </div>
</template>

<script setup lang="ts">
/**
 * 即时通讯(IM)聊天页面
 * - 左侧显示会话列表，支持按店铺分组
 * - 中间为聊天消息区域，支持文本、图片、商品卡片、订单卡片发送
 * - 右侧显示店铺/商品/订单信息面板
 * - 使用 WebSocket 实现实时消息收发和已读回执
 * - 支持从商品页/订单页跳转并自动发送商品或订单信息
 */
import { ChatDotSquare } from "@element-plus/icons-vue";
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
import ChatList from "./src/ChatList.vue";
import ChatMessageList from "./src/ChatMessageList.vue";
import ChatInput from "./src/ChatInput.vue";
import ChatRight from "./src/ChatRight.vue";
import { getConversationMessage, sendMessage, setRead, getShopDetail } from "~/api/im/im";
import { getProductDetailIm } from "~/api/product/product";
import { getImOrder } from "~/api/user/order";
import WebSocketClient from "~/utils/websocket";
import { useUserStore } from "~/store/user";
import { useCommonStore } from "~/store/common";
const { t } = useI18n();

// 定义页面 meta
definePageMeta({
    middleware: ["auth"]
});

const route = useRoute();
const userStore = useUserStore();
const commonStore = useCommonStore();

// Refs
const chatListRef = ref<InstanceType<typeof ChatList> | null>(null);
const messageListRef = ref<InstanceType<typeof ChatMessageList> | null>(null);
const audioRef = ref<HTMLAudioElement | null>(null);

// 状态
const loading = ref(false);
const loadingMore = ref(false);
const loadEnd = ref(false);
const messageList = ref<any[]>([]);
const currentConversationId = ref<number | undefined>();
const currentShopInfo = ref<any>(null);
const productInfo = ref<any>(null);
const orderInfoData = ref<any>(null);
const orderInfoShopId = ref<number | null>(null);
const pendingProduct = ref<any>(null);
const pendingOrderData = ref<any>(null);
const pendingOrderShopId = ref<number | null>(null);

// 只在对应店铺的会话中显示右侧订单信息
const orderInfo = computed(() => {
    if (!orderInfoData.value) return null;
    if (filterParams.value.shopId === orderInfoShopId.value) {
        return orderInfoData.value;
    }
    return null;
});

// 只在对应店铺的会话中显示订单预览卡片
const pendingOrder = computed(() => {
    if (!pendingOrderData.value) return null;
    // 检查当前会话的 shopId 是否与订单的 shopId 匹配
    if (filterParams.value.shopId === pendingOrderShopId.value) {
        return pendingOrderData.value;
    }
    return null;
});

// WebSocket 实例
let ws: WebSocketClient | null = null;

// 请求参数
const filterParams = ref({
    firstId: -1,
    sortOrder: "desc",
    shopId: 0,
    conversationId: undefined as number | undefined
});

// 当前店铺名称
const currentShopName = computed(() => {
    return t(currentShopInfo.value?.shopTitle || commonStore.shopName || "在线客服");
});

// 获取 WebSocket URL
const getSocketUrl = () => {
    const { VITE_SOCKET_URL } = import.meta.env;
    if (VITE_SOCKET_URL) {
        return VITE_SOCKET_URL;
    }
    // 根据当前页面协议生成 WebSocket 地址
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${window.location.host}/ws`;
};

// 初始化 WebSocket
const initWebSocket = () => {
    if (!userStore.token) return;

    const socketUrl = `${getSocketUrl()}?token=${userStore.token}&platform=pc`;
    ws = new WebSocketClient(socketUrl);

    ws.onopen(() => {
        console.log("WebSocket 连接成功");
    });

    ws.onmessage((event: MessageEvent) => {
        console.log("收到消息", event.data);
        try {
            const data = JSON.parse(event.data);
            console.log("解析后的数据", data);
            if (data.type === "message") {
                // 收到新消息
                if (data.data && data.data.length > 0) {
                    handleNewMessage(data.data[0]);
                }
            } else if (data.type === "read") {
                // 消息已读回执
                messageList.value.forEach((item: any) => {
                    if (item.type === 1) {
                        item.isRead = 1;
                    }
                });
            }
        } catch (error) {
            console.error("WebSocket 消息解析失败:", error);
        }
    });

    ws.onclose(() => {
        console.log("WebSocket 连接关闭");
    });

    ws.onerror(() => {
        console.error("WebSocket 连接错误");
    });

    ws.connect();
};

// 处理新消息
const handleNewMessage = (data: any) => {
    // 只处理当前会话的消息
    if (data.shopId === filterParams.value.shopId) {
        // 检查消息是否已存在
        const exists = messageList.value.some((item) => item.id === data.id);
        if (!exists) {
            messageList.value.push(data);
            // 播放提示音
            playNotifySound();
            // 滚动到底部
            nextTick(() => {
                messageListRef.value?.scrollToBottom();
            });
        }
    }
    // 刷新会话列表
    chatListRef.value?.refreshList();
};

// 播放提示音
const playNotifySound = () => {
    if (audioRef.value) {
        audioRef.value.currentTime = 0;
        audioRef.value.play().catch(() => {
            // 忽略自动播放错误
        });
    }
};

// 获取消息列表
const fetchMessageList = async (isLoadMore = false) => {
    if (isLoadMore) {
        loadingMore.value = true;
    } else {
        loading.value = true;
        loadEnd.value = false;
        filterParams.value.firstId = -1;
    }

    try {
        const result = await getConversationMessage({
            shopId: filterParams.value.shopId,
            conversationId: filterParams.value.conversationId,
            firstId: filterParams.value.firstId,
            sortOrder: filterParams.value.sortOrder
        });

        if (result.records.length === 0) {
            loadEnd.value = true;
            return;
        }

        if (isLoadMore) {
            messageList.value = [...result.records, ...messageList.value];
        } else {
            messageList.value = result.records;
            nextTick(() => {
                messageListRef.value?.scrollToBottom();
            });
        }
    } catch (error) {
        console.error("获取消息列表失败:", error);
    } finally {
        loading.value = false;
        loadingMore.value = false;
    }
};

// 加载更多消息
const handleLoadMore = () => {
    if (loadEnd.value || loadingMore.value || messageList.value.length === 0) return;
    filterParams.value.firstId = messageList.value[0].id;
    fetchMessageList(true);
};

// 获取店铺详情
const fetchShopDetail = async (shopId: number) => {
    try {
        const result = await getShopDetail(shopId);
        currentShopInfo.value = result;
    } catch (error) {
        console.error("获取店铺详情失败:", error);
    }
};

// 获取商品详情
const fetchProductDetail = async (productId: number | string) => {
    try {
        const result = await getProductDetailIm(productId);
        productInfo.value = {
            productId: result.item.productId,
            picUrl: result.item.picUrl,
            productName: result.item.productName,
            productPrice: result.item.productPrice,
            productSn: result.item.productSn
        };
        // 设置待发送的商品
        pendingProduct.value = productInfo.value;
    } catch (error) {
        console.error("获取商品详情失败:", error);
    }
};

// 获取订单详情
const fetchOrderDetail = async (orderId: number, shopId?: number) => {
    try {
        const result = await getImOrder({ id: orderId });
        const orderData = {
            orderId: result.orderId,
            orderSn: result.orderSn,
            picUrl: result.items?.[0]?.picThumb,
            productName: result.items?.[0]?.productName,
            productType: result.items?.[0]?.productType,
            productNum: result.items?.length || 0,
            totalAmount: result.totalAmount,
            orderStatusName: result.orderStatusName,
            addTime: result.addTime,
            quantity: result.items?.[0]?.quantity,
            skuData: result.items?.[0]?.skuData,
            shopId: result.shopId
        };
        const targetShopId = shopId !== undefined ? shopId : (result.shopId ?? filterParams.value.shopId);

        // 设置右侧订单信息
        orderInfoData.value = orderData;
        orderInfoShopId.value = targetShopId;

        // 设置待发送的订单和对应的店铺 ID
        pendingOrderData.value = orderData;
        pendingOrderShopId.value = targetShopId;
    } catch (error) {
        console.error("获取订单详情失败:", error);
    }
};


// 选择会话
const handleSelectConversation = async (conversation: any) => {
    currentConversationId.value = conversation.conversationId;
    filterParams.value.shopId = conversation.shopId;
    filterParams.value.conversationId = conversation.conversationId;

    // 重置状态
    messageList.value = [];
    loadEnd.value = false;
    productInfo.value = null;
    pendingProduct.value = null;
    // 注意：orderInfo 和 pendingOrder 通过 computed 根据 shopId 自动判断是否显示

    // 获取店铺详情（shopId 可能为 0，表示平台客服）
    if (conversation.shopId !== undefined && conversation.shopId !== null) {
        await fetchShopDetail(conversation.shopId);
    } else {
        currentShopInfo.value = conversation.shop || null;
    }

    // 获取消息列表
    await fetchMessageList();
};

// 发送消息
const handleSendMessage = async (data: { type: string; content?: string; pic?: string }) => {
    try {
        // 如果是新会话（conversationId 为 -1 或 undefined），不传 conversationId
        const isNewConversation = !filterParams.value.conversationId || currentConversationId.value === -1;

        const result = await sendMessage({
            content: {
                messageType: data.type,
                content: data.content || "",
                pic: data.pic
            },
            conversationId: isNewConversation ? undefined : filterParams.value.conversationId,
            shopId: filterParams.value.shopId
        });

        // 如果是新会话，更新 conversationId
        if (isNewConversation && result.conversationId) {
            filterParams.value.conversationId = result.conversationId;
            currentConversationId.value = result.conversationId;
        }

        // 添加到消息列表
        messageList.value.push(result);
        nextTick(() => {
            messageListRef.value?.scrollToBottom();
        });

        // 刷新会话列表
        chatListRef.value?.refreshList();
    } catch (error: any) {
        ElMessage.error(error.message || t("发送消息失败"));
    }
};

// 发送商品
const handleSendProduct = async (product: any) => {
    // shopId 可能为 0（平台自营），所以判断是否为 undefined 或 null
    if (filterParams.value.shopId === undefined || filterParams.value.shopId === null) return;

    try {
        // 如果是新会话（conversationId 为 -1 或 undefined），不传 conversationId
        const isNewConversation = !filterParams.value.conversationId || currentConversationId.value === -1;

        const result = await sendMessage({
            content: {
                messageType: "custom",
                product: product
            },
            conversationId: isNewConversation ? undefined : filterParams.value.conversationId,
            shopId: filterParams.value.shopId,
            productId: product.productId
        });

        // 如果是新会话，更新 conversationId
        if (isNewConversation && result.conversationId) {
            filterParams.value.conversationId = result.conversationId;
            currentConversationId.value = result.conversationId;
        }

        // 添加到消息列表
        messageList.value.push(result);
        // 清除待发送商品
        pendingProduct.value = null;

        nextTick(() => {
            messageListRef.value?.scrollToBottom();
        });

        // 刷新会话列表
        chatListRef.value?.refreshList();
    } catch (error: any) {
        console.error("发送商品失败:", error);
        ElMessage.error(error.message || t("发送商品失败"));
    }
};

// 发送订单
const handleSendOrder = async (order: any) => {
    // shopId 可能为 0（平台自营），所以判断是否为 undefined 或 null
    if (filterParams.value.shopId === undefined || filterParams.value.shopId === null) return;

    try {
        // 如果是新会话（conversationId 为 -1 或 undefined），不传 conversationId
        const isNewConversation = !filterParams.value.conversationId || currentConversationId.value === -1;

        const result = await sendMessage({
            content: {
                messageType: "custom",
                order: order
            },
            conversationId: isNewConversation ? undefined : filterParams.value.conversationId,
            shopId: filterParams.value.shopId,
            orderId: order.orderId
        });

        // 如果是新会话，更新 conversationId
        if (isNewConversation && result.conversationId) {
            filterParams.value.conversationId = result.conversationId;
            currentConversationId.value = result.conversationId;
        }

        // 添加到消息列表
        messageList.value.push(result);
        // 清除待发送订单
        pendingOrderData.value = null;
        pendingOrderShopId.value = null;

        nextTick(() => {
            messageListRef.value?.scrollToBottom();
        });

        // 刷新会话列表
        chatListRef.value?.refreshList();
    } catch (error: any) {
        console.error("发送订单失败:", error);
        ElMessage.error(error.message || t("发送订单失败"));
    }
};

// 从消息列表发送订单（预览卡片）
const handleSendOrderFromMessageList = async (order: any) => {
    await handleSendOrder(order);
};

// 设置已读
const triggerSetReadStatus = ref(false);
const handleSetRead = async () => {
    if (
        messageList.value.length > 0 &&
        messageList.value[messageList.value.length - 1].isRead === 0 &&
        messageList.value[messageList.value.length - 1].type === 2 &&
        !triggerSetReadStatus.value
    ) {
        try {
            triggerSetReadStatus.value = true;
            await setRead({
                shopId: filterParams.value.shopId,
                servantId: messageList.value[messageList.value.length - 1].servantId
            });
        } catch (error) {
            console.error("设置已读失败:", error);
        } finally {
            triggerSetReadStatus.value = false;
        }
    }
};

// 初始化
onMounted(async () => {
    // 从 URL 获取参数
    const { shopId, productId, orderId, conversationId } = route.query;

    // 判断是否有 shopId 参数（shopId 可能为 0，表示平台自营）
    const hasShopId = shopId !== undefined && shopId !== null && shopId !== '';

    // 如果有 shopId，尝试在会话列表中查找并选中对应会话
    if (hasShopId) {
        const shopIdNum = Number(shopId);
        // 等待会话列表加载完成后，查找并选中对应的会话
        nextTick(async () => {
            const conversation = await chatListRef.value?.waitAndSelectByShopId(shopIdNum);
            if (conversation) {
                // 找到会话，已自动选中，不需要额外处理
                // 如果有 productId，获取商品详情
                if (productId && Number(productId)) {
                    await fetchProductDetail(Number(productId));
                }
                // 如果有 orderId，获取订单详情
                if (orderId && Number(orderId)) {
                    await fetchOrderDetail(Number(orderId), shopIdNum);
                }
            } else {
                // 会话列表中未找到该 shopId
                filterParams.value.shopId = shopIdNum;
                await fetchShopDetail(shopIdNum);

                // 如果有 conversationId，使用该 conversationId
                if (conversationId && Number(conversationId)) {
                    filterParams.value.conversationId = Number(conversationId);
                    currentConversationId.value = Number(conversationId);

                    if (productId && Number(productId)) {
                        await fetchProductDetail(Number(productId));
                    }

                    if (orderId && Number(orderId)) {
                        await fetchOrderDetail(Number(orderId), shopIdNum);
                    }

                    // 获取消息列表
                    await fetchMessageList();
                } else if (productId || orderId) {
                    // 如果有 productId 或 orderId，创建新会话模式
                    // 设置 currentConversationId 为 -1 表示新会话
                    currentConversationId.value = -1;

                    // 获取商品/订单详情并自动发送
                    if (productId && Number(productId)) {
                        await fetchProductDetail(Number(productId));
                        // 自动发送商品信息创建新会话
                        if (productInfo.value) {
                            await handleSendProduct(productInfo.value);
                        }
                    }

                    if (orderId && Number(orderId)) {
                        await fetchOrderDetail(Number(orderId), shopIdNum);
                        // 自动发送订单信息创建新会话
                        if (orderInfo.value) {
                            await handleSendOrder(orderInfo.value);
                        }
                    }
                } else {
                    // 没有 conversationId、productId、orderId，但有 shopId（包括 shopId=0 平台客服）
                    // 创建新会话模式
                    currentConversationId.value = -1;
                }
            }
        });
    } else {
        // 没有 shopId，处理其他参数
        if (conversationId && Number(conversationId)) {
            filterParams.value.conversationId = Number(conversationId);
            currentConversationId.value = Number(conversationId);
        }

        if (productId && Number(productId)) {
            await fetchProductDetail(Number(productId));
        }

        if (orderId && Number(orderId)) {
            await fetchOrderDetail(Number(orderId));
        }
    }

    // 初始化 WebSocket
    initWebSocket();
});

// 清理
onUnmounted(() => {
    if (ws) {
        ws.close();
        ws = null;
    }
});
</script>

<style lang="less" scoped>
.im-page {
    height: 100vh;
    background-color: #f0f2f5;
    display: flex;
    justify-content: center;
    align-items: center;
    box-sizing: border-box;
    position: relative;

    .im-container {
        width: 100vw;
        height: 100%;
        // max-height: 800px;
        background-color: #f1f2f5;
        display: flex;
        box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.1);
        border-radius: 8px;
        overflow: hidden;

        .chat-main {
            flex: 1;
            display: flex;
            flex-direction: column;
            border-right: 1px solid #f0f0f0;
            position: relative;
            min-width: 0;

            .chat-header {
                height: 44px;
                border-bottom: 1px solid #e6e9ed;
                display: flex;
                align-items: center;
                padding: 0 20px;
                background-color: #fff;
                flex-shrink: 0;

                .title {
                    font-size: 20px;
                    line-height: 20px;
                    font-weight: 600;
                    color: #1a1a1a;
                }

                .official-tag {
                    margin-left: 8px;
                    font-size: 11px;
                    color: #ff4d4f;
                    border: 1px solid #ff4d4f;
                    padding: 2px 6px;
                    border-radius: 3px;
                    background-color: rgba(255, 77, 79, 0.05);
                }
            }
        }
    }
}

.im-layout {
    display: flex;
    flex: 1;
    height: 100%;
    // background-color: #f1f2f5;
}

.im-default-box {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;

    .im-default {
        text-align: center;

        .im-default-icon {
            width: 100px;
            height: 100px;
            background: #f8f8f8;
            border-radius: 50%;
            display: flex;
            justify-content: center;
            align-items: center;
        }

        .im-default-text {
            color: #999;
            margin-top: 10px;
        }
    }
}
</style>
