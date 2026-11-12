<!--
  o2o 原版组件存档（v1）
  切回方法：删除当前 ChatList.vue，将本文件重命名为 ChatList.vue
-->
<template>
    <div class="chat-list-container">
        <div class="title-box">
            <el-icon color="#2A2A2A" size="20"><ChatDotRound /></el-icon>
            <div class="title-text">{{ $t("聊天列表") }}</div>
        </div>
        <div class="search-box" @click.stop>
            <div class="search-input-wrapper">
                <span class="search-icon">
                    <el-icon size="18"><Search /></el-icon>
                </span>
                <el-input
                    type="text"
                    :placeholder="$t('搜索最近联系人/聊天记录')"
                    class="search-input"
                    v-model="searchKeyword"
                    @keyup.enter="searchConversationList"
                    clearable
                    @clear="handleClearSearch"
                />
            </div>
        </div>
        <div class="search-list" v-if="showSearchList" ref="searchListRef" @click.stop>
            <div class="search-item">
                <div class="title">{{ $t("联系人") }}</div>
                <div class="list" v-if="searchData?.shopList && searchData.shopList.length > 0">
                    <div class="item" v-for="item in searchData.shopList" :key="item.shopId" @click="handleSearchItemClick(item.shopId)">
                        <div class="avatar">
                            <img :src="imageFormat(item.shopLogo)" alt="avatar" class="avatar" />
                        </div>
                        <div class="info">
                            <div class="name">{{ $t(item.shopTitle) }}</div>
                        </div>
                    </div>
                </div>
                <div class="empty-tip" v-else>{{ $t("暂无匹配的联系人") }}</div>
            </div>
            <div class="search-item">
                <div class="title">{{ $t("聊天记录") }}</div>
                <div class="list" v-if="searchData?.conversationList && searchData.conversationList.length > 0">
                    <div class="item" v-for="item in searchData.conversationList" :key="item.conversationId" @click="handleSearchItemClick(item.shopId)">
                        <div class="avatar">
                            <img v-if="item.shopId && item.shopId > 0" :src="imageFormat(item.shop?.shopLogo) || '/assets/images/im/default_shop.png'" alt="avatar" class="avatar" />
                            <img v-else :src="imageFormat(commonStore.icoImg) || '/assets/images/im/self_shop.png'" alt="avatar" class="avatar" />
                        </div>
                        <div class="info">
                            <div class="name">{{ $t(item.shopId && item.shopId > 0 ? item.shopTitle : "平台自营") }}</div>
                            <div class="content">
                                <div class="text">{{ item.content?.content?.replace(/<[^>]+>/g, "").replace(/\[.*?\]/g, $t("[表情]")) || "" }}</div>
                                <div class="time">{{ formatTime(item.addTime) }}</div>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="empty-tip" v-else>{{ $t("暂无匹配的聊天记录") }}</div>
            </div>
        </div>

        <div class="list-content">
            <template v-if="chatList.length > 0">
                <div
                    v-for="(item, index) in chatList"
                    :key="item.conversationId"
                    class="chat-item"
                    :class="{ active: currentConversationId === item.conversationId }"
                    @click="handleSelectConversation(item)"
                >
                    <div class="avatar-wrapper">
                        <img :src="imageFormat(getShopAvatar(item))" alt="avatar" class="avatar" />
                        <span v-if="item.unreadServantMessageCount > 0" class="badge">
                            {{ item.unreadServantMessageCount > 99 ? "99+" : item.unreadServantMessageCount }}
                        </span>
                    </div>
                    <div class="info-wrapper">
                        <div class="top-row">
                            <span class="name">{{ getShopName(item) }}</span>
                            <span class="time">{{ formatTime(item.lastUpdateTime) }}</span>
                        </div>
                        <div class="bottom-row">
                            <span class="last-msg">{{ getMessagePreview(item) }}</span>
                        </div>
                    </div>
                </div>
            </template>
            <template v-else-if="loading">
                <div class="loading-wrapper">
                    <span>{{ $t("加载中...") }}</span>
                </div>
            </template>
            <template v-else>
                <div class="empty-wrapper">
                    <span>{{ $t("暂无消息") }}</span>
                </div>
            </template>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ChatDotRound, Search } from "@element-plus/icons-vue";
