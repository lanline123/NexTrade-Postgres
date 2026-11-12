<template>
    <CommonPageHeaderNew></CommonPageHeaderNew>
    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="title-or-tabs">
                <span>{{ $t("我的团购") }}</span>
            </div>
            <div class="page-info-body">
                <div class="pin-list">
                    <div class="tag-and-input">
                        <div class="tig-tabs">
                            <div v-for="item in orderTypeList" :class="orderType == item.type ? 'active' : ''" class="tab" @click="changeType(item.type)">
                                <span v-if="item.length">{{ item.length }}</span>
                                <p>{{ $t(item.value) }}</p>
                            </div>
                        </div>
                        <div class="input">
                            <input class="tig-input" :placeholder="$t('订单号或商品名称')" type="text" />
                            <div class="tig-button ml10">{{ $t("搜索") }}</div>
                        </div>
                    </div>
                    <div class="order-title">
                        <div class="ul">
                            <div style="width: 40%">
                                <MemberOrderSelect v-model="time" :selectList="orderTimeList"></MemberOrderSelect>
                            </div>
                            <div style="width: 12%">{{ $t("数量") }}</div>
                            <div style="width: 12%">{{ $t("金额") }}</div>
                            <div style="width: 12%">{{ $t("团购状态") }}</div>
                            <div style="width: 12%">{{ $t("订单状态") }}</div>
                            <div style="width: 12%">{{ $t("操作") }}</div>
                        </div>
                    </div>
                    <template v-if="tableData > 0">
                        <table v-for="item in 3" class="order-table pale-green">
                            <thead>
                                <tr>
                                    <th colspan="4">
                                        {{ $t("团购订单号") }}：<span class="red">20240130570732{{ item }}</span>
                                        <span style="margin-left: 30px">{{ $t("创团时间") }}：2024-01-30 13:47:32</span>
                                        <span style="margin-left: 30px">{{ $t("创团类型") }}：{{ $t("建团（团长）") }}</span>
                                    </th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr class="order-tr">
                                    <td class="column-1 wid-1">
                                        <div v-for="product in productList">
                                            <!--                                        <img alt="" src="https://oss.lyecs.com/img/item/demo/1681981128uYMtOlcTmoQ2hWRTd0!!pic200x200.jpeg">-->
                                            <span>{{ product }}</span>
                                            <div>
                                                <ul style="line-height: 1.5">
                                                    <li>
                                                        <FormatPrice v-model="price" :fontStyle="{ color: '#aaa' }"></FormatPrice>
                                                    </li>
                                                    <li style="color: #aaa">X2</li>
                                                </ul>
                                            </div>
                                        </div>
                                    </td>
                                    <td class="column-2-4">
                                        <div>
                                            <div>
                                                {{ $t("总额") }}:
                                                <FormatPrice v-model="price" :fontStyle="{ color: '#aaa' }"></FormatPrice>
                                            </div>
                                        </div>
                                    </td>
                                    <td class="column-2-4">
                                        <div>
                                            <div style="color: #149c24">{{ $t("已成团，正在进行") }}</div>
                                            <div>{{ $t("剩余") }}：1906502340</div>
                                        </div>
                                    </td>
                                    <td class="column-2-4">
                                        <div>
                                            <div style="color: #149c24">{{ $t("已确认付款") }}</div>
                                            <div class="black"><a href="/member/order/info">{{ $t("订单详情") }}</a></div>
                                        </div>
                                    </td>
                                    <td class="column-2-4">
                                        <div>
                                            <div class="black"><a href="/member/pin/group" target="_blank">{{ $t("团购详情") }}</a></div>
                                        </div>
                                    </td>
                                </tr>
                            </tbody>
                        </table>
                    </template>
                    <template v-else>
                        <div class="no-result">
                            <div>
                                <span class="sign sign1"></span>
                            </div>
                            <div class="tip-message">
                                <ul>
                                    <li style="font-size: 16px; font-weight: 700">{{ $t("当前筛选下还没有任何订单哦!") }}</li>
                                    <li>{{ $t("现在就去") }}：<a class="font-color">{{ $t("选购商品") }}</a></li>
                                </ul>
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
 * @description 拼团订单列表页面
 * - 展示用户参与的拼团订单
 * - 支持按状态筛选（全部/未确认/进行中/已取消/已成功/已失败）
 * - 支持按时间范围筛选
 */
import { reactive, ref, shallowRef, onMounted, onUnmounted } from "vue";
const emit = defineEmits(["change"]);

const tableData = ref(3);
const price = ref("9118.40");
const time = ref(4);
const orderTimeList = reactive([
    { value: 3, label: "最近三个月订单" },
    { value: 2, label: "最近六个月订单" },
    { value: 1, label: "最近一年内的订单" },
    { value: 4, label: "全部订单" }
]);

const orderType = ref(0);
const orderTypeList = reactive<any>([
    { type: 0, value: "全部拼团订单" },
    { type: 2, value: "未确认", length: 1 },
    { type: 3, value: "已成团，正在进行", length: 3 },
    { type: 5, value: "已取消", length: 0 },
    { type: 6, value: "已成功", length: 0 },
    { type: 7, value: "已失败", length: 0 }
]);
const changeType = (type: number) => {
    orderType.value = type;
};

const pay = ref(0);
const payTypeList = reactive([
    { value: 0, label: "交易状态" },
    { value: 2, label: "未支付" },
    { value: 1, label: "已支付" }
]);

const handleChange = () => {};

const productList = ref([
    "赛八仙贡品芽尖2023新茶毛尖茶叶信阳绿茶明前特级嫩芽高端毛尖茶",
    "TCL空调挂机新一级能效三级 大1匹1.5匹变频冷暖智能挂式客厅空调",
    "潜艇级净化丨508升法式四开门双系统冰箱家用一级节能变频无霜",
    "80 GT 5G【蓝牙无线耳机套餐】智能手机"
]);
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.pin-list {
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

    .pale-green {
        background-color: #e5f8e0 !important;
    }

    .order-table {
        color: #666;
        border: 1px solid #eee;
        width: 100%;
        background-color: #f7f7f7;
        height: 40px;
        line-height: 40px;

        & > thead > tr > th {
            padding-left: 10px;
        }

        .order-tr {
            background-color: white;

            .column-1 {
                width: 52%;
                border: 1px solid #eee;

                & > div {
                    padding: 15px;
                    box-sizing: border-box;
                    display: flex;
                    align-items: center;
                    flex-direction: row;
                    border-bottom: 1px solid #eee;
                    width: 100%;

                    & > img {
                        max-width: 40px;
                        min-width: 40px;
                        max-height: 40px;
                        min-height: 40px;
                    }

                    & > span {
                        flex: 1;
                        margin-left: 10px;
                        line-height: 2;
                    }

                    & > div {
                        margin-left: 10px;
                        width: 60px;
                        text-align: center;
                    }
                }

                & > div:last-child {
                    border-bottom: none;
                }
            }

            .column-2-4 {
                width: 12%;
                border: 1px solid #eee;

                & > div {
                    display: flex;
                    height: 100%;
                    padding: 20px 0;
                    box-sizing: border-box;
                    flex-direction: column;
                    justify-content: flex-start;
                    align-items: center;
                    color: #aaaaaa;

                    & > .black {
                        color: black;
                    }

                    & > .money {
                        text-align: center;
                        width: 80%;
                        border-bottom: 1px solid #e5e5e5;
                    }

                    & > div {
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
</style>
