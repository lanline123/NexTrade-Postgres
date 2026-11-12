<template>
    <div class="form_item">
        <div class="top_row flex justify-between" style="padding-bottom: 0">
            <div class="tit">{{ $t("收货地址") }}</div>
            <div class="add_btn" @click="addDialog">
                <span class="flex_cc">
                    {{ $t("更改") }}
                    <SvgIcon class="ml10" name="tigRightIcon" width="10" height="18" style="transform: rotate(270deg)"></SvgIcon>
                </span>
            </div>
        </div>
        <el-dialog v-model="dialogVisible" :title="$t('收货地址')" width="880px">
            <div class="address-info">
                <div class="tips">{{ $t("不同地区的配送选项和配送速度可能会有所不同") }}</div>
                <div class="card-list">
                    <template v-if="addressList.length > 0">
                        <ul v-for="item in addressList" class="card" :class="{ selected: item?.isSelected == 1 }" @click="onChangeAddress(item)">
                            <li class="first">
                                <div class="top-info">
                                    <div class="name">
                                        <span>{{ item.consignee || "-" }}</span>
                                    </div>
                                    <div class="mobile">
                                        <span style="margin-right: 8px;">
                                            <template v-if="isOverseas() && item?.mobileAreaCode">+{{ item.mobileAreaCode }} </template>{{ item.mobile || "-" }}
                                        </span>
                                        <span v-if="item.isInvalid" class="invalid-tag">{{ $t("无效地址") }}</span>
                                    </div>
                                </div>
                                <div class="address">{{ item.formattedAddress || item.fullAddress || [item.regionName, item.address].filter(Boolean).join(" ") }}</div>
                                <div class="other">
                                    <div>{{ item.email || "-" }}</div>
                                    <div>{{ item.telephone || "-" }}</div>
                                </div>
                                <div class="bottom-operation">
                                    <div class="is-default">
                                        <el-checkbox :label="$t('默认地址')" :model-value="item.isSelected == 1" @change="handleAddressChange(item)" />
                                    </div>
                                    <div class="operation" @click.stop>
                                        <MemberAddressDialogEditRegion
                                            :title="$t('编辑收货地址')"
                                            :maskClosable="true"
                                            :closable="true"
                                            width="900px"
                                            path="MemberEditRegion"
                                            :params="{ act: 'detail', addressId: item.addressId, showClose: true }"
                                            :showFooter="false"
                                        >
                                            <SvgIcon name="edit" width="20px" height="20px"></SvgIcon>
                                        </MemberAddressDialogEditRegion>
                                        <DeleteRecord :params="{ id: item.addressId }" :requestApi="delAddress" @afterDelete="afterDelete">
                                            <SvgIcon name="delete" width="20px" height="20px" class="del"></SvgIcon>
                                        </DeleteRecord>
                                    </div>
                                </div>
                            </li>
                        </ul>
                    </template>
                    <template v-else>
                        <div class="no-result">
                            <span>{{ $t("暂无收货地址") }}</span>
                        </div>
                    </template>
                </div>
            </div>
            <div class="title-or-tabs">
                <MemberAddressDialogEditRegion
                    ref="addAddressRef"
                    :maskClosable="true"
                    :closable="true"
                    :title="$t('添加收货地址')"
                    width="900px"
                    path="MemberEditRegion"
                    :params="{ act: 'add', showClose: true }"
                    :showFooter="false"
                    @okCallback="onAddSuccess"
                >
                    <el-button plain class="btn">{{ $t(" + 添加新地址") }}</el-button>
                </MemberAddressDialogEditRegion>
            </div>
        </el-dialog>
        <div class="addressManage" v-if="addressList.length > 0">
            <div class="addressManage_name">
                <span>{{ addressList[0].consignee }}</span>
                <span v-if="addressList[0].isInvalid" class="invalid-tag">{{ $t("无效地址") }}</span>
            </div>
            <div class="info">
                <span>{{ addressList[0].regionName }}</span>
                <span>&nbsp;{{ addressList[0].address }}</span>
                <span v-if="addressList[0].mobile">
                    （<template v-if="isOverseas() && addressList[0]?.mobileAreaCode">+{{ addressList[0].mobileAreaCode }} </template>{{ addressList[0].mobile }}）
                </span>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