import { ref, onMounted, onUnmounted } from "vue";
import { getConversation, setRead, frontSearch } from "~/api/im/im";
import { useCommonStore } from "~/store/common";
import { imageFormat } from "~/utils/format";
const { t } = useI18n();
const props = defineProps<{
    currentConversationId?: number;
}>();

const emit = defineEmits<{
    (e: "select", conversation: any): void;
}>();

const commonStore = useCommonStore();
const searchKeyword = ref("");
const loading = ref(false);
const chatList = ref<any[]>([]);
let fetchConversationSeq = 0;

// 获取店铺头像
const getShopAvatar = (item: any) => {
    if (item.shop?.shopLogo) {
        return item.shop.shopLogo;
    }
    if (commonStore.icoImg) {
        return commonStore.icoImg;
    }
    return new URL("/assets/images/im/self_shop.png", import.meta.url).href;
};

// 获取店铺名称
const getShopName = (item: any) => {
    return t(item?.shop?.shopTitle || commonStore.shopName || "平台自营");
};

// 格式化时间
const formatTime = (timeStr: string) => {
    if (!timeStr) return "";
    const date = new Date(timeStr);
    const now = new Date();
    if (isNaN(date.getTime())) return timeStr;

    const isToday = date.getDate() === now.getDate() && date.getMonth() === now.getMonth() && date.getFullYear() === now.getFullYear();

    if (isToday) {
        return `${date.getHours().toString().padStart(2, "0")}:${date.getMinutes().toString().padStart(2, "0")}`;
    } else {
        const yesterday = new Date(now);
        yesterday.setDate(yesterday.getDate() - 1);
        if (date.getDate() === yesterday.getDate() && date.getMonth() === yesterday.getMonth() && date.getFullYear() === yesterday.getFullYear()) {
            return `${t("昨天")}`;
        }
        return `${date.getMonth() + 1}-${date.getDate()}`;
    }
};

// 获取消息预览
const getMessagePreview = (item: any) => {
    if (!item.lastMessage || item.lastMessage.length === 0) return "";
    const msg = item.lastMessage[0];
    if (msg.messageType === "text") {
        return msg.content?.content?.replace(/<[^>]+>/g, "").replace(/\[.*?\]/g, t("[表情]")) || "";
    }
    return `[${msg.messageTypeText || t("消息")}]`;
};
const _setRead = async (shopId: number, servantId: number) => {
    try {
        await setRead({
            shopId: shopId,
            servantId: servantId
        });
    } catch (error: any) {
        console.error(error);
    }
};
// 选择会话
const handleSelectConversation = async (item: any) => {
    await _setRead(item.shopId, item.lastServantId);
    item.unreadServantMessageCount = 0;
    emit("select", item);
};

// 获取会话列表
const fetchConversationList = async () => {
    loading.value = true;
    const seq = ++fetchConversationSeq;
    try {
        const result = await getConversation({ page: 1, size: 50 });
        // 防止并发刷新导致“旧请求后返回覆盖新数据”
        if (seq !== fetchConversationSeq) return;

        chatList.value = result.records || [];
        // 当前会话已打开时，不应再显示未读（避免 setRead 生效延迟导致 badge 闪回）
        if (props.currentConversationId) {
            const active = chatList.value.find((x: any) => x?.conversationId === props.currentConversationId);
            if (active) active.unreadServantMessageCount = 0;
        }
    } catch (error) {
        console.error("获取会话列表失败:", error);
    } finally {
        // 只由“最后一次请求”负责关闭 loading，避免 loading 闪烁
        if (seq === fetchConversationSeq) loading.value = false;
    }
};

