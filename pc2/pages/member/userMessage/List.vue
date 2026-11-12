<template>
    <CommonHeader title="站内信"></CommonHeader>
    <CommonPageHeaderNew></CommonPageHeaderNew>
    <CommonPageContainer flex class="message_center">
        <div class="mc_side_bar">
            <div class="nav_card">
                <div class="nav_group__title">{{ $t("消息中心") }}</div>
                <div class="nav_item active">
                    <span class="nav_item__name">{{ $t("账户信息") }}</span>
                </div>
            </div>
        </div>
        <div class="msg_list">
            <div class="flex justify-end msg_list__toolbar">
                <el-button @click="unreadReadFn">{{ $t("只看未读") }}</el-button>
                <el-button @click="addMessageAllReadFn">{{ $t("全部标记为已读") }}</el-button>
            </div>
            <div class="list">
                <template v-if="total > 0">
                    <div class="item" :class="item.isRead ? 'is-read' : 'is-unread'" v-for="item in filterState" @click="readItem(item)">
                        <div class="msg_card">
                            <div class="msg_card__head">
                                <div class="msg_card__source">
                                    <span class="msg_card__icon"><i class="iconfont-pc icon-xiaoxi"></i></span>
                                    <span class="msg_card__label">{{ $t("系统通知") }}</span>
                                    <span class="msg_card__dot" v-if="!item.isRead"></span>
                                </div>
                                <div class="msg_card__date">{{ item.addTimeDate }}</div>
                            </div>
                            <h3 class="msg_card__title">{{ item.title }}</h3>
                            <p class="msg_card__txt">{{ item.content }}</p>
                            <i class="iconfont-pc icon-changyonggoupiaorenshanchu msg_card__del" @click.stop="delMessageFn({ id: item.messageId })"></i>
                        </div>
                    </div>
                </template>
                <template v-else>
                    <el-empty :description="$t('暂无消息')" />
                </template>
                <div class="el-page flex justify-end" v-if="filterState.length > 0 && !loading">
                    <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
                </div>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * @description 站内消息列表页面
 * - 展示系统发送给用户的站内消息
 * - 支持筛选全部/未读消息
 * - 支持单条删除、标记已读、全部已读操作
 * - 支持分页浏览消息列表
 */
