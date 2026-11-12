<!--
  积分换购商品列表页面
  - 展示所有可用积分兑换的商品
  - 显示商品原价、积分价（金额+积分组合）
  - 支持分页浏览
  - 点击商品跳转到积分兑换商品详情页
-->
<template>
    <CommonPageHeader></CommonPageHeader>
    <CommonHeader :title="$t('{0}换购', [commonStore.integralName])"></CommonHeader>
    <CommonPageContainer>
        <!-- <div class="exchange-tit">
            <img src="@/assets/images/exchange_tit.png" alt="" />
        </div> -->
        <div class="exchange-list">
            <div class="goods-list-c">
                <ul class="flex-wrap">
                    <li class="tab-box g_list" v-for="item in filterState">
                        <div :title="item.productName" class="nuxt" style="display: flex; align-items: center; justify-content: space-between">
                            <div class="g-item-warp">
                                <NuxtLink class="pic" :to="urlFormat({ path: 'exchange', id: item.id })" target="_blank">
                                    <span class="exchange-list__pic">
                                        <el-image style="width: 250px; height: 250px" :src="productImage(item)" fit="fill" :alt="item.productName || ''" />
                                        <img
                                            v-if="styleMap[item.productId]?.borderImage"
                                            class="product-style-border"
                                            :src="styleMap[item.productId].borderImage"
                                            :alt="item.productName || ''"
                                        />
                                    </span>
                                </NuxtLink>
                                <div class="info">
                                    <p class="name ellipsis">
                                        <NuxtLink :to="urlFormat({ path: 'exchange', id: item.id })" target="_blank">
                                            {{ item.productName }}
                                        </NuxtLink>
                                    </p>
                                    <p class="market_price">{{ $t("原价") }}：<FormatPrice v-model="item.productPrice"></FormatPrice></p>
                                    <p class="exchange_price">
                                        {{ $t("{0}价", [commonStore.integralName]) }}：<FormatPrice v-model="item.discountsPrice"></FormatPrice> +&nbsp;<span>{{
                                            item.exchangeIntegral
                                        }}</span>
                                        {{ commonStore.integralName }}
                                    </p>
                                    <NuxtLink :to="urlFormat({ path: 'exchange', id: item.id })" target="_blank">
                                        <p class="exchange_btn">
                                            {{ $t("立即兑换") }}
                                        </p>
                                    </NuxtLink>
                                </div>
                            </div>
                        </div>
                    </li>
                </ul>
            </div>
        </div>
        <div class="search-empty" v-if="filterState.length === 0 && !loading">
            <div class="bd">
                <h4>{{ $t("暂时没有符合条件的商品！") }}</h4>
            </div>
        </div>
        <div class="el-page" v-if="filterState.length > 0 && !loading">
            <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
        </div>
    </CommonPageContainer>
    <CommonPageFooter :type="2" bg="#fff"></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 积分换购商品列表页面
 * - 展示所有可用积分兑换的商品
 * - 显示商品原价、积分价（金额+积分）
 * - 支持分页浏览
 * - 点击商品跳转到积分兑换商品详情页
 */
import { reactive, ref, onMounted } from "vue";
import { Pagination } from "~/components/list";
import { productImage, urlFormat } from "@/utils/format";
import { getExchangeList } from "~/api/exchange/exchange";
import type { ExchangeFilterParams, ExchangeFilterState } from "~/types/exchange/exchange.d";
import { useCommonStore } from "~/store/common";
import { useProductStyle } from "~/hooks";

const commonStore = useCommonStore();

const filterState = ref(<ExchangeFilterState[]>[]);
const { styleMap } = useProductStyle(filterState);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<ExchangeFilterParams>({
    //初使化用于查询的参数
    page: 1,
    size: 20,
    sortField: "id",
    sortOrder: "desc",
    isEnabled: 1
});
const loadFilter = async () => {
    filterState.value = [];
    loading.value = true;
    try {
        const result = await getExchangeList({ ...filterParams });
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
</script>

<style scoped lang="less">
.el-page {
    display: flex;
    justify-content: flex-end;
    padding: 20px 0;
}
.exchange-tit {
    text-align: center;
    padding-top: 20px;
    img {
        width: 80%;
    }
}
.exchange-list {
    padding: 20px 0;
    .goods-list-c {
        .tab-box {
            background-color: #fff;
            padding: 15px;
            border: 2px solid rgba(0, 0, 0, 0);
            margin: 0 13px 14px 0;
            transition: all 0.5s;
            &:hover {
                box-shadow: 0px 0px 10px 1px rgba(0, 0, 0, 0.2);
            }
            .g-item-warp {
                .pic {
                    margin-bottom: 5px;
                }
                .exchange-list__pic {
                    position: relative;
                    display: inline-block;
                    vertical-align: top;
                }
                .product-style-border {
                    position: absolute;
                    left: 0;
                    top: 0;
                    width: 100%;
                    height: 100%;
                    z-index: 5;
                    pointer-events: none;
                    object-fit: fill;
                }
                .info {
                    .name {
                        width: 250px;
                        margin-bottom: 5px;
                    }
                    .market_price {
                        margin-bottom: 5px;
                        :deep(.price) {
                            text-decoration: line-through;
                        }
                    }
                    .exchange_price {
                        margin-bottom: 5px;
                        :deep(.price) {
                            color: var(--general);
                            font-size: 20px;
                            line-height: 16px;
                            font-weight: bold;
                            .util {
                                font-weight: bold;
                                line-height: 20px;
                            }
                        }
                        span {
                            color: var(--general);
                            font-weight: bold;
                        }
                    }
                    .exchange_btn {
                        display: block;
                        width: 188px;
                        height: 28px;
                        color: var(--main-text);
                        font-size: 14px;
                        text-align: center;
                        line-height: 28px;
                        margin: 16px auto 0px;
                        border-width: 1px;
                        border-style: solid;
                        border-color: var(--main-bg);
                        border-image: initial;
                        background: var(--main-bg);
                        border-radius: 28px;
                        cursor: pointer;
                    }
                    .exchange_btn:hover {
                        background: var(--main-bg);
                    }
                }
            }
        }
    }
}
.search-empty {
    margin-bottom: 10px;
    position: relative;
    display: flex;
    justify-content: center;
    align-items: center;
    width: 100%;
    height: 200px;
}

.search-empty .bd {
    background: url("/assets/images/common/empty.gif") no-repeat scroll 56px 0px;
    padding: 45px 50px 45px 200px;
}

.search-empty h4 {
    font-size: 14px;
}
</style>