// 刷新列表
const refreshList = async () => {
    await fetchConversationList();
};

const showSearchList = ref(false);
const searchData = ref<any>(null);
const searchListRef = ref<HTMLElement | null>(null);

const searchConversationList = async () => {
    try {
        const result = await frontSearch({ keyword: searchKeyword.value });
        showSearchList.value = true;
        searchData.value = result || null;
    } catch (error) {
        console.error("筛选会话列表失败:", error);
    }
};

const handleClearSearch = () => {
    showSearchList.value = false;
    searchData.value = null;
};

// 处理搜索结果项点击
const handleSearchItemClick = async (shopId: number) => {
    const conversation = findConversationByShopId(shopId);
    if (conversation) {
        await handleSelectConversation(conversation);
    }
    // 关闭搜索列表
    showSearchList.value = false;
    searchKeyword.value = "";
};

// 点击外部区域关闭搜索列表
const handleClickOutside = (event: MouseEvent) => {
    if (searchListRef.value && !searchListRef.value.contains(event.target as Node)) {
        showSearchList.value = false;
    }
};

// 根据 shopId 查找会话
const findConversationByShopId = (shopId: number) => {
    return chatList.value.find((item) => item.shopId === shopId);
};

// 根据 shopId 选中会话
const selectByShopId = async (shopId: number) => {
    const conversation = findConversationByShopId(shopId);
    if (conversation) {
        await handleSelectConversation(conversation);
        return conversation;
    }
    return null;
};

// 等待加载完成并根据 shopId 选中会话
const waitAndSelectByShopId = async (shopId: number) => {
    // 如果正在加载，等待加载完成
    if (loading.value) {
        await new Promise<void>((resolve) => {
            const checkLoading = setInterval(() => {
                if (!loading.value) {
                    clearInterval(checkLoading);
                    resolve();
                }
            }, 50);
        });
    }
    return selectByShopId(shopId);
};

onMounted(() => {
    fetchConversationList();
    // 添加点击外部区域关闭搜索列表的监听
    document.addEventListener("click", handleClickOutside);
});

onUnmounted(() => {
    // 移除事件监听
    document.removeEventListener("click", handleClickOutside);
});

// 暴露方法给父组件
defineExpose({
    refreshList,
    findConversationByShopId,
    selectByShopId,
    waitAndSelectByShopId
});
</script>

