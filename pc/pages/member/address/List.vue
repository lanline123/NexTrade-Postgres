<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader title="地址"></CommonHeader>
    <TopJumpBar tltle="地址"></TopJumpBar>

    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div data-content-rev="YWZ5MjI4Mnc" class="info-row">
            <div class="title-or-tabs">
                <span class="card-title">{{ $t("地址") }}</span>
                <MemberAddressDialogEditRegion
                    ref="addAddressRef"
                    :maskClosable="true"
                    :closable="true"
                    @okCallback="loadFilter"
                    :title="$t('添加收货地址')"
                    :width="isOverseas() ? '900px' : '880px'"
                    path="MemberEditRegion"
                    :params="{ act: 'add', showClose: true }"
                    :showFooter="false"
                >
                    <el-button type="primary" class="btn new-address" :icon="Plus">{{ $t("添加新地址") }}</el-button>
                </MemberAddressDialogEditRegion>
            </div>
            <div class="address-info">
                <div v-loading="loading" class="card-list">
                    <template v-if="total > 0">
                        <ul v-for="item in filterState" class="card" :class="{ selected: item?.isSelected == 1 }">
                            <li class="first">
                                <div class="top-info">
                                    <div class="name">{{ item.consignee || "-" }}</div>
                                    <div class="mobile">
                                        <span>+{{ item?.mobileAreaCode }} {{ item.mobile || "-" }}</span>
                                        <span v-if="item.isInvalid" class="invalid-tag">{{ $t("无效地址") }}</span>
                                    </div>
                                </div>
                                <div class="address">
                                    {{ item.formattedAddress || item.fullAddress || [item.regionName, item.address].filter(Boolean).join(" ") }}
                                </div>
                                <div class="other">
                                    <div>{{ item.email || "-" }}</div>
                                    <div>{{ item.telephone || "-" }}</div>
                                </div>
                                <div class="bottom-operation">
                                    <div class="is-default">
                                        <el-checkbox :label="$t('默认地址')" :model-value="item.isSelected == 1" @change="handleAddressChange(item)" />
                                    </div>
                                    <div class="operation">
                                        <MemberAddressDialogEditRegion
                                            @okCallback="loadFilter"
                                            :title="$t('编辑收货地址')"
                                            :width="isOverseas() ? '900px' : '880px'"
                                            path="MemberEditRegion"
                                            :params="{ act: 'detail', addressId: item.addressId }"
                                            :showFooter="false"
                                        >
                                            <SvgIcon name="edit" width="20px" height="20px"></SvgIcon>
                                        </MemberAddressDialogEditRegion>
                                        <DeleteRecord :params="{ id: item.addressId }" :requestApi="delAddress" @afterDelete="loadFilter">
                                            <SvgIcon name="delete" width="20px" height="20px" class="del"></SvgIcon>
                                        </DeleteRecord>
                                    </div>
                                </div>
                            </li>
                        </ul>
                        <div class="el-page">
                            <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
                        </div>
                    </template>
                    <template v-else>
                        <div class="no-result">
                            <div v-if="!loading">
                                <img class="empty-img" src="/assets/images/empty-box.png" alt="" />
                                <div class="empty-text">{{ $t("暂无收货地址") }}</div>
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
 * @description 收货地址列表页面
 * - 展示用户所有收货地址，支持设置默认地址、编辑和删除操作
 * - 提供新增地址入口，点击跳转到地址编辑页面
 * - 支持分页加载地址列表
 */
import { reactive, ref } from "vue";
import type { AddressFilterParams, AddressFilterState } from "~/types/user/address.d";
import { delAddress, getAddressList, setSelectedAddress } from "~/api/user/address";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";

definePageMeta({
    middleware: "auth"
});
import DeleteRecord from "~/components/member/DeleteRecord.vue";
import { Pagination } from "~/components/list";
import { Plus } from "@element-plus/icons-vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
import { isOverseas } from "~/utils/util";

const { t } = useI18n();
const filterState = ref(<AddressFilterState[]>[]);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<AddressFilterParams>({
    //初使化用于查询的参数
    page: 1
} as AddressFilterParams);

