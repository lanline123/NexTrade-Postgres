<template>
    <!-- 新版 UI：与 SKU 新版无图标签风格一致 -->
    <div class="product-sku-info new-ui" v-if="uiVersion === 'new' && modelValue && modelValue.extra && modelValue.extra.length > 0">
        <div class="sku-item" v-for="(item, index) in modelValue.extra" :key="index">
            <div class="sku-header">
                <div class="tit">{{ item.attrName }}</div>
            </div>
            <div class="sku-tag flex-wrap">
                <template v-for="(subItem, i) in item.attrList" :key="subItem.attributesId">
                    <div
                        @click.stop="handleAttrClick(index, subItem.attributesId)"
                        :class="{
                            tag: true,
                            checked: checkedAttrs[index] === subItem.attributesId
                        }"
                    >
                        <p>{{ subItem.attrValue }}</p>
                        <FormatPrice class="price" v-model="subItem.attrPrice"></FormatPrice>
                    </div>
                </template>
            </div>
        </div>
    </div>
    <!-- 旧版 UI：保持原有展示（跨境版复用此分支，仅文案走 i18n 且标签带间距） -->
    <div class="product-sku-info" :class="{ 'attr-overseas': isOverseasUi }" v-else-if="modelValue && modelValue.extra && modelValue.extra.length > 0">
        <div class="sku-item flex align-start" v-for="(item, index) in modelValue.extra" :key="index">
            <div class="tit">{{ item.attrName }}：</div>
            <div class="sku-tag flex-wrap">
                <template v-for="(subItem, i) in item.attrList" :key="subItem.attributesId">
                    <div
                        @click.stop="handleAttrClick(index, subItem.attributesId)"
                        :class="{
                            tag: true,
                            checked: checkedAttrs[index] === subItem.attributesId
                        }"
                    >
                        <p>{{ isOverseasUi ? $t(subItem.attrValue) : subItem.attrValue }}</p>
                        <FormatPrice class="price" v-model="subItem.attrPrice"></FormatPrice>
                        <i class="icon-gou"></i>
                    </div>
                </template>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, unref, inject } from "vue";
import type { AttrList } from "~/types/product/product.d";
const emit = defineEmits(["change"]);
// 跨境版 UI 开关（由商详页 Product.vue provide 下发；非跨境时默认原逻辑）
const detailUiVersion = inject("detailUiVersion", null);
const isOverseasUi = computed(() => unref(detailUiVersion) === "overseas");

const props = defineProps({
    modelValue: {
        type: Object as PropType<AttrList>,
        default: () => ({})
    },
    // UI 版本：old 老版，new 新版
    uiVersion: {
        type: String,
        default: "old"
    }
});

const checkedAttrs = ref<any[]>([]);
const handleAttrClick = (index: number, id: number | string) => {
    if (checkedAttrs.value[index] && checkedAttrs.value[index] === id) {
        checkedAttrs.value[index] = null;
    } else {
        checkedAttrs.value[index] = id;
    }
    emit("change", checkedAttrs.value.length > 0 ? checkedAttrs.value.filter((item) => !!item).join(",") : "");
};
</script>
<style lang="less" scoped>
.outstock {
    color: #c5c5c5 !important;
}
.product-sku-info {
    padding: 0px 1px 0 0;
    margin-bottom: 25px;
    .sku-item {
        margin-bottom: 5px;
        .tit {
            margin-bottom: 0.625rem;
            height: 33px;
            line-height: 32px;
            padding-right: 10px;
            color: #999;
        }

        .price {
            padding-left: 10px;
        }

        .sku-tag {
            flex: 1;
            // gap: 10px;
            .tag {
                background-color: #ffffff;
                border: 1px solid #e7e7e7;
                color: #333;
                height: 30px;
                line-height: 32px;
                padding: 0 6px;
                cursor: pointer;
                min-width: 30px;
                text-align: center;
                position: relative;
                justify-content: center;
                display: flex;
                align-items: center;
                img {
                    width: 20px;
                    height: 20px;
                    margin-right: 5px;
                }
            }

            .checked {
                border-color: var(--general);
                color: #333;

                .icon-gou {
                    display: block;
                }
            }

            .disabled {
                border: 1px dashed #eee;
                color: #999;
            }
        }
    }
}

// 跨境版：属性标签之间保留间距
.product-sku-info.attr-overseas {
    .sku-item .sku-tag {
        gap: 10px;
    }
}

// 新版 UI：与 SKU 新版无图标签风格保持一致
.product-sku-info.new-ui {
    padding: 0;
    margin-bottom: 24px;

    .sku-item {
        margin-bottom: 24px;

        &:last-child {
            margin-bottom: 0;
        }
    }

    .sku-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 12px;

        .tit {
            height: auto;
            margin-bottom: 0;
            line-height: 22px;
            padding-right: 0;
            color: #999;
            font-size: 14px;
            font-weight: 500;
        }
    }

    .sku-tag {
        display: flex;
        flex-wrap: wrap;

        .tag {
            align-items: center;
            background-color: #fff;
            border: 1px solid rgba(31, 31, 51, 0.1);
            border-radius: 8px;
            box-sizing: border-box;
            color: rgba(9, 9, 10, 0.92);
            cursor: pointer;
            display: flex;
            flex-direction: row;
            font-weight: 400;
            height: 40px;
            justify-content: center;
            margin: 0 8px 12px 0;
            max-width: calc(100% - 8px);
            min-width: 48px;
            padding: 0 14px;
            position: relative;
            overflow: hidden;
            font-size: 14px;
            line-height: 1;

            p {
                margin: 0;
            }
        }

        .price {
            padding-left: 8px;
        }

        .checked {
            border-color: var(--general);
            color: var(--general);

            .icon-gou {
                display: block;
            }
        }

        .disabled {
            border-style: dashed;
            color: #999;
        }
    }
}
</style>
