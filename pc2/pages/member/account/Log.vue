<template>
    <!-- <CommonPageHeaderNew></CommonPageHeaderNew> -->
    <PageHeaderNew :isWhite="true"></PageHeaderNew>

    <CommonHeader title="申请记录"></CommonHeader>
    <TopJumpBar tltle="申请记录"></TopJumpBar>
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
            <div class="account-detail-content">
                <div v-loading="loading" class="table">
                    <template v-if="total > 0">
                        <table>
                            <thead class="table-header">
                                <tr>
                                    <th class="header-cell">{{ $t("操作时间") }}</th>
                                    <th class="header-cell">{{ $t("金额") }}</th>
                                    <th class="header-cell">{{ $t("类型") }}</th>
                                    <th class="header-cell">{{ $t("管理员备注") }}</th>
                                    <th class="header-cell">{{ $t("状态") }}</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr v-for="item in filterState" class="body-tr">
                                    <td>{{ item.addTime }}</td>
                                    <td class="price">
                                        <FormatPrice :showText="false" v-model="item.amount" :priceStyle="{ fontSize: '14px' }"></FormatPrice>
                                    </td>
                                    <td>{{ item.type }}</td>
                                    <td class="last">{{ item.postscript || '-'}}</td>
                                    <td class="last">{{ item.statusType }}</td>
                                </tr>
                            </tbody>
                        </table>
                        <div class="el-page">
                            <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
                        </div>
                    </template>
                    <template v-else>
                        <div class="no-result">
                            <span v-if="!loading">{{ $t("暂无申请记录") }}</span>
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
 * @description 充值/提现申请记录页面
 * - 展示用户的充值和提现订单记录
 * - 显示订单号、金额、状态和创建时间
 * - 支持分页加载记录列表
 */
import { reactive, ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useCommonStore } from "~/store/common";
import type { AccountFilterParams, AccountFilterState } from "~/types/user/account.d";
definePageMeta({
    middleware: "auth"
});
import { getRechargeOrderList } from "~/api/user/account";
import { Pagination } from "~/components/list";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
const filterState = ref(<AccountFilterState[]>[]);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<AccountFilterParams>({
    //初使化用于查询的参数
    page: 1
} as AccountFilterParams);

const loadFilter = async () => {
    loading.value = true;
    try {
        const result = await getRechargeOrderList({ ...filterParams });
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

const commonStore: any = useCommonStore();
// 会员余额提现开关（D1107）：关闭时隐藏“提现”入口，不影响分销佣金提现
const menuList = computed<any>(() =>
    [
        { value: "账户明细", url: "/member/account/detail", size: 0 },
        { value: "申请记录", url: "/member/account/log", size: 0 },
        { value: "充值", url: "/member/account/deposit", size: 0 },
        { value: "提现", url: "/member/account/raplytocard", size: 0 },
        { value: "卡管理", url: "/member/account/cardManagement", size: 0 }
    ].filter((item) => item.url !== "/member/account/raplytocard" || commonStore.memberWithdrawalEnabled === 1)
);
const router = useRouter();
const splitStr = (str: string) => {
    return str.split("?")[0];
};
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.account-detail-content {
    background: #fff;
    border: 0;
    padding:0 10px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;

    .table {
        width: 100%;
        font-size: 14px;
        text-align: center;
        color: #1a1a1a;

        .table-header {
            background-color: #f5f6fa;
            height: 50px;
            font-size: 14px;

            .header-cell {
                font-weight: 700;
            }
        }

        .body-tr {
            box-sizing: border-box;
            text-align: center;
            line-height: 75px;
            height: 75px;
            border-bottom: 0.5px solid #f2f2f2;

            .price {
                display: flex;
                justify-content: center;
            }
        }
    }
}
</style>