const loadFilter = async () => {
    loading.value = true;
    try {
        const result = await getAddressList({ ...filterParams });
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

const handleAddressChange = async (item: any) => {
    loading.value = true;
    filterState.value?.forEach((item: any) => {
        item.isSelected = 0;
    });
    item.isSelected = 1;
    try {
        let res = await setSelectedAddress(item.addressId);
        loadFilter();
        message.success(t("操作成功"));
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.card-title {
    font-size: 18px !important;
    font-weight: 700 !important;
    color: #1a1a1a !important;
}

.btn {
    height: 28px;
}

.address-info {
    background: #fff;
    border: 0;

    :deep(.el-checkbox__inner) {
        height: 18px;
        width: 18px;
        border-width: 2px;
        border-radius: 2.5px;
    }
    :deep(.el-checkbox__inner::after) {
        width: 4px;
        height: 8px;
        border-width: 2px;
    }

    .card-list {
        display: grid;
        grid-template-columns: repeat(2, 1fr);
        gap: 8px;
        padding: 10px 0;

        .el-page,
        .no-result {
            grid-column: 1 / -1;
        }

        .card {
            box-sizing: border-box;
            border: 1px solid #e0e1e5;
            padding: 15px;
            overflow: hidden;
            line-height: 2;
            border-radius: 4px;
            &.selected {
                border: 1px solid var(--main-bg);
            }

            .first {
                .top-info {
                    display: flex;
                    align-items: center;
                    font-size: 16px;
                    font-weight: 700;
                    line-height: 20px;
                    text-align: left;
                    color: #000;
                    white-space: nowrap;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    .mobile {
                        display: flex;
                        align-items: center;
                        gap: 8px;
                        margin-left: 8px;
                        font-size: 14px;
                        font-weight: 400;
                        line-height: 18px;
                        text-align: left;
                        color: #1a1a1a;
                        white-space: nowrap;
                    }
                }
                .address {
                    margin-top: 8px;
                    font-size: 14px;
                    font-weight: 400;
                    line-height: 18px;
                    height: 36px;
                    text-align: left;
                    display: -webkit-box;
                    line-clamp: 2;
                    -webkit-line-clamp: 2;
                    -webkit-box-orient: vertical;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    color: #505259;
                    word-break: break-all;
                }
                .other {
                    display: flex;
                    align-items: center;
                    column-gap: 8px;
                    margin-top: 8px;
                    font-size: 14px;
                    font-weight: 400;
                    line-height: 18px;
                    height: 24px;
                    text-align: left;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    color: #505259;
                    word-break: break-all;
                }
                .bottom-operation {
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    height: 24px;
                    .is-default {
                        :deep(.el-checkbox__label) {
                            color: #1a1a1a;
                            font-size: 14px;
                        }
                    }
                    .operation {
                        display: flex;
                        align-items: center;
                        color: #1a1a1a;
                        column-gap: 8px;

                        :deep(.dialog-link),
                        :deep(.btn-link) {
                            display: inline-flex !important;
                            align-items: center;
                            justify-content: center;
                            width: 20px;
                            height: 20px;
                            line-height: 1 !important;
                            cursor: pointer;
                        }
                    }
                }
            }

            .ju {
                display: flex;

                & > div {
                    flex: 1;
                }
            }

            .operation {
                color: #005ea7;
                cursor: pointer;
            }
        }
    }
}

.invalid-tag {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    height: 20px;
    padding: 0 8px;
    border-radius: 2px;
    font-size: 12px;
    font-weight: 400;
    line-height: 18px;
    color: #cc0c1c;
    background: rgba(204, 12, 28, 0.08);
}

.new-address {
    border-radius: 4px;
    border: 1px solid #cc0c1c;
    font-size: 12px;
    font-weight: 400;
    line-height: 16px;
    text-align: left;
    color: #cc0c1c;
    background-color: #fff;
}
.new-address:hover {
    background-color: rgba(204, 12, 28, 0.04);
}
:deep(.el-select__wrapper) {
    box-shadow: 0 0 0 1px var(--el-border-color) inset;
}
</style>
