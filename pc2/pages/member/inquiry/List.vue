<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader title="我的询价单"></CommonHeader>
    <TopJumpBar tltle="我的询价单"></TopJumpBar>

    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="inquiry-page-box">
                <div class="custom-tabs">
                    <div v-for="item in menuList" :key="item.type" :class="['tab', currentStatus === item.type ? 'active' : '']" @click="changeType(item.type)">
                        {{ $t(item.value) }}
                    </div>
                </div>
                
                <div v-loading="loading" class="inquiry-content">
                    <template v-if="total > 0">
                        <table class="custom-table">
                            <thead class="table-header">
                                <tr>
                                    <th style="width: 420px">{{ $t("询价商品") }}</th>
                                    <th>{{ $t("询价内容") }}</th>
                                    <th style="width: 120px">{{ $t("状态") }}</th>
                                    <th style="width: 120px">{{ $t("操作") }}</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr v-for="item in filterState" :key="item.id">
                                    <td>
                                        <div class="goods-info">
                                            <el-image :src="productImage(item.product)" class="goods-img" :alt="item.product?.productName || ''" />
                                            <div class="goods-name line2">{{ item.product?.productName || "-" }}</div>
                                        </div>
                                    </td>
                                    <td>
                                        <div class="content line2">{{ item.content || "-" }}</div>
                                    </td>
                                    <td>{{ getStatusText(item) }}</td>
                                    <td>
                                        <NuxtLink :to="'/member/inquiry/info?id=' + item.id">
                                            <TigButton theme type="primary" class="deleteWeigh">{{ $t("查看") }}</TigButton>
                                        </NuxtLink>
                                    </td>
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
                                <img class="empty-img" src="/assets/images/empty-box.png" alt="" />
                                <div class="empty-text">{{ $t("暂无询价单") }}</div>
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
import { reactive, ref } from "vue";
import { getInquiryList } from "~/api/user/inquiry";
import type { InquiryFilterParams, InquiryItem } from "~/types/user/inquiry.d";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
import { Pagination } from "~/components/list";

definePageMeta({
    middleware: "auth"
});

const { t } = useI18n();

const menuList = reactive([
    { value: "全部询价", type: -1 },
    { value: "已回复", type: 1 },
    { value: "未回复", type: 0 }
]);

const currentStatus = ref<number>(-1);
const filterState = ref<InquiryItem[]>([]);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<InquiryFilterParams>({
    page: 1,
    size: 10
});

const getStatusText = (item: InquiryItem) => {
    return item.status === 1 ? t("已回复") : t("未回复");
};

const loadFilter = async () => {
    loading.value = true;
    try {
        const params: InquiryFilterParams = {
            ...filterParams
        };
        if (currentStatus.value !== -1) {
            params.status = currentStatus.value;
        } else {
            delete params.status;
        }
        const result = await getInquiryList(params);
        filterState.value = result.records || [];
        total.value = result.total || 0;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

const changeType = (type: number) => {
    if (currentStatus.value === type) return;
    currentStatus.value = type;
    filterParams.page = 1;
    loadFilter();
};

onMounted(() => {
    loadFilter();
});
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.inquiry-page-box {
    background: #fff;
    padding: 20px 0;
    border-radius: 4px;
    min-height: 600px;
}

.custom-tabs {
    display: flex;
    gap: 40px;
    padding-bottom: 20px;
    
    .tab {
        cursor: pointer;
        font-size: 16px;
        color: #999;
        padding-bottom: 6px;
        position: relative;
        
        &.active {
            color: #333;
            font-weight: bold;
            
            &::after {
                content: "";
                position: absolute;
                bottom: 0;
                left: 50%;
                transform: translateX(-50%);
                width: 30px;
                height: 3px;
                background-color: #333;
                border-radius: 2px;
            }
        }
    }
}

.inquiry-content {
    background: #fff;
    border: 0;
    font-size: 14px;

    .custom-table {
        width: 100%;
        border-collapse: collapse;
        border: none;
    }

    table th, table td {
        border-bottom: 1px solid #ebeef5;
        padding: 16px 10px;
        text-align: left;
        vertical-align: middle;
    }

    table th {
        color: #333;
        font-weight: bold;
        background-color: #fff;
        border-top: none;
        border-left: none;
        border-right: none;
    }

    table td {
        border-left: none;
        border-right: none;
    }

    .goods-info {
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .goods-img {
        width: 60px;
        height: 60px;
        border: 1px solid #eee;
        flex-shrink: 0;
        border-radius: 4px;
    }

    .goods-name,
    .content {
        line-height: 1.5;
        color: #333;
    }

    .line2 {
        display: -webkit-box;
        line-clamp: 2;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
    }
}
</style>
