<template>
    <div class="goods-list__box" ref="productBox">
        <div class="goods-list-c">
            <div class="wrap">
                <div class="item" v-for="(item, index) in productList" :key="index">
                    <NuxtLink :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.productSn })" target="_blank">
                        <div class="g-item-warp">
                            <div class="pic">
                                <div class="pic-img">
                                    <el-image
                                        class="ease"
                                        lazy
                                        :src="productImage(item)"
                                        style="transition: opacity 0.2s linear"
                                        :alt="item.productName || ''"
                                    />
                                    <img
                                        v-if="styleMap[item.productId]?.borderImage"
                                        class="product-style-border"
                                        :src="styleMap[item.productId].borderImage || undefined"
                                        alt=""
                                    />
                                </div>
                                <div class="countdown">
                                    <div class="title">{{ $t("拼团") }}</div>
                                    <div class="countdown-time" v-if="item.countdownText">
                                        <div class="time">{{ $t("距结束") }}</div>
                                        <div class="text">{{ item.countdownText }}</div>
                                    </div>
                                    <div class="countdown-time" v-else>
                                        <div class="time">{{ $t("已结束") }}</div>
                                    </div>
                                </div>
                            </div>
                            <div class="titleBox">
                                <div class="tag">{{ getMaxGroupNum(item.groupNum) }}{{ $t("人团") }}</div>
                                <div class="title">{{ item.productName }}</div>
                            </div>
                            <!--                        <div class="describe">-->
                            <!--                            此处显示商品描述，最多显示一行此处显示商品描述，最多显示一行-->
                            <!--                        </div>-->
                            <div class="priceBox">
                                <div class="priceBoxNum">
                                    <FormatPrice
                                        class="minGrouponPrice"
                                        :decimalsStyle="{ fontSize: '14px', alignSelf: 'self-end' }"
                                        v-model="item.minGrouponPrice"
                                    ></FormatPrice>
                                    <div class="old">¥{{ item.productPrice }}</div>
                                </div>
                                <div
                                    class="text"
                                    :title="item.canJoin === false ? item.restrictTip || $t('仅限指定会员参与') : ''"
                                    :style="item.canJoin === false ? { background: '#b7b7b7' } : {}"
                                >
                                    {{ item.canJoin === false ? $t("仅限会员") : $t("拼") }}
                                </div>
                            </div>
                        </div>
                    </NuxtLink>
                </div>
                <div class="search-empty" v-if="productList.length === 0 && !loading">
                    <div class="bd">
                        <h4>{{ $t("好狠的筛选条件啊，把相关的商品都埋没了！") }}</h4>
                        <p>{{ $t("建议您") }}</p>
                        <p>{{ $t("1、适当减少筛选条件") }}</p>
                        <p>{{ $t("2、调整价格区间") }}</p>
                        <p>{{ $t("3、尝试其他关键字") }}</p>
                        <el-button class="graybtn mt10" @click="goBack">{{ $t("返回上一步") }}</el-button>
                    </div>
                </div>
            </div>
        </div>
    </div>
    <div class="pagination">
        <Pagination v-if="joiningStore.total > 0" v-model:page="filter.page" :size="size" :total="joiningStore.total" @callback="pageChange" />
    </div>
</template>
<script lang="ts" setup>
import { ref, onMounted } from "vue";
import { useCommonStore } from "@/store/common";
import { urlFormat, productImage, getCountdownFormat } from "@/utils/format";
import { useJoiningStore } from "@/store/joining";
import { Pagination } from "@/components/list";
import { useRouter } from "vue-router";
import type { SearchFilter, JoiningItem } from "@/types/joining/joining.d";
import { getProductList } from "@/api/joining/joining";
import { useProductStyle } from "~/hooks";

const { t } = useI18n();
const commonStore = useCommonStore();

const props = defineProps({
    attrs: {
        type: Array,
        default: () => []
    }
});
const joiningStore = useJoiningStore();
const router = useRouter();
const loading = ref(true);
const size = ref(25);
const emit = defineEmits(["change", "pageChange"]);
const filter = defineModel<SearchFilter>("filter") as Ref<SearchFilter>;
const queryParams = defineModel("queryParams");
const loadingProduct = defineModel("loadingProduct");
const productList = ref<JoiningItem[]>([]);
const { styleMap } = useProductStyle(productList);
let countdownTimer: number | null = null;

