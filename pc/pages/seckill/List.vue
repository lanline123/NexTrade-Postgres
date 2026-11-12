<!--
  限时秒杀商品列表页面
  - 展示所有秒杀商品，包含秒杀价格、销量进度条
  - 商品售罄时显示"已抢完"状态
  - 支持分页浏览
  - 点击商品跳转到商品详情页参与秒杀
-->
<template>
    <div style="background-color: #f9f9f9">
        <CommonHeader title="限时秒杀"></CommonHeader>
        <CommonPageHeader></CommonPageHeader>
        <div class="seckill-con">
            <div class="slogan"></div>
            <CommonPageContainer>
                <!--                <div class="time">-->
                <!--                    <div v-for="(item, index) in timeList" :class="filterParams.unStarted === index ? 'be-being' : 'have-not-begun'" class="react">-->
                <!--                        <div @click="switchingTime(item)" :class="filterParams.unStarted === index ? 'be-being-text' : 'have-not-begun-text'" class="text">-->
                <!--                            <span>{{ item.time }}</span>-->
                <!--                            <span class="right-text">-->
                <!--                                <template v-if="filterParams.unStarted > index">&ndash;&gt;-->
                <!--                                    <div class="jjks">已结束</div>&ndash;&gt;-->
                <!--                                </template>&ndash;&gt;-->
                <!--                                <template v-else-if="filterParams.unStarted === index"> 正在秒杀 </template>&ndash;&gt;-->
                <!--                                <template v-else>&ndash;&gt;-->
                <!--                                    <div class="jjks">即将开始</div>&ndash;&gt;-->
                <!--                                </template>&ndash;&gt;-->
                <!--                            </span>-->
                <!--                        </div>-->
                <!--                    </div>-->
                <!--                </div>-->
                <div class="seckill-product-ist">
                    <ul>
                        <li v-for="item in filterState" class="seckill_mod_product">
                            <NuxtLink
                                :title="item.productName"
                                :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.skuSn || item.productSn })"
                                class="nuxt"
                                target="_blank"
                            >
                                <span class="seckill-list__pic">
                                    <el-image :src="productImage(item)" class="pro_pic" loading="lazy" style="transition: opacity 0.2s linear" :alt="item.productName || ''" />
                                    <img
                                        v-if="styleMap[item.productId]?.borderImage"
                                        class="product-style-border"
                                        :src="styleMap[item.productId].borderImage"
                                        :alt="item.productName || ''"
                                    />
                                </span>
                                <div>
                                    <div class="text-ellipsis">{{ item.productName }}</div>
                                    <div class="tips">{{ $t("限时秒杀 抢先提醒") }}</div>
                                </div>
                            </NuxtLink>
                            <div class="operation">
                                <div class="price">
                                    <div class="price1">
                                        <FormatPrice v-model="item.seckillPrice" :fontStyle="{ fontSize: '24px', color: 'var(--general' }"></FormatPrice>
                                    </div>
                                    <div class="sell">
                                        <div>{{ $t("已售") }}{{ Math.round((item.seckillSales / ((item.seckillStock + item.seckillSales) || 1)) * 100) }}%</div>
                                        <div class="progress-bar">
                                            <div
                                                class="progress"
                                                :style="{
                                                    width: '' + Math.round((item.seckillSales / ((item.seckillStock + item.seckillSales) || 1)) * 100) + '%'
                                                }"
                                            ></div>
                                        </div>
                                    </div>
                                </div>
                                <div
                                    v-if="item.canJoin === false"
                                    class="buy seckill_sold_out"
                                    :title="item.restrictTip || $t('仅限指定会员参与')"
                                >
                                    {{ $t("仅限会员") }}
                                </div>
                                <NuxtLink
                                    v-else-if="item.seckillStock > 0"
                                    :title="item.productName"
                                    :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.skuSn || item.productSn, shopId: item.shopId })"
                                    class=""
                                    target="_blank"
                                >
                                    <div class="buy">{{ $t("立即抢购") }}</div>
                                </NuxtLink>
                                <div v-else class="buy seckill_sold_out">{{ $t("已抢完") }}</div>
                            </div>
                        </li>
                    </ul>
                </div>
                <div class="el-page" v-if="filterState.length > 0 && !loading">
                    <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
                </div>
            </CommonPageContainer>
        </div>
    </div>
    <CommonPageFooter :mt30="false"></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 限时秒杀商品列表页面
 * - 展示所有秒杀商品，包含秒杀价格、销量进度条
 * - 商品售罄时显示"已抢完"状态
 * - 支持分页浏览
 * - 点击商品跳转到商品详情页参与秒杀
 */
import { urlFormat } from "~/utils/format";

import { computed, reactive, ref } from "vue";
import { useProductStyle } from "~/hooks";
import type { SeckillFilterState } from "~/types/home/seckill.d";
import { getHomeSeckill } from "~/api/home/home";
import { Pagination } from "~/components/list";

