<template>
    <!-- <CommonPageHeaderNew></CommonPageHeaderNew> -->
    <PageHeaderNew :isWhite="true"></PageHeaderNew>

    <CommonHeader title="卡管理"></CommonHeader>
    <TopJumpBar tltle="卡管理"></TopJumpBar>

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
                <div class="flex justify-between mb10">
                    <el-radio-group v-model="filterParams.accountType" @change="loadFilter" size="large">
                        <el-radio-button :value="1">{{ $t("银行卡") }}</el-radio-button>
                        <el-radio-button :value="2">{{ $t("支付宝") }}</el-radio-button>
                        <el-radio-button :value="3">{{ $t("微信") }}</el-radio-button>
                    </el-radio-group>
                    <InfoCard act="add" @loadFilter="loadFilter">
                        <el-button size="large" type="primary">{{ $t("添加新卡") }}</el-button>
                    </InfoCard>
                </div>

                <div v-loading="loading" class="table">
                    <template v-if="filterState.length > 0">
                        <table>
                            <thead class="table-header">
                                <tr>
                                    <th class="header-cell">{{ $t("真实姓名") }}</th>
                                    <th class="header-cell">{{ $t("身份证号码") }}</th>
                                    <th class="header-cell">{{ $t("账户号码") }}</th>
                                    <th v-if="filterParams.accountType === 1" class="header-cell">{{ $t("银行卡详情") }}</th>
                                    <th class="header-cell" style="width: 130px">{{ $t("操作") }}</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr v-for="item in filterState" class="body-tr">
                                    <td>{{ item.accountName }}</td>
                                    <td>{{ item.identity }}</td>
                                    <td>{{ maskString(item.accountNo) }}</td>
                                    <td v-if="filterParams.accountType === 1">{{ item.bankName || "--" }}</td>
                                    <td>
                                        <div class="flex justify-center align-center">
                                            <InfoCard :id="item.accountId" act="edit" @loadFilter="loadFilter">
                                                <span class="font-color">{{ $t("编辑") }}</span>
                                            </InfoCard>
                                            ｜
                                            <DeleteRecord :params="{ accountId: item.accountId }" :requestApi="delAccount" @afterDelete="loadFilter"
                                                ><span class="font-color">{{ $t("删除") }}</span>
                                            </DeleteRecord>
                                        </div>
                                    </td>
                                </tr>
                            </tbody>
                        </table>
                    </template>
                    <template v-else>
                        <div class="no-result">
                            <span v-if="!loading">{{ $t("暂无卡片") }}</span>
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
 * @description 银行卡/账号管理页面
 * - 展示用户已绑定的银行卡和提现账号列表
 * - 支持添加新的银行卡或提现账号
 * - 支持删除已绑定的账号
 * - 账号信息显示时进行脱敏处理
 */
import { reactive, ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useCommonStore } from "~/store/common";
import type { AccountFilterParams, AccountInfo } from "~/types/user/account";
import { delAccount, getAccountNoList } from "~/api/user/account";
import InfoCard from "~/pages/member/account/src/InfoCard.vue";
import DeleteRecord from "~/components/member/DeleteRecord.vue";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
definePageMeta({
    middleware: "auth"
});

const loading = ref<boolean>(true);

const filterParams = reactive<AccountFilterParams>({
    //初使化用于查询的参数
    page: 1,
    accountType: 1
} as AccountFilterParams);
onMounted(() => {
    loadFilter();
});
const filterState = ref<AccountInfo[]>([]);
//特殊方法
function maskString(str: any) {
    if (str) {
        if (str.length < 7) {
            return str;
        } else {
            const firstTwo = str.substring(0, 2);
            const lastFour = str.substring(str.length - 4, str.length);
            const masked = "*".repeat(str.length - 6);
            return firstTwo + masked + lastFour;
        }
    }
}
const loadFilter = async () => {
    try {
        const result = await getAccountNoList({ ...filterParams });
        filterState.value = result.records;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

const router = useRouter();
const commonStore: any = useCommonStore();
const splitStr = (str: string) => {
    return str.split("?")[0];
};
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
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.account-detail-content {
    background: #fff;
    border: 0;
    padding: 20px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;

    .table {
        width: 100%;
        margin-top: 10px;
        font-size: 14px;
        color: #1a1a1a;

        .table-header {
            background-color: #f5f6fa;
            height: 50px;
            font-size: 14px;

            .header-cell {
                font-weight: 700;
                text-align: center;
            }
        }

        .body-tr {
            height: 70px;
            text-align: center;
            box-sizing: border-box;
            border-bottom: 0.5px solid #f2f2f2;
        }
    }
}
</style>