const loadFilter = async () => {
    loading.value = true;
    try {
        // 避免多次请求导致定时器叠加
        if (countdownTimer) {
            clearInterval(countdownTimer);
            countdownTimer = null;
        }
        const qp = ((queryParams.value ?? {}) as Record<string, any>) || {};
        const result = await getProductList({ ...qp, page: filter.value.page, size: size.value });
        productList.value = result.records;
        joiningStore.total = result.total;
        updateCountdown();
        countdownTimer = window.setInterval(() => {
            updateCountdown();
        }, 1000);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

const productBox: Ref<HTMLElement | null> = ref(null);
const pageChange = () => {
    emit("change", filter.value.page);
    emit("pageChange");
};

const goBack = () => {
    router.replace({
        path: router.currentRoute.value.path,
        query: {}
    });
};

// 获取groupNum的最大值
const getMaxGroupNum = (groupNumStr: string | undefined): number => {
    if (!groupNumStr) return 0;
    try {
        const arr = JSON.parse(groupNumStr);
        if (Array.isArray(arr) && arr.length > 0) {
            return Math.max(...arr.map((item: any) => item.groupNum));
        }
        return 0;
    } catch {
        return 0;
    }
};

// 更新倒计时显示
const updateCountdown = () => {
    productList.value.forEach((item) => {
        if (item.endTime) {
            item.countdownText = getCountdownFormat(new Date(item.endTime).getTime(), t("天"));
        }
    });
};

onMounted(() => {
    loadFilter();
});

// 清理定时器
onUnmounted(() => {
    if (countdownTimer) {
        clearInterval(countdownTimer);
    }
});
</script>
<style lang="less" scoped>
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

.goods-list-c {
    width: 100%;
    overflow: hidden;

    .wrap {
        display: flex;
        flex-wrap: wrap;
        min-height: 333px;
        gap: 18px;

        .item {
            border: 1px solid #fff;
            padding: 20px;
            box-sizing: border-box;
            transition: border-color 0.1s ease 0s;
            position: relative;
            background: #fff;
            width: calc((100% - 18px * 4) / 5);
            height: 408px;

            .pic {
                .pic-img {
                    position: relative;
                    display: flex;
                    justify-content: center;
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
                .ease {
                    width: 100%;
                    max-width: 248.67px;
                    height: 248.67px;
                    vertical-align: bottom;

                    :deep(.el-image__inner) {
                        object-fit: contain;
                    }
                }
            }

            .pic:hover {
                transform: translateY(-10px);
                /* 鼠标悬停时向上平移 10px */
                transition: transform 0.3s ease;
            }

            .pic:not(:hover) {
                transform: translateY(0);
                transition: transform 0.3s ease;
            }

            .countdown {
                display: flex;
                justify-content: space-between;
                align-items: center;
                background: linear-gradient(90deg, #f86e30, #e91817);
                height: 34px;
                padding: 0 10px;
                box-sizing: border-box;
                font-size: 10px;
                color: #ffffff;

                .countdown-time {
                    display: flex;
                    align-items: center;
                    justify-content: flex-end;
                    flex: 1;
                    gap: 6px;
                    white-space: nowrap;
                }

                .title {
                    font-weight: bold;
                    font-size: 16px;
                    color: #ffffff;
                }
            }

            .titleBox {
                display: flex;
                align-items: center;
                gap: 5px;
                margin-top: 11px;

                .tag {
                    font-weight: 500;
                    font-size: 12px;
                    color: var(--general);
                    min-width: 40px;
                    height: 20px;
                    background: var(--ump-tag-bg);
                    border-radius: 4px;
                    line-height: 20px;
                    text-align: center;
                    flex-shrink: 0;
                    padding: 0 5px;
                    box-sizing: border-box;
                }

                .title {
                    flex: 1;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    white-space: nowrap;
                    font-weight: 500;
                    font-size: 14px;
                    color: #000000;
                }
            }

            .describe {
                font-weight: 500;
                font-size: 12px;
                color: #9a9a9a;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
                margin-top: 8px;
            }

            .priceBox {
                width: 100%;
                height: 35px;
                background: #fff2f3;
                border-radius: 3px;
                margin-top: 18px;
                display: flex;
                align-items: center;
                position: relative;
                .priceBoxNum {
                    display: flex;
                    align-items: flex-end;
                    .old {
                        font-weight: 500;
                        font-size: 12px;
                        color: #9a9a9a;
                        text-decoration-line: line-through;
                        margin-left: 7px;
                        align-items: flex-end;
                    }
                    .minGrouponPrice {
                        color: var(--price);
                        font-weight: 500;
                        font-size: 22px;
                        padding-left: 9px;
                        height: 26px;
                    }
                }

                &:before {
                    content: "";
                    position: absolute;
                    left: -23px;
                    top: -11px;
                    width: calc(100% + 46px);
                    height: 1px;
                    // background: #f6f6f6;
                }

                .text {
                    width: 56px;
                    height: 30px;
                    background: #ee0a25;
                    border-radius: 3px;
                    font-weight: 600;
                    font-size: 20px;
                    color: #ffffff;
                    line-height: 30px;
                    margin-left: auto;
                    clip-path: polygon(20% 0, 100% 0, 100% 100%, 0 100%);
                    text-align: center;
                }
            }
        }

        @media (max-width: 1599px) {
            gap: 16px;
            .item {
                width: calc((100% - 16px * 3) / 4);
            }
        }
    }
}

.search-empty {
    margin-bottom: 10px;
    position: relative;
    display: flex;
    justify-content: center;
    width: 100%;
}

.search-empty .bd {
    background: url("/assets/images/common/empty.gif") no-repeat scroll 56px 45px;
    padding: 45px 50px 45px 200px;
}

.search-empty h4 {
    font-size: 14px;
    padding-bottom: 10px;
}

.search-empty p {
    color: #666666;
    margin-bottom: 5px;
}

.pagination {
    display: flex;
    justify-content: end;
    margin-top: 10px;
    margin-bottom: 20px;
}
</style>
