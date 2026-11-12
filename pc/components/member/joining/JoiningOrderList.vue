<template>
    <div class="title-or-tabs">
        <span>{{ $t("我的拼团") }}</span>
    </div>
    <div class="order-list">
        <div v-if="modelValue" class="tag-and-input">
            <div class="tig-tabs">
                <div v-for="item in menuTypeList" :class="menuType == item.type ? 'activeert' : ''" class="tab" @click="changeType(item.type)">
                    <p>
                        {{ $t(item.value) }}
                        <span v-if="item.num">{{ item.num }}</span>
                    </p>
                </div>
            </div>
        </div>
        <div v-loading="loading" style="padding-top: 12px">
            <template v-if="total > 0">
                <div v-for="(item, index) in filterState" :key="index" class="memberOrder">
                    <div class="memberOrder_top">
                        <div class="memberOrder_top_left">
                            <span :class="[{ 'joinStatus-color': item.joinStatus === 1 }]">
                                {{ joinStatusName[item.joinStatus] }}
                                <span v-if="item.joinStatus == 1" class="getjoinNumCount">（{{ $t('还差') }} {{ getjoinNumCount(item) }} {{ $t('人') }}）</span>
                            </span>
                        </div>
                        <div>
                            <span class="black">
                                <NuxtLink class="flex_cc" :to="'/member/order/info?id=' + item.orderId" target="_blank">
                                    {{ $t("订单详情") }}
                                    <SvgIcon class="icon" name="rightIcon" width="16px" height="16px"></SvgIcon>
                                </NuxtLink>
                            </span>
                        </div>
                    </div>
                    <div class="memberOrder_middle">
                        <div class="fs_14">
                            {{ $t("订单号") }}：<NuxtLink :to="'/member/order/info?id=' + item.orderId" target="_blank"
                                ><span>{{ item.orderSn }}</span></NuxtLink
                            >
                            <span style="margin-left: 30px">{{ $t("下单时间") }}：{{ item.orderAddTime }}</span>
                        </div>
                        <div class="fs_14">
                            {{ $t("共{0}件", [item.quantity || 1]) }}:
                            <span style="margin-left: 8px">
                                {{ $t("拼团价") }}:
                                <FormatPrice
                                    v-model="item.grouponPrice"
                                    :showText="false"
                                    :priceStyle="{ fontSize: '16px', color: '#1a1a1a', fontWeight: 'bold' }"
                                    :currencyStyle="{ fontSize: '14px' }"
                                ></FormatPrice>
                            </span>
                        </div>
                    </div>
                    <div class="memberOrder_shop">
                        <NuxtLink
                            :title="item.productName"
                            :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.productSn })"
                            class="nuxt"
                            target="_blank"
                        >
                            <div class="flex">
                                <el-image
                                    :src="productImage(item)"
                                    loading="lazy"
                                    style="width: 60px; height: 60px; cursor: pointer; transition: opacity 0.2s linear; flex-shrink: 0; border-radius: 4px"
                                    :alt="item.productName || ''"
                                />
                                <div class="memberOrder_shop_text">
                                    <div class="text-ellipsis">
                                        {{ item.productName }}
                                    </div>
                                </div>
                            </div>
                        </NuxtLink>
                        <div class="memberOrder_shop_price">
                            <div>
                                <NuxtLink
                                    :title="item.productName"
                                    :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.productSn })"
                                    target="_blank"
                                >

                                    <FormatPrice
                                        v-model="item.grouponPrice"
                                        :showText="false"
                                        :priceStyle="{ color: '#1a1a1a' }"
                                        :fontStyle="{ fontSize: '18px', fontWeight: 'bold', lineHeight: '18px' }"
                                        :currencyStyle="{ fontSize: '12px' }"
                                        :decimalsStyle="{ fontSize: '12px' }"
                                    ></FormatPrice>
                                </NuxtLink>
                            </div>
                            <NuxtLink
                                :title="item.productName"
                                :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.productSn })"
                                target="_blank"
                            >
                                <div style="color: #aaa">X{{ item.quantity || 1 }}</div>
                            </NuxtLink>
                        </div>
                    </div>
                    <div class="memberOrder_bot">
                        <div></div>
                        <div class="memberOrder_bot_but">
                            <div v-if="item.joinStatus === 1" style="margin-right: 8px">
                                <TigButton @click="onInviteFriend(item.grouponPromotionRecordId)" theme type="primary" class="deleteWeigh">
                                    {{ $t("邀请好友") }}
                                </TigButton>
                            </div>
                            <div>
                                <NuxtLink
                                    :to="{ path: '/member/group/info', query: { id: item.grouponPromotionRecordId } }"
                                    target="_blank">
                                    <TigButton class="deleteWeigh">{{ $t("拼团详情") }}</TigButton>
                                </NuxtLink>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="el-page">
                    <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
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
const filterState = ref(<OrderFilterState[]>[]);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<OrderFilterParams>({
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

const orderTimeList = reactive([
    { value: 0, label: "全部订单" },
    { value: 1, label: "最近三个月订单" },
    { value: 2, label: "最近六个月订单" },
    { value: 3, label: "最近一年内订单" }
]);

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
@import "/assets/css/member/member";

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
.memberOrder {
    margin-bottom: 16px;
    border: 1px solid #f0f1f2;
    border-radius: 4px;
    overflow: hidden;
    cursor: pointer;
    font-family: "Trebuchet MS", "Lucida Sans Unicode", "Lucida Grande", "Lucida Sans", Arial, sans-serif;
    .memberOrder_top {
        display: flex;
        align-items: center;
        justify-content: space-between;
        height: 48px;
        background-color: #f5f6fa;
        padding: 0 16px;
        border-bottom: 1px solid #f0f1f2;
        font-size: 14px;
        .memberOrder_top_left {
            font-size: 16px;
            font-weight: 700;
            line-height: 20px;
            color: #1a1a1a;
        }
        .icon {
            margin-top: 2px;
            margin-left: 3px;
        }
        &:hover {
            a {
                color: var(--general);
            }
            .icon {
                color: var(--general);
            }
        }
    }
    .memberOrder_middle {
        display: flex;
        align-items: center;
        justify-content: space-between;
        height: 48px;
        padding: 0 16px;
        border-bottom: 1px solid #f0f1f2;
    }
    .memberOrder_shop {
        display: flex;
        padding: 12px 16px;
        .memberOrder_shop_text {
            margin-left: 16px;
            width: 680px;
            .text-ellipsis {
                font-size: 16px;
                color: #1a1a1a;
            }
            .sku {
                font-size: 14px;
                line-height: 18px;
                color: #888b94;
                margin-top: 4px;
            }
        }
        .memberOrder_shop_price {
            display: flex;
            justify-content: space-between;
            flex: 1;
        }
    }
    .memberOrder_bot {
        display: flex;
        align-items: center;
        justify-content: space-between;
        height: 48px;
        padding: 0 16px;
        border-top: 1px solid #f0f1f2;
        .memberOrder_bot_but {
            display: flex;
            align-items: center;
            font-weight: 500;
        }
    }
}
.flex_csb {
    display: flex;
    justify-content: space-between;
    padding: 0 15px;
}
.flex_cc {
    display: flex;
    justify-content: center;
    align-items: center;
}
.fs_14 {
    font-size: 14px;
}
.activeert {
    color: #1a1a1a !important;
    font-weight: 700 !important;
}
.activeert::before {
    content: "";
    position: absolute;
    width: 36px;
    height: 3px;
    left: 50%;
    transform: translate(-50%);
    bottom: -1px;
    background: #1a1a1a;
}
.deleteWeigh {
    font-weight: 500 !important;
    border-radius: 4px;
}
.joinStatus-color {
    color: var(--general);
}
.getjoinNumCount {
    color: #666;
    font-size: 14px;
    font-weight: 400;
}
// FormatPrice 小数位对齐（与 MemberOrderList 同源修复：跨境 .price-row 有 baseline，o2o FormatPrice 缺此层，局部兜底不影响其他页面）
:deep(.price) {
    align-items: baseline;
}
</style>
