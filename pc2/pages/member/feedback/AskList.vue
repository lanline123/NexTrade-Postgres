<template>
    <!-- <CommonPageHeaderNew></CommonPageHeaderNew> -->
    <PageHeaderNew :isWhite="true"></PageHeaderNew>

    <CommonHeader title="订单咨询"></CommonHeader>
    <TopJumpBar tltle="订单咨询"></TopJumpBar>

    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="title-or-tabs">
                <div class="tag-and-input">
                    <div class="tig-tabs">
                        <MemberTopMenu :menuList="menuList"></MemberTopMenu>
                    </div>
                </div>
            </div>
            <div class="message-content">
                <Info v-if="router.currentRoute.value.query.orderSn" type="consult" @loadFilter="loadFilter">
                    <el-button type="primary">{{ $t("添加新咨询") }}</el-button>
                </Info>
                <div v-loading="loading" class="card-list">
                    <template v-if="total > 0">
                        <div v-for="item in filterState" class="card">
                            <div class="info">
                                <i class="iconfont-pc icon-pinlun mr10"></i>
                                {{ item.typeName }}：<span class="font-color">{{ item.title }}</span>
                                <span style="margin-left: 20px">{{ $t("订单编号") }}：{{ item.orderSn }}</span>
                                <div class="time">{{ item.addTime }}</div>
                            </div>
                            <div class="text">{{ item.content }}</div>
                            <div v-if="item.reply" class="reply">
                                <span>{{ $t("客服回复") }}：{{ item.reply?.addTime }}</span>
                                <br />
                                <span class="ml10">{{ item.reply?.content }}</span>
                            </div>
                        </div>
                        <div class="el-page">
                            <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
                        </div>
                    </template>
                    <template v-else>
                        <div class="no-result">
                            <span v-if="!loading">{{ $t("您还没有留言过") }}</span>
                        </div>
                    </template>
                </div>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * @description 我的咨询列表页面
 * - 展示用户提交的商品或服务咨询记录
 * - 显示咨询内容和商家回复
 * - 支持提交新的咨询问题
 */
import { reactive, ref } from "vue";
import { useRouter } from "vue-router";
import type { FeedbackFilterParams, FeedbackFilterState } from "~/types/user/feedback";
import { getFeedbackList } from "~/api/user/feedback";
import { Pagination } from "~/components/list";
import Info from "~/pages/member/feedback/Info.vue";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from '~/components/common/TopJumpBar.vue'
definePageMeta({
    middleware: "auth"
});

const filterState = ref(<FeedbackFilterState[]>[]);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<FeedbackFilterParams>({
    //初使化用于查询的参数
    page: 1,
    isOrder: 2
});

const loadFilter = async () => {
    loading.value = true;
    try {
        if (router.currentRoute.value.query.orderId) {
            filterParams.orderId = Number(router.currentRoute.value.query.orderId);
        }
        const result = await getFeedbackList({ ...filterParams });
        filterState.value = result.records;
        total.value = result.total;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
onMounted(() => {
    loadFilter();
});

const router = useRouter();
const splitStr = (str: string) => {
    return str.split("?")[0];
};
const menuList = reactive<any>([
    { value: "我的留言", url: "/member/feedback/list", size: 0 },
    { value: "订单咨询", url: "/member/feedback/askList", size: 0 }
]);
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.message-content {
    background: #fff;
    border: 0;

    .card-list {
        margin-top: 20px;
        display: flex;
        flex-direction: column;
        font-size: 14px;

        .card {
            margin-bottom: 15px;

            .info {
                // background: url("/assets/images/user/bg_1126.png") repeat-x scroll 0 -311px transparent;
                background: #f5f6fa;
                border-left: 1px solid #e4e4e4;
                border-right: 1px solid #e4e4e4;
                border-top: 1px solid #e4e4e4;
                height: 40px;
                line-height: 40px;
                padding: 0 10px;
                display: flex;
                border-radius: 5px 5px 0 0;

                .icon {
                    background: url("/assets/images/user/bg_1126.png") no-repeat scroll -61px -285px transparent;
                    display: inline-block;
                    height: 16px;
                    margin: 10px 5px 0 0;
                    width: 16px;
                }

                .time {
                    flex: 1;
                    text-align: right;
                    color: #999999;
                }
            }

            .text {
                padding: 20px;
                border: 1px solid #eee;
                border-top: none;
                border-radius: 0 0 5px 5px;
            }

            .reply {
                margin: 20px;
                position: relative;
                background: none repeat scroll 0 0 var(--tag-bg);
                color: var(--tag-text);
                border: 1px dashed var(--tag-bg);
                display: block;
                line-height: 2;
                z-index: 999;
                padding: 10px;
            }

            .reply::before {
                content: "";
                position: absolute;
                top: -10px; /* Adjust this value to adjust the triangle vertical position */
                left: 4%; /* Center the triangle */
                transform: translateX(-50%); /* Ensure the triangle is centered */
                border-left: 10px solid transparent; /* Adjust this value to scale the triangle */
                border-right: 10px solid transparent; /* Must be the same as border-left */
                border-bottom: 10px solid var(--tag-bg); /* change the color to text's color */
            }
        }
    }

    .form-body {
        width: 600px;
        margin-top: 20px;

        .a-btn {
            color: #005ea7;
            margin-left: 6px;
        }
    }
}
</style>
