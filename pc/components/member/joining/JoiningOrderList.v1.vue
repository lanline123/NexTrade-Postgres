<template>
    <div class="order-list">
        <div v-if="modelValue" class="tag-and-input">
            <div class="tig-tabs">
                <div v-for="item in menuTypeList" :class="menuType == item.type ? 'active' : ''" class="tab"
                    @click="changeType(item.type)">
                    <span v-if="item.num">{{ item.num }}</span>
                    <p>{{ $t(item.value) }}</p>
                </div>
            </div>
            <div class="input">
                <el-input v-model="filterParams.keyword" :placeholder="$t('收货人名称或订单号')" type="text" />
                <el-button class="ml10" @click="loadFilter()">{{ $t("搜索") }}</el-button>
            </div>
        </div>
        <div class="order-title">
            <div class="ul">
                <template v-if="modelValue">
                    <div style="width: 23%">
                        <MemberOrderSelect v-model="filterParams.dateType" :selectList="orderTimeList"
                            @loadFilter="loadFilter"></MemberOrderSelect>
                    </div>
                    <div style="width: 28%">{{ $t("商品名称") }}</div>
                </template>
                <template v-else>
                    <div style="width: 49%">{{ $t("商品名称") }}</div>
                </template>
                <div style="width: 6%">{{ $t("数量") }}</div>
                <div style="width: 7%"></div>
                <div style="width: 12%">{{ $t("拼团价") }}</div>
                <div style="width: 14%">{{ $t("拼团状态") }}</div>
                <div style="width: 12%">{{ $t("操作") }}</div>
            </div>
        </div>
        <div v-loading="loading">
            <template v-if="total > 0">
                <table v-for="(item, index) in filterState" class="order-table">
                    <thead>
                        <tr>
                            <th colspan="4" class="order-table-title">
                                {{ $t("订单号") }}：<NuxtLink :to="'/member/order/info?id=' + item.orderId" target="_blank">
                                    <span class="font-color">{{ item.orderSn }}</span>
                                </NuxtLink>
                                <span style="margin-left: 30px">{{ $t("下单时间") }}：{{ item.orderAddTime }}</span>
                            </th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr class="order-tr">
                            <td class="column-1">
                                <div class="product-info">
                                    <NuxtLink :title="item.productName"
                                        :to="urlFormat({ path: 'product', sn: item.productSn })"
                                        class="nuxt" target="_blank">
                                        <el-image :src="productImage(item)" loading="lazy"
                                            style="height: 40px; width: 40px; transition: opacity 0.2s linear; flex-shrink: 0" :alt="item.productName || ''" />
                                        <div>
                                            <div class="text-ellipsis">
                                                {{ item.productName }}
                                            </div>
                                        </div>
                                    </NuxtLink>
                                    <div style="color: #aaa">X{{ item.quantity || 1 }}</div>
                                </div>
                            </td>
                            <td class="column-2-4">
                                <div>
                                    <div class="flex">
                                        <div>{{ $t("拼团价") }}:</div>
                                        <FormatPrice v-model="item.grouponPrice" :showText="false"></FormatPrice>
                                    </div>
                                    <div>{{ item.payTypeId === 1 ? $t("在线支付") : item.payTypeId === 2 ? $t("货到付款") :
                                        $t("线下支付") }}</div>
                                </div>
                            </td>
                            <td class="column-2-4">
                                <div>
                                    <div :class="[{ 'joinStatus-color': item.joinStatus === 1 }]">
                                        {{ joinStatusName[item.joinStatus] }}
                                    </div>
                                    <div class="getjoinNumCount" v-if="item.joinStatus == 1">还差{{ getjoinNumCount(item)
                                        }}人</div>
                                </div>
                            </td>
                            <td class="column-2-4">
                                <div>
                                    <div>
                                        <div class="order-right">
                                            <el-button size="small"
                                                @click="onInviteFriend(item.grouponPromotionRecordId)" type="primary"
                                                v-if="item.joinStatus === 1">{{ $t("邀请好友") }}</el-button>
                                            <!--                                        <div class="invited" v-if="item.joinStatus === 1">邀请好友</div>-->
                                        </div>
                                        <NuxtLink
                                            :to="{ path: '/member/group/info', query: { id: item.grouponPromotionRecordId } }"
                                            target="_blank">
                                            <div style="text-align: center">拼团详情</div>
                                        </NuxtLink>
                                    </div>
                                </div>
                            </td>
                        </tr>
                    </tbody>
                </table>
                <div class="el-page">
                    <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total"
                        @callback="loadFilter" />
                </div>
            </template>
            <template v-else>
                <div class="no-result">
                    <div v-if="!loading">
                        <span class="sign sign1"></span>
                    </div>
                    <div v-if="!loading">
                        <ul>
                            <li>{{ $t("当前筛选下还没有任何订单哦") }}</li>
                            <li style="font-size: 12px">
                                {{ $t("现在就去") }}：
                                <NuxtLink class="font-color" to="/"> {{ $t("选购商品") }}</NuxtLink>
                            </li>
                        </ul>
                    </div>
                </div>
            </template>
        </div>
        <showInviteDialog ref="showInviteDialogRef"></showInviteDialog>
    </div>
</template>

<script lang="ts" setup>
import { onMounted, reactive, ref } from "vue";
import type { OrderFilterParams, OrderFilterState } from "~/types/joining/joining.d";
import { getOrderList, getOrderNum } from "~/api/user/order";
import { getProductRecordList } from "~/api/joining/joining";
import { Pagination } from "~/components/list";
import { urlFormat } from "~/utils/format";
import { message } from "ant-design-vue";
import { useRoute } from "vue-router";
import { useRouter } from "#vue-router";
import showInviteDialog from './src/showInviteDialog.vue'