/**
 * @description 收货地址选择组件
 * - 展示用户收货地址列表，支持选择默认地址
 * - 提供新增、编辑、删除地址功能
 * - 支持展开/收起更多地址
 */
import { delAddress, setSelectedAddress } from "~/api/user/address";
import type { AddressFilterState } from "~/types/user/address.d";
import DeleteRecord from "~/components/member/DeleteRecord.vue";
import { isOverseas } from "~/utils/util";

type AddressItem = AddressFilterState & {
    mobileAreaCode?: string;
    isSelected?: number;
    isInvalid?: boolean;
};

const props = defineProps({
    activeAddressId: {
        type: Number
    },
    addressList: {
        type: Array as PropType<AddressItem[]>,
        default: () => []
    }
});
const addressList = computed(() => {
    return props.addressList;
});
const showMoreAddress = defineModel({ type: Boolean, default: true });

const emit = defineEmits(["okCallback", "addrssRefresh"]);
const dialogVisible = ref(false);
const addDialog = () => {
    dialogVisible.value = true;
};

const show = () => {
    dialogVisible.value = true;
};

const onAddSuccess = () => {
    dialogVisible.value = false;
    emit("addrssRefresh");
};

defineExpose({
    show
});

const addressScrollRef = ref<null | Element>(null);

const onChangeAddress = (item: AddressItem) => {
    dialogVisible.value = false;
    emit("okCallback", item);
};

const handleAddressChange = async (item: any) => {
    addressList.value?.forEach((item: any) => {
        item.isSelected = 0;
    });
    item.isSelected = 1;
    try {
        let res = await setSelectedAddress(item.addressId);
    } catch (error: any) {
        message.error(error.message);
    }
};

const afterDelete = (result: string) => {
    emit("addrssRefresh");
};
</script>

<style lang="less" scoped>
.form_item {
    :deep(.el-dialog__title) {
        font-size: 18px !important;
    }
    :deep(.el-dialog__header) {
        padding-bottom: 8px;
    }
}
.flex_cc {
    display: flex;
    align-items: center;
    color: #888b94;
    font-size: 14px;
}
.add_btn {
    cursor: pointer;
}
.address-info {
    background: #fff;
    border: 0;
    .tips {
        font-size: 14px;
        font-weight: 400;
        line-height: 18px;
        text-align: justified;
        color: #1a1a1a;
        background-color: #fff;
        padding-bottom: 10px;
    }
    // padding: 16px;

    .card-list {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 8px;
        padding: 10px 0;
        max-height: 380px;
        min-height: 248px;
        scrollbar-width: none;
        overflow-y: auto;
        scrollbar-gutter: stable;
        .card {
            box-sizing: border-box;
            width: 100%;
            min-width: 0;
            border: 1px solid #e0e1e5;
            padding: 15px;
            overflow: hidden;
            line-height: 2;
            height: 140px;
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
                    .name {
                        display: flex;
                        align-items: center;
                        min-width: 0;
                        overflow: hidden;
                        text-overflow: ellipsis;
                        gap: 8px;

                        span:first-child {
                            min-width: 0;
                            overflow: hidden;
                            text-overflow: ellipsis;
                        }
                    }
                    .mobile {
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
                    text-transform: none;
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
                        gap: 8px;
                        .del {
                            align-self: center;
                        }
                        :deep(.btn-link) {
                            display: inline-flex;
                            align-items: center;
                            line-height: 1;
                            height: 20px;
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
                > svg {
                    cursor: pointer;
                }
            }
        }
    }
}

.title-or-tabs {
    height: 68px;
    display: flex;
    align-items: center;
    :deep(.el-button) {
        height: 28px;
    }
}
.addressManage {
    .addressManage_name {
        margin-top: 16px;
        display: flex;
        align-items: center;
        gap: 8px;
        font-size: 16px;
        font-weight: 700;
        line-height: 20px;
        text-align: left;
        color: #000;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .info {
        margin-top: 8px;
        font-size: 14px;
        font-weight: 400;
        line-height: 18px;
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
:deep(.address-info) {
    .card-list {
        .card {
            width: 100%;
        }
    }
}
</style>
