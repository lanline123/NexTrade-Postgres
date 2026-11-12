<template>
    <div :class="isOverseasUi ? 'b2b-sku-overseas' : ['b2b-sku', { 'new-ui': uiVersion === 'new' }]">
        <div class="b2b-sku-content" :style="{ height: height }">
            <template v-for="item in b2bSkuList" :key="item">
                <div class="sku-item">
                    <div class="title">{{ item.title }}</div>
                    <div class="price-box">
                        <FormatPrice :fontStyle="{ fontSize: '13px', color: '#666', 'font-weight': 700, 'white-space': 'nowrap' }" v-model="item.price"></FormatPrice>
                        <div class="stock">
                            <template v-if="isOverseasUi">{{ $t("库存") }}：{{ item.stock }}</template>
                            <template v-else>{{ item.stock }}{{ $t("件可售") }}</template>
                        </div>
                    </div>
                    <div class="num">
                        <InputNumber v-model="item.num" :max="item.stock" :min="0" :rounded="uiVersion === 'new'" @change="handleNumberChange"></InputNumber>
                    </div>
                </div>
            </template>
        </div>
        <template v-if="props.b2bSkuList.length > 4 && !show">
            <img @click="show = !show" class="sku-wrapper-expend-button" src="@/assets/images/product/pulldown_sku.png" alt="" srcset="" />
        </template>
    </div>
</template>

<script setup lang="ts">
import { computed, ref, unref, inject } from "vue";
import { InputNumber } from "@/components/product";
import { getProductAmount } from "@/api/product/product";
import type { ProductAmountItem, SkuList } from "@/types/product/product.d.ts";

// 跨境版 UI 开关（由商详页 Product.vue provide 下发；非跨境时默认原逻辑）
const detailUiVersion = inject("detailUiVersion", null);
const isOverseasUi = computed(() => unref(detailUiVersion) === "overseas");

const props = defineProps({
    b2bSkuList: {
        type: Array as PropType<SkuList[]>,
        default: []
    },
    id: {
        type: Number
    },
    skuItem: {
        type: Array,
        default: []
    },
    selectSkus: {
        type: Array,
        default: []
    },
    // UI 版本：old 老版，new 新版
    uiVersion: {
        type: String,
        default: "old"
    }
});
const productAmount = defineModel<ProductAmountItem>("productAmount", {
    type: Object as PropType<ProductAmountItem>,
    default: () => ({
        count: 0,
        total: "0"
    })
});
const show = ref(false);
const height = computed(() => {
    const rowHeight = props.uiVersion === "new" ? 69 : 63;
    let length = props.b2bSkuList.length;
    let num = length < 5 ? length * rowHeight : 4 * rowHeight;

    if (length > 4 && show.value) {
        num = length * rowHeight;
    }
    return num + "px";
});

const __getProductAmount = async () => {
    try {
        const result = await getProductAmount({
            id: props.id,
            skuItem: props.skuItem
        });
        productAmount.value = result;
    } catch (error) {
        console.error(error);
    }
};

watchEffect(() => {
    __getProductAmount();
});

const emit = defineEmits(["update:skuItem", "update:selectSkus"]);

let delayTimer: NodeJS.Timeout | null = null; // 延时定时器
const handleNumberChange = () => {
    if (delayTimer) {
        clearTimeout(delayTimer);
    }
    delayTimer = setTimeout(() => {
        const skus = props.b2bSkuList.filter((item: any) => item.num);
        const data = skus.map((item: any) => {
            return {
                skuId: item.skuId,
                num: item.num
            };
        });
        emit("update:skuItem", data);
        const data2 = skus.map((item: any) => {
            return {
                skuData: item.skuData,
                num: item.num
            };
        });
        emit("update:selectSkus", data2);
    }, 300);
};
</script>

<style lang="less" scoped>
.b2b-sku {
    margin-bottom: 35px;
    position: relative;
    .b2b-sku-content {
        overflow: hidden;
        transition: height 0.3s ease-in-out;
    }
    .sku-item {
        display: flex;
        align-items: center;
        padding: 15px 0;
        border-bottom: 1px solid #f0f0f0;
        font-size: 13px;
        color: #666;
        word-break: break-all;
        font-weight: 700;
        .title {
            flex: 2;
        }
        .price-box {
            flex: 1;
            display: flex;
            align-items: center;
            .stock {
                padding-left: 20px;
                font-weight: 500;
                color: #999;
            }
        }
        .num {
            flex: 1;
        }
    }
    .sku-wrapper-expend-button {
        cursor: pointer;
        width: 60px;
        height: 20px;
        position: absolute;
        bottom: -18px;
        left: 88px;
    }

    // 新版 UI：与商品信息区一致的现代风格，行宽撑满
    &.new-ui {
        width: 100%;
        margin-bottom: 24px;
        font-family: Inter, "SF Pro Text", Roboto, "Helvetica Neue", Helvetica, Tahoma, Arial, "PingFang SC", "Microsoft YaHei";

        .sku-item {
            display: flex;
            align-items: center;
            gap: 16px;
            padding: 14px 0;
            border-bottom: 1px solid #f0f0f0;
            color: rgba(9, 9, 10, 0.92);
            font-size: 14px;
            font-weight: 500;

            .title {
                flex: 1;
                min-width: 0;
                word-break: break-all;
            }

            .price-box {
                flex: 0 0 auto;
                display: flex;
                flex-direction: row;
                align-items: center;
                gap: 12px;
                white-space: nowrap;

                .stock {
                    padding-left: 0;
                    color: #999;
                    font-size: 12px;
                    font-weight: 500;
                    white-space: nowrap;
                }
            }

            .num {
                flex: 0 0 auto;
                margin-left: auto;
            }
        }
    }
}

// 跨境版 B2B 规格样式，独立根类名隔离
.b2b-sku-overseas {
    margin-bottom: 35px;
    position: relative;
    .b2b-sku-content {
        overflow: hidden;
        transition: height 0.3s ease-in-out;
    }
    .sku-item {
        display: flex;
        align-items: center;
        padding: 15px 0;
        border-bottom: 1px solid #f0f0f0;
        color: #666;
        word-break: break-all;
        font-weight: 700;
        .title {
            width: 150px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: normal;
            line-height: 1.4;
            display: -webkit-box;
            -webkit-line-clamp: 2;
            -webkit-box-orient: vertical;
            margin-right: 20px;
        }
        .price-box {
            flex: 1;
            display: flex;
            align-items: center;
            .stock {
                padding-left: 20px;
                padding-right: 20px;
                font-weight: 500;
                color: #999;
                white-space: nowrap;
            }
        }
        .num {
            :deep(.el-input-number) {
                width: 98px;
                border: none;
            }
            :deep(.el-input-number__decrease) {
                border-right: none !important;
            }
            :deep(.el-input-number__increase) {
                border-left: none !important;
            }
            :deep(.el-input__wrapper) {
                box-shadow: none !important;
                background: #f7f8fc;
            }
        }
    }
    .sku-wrapper-expend-button {
        cursor: pointer;
        width: 60px;
        height: 20px;
        position: absolute;
        bottom: -18px;
        left: 88px;
    }
}
</style>