const showInviteDialogRef = ref<any>(null);
const { t } = useI18n();
const props = defineProps({
    modelValue: {
        type: Number,
        default: 1
    }
});

const emit = defineEmits(["change"]);

const joinStatusName: any = {
    1: t("拼团中"),
    2: t("拼团成功"),
    3: t("拼团失败"),
};
const router = useRouter();
// 基本参数定义
const filterState = ref(<OrderFilterState[]>[]);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<OrderFilterParams>({
    //初使化用于查询的参数
    page: 1,
    sortField: "addTime",
    sortOrder: "desc",
    keyword: "",
    dateType: 1,
    joining: 1,
    isMine: true,
    joinType: 1
});
const route = useRoute();
const menuType = ref<string>("all");

interface menuType {
    type: string;
    value: string;
    num: number;
}

const menuTypeList = reactive<menuType[]>([
    { type: "all", value: "我发起的团", num: 0 },
    { type: "awaitPay", value: "我参与的团", num: 0 },
]);
const changeType = (type: string) => {
    filterParams.page = 1;
    if (type === "all") {
        filterParams.joinType = 1;
    } else {
        filterParams.joinType = 2;
    }
    menuType.value = type;
    loadFilter();
};
// const loadNum = async () => {
//     loading.value = true;
//     try {
//         const result = await getOrderNum();
//         menuTypeList.forEach((item: any) => {
//             let temp: any = result;
//             if (temp[item.type] !== undefined) {
//                 item.num = temp[item.type];
//             }
//         });
//     } catch (error: any) {
//         message.error(error.message);
//     } finally {
//         loading.value = false;
//     }
// };

const orderTimeList = reactive([
    { value: 0, label: "全部订单" },
    { value: 1, label: "最近三个月订单" },
    { value: 2, label: "最近六个月订单" },
    { value: 3, label: "最近一年内订单" }
]);

// 获取还差多少人成团
const getjoinNumCount = (item: any) => {
    return item.minGroupNum - item.joinNumCount > 0 ? item.minGroupNum - item.joinNumCount : 0;
};

const loadFilter = async () => {
    loading.value = true;
    try {
        total.value = 0;
        const result: any = await getProductRecordList({ ...filterParams });
        loading.value = false;
        filterState.value = result.records;
        total.value = result.total;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const onInviteFriend = (id: any) => {
    showInviteDialogRef.value.show(location.origin + '/joining/groupByInvited?' + 'id=' + id)
};

onMounted(() => {
    loadFilter();
});
</script>

<style lang="less" scoped>
@import "/assets/css/member/member.v1";

.order-list {
    .tag-and-input {
        display: flex;
        flex-direction: row;
        justify-content: space-between;

        .image {
            margin-bottom: 20px;
        }

        .input {
            display: flex;
            align-items: center;
        }
    }

    .order-table {
        color: #666;
        border: 1px solid #eee;
        width: 100%;
        background-color: #f7f7f7;
        height: 40px;
        line-height: 40px;
        margin-bottom: 10px;

        &>thead>tr>th {
            padding-left: 10px;
        }

        .order-tr {
            background-color: white;
            box-sizing: border-box;

            .column-1 {
                width: 64%;
                border: 1px solid #eee;

                .nuxt {
                    display: flex;
                    gap: 10px;
                    flex: 1;
                }

                &>div {
                    padding: 15px;
                    box-sizing: border-box;
                    display: flex;
                    align-items: center;
                    flex-direction: row;
                    border-bottom: 1px solid #eee;
                    width: 100%;

                    &>span {
                        flex: 1;
                        margin-left: 10px;
                        line-height: 2;
                    }

                    &>div {
                        margin-left: 10px;
                        width: 60px;
                        text-align: center;
                    }
                }

                &>div:last-child {
                    border-bottom: none;
                }
            }

            .column-2-4 {
                width: 12%;
                border: 1px solid #eee;

                &>div {
                    display: flex;
                    height: 100%;
                    padding: 20px 0;
                    box-sizing: border-box;
                    flex-direction: column;
                    justify-content: flex-start;
                    align-items: center;
                    color: #aaaaaa;

                    &>.black {
                        color: black;
                    }

                    &>.money {
                        text-align: center;
                        width: 80%;
                        border-bottom: 1px solid #e5e5e5;
                    }

                    &>div {
                        line-height: 2;
                    }
                }

                a {
                    cursor: pointer;
                }
            }
        }
    }
}

.orange {
    color: #ed6a0c;
}

.green {
    color: #31c19e;
}

.gray {
    color: #999999;
}

// 拼团
.joining-tag {
    background: var(--general);
    border-radius: 2px;
    padding: 2px 5px;
    font-weight: bold;
    font-size: 10px;
    color: #FFFFFF;
    text-align: center;
    display: inline;
    margin-right: 5px;
}

.invited {
    background: #fff;
    border-radius: 2px;
    padding: 3px 6px;
    box-sizing: border-box;
    height: 24px;
    line-height: 26px;
    font-size: 12px;
    color: #6A6A6A;
    border: 1px solid #E5E5E5;
    text-align: center;
    display: inline;
    cursor: pointer;

    &:hover {
        color: var(--general);
    }
}

.product-info {
    display: flex;
    align-items: center;
}

.joinStatus-color {
    color: var(--general);
}

.getjoinNumCount {
    color: #000;
}
</style>