const { t } = useI18n();

// const timeList: any = ref([{ time: "正在秒杀" }, { time: "未开始" }]);

const filterState = ref(<SeckillFilterState[]>[]);
const { styleMap } = useProductStyle(filterState);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<any>({
    //初使化用于查询的参数
    page: 1
});

useSeo({
    title: computed(() => t("限时秒杀")),
    canonical: computed(() => "/seckill/list")
});

const loadFilter = async () => {
    loading.value = true;
    try {
        const result = await getHomeSeckill({ ...filterParams });
        filterState.value = result.records;
        total.value = result.total;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const switchingTime = (value: any) => {};
loadFilter();
</script>
<style lang="less" scoped>
.el-page {
    display: flex;
    justify-content: flex-end;
    padding: 20px 0;
}
.seckill-con {
    background: rgb(249, 249, 249) url("/assets/images/seckill/bg.png") no-repeat scroll center top;

    .slogan {
        width: 656px;
        height: 90px;
        margin: 0 auto;
        background: url("/assets/images/seckill/slogan2.png") no-repeat center;
    }

    padding-bottom: 10px;
}

.time {
    display: flex;
    height: 60px;
    margin-left: 30px;

    .react {
        cursor: pointer;
        height: 60px;
        line-height: 60px;
        text-align: center;
        transform: skewX(-45deg);
        display: flex;
        justify-content: center;
        align-items: center;

        .text {
            transform: skewX(45deg);
            display: flex;
            gap: 10px;
            font-weight: 700;
            justify-content: center;
            align-items: center;
        }

        .be-being-text {
            font-size: 20px;

            .right-text {
                font-size: 15px;
                display: flex;
            }
        }

        .have-not-begun-text {
            font-size: 14px;
        }

        .jjks {
            border: 1px solid #666;
            padding: 0 14px;
            vertical-align: middle;
            height: 26px;
            line-height: 25px;
            color: #666666;
            border-radius: 22px;
            font-size: 12px;
            display: inline-block;
        }
    }

    .be-being {
        width: 230px;
        background: #e01222;
        color: #ffffff;
    }

    .have-not-begun {
        width: 180px;
        background: #ffffff;
        color: #000;
    }

    .have-not-begun:hover {
        color: #e01222;
    }
}

.seckill-product-ist {
    margin: 30px auto;

    & > ul {
        display: flex;
        gap: 10px;
        flex-wrap: wrap;
    }

    .seckill_mod_product {
        background-color: #ffffff;
        width: 290px;
        height: 420px;
        display: flex;
        flex-direction: column;
        padding: 20px 0;
        box-sizing: border-box;

        .nuxt {
            padding-bottom: 4px;
            transition: transform 0.3s ease; /* 平移动画效果 */
            display: flex;
            flex-direction: column;
            align-items: center;

            .seckill-list__pic {
                position: relative;
                display: block;
                width: 230px;
                height: 230px;
            }

            .pro_pic {
                width: 230px;
                height: 230px;
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

            & > div {
                display: flex;
                flex-direction: column;
                justify-content: center;
                align-items: center;
                flex-wrap: nowrap;
                margin-top: 16px;

                .text-ellipsis {
                    font-size: 14px;
                    white-space: nowrap;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    width: 230px;
                    margin-bottom: 10px;
                }

                .tips {
                    width: 100%;
                    font-size: 14px;
                    color: var(--general);
                }
            }
        }

        .nuxt:hover {
            transform: translateY(-10px); /* 鼠标悬停时向上平移 10px */
        }

        .operation {
            border-top: 1px solid #f6f6f6;
            display: flex;
            margin: auto;
            width: 100%;
            padding: 10px 10px;
            box-sizing: border-box;
            flex-direction: row;
            align-items: center;
            justify-content: center;

            .price {
                width: 170px;

                .price1 {
                    width: 100%;
                }

                .sell {
                    display: flex;
                    align-items: center;
                    gap: 10px;

                    .progress-bar {
                        width: 88px; /* 进度条容器的总宽度 */
                        height: 8px; /* 进度条的高度 */
                        background-color: #b8b8b8; /* 进度条背景色 */
                        border-radius: 4px; /* 进度条容器的圆角 */
                        box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.2); /* 可选：为进度条添加内阴影效果 */
                    }

                    .progress {
                        height: 8px; /* 进度条的高度 */
                        background-color: var(--general); /* 进度条的颜色 */
                        border-radius: 4px; /* 进度部分的圆角 */
                        transition: width 0.4s ease; /* 进度变化时的过渡动画效果 */
                    }
                }
            }

            .buy {
                display: flex;
                justify-content: center;
                align-items: center;
                cursor: pointer;
                width: 80px;
                height: 40px;
                background-color: var(--general);
                color: var(--main-text);
                font-size: 16px;
                border-radius: 4px;
            }
            .seckill_sold_out {
                background-color: #b7b7b7;
            }
        }
    }
}
</style>