import { reactive, ref, onMounted } from "vue";
import { Pagination } from "~/components/list";
import { getMessageList, delMessage, addMessageRead, addMessageAllRead } from "~/api/user/userMessage";
import type { UserMsgFilterParams, UserMsgFilterState } from "~/types/user/userMessage.d";
import { urlFormat } from "~/utils/format";
// 用于存储查询条件
const filterState = ref<UserMsgFilterState[]>([]);
// 用于存储加载状态
const loading = ref<boolean>(true);
// 用于存储订单总数
const total = ref<number>(0);
// 用于存储查询参数
const filterParams = reactive<UserMsgFilterParams>({
    //初使化用于查询的参数
    page: 1,
    unread: 0
});
definePageMeta({
    middleware: "auth"
});
const loadFilter = async () => {
    // 设置加载状态
    // filterState.value = [];
    loading.value = true;
    try {
        const result = await getMessageList({ ...filterParams });
        filterState.value = result.records;
        total.value = result.total;
    } catch (error: any) {
        // 处理错误信息
        message.error(error.message);
    } finally {
        // 设置加载状态
        loading.value = false;
    }
};
const unreadReadFn = async () => {
    filterParams.unread = 1;
    try {
        await loadFilter();
    } catch (error: any) {
        // 处理错误信息
        message.error(error.message);
    }
};
const delMessageFn = async (data: Object) => {
    try {
        await delMessage(data);
        await loadFilter();
    } catch (error: any) {
        // 处理错误信息
        message.error(error.message);
    }
};
const readItem = async (data: UserMsgFilterState) => {
    try {
        if (data.isRead !== 1) {
            await addMessageRead(data.messageId);
            data.isRead = 1;
        }
        let temp: any = JSON.parse(data.link);
        navigateTo(urlFormat(temp.data), { open: { target: "_blank" } });
    } catch (error: any) {
        // 处理错误信息
        // message.error(error.message);
    }
};
const addMessageAllReadFn = async () => {
    try {
        await addMessageAllRead();
        filterParams.unread = 0;
        await loadFilter();
    } catch (error: any) {
        // 处理错误信息
        message.error(error.message);
    }
};
onMounted(() => {
    loadFilter();
});
</script>
<style lang="less" scoped>
.message_center {
    align-items: flex-start;
    padding-top: 20px;
    padding-bottom: 50px;
    .mc_side_bar {
        width: 216px;
        flex-shrink: 0;
        margin-right: 16px;
        background-color: #fff;
        border-radius: 8px;
        padding: 12px 0;
        .nav_card {
            .nav_group__title {
                padding: 8px 24px;
                font-size: 16px;
                color: #333;
                font-weight: 600;
            }
            .nav_item {
                height: 40px;
                margin: 4px 0;
                padding: 0 24px 0 36px;
                display: flex;
                align-items: center;
                color: #666;
                font-size: 15px;
                cursor: pointer;
                transition: all 0.2s;
                .nav_item__name {
                    flex: 1;
                }
                &:hover:not(.active) {
                    background-color: #fafafa;
                    color: var(--general);
                }
                &.active {
                    color: var(--general);
                    font-weight: 500;
                    background-color: color-mix(in srgb, var(--general) 8%, transparent);
                }
            }
        }
    }
    .msg_list {
        flex: 1;
        min-width: 0;
        background-color: #fff;
        border-radius: 8px;
        padding: 20px;
        .msg_list__toolbar {
            gap: 10px;
        }
        .list {
            margin-top: 20px;
            .item {
                margin-bottom: 14px;
                .msg_card {
                    position: relative;
                    cursor: pointer;
                    padding: 16px 20px;
                    border: 1px solid #eee;
                    border-radius: 8px;
                    background-color: #fff;
                    transition: box-shadow 0.2s;
                    &:hover {
                        box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
                        .msg_card__del {
                            opacity: 1;
                        }
                    }
                    .msg_card__head {
                        display: flex;
                        align-items: center;
                        justify-content: space-between;
                    }
                    .msg_card__source {
                        display: flex;
                        align-items: center;
                        .msg_card__icon {
                            width: 26px;
                            height: 26px;
                            border-radius: 8px;
                            background-color: var(--general);
                            display: flex;
                            align-items: center;
                            justify-content: center;
                            margin-right: 8px;
                            i {
                                color: #fff;
                                font-size: 15px;
                            }
                        }
                        .msg_card__label {
                            font-size: 14px;
                            color: #333;
                            font-weight: 500;
                        }
                        .msg_card__dot {
                            width: 7px;
                            height: 7px;
                            border-radius: 50%;
                            background-color: #e4393c;
                            margin-left: 8px;
                        }
                    }
                    .msg_card__date {
                        font-size: 13px;
                        color: #999;
                    }
                    .msg_card__title {
                        margin-top: 12px;
                        font-size: 16px;
                        font-weight: bold;
                        color: #222;
                        line-height: 1.5;
                    }
                    .msg_card__txt {
                        margin-top: 8px;
                        color: #666;
                        font-size: 14px;
                        line-height: 22px;
                    }
                    .msg_card__del {
                        position: absolute;
                        right: 20px;
                        bottom: 16px;
                        color: #bbb;
                        opacity: 0;
                        cursor: pointer;
                        transition: color 0.2s;
                        &:hover {
                            color: var(--general);
                        }
                    }
                }
                &.is-unread {
                    .msg_card {
                        background-color: #fafbff;
                        border-color: #e6ebff;
                    }
                }
                &.is-read {
                    .msg_card__label {
                        font-weight: 400;
                        color: #666;
                    }
                }
            }
            .el-page {
                margin-top: 20px;
            }
        }
    }
}
</style>
