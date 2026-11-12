<template>
    <div class="product-total" :class="{ 'new-ui': uiVersion === 'new' }">
        <div class="product-total-content">
            <div class="product-total-left">
                <div class="total-count">
                    <template v-if="isOverseasUi">{{ productAmount?.count }} {{ $t("件") }}</template>
                    <template v-else>{{ $t("合计") }}{{ productAmount?.count }}{{ $t("件") }}</template>
                </div>
                <div class="total-price">
                    <span>{{ $t("商品金额") }}：</span>
                    <FormatPrice v-model="productAmount.total" :font-style="{ fontSize: '20px', color: 'var(--general)' }"></FormatPrice>
                </div>
            </div>
            <div class="product-has-select-button" :class="{ active: showMore }" @click="showMore = !showMore">
                {{ $t("已选清单") }}
                <span class="iconfont-pc icon-xiajiantou"></span>
            </div>
        </div>

        <Transition name="fadey" @after-enter="onAfterEnter" @leave="onLeave">
            <div class="select-list" :class="{ transform: isEnter }" v-show="showMore">
                <table class="table-product" width="100%" cellspacing="1" cellpadding="3">
                    <thead>
                        <tr>
                            <th class="attr-th" v-for="attr in selectSkus[0].skuData">{{ attr.name }}</th>
                            <th class="attr-th">{{ $t("数量") }}</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr class="attr-tr" v-for="(item, key) in selectSkus" :key="item">
                            <td v-for="attr in item.skuData">{{ attr.value }}</td>
                            <td>{{ item.num }}</td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </Transition>
    </div>
</template>

<script setup lang="ts">
import { computed, unref, inject } from "vue";
import type { ProductAmountItem } from "@/types/product/product.d.ts";
// 跨境版 UI 开关（由商详页 Product.vue provide 下发；非跨境时默认原逻辑）
const detailUiVersion = inject<any>("detailUiVersion", null);
const isOverseasUi = computed(() => unref(detailUiVersion) === "overseas");
const props = defineProps({
    productAmount: {
        type: Object as PropType<ProductAmountItem>,
        default: () => ({ count: 0, total: 0 })
    },
    selectSkus: {
        type: Array as PropType<any>
    },
    // UI 版本：old 老版，new 新版
    uiVersion: {
        type: String,
        default: "old"
    }
});

const showMore = ref(false);
const isEnter = ref(false);
const onAfterEnter = () => {
    isEnter.value = true;
};
const onLeave = () => {
    isEnter.value = false;
};
</script>

<style lang="less" scoped>
.fadey-enter-from {
    opacity: 0;
    transform: translateY(-80%);
}
.fadey-enter-active {
    transition: all 0.3s ease-in-out;
}
.fadey-enter-to {
    opacity: 1;
    transform: translateY(-100%);
}

.fadey-leave-from {
    transform: translateY(-100%);
    opacity: 1;
}
.fadey-leave-active {
    transition: all 0.3s ease-in-out;
}
.fadey-leave-to {
    opacity: 0;
    transform: translateY(-80%);
}

.product-total {
    position: relative;
    margin-bottom: 25px;
    .select-list {
        position: absolute;
        left: 0;
        top: 0;
        width: 100%;
        height: auto;
        z-index: 2;
        background-color: #fff;
        &.transform {
            transform: translateY(-100%);
        }

        thead {
            .attr-th {
                font-weight: bold;
            }
        }
        .table-product td,
        .table-product th {
            border: 1px solid #d2d2d2 !important;
            line-height: 26px;
            padding: 5px 5px;
            text-align: center;
        }

        .table-product th {
            min-width: 80px;
        }

        .table-product td {
            background: #fff;
        }
    }

    .product-total-content {
        background: #f5f5f5;
        padding-left: 12px;
        position: relative;
        z-index: 5;
        display: flex;
        align-items: baseline;
    }
    .product-total-left {
        display: flex;
        align-items: baseline;
        flex-grow: 1;

        .total-count {
            color: #666;
            line-height: 56px;
        }

        .total-price {
            height: 56px;
            color: #666;
            margin-left: 19px;
            display: flex;
            justify-content: center;
            align-items: center;
            margin-right: 36px;
        }
    }

    .product-has-select-button {
        &::before {
            content: "";
            display: block;
            width: 1px;
            height: 40px;
            background: #e5e5e5;
            position: absolute;
            left: 0;
            top: 50%;
            transform: translateY(-50%);
        }

        font-size: 14px;
        width: 148px;
        height: 56px;
        cursor: pointer;
        display: flex;
        justify-content: center;
        align-items: center;
        position: relative;

        .icon-xiajiantou {
            font-size: 12px;
            margin-left: 5px;
            transition: transform 0.3s;
            transform: rotate(180deg) scale(0.8);
        }

        &.active {
            .icon-xiajiantou {
                transform: rotate(0deg) scale(0.8);
            }
        }
    }

    // 新版 UI：圆角卡片风格，与商品信息区一致
    &.new-ui {
        // max-width: 490px;
        font-family: Inter, "SF Pro Text", Roboto, "Helvetica Neue", Helvetica, Tahoma, Arial, "PingFang SC", "Microsoft YaHei";

        .product-total-content {
            background: #f5f5f7;
            padding-left: 16px;
            border-radius: 12px;
            overflow: hidden;
        }

        .product-total-left {
            .total-count {
                color: #999;
                font-size: 14px;
                font-weight: 500;
            }

            .total-price {
                color: rgba(9, 9, 10, 0.92);
                font-size: 14px;
                font-weight: 500;
            }
        }

        .product-has-select-button {
            color: rgba(9, 9, 10, 0.92);
            font-weight: 500;

            &::before {
                background: #e5e5e5;
            }
        }
    }
}
</style>