<style lang="less" scoped>
.chat-list-container {
    width: 320px;
    background-color: #fff;
    height: 100%;
    border-right: 1px solid #f0f0f0;
    display: flex;
    flex-direction: column;
    position: relative;

    .title-box {
        padding: 12px 0 0 15px;
        display: flex;
        align-items: center;
        font-size: 14px;
        .title-text {
            margin-left: 5px;
            font-size: 20px;
            line-height: 20px;
            font-weight: 600;
            color: #1a1a1a;
        }
    }

    .search-box {
        padding: 12px;
        .search-input-wrapper {
            background-color: #f5f6fa;
            color: #c2c4cc;
            border-radius: 8px;
            padding: 8px;
            display: flex;
            align-items: center;
            box-sizing: border-box;
            height: 46px;
            .search-icon {
                margin-top: 4px;
                margin-right: 8px;
                color: #9a9a9a;
            }
            .search-input {
                flex: 1;
                :deep(.el-input__wrapper) {
                    box-shadow: none;
                    background: transparent;
                    padding: 0;
                }
                :deep(.el-input__inner) {
                    font-size: 14px;
                    color: #000;
                    &::placeholder {
                        color: #888b94;
                    }
                }
                :deep(.el-input__suffix) {
                    .el-input__clear {
                        font-size: 14px;
                    }
                }
            }
        }
    }

    .list-content {
        flex: 1;
        overflow-y: auto;
        background-color: #fff;
        // padding-top: 5px;
        margin-left: 12px;
        margin-right: 12px;

        &::-webkit-scrollbar {
            width: 4px;
        }
        &::-webkit-scrollbar-thumb {
            background: #e0e0e0;
            border-radius: 2px;
        }

        .loading-wrapper,
        .empty-wrapper {
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100px;
            color: #999;
            font-size: 14px;
        }

        .chat-item {
            padding: 10px;
            display: flex;
            cursor: pointer;
            transition: background-color 0.2s;
            border-radius: 8px;
            // margin-left: 5px;
            // margin-right: 5px;

            &:hover {
                background-color: #edf3fa;
                border-radius: 8px;
            }

            &.active {
                background-color: #fff5e8;
                border-radius: 8px;
            }

            .avatar-wrapper {
                margin-right: 12px;
                position: relative;

                .avatar {
                    width: 44px;
                    height: 44px;
                    border-radius: 8px;
                    object-fit: cover;
                }

                .badge {
                    position: absolute;
                    top: -5px;
                    right: -5px;
                    background-color: #ff4d4f;
                    color: #fff;
                    font-size: 10px;
                    padding: 2px 5px;
                    border-radius: 10px;
                    min-width: 16px;
                    text-align: center;
                }
            }

            .info-wrapper {
                flex: 1;
                overflow: hidden;
                display: flex;
                flex-direction: column;
                justify-content: center;

                .top-row {
                    display: flex;
                    justify-content: space-between;
                    margin-bottom: 4px;

                    .name {
                        font-size: 16px;
                        color: #1a1a1a;
                        //font-weight: 500;
                        overflow: hidden;
                        text-overflow: ellipsis;
                        white-space: nowrap;
                        max-width: 140px;
                    }

                    .time {
                        font-size: 12px;
                        color: #999;
                        flex-shrink: 0;
                    }
                }

                .bottom-row {
                    .last-msg {
                        font-size: 12px;
                        color: #999;
                        white-space: nowrap;
                        overflow: hidden;
                        text-overflow: ellipsis;
                        display: block;
                    }
                }
            }
        }
    }
}
.search-list {
    position: absolute;
    width: 300px;
    min-height: 50px;
    max-height: 70vh;
    z-index: 999;
    top: 100px;
    left: 30px;
    background-color: #fff;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    padding: 0 15px;
    border-radius: 10px;
    overflow-y: auto;
    .search-item {
        .title {
            padding: 10px 0;
            font-size: 14px;
            color: #1a1a1a;
        }
        .empty-tip {
            padding: 15px 0;
            font-size: 13px;
            color: #999;
            text-align: center;
            border-bottom: 1px solid #f0f0f0;
        }
        &:last-child .empty-tip {
            border-bottom: none;
        }
        .list {
            padding: 5px 0;
            border-bottom: 1px solid #f0f0f0;
            &::last-child {
                border-bottom: none;
            }
            .item {
                display: flex;
                align-items: center;
                padding: 10px 0;
                .avatar {
                    width: 40px;
                    height: 40px;
                    border-radius: 50%;
                    margin-right: 10px;
                    img {
                        width: 100%;
                        height: 100%;
                        object-fit: cover;
                    }
                }
                .info {
                    flex: 1;
                    .name {
                        font-size: 14px;
                        color: #333;
                        overflow: hidden;
                        text-overflow: ellipsis;
                        white-space: nowrap;
                        max-width: 200px;
                        display: block;
                    }
                    .content {
                        width: 100%;
                        margin-top: 12px;
                        display: flex;
                        align-items: center;
                        justify-content: space-between;
                        .text {
                            font-size: 12px;
                            color: #999;
                            overflow: hidden;
                            text-overflow: ellipsis;
                            white-space: nowrap;
                            max-width: 200px;
                            display: block;
                        }
                        .time {
                            font-size: 12px;
                        }
                    }
                }
            }
        }
    }
}
</style>
