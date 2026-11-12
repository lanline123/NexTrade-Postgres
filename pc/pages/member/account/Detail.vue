<template>
    <!-- <CommonPageHeaderNew></CommonPageHeaderNew> -->
    <PageHeaderNew :isWhite="true"></PageHeaderNew>

    <CommonHeader title="账户明细"></CommonHeader>
    <TopJumpBar tltle="账户信息"></TopJumpBar>

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
                <div class="tips">
                    <NuxtLink to="/member/security/info">{{ $t("安全中心") }}</NuxtLink>
                    {{ $t("保障账号资产安全") }}
                </div>
                <div class="money">
                    <div class="td1-title">
                        <ul>
                            <li>{{ $t("我的可用余额") }}</li>
                            <FormatPrice v-model="formState.balance" :fontStyle="{ fontSize: '24px', fontWeight: '700' }" :showText="false"></FormatPrice>
                        </ul>
                    </div>
                    <div class="td2-title">
                        <ul>
                            <li>
                                <i class="iconfont-pc icon-qianbao mr10"></i>
                                {{ $t("全部余额") }}：
                                <FormatPrice
                                    v-model="formState.totalBalance"
                                    :fontStyle="{ fontSize: '16px', fontWeight: '700' }"
                                    :showText="false"
                                ></FormatPrice>
                            </li>
                            <li>
                                <i class="iconfont-pc icon-frozen-amount mr10"></i>
                                {{ $t("冻结余额") }}：
                                <FormatPrice
                                    v-model="formState.frozenBalance"
                                    :fontStyle="{ fontSize: '16px', fontWeight: '700' }"
                                    :showText="false"
                                ></FormatPrice>
                            </li>
                        </ul>
                    </div>
                    <div class="td3-title">
                        <ul>
                            <li>
                                <NuxtLink to="/member/account/deposit">
                                    <el-button type="primary" class="big-button">{{ $t("充值") }}</el-button>
                                </NuxtLink>
                            </li>
                            <li class="history">
                                <NuxtLink to="/member/account/log">{{ $t("历史充值记录") }}></NuxtLink>
                            </li>
                        </ul>
                    </div>
                </div>
                <div v-loading="loading" class="table">
                    <template v-if="total > 0">
                        <table>
                            <thead class="table-header">
                                <tr>
                                    <th class="header-cell">{{ $t("操作时间") }}</th>
                                    <th class="header-cell">{{ $t("金额") }}</th>
                                    <th class="header-cell">{{ $t("类型") }}</th>
                                    <th class="header-cell">{{ $t("备注") }}</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr v-for="item in filterState" class="body-tr">
                                    <td>{{ item.changeTime }}</td>
                                    <td class="price">
                                        <FormatPrice v-model="item.balance" v-if="item.balance && item.balance > 0" :showText="false" :priceStyle="{ fontSize: '14px' }"></FormatPrice>
                                        <FormatPrice
                                            v-model="item.frozenBalance"
                                            v-else
                                            :showText="false"
                                            :priceStyle="{ fontSize: '14px' }"
                                        ></FormatPrice>
                                    </td>
                                    <td>{{ item.changeTypeName }}</td>
                                    <td class="last">{{ item.changeDesc }}</td>
                                </tr>
                            </tbody>
                        </table>
                        <div class="el-page">
                            <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
                        </div>
                    </template>
                    <template v-else>
                        <div class="no-result">
                            <div v-if="!loading">
                                <span v-if="!loading">{{ $t("您还没有明细记录") }}</span>
                            </div>
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
 * @description 账户明细页面
 * - 展示用户余额变动明细记录
 * - 显示当前可用余额和冻结金额
 * - 提供账户相关功能的快捷导航（充值、提现、卡管理等）
 * - 支持分页加载明细列表
 */
import { reactive, ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useCommonStore } from "~/store/common";
import { Pagination } from "~/components/list";
import { getAccountList } from "~/api/user/account";
import type { AccountFilterParams, AccountDetailFilterState } from "~/types/user/account";
import type { UserFormState } from "~/types/user/user";
import { getUser } from "~/api/user/user";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";

definePageMeta({
    middleware: "auth"
});
const router = useRouter();
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
const splitStr = (str: string) => {
    return str.split("?")[0];
};
const filterState = ref<AccountDetailFilterState[]>([]);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<AccountFilterParams>({
    //初使化用于查询的参数
    page: 1,
    status: 1
} as AccountFilterParams);

const loadFilter = async () => {
    loading.value = true;
    try {
        const result = await getAccountList({ ...filterParams });
        filterState.value = result.records;
        total.value = result.total;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const formState = ref<UserFormState>({} as UserFormState);

const getUserInfo = async () => {
    try {
        const result = await getUser();
        Object.assign(formState.value, result);
    } catch (error: any) {
        message.error(error.message);
    }
};
onMounted(() => {
    loadFilter();
    getUserInfo();
});
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.account-detail-content {
    background: #fff;
    border: 0;
    // padding: 20px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;

    .tips {
        display: flex;
        justify-content: flex-end;
        color: #999999;

        & > a {
            color: var(--general);
            margin-right: 5px;
        }
    }

    .money {
        display: flex;
        gap: 10px;

        & > div {
            flex: 1;
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: center;
            margin: 20px;
            color: #474e5d;
            line-height: 1.5;
        }

        .td1-title {
            font-weight: 700;
            font-size: 16px;
        }

        .td2-title {
            line-height: 2;
            font-size: 14px;

            li {
                display: flex;
                align-items: center;

                .td2-ico1 {
                    background: url("/assets/images/user/slice-s924f6ceef1.png") repeat scroll 0 -338px;
                    height: 14px;
                    width: 15px;
                }

                .td2-ico2 {
                    background: url("/assets/images/user/slice-s924f6ceef1.png") repeat scroll 0 -355px;
                    height: 14px;
                    width: 15px;
                }

                .mr10 {
                    margin-right: 10px;
                }
            }
        }

        .td3-title {
            display: flex;

            .history {
                margin-top: 10px;
                text-align: center;

                & > a {
                    color: var(--general);
                    font-size: 14px;
                }
            }
        }
    }

    .table {
        width: 100%;
        margin-top: 10px;
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
