<template>
    <!-- <CommonPageHeaderNew></CommonPageHeaderNew> -->
    <PageHeaderNew :isWhite="true"></PageHeaderNew>

    <CommonHeader :title="`${commonStore.integralName}明细`"></CommonHeader>
    <TopJumpBar :tltle="`${commonStore.integralName}明细`"></TopJumpBar>

    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="title-or-tabs">
                <span>{{ $t(`${commonStore.integralName}明细`) }}</span>
            </div>
            <div v-loading="loading" class="point-info">
                <table class="custom-table">
                    <thead class="table-header">
                        <tr>
                            <th class="header-cell header-cell-first" style="width: 160px">{{ $t("操作时间") }}</th>
                            <th class="header-cell" style="width: 80px">{{ $t("类型") }}</th>
                            <th class="header-cell" style="width: 80px">{{ $t(commonStore.integralName) }}</th>
                            <th class="header-cell header-cell-last">{{ $t("备注") }}</th>
                        </tr>
                    </thead>
                    <tbody v-if="total > 0">
                        <tr v-for="(item, index) in filterState" :key="index">
                            <td>{{ item.changeTime }}</td>
                            <td>{{ item.changeTypeName }}</td>
                            <td class="font-color">{{ item.points }}</td>
                            <td>{{ item.changeDesc }}</td>
                        </tr>
                    </tbody>
                    <tbody v-else>
                        <tr>
                            <td v-if="!loading" class="no-table-result" colspan="4">{{ $t(`您还没有${commonStore.integralName}明细`) }}</td>
                            <td v-else class="no-table-result" colspan="4"></td>
                        </tr>
                    </tbody>
                    <tfoot class="table-footer">
                        <tr>
                            <td colspan="4">
                                {{ $t(`您的当前${commonStore.integralName}为`) }}：<span class="font-color">{{ userPoints }}</span>
                            </td>
                        </tr>
                    </tfoot>
                </table>
                <div class="el-page">
                    <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
                </div>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * @description 积分明细页面
 * - 展示用户的积分变动记录，包括积分来源、变动数量和时间
 * - 显示用户当前可用积分总数
 * - 支持分页加载积分明细列表
 */
import { reactive, ref } from "vue";
import type { PointFilterParams, PointFilterState } from "~/types/user/point.d";
import { getPointList } from "~/api/user/point";
import { Pagination } from "~/components/list";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
import { useCommonStore } from "~/store/common";

definePageMeta({
    middleware: "auth"
});

const commonStore = useCommonStore();

const filterState = ref<PointFilterState[]>([]);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const userPoints = ref<number>(0);
const filterParams = reactive<PointFilterParams>({
    //初使化用于查询的参数
    page: 1
});

const loadFilter = async () => {
    loading.value = true;
    try {
        const result = await getPointList({ ...filterParams });
        filterState.value = result.records;
        total.value = result.total;
        userPoints.value = result.userPoints;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
onMounted(() => {
    loadFilter();
});
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.point-info {
    background: #fff;
    border: 0;
    // padding: 20px 20px;

    .custom-table {
        width: 100%;
        border-collapse: collapse;
        border: 1px solid #ddd;
    }

    .table-header {
        height: 48px;
        background-color: #f5f6fa;
    }

    .header-cell {
        border: 1px solid #ddd;
        border-right: none;
        padding: 8px 10px;
        text-align: left;
    }

    .header-cell-first {
        border-left: none;
    }

    .header-cell-last {
        border-right: 1px solid #ddd;
    }

    table td,
    .header-cell {
        border: 1px solid #ddd;
        padding: 15px 10px;
        text-align: left;
        font-size: 14px;
    }

    .table-footer td {
        text-align: left;
        background-color: #f5f6fa;
        border-top: 1px solid #ddd;
    }

    .no-table-result {
        text-align: center;
        height: 150px;
        font-size: 20px;
    }
}
</style>
