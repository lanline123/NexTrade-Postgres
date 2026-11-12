<template>
    <!-- 跨境版 UI：大图规格 + 无勾选图标 -->
    <div class="product-sku-info-overseas" v-if="isOverseasUi && attrList.length > 0">
        <div class="sku-item flex align-start" v-for="(item, index) in attrList">
            <div class="tit">{{ item.attrName }}：</div>
            <div class="sku-tag flex-wrap">
                <template v-for="(value, i) in item.attrList">
                    <div
                        :class="{
                            tag: true,
                            checked: value.checked,
                            disabled: !value.checked && value.skuStock == 0,
                            outstock: value.skuStock == 0
                        }"
                        @click="onChecked(index, i)"
                    >
                        <div v-if="index === 0 && value.attrPic">
                            <img :src="imageFormat(value.attrPic)" alt="" />
                            <p class="attrWidth">{{ value.attrValue }}</p>
                        </div>
                        <div v-else>
                            <p style="margin: 7px">{{ value.attrValue }}</p>
                        </div>
                    </div>
                </template>
            </div>
        </div>
    </div>
    <!-- 新版 UI：带图规格支持「列表模式」与「大图模式」切换 -->
    <div class="product-sku-info new-ui" v-else-if="uiVersion === 'new' && attrList.length > 0">
        <div class="sku-item" v-for="(item, index) in attrList" :key="index">
            <div class="sku-header">
                <div class="tit">{{ item.attrName }}</div>
                <div class="view-toggle" v-if="groupHasPic(item) && index === firstPicGroupIndex" @click="toggleViewMode">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <rect x="1" y="1" width="6" height="6" rx="1.5" stroke="currentColor" stroke-width="1.4" />
                        <rect x="9" y="1" width="6" height="6" rx="1.5" stroke="currentColor" stroke-width="1.4" />
                        <rect x="1" y="9" width="6" height="6" rx="1.5" stroke="currentColor" stroke-width="1.4" />
                        <rect x="9" y="9" width="6" height="6" rx="1.5" stroke="currentColor" stroke-width="1.4" />
                    </svg>
                    <span>{{ viewMode === "list" ? $t("切换大图模式") : $t("切换列表模式") }}</span>
                </div>
            </div>
            <!-- 大图模式（带图） -->
            <div class="sku-grid big" v-if="groupHasPic(item) && viewMode === 'big'">
                <div
                    v-for="(value, i) in item.attrList"
                    :key="i"
                    :class="{
                        'sku-card': true,
                        big: true,
                        checked: value.checked,
                        disabled: !value.checked && value.skuStock == 0,
                        outstock: value.skuStock == 0
                    }"
                    @click="onChecked(index, i)"
                >
                    <div class="card-img">
                        <img :src="imageFormat(value.attrPic || value.attrPicThumb)" alt="" />
                    </div>
                    <p class="card-label">{{ value.attrValue }}</p>
                </div>
            </div>
            <!-- 列表模式（带图） -->
            <div class="sku-grid list" v-else-if="groupHasPic(item)">
                <div
                    v-for="(value, i) in item.attrList"
                    :key="i"
                    :class="{
                        'sku-card': true,
                        list: true,
                        checked: value.checked,
                        disabled: !value.checked && value.skuStock == 0,
                        outstock: value.skuStock == 0
                    }"
                    @click="onChecked(index, i)"
                >
                    <div class="card-thumb">
                        <img :src="imageFormat(value.attrPicThumb || value.attrPic)" alt="" />
                    </div>
                    <p class="card-label">{{ value.attrValue }}</p>
                </div>
            </div>
            <!-- 无图规格：普通标签 -->
            <div class="sku-tag flex-wrap" v-else>
                <template v-for="(value, i) in item.attrList" :key="i">
                    <div
                        :class="{
                            tag: true,
                            checked: value.checked,
                            disabled: !value.checked && value.skuStock == 0,
                            outstock: value.skuStock == 0
                        }"
                        @click="onChecked(index, i)"
                    >
                        <p>{{ value.attrValue }}</p>
                    </div>
                </template>
            </div>
        </div>
    </div>
    <!-- 旧版 UI：保持原有展示 -->
    <div class="product-sku-info" v-else-if="attrList.length > 0">
        <div class="sku-item flex align-start" v-for="(item, index) in attrList">
            <div class="tit">{{ item.attrName }}：</div>
            <div class="sku-tag flex-wrap">
                <template v-for="(value, i) in item.attrList">
                    <div
                        :class="{
                            tag: true,
                            checked: value.checked,
                            disabled: !value.checked && value.skuStock == 0,
                            outstock: value.skuStock == 0
                        }"
                        @click="onChecked(index, i)"
                    >
                        <img v-if="value.attrPic" :src="imageFormat(value.attrPic)" alt="" />
                        <p>{{ value.attrValue }}</p>
                        <i class="icon-gou"></i>
                    </div>
                </template>
            </div>
        </div>
    </div>
</template>

<script setup>
import { computed, ref, unref, inject } from "vue";
import { imageFormat } from "@/utils/format";
const emit = defineEmits(["change"]);
// 跨境版 UI 开关（由商详页 Product.vue provide 下发；非跨境时默认原逻辑）
const detailUiVersion = inject("detailUiVersion", null);
const isOverseasUi = computed(() => unref(detailUiVersion) === "overseas");
// 向主图区同步用户选中的规格图（商详页外的复用场景下为空操作）
const writeSpecPic = useSpecPicWriter();
// 与地址栏 skuId 同步（商详页外的复用场景下为空操作）
const skuUrl = useSkuUrlSync();

const props = defineProps({
    modelValue: {
        type: Object,
        default: {}
    },
    skuList: {
        type: Array
    },
    checkedValue: {
        type: Array
    },
    productType: {
        type: String,
        default: "product"
    },
    // UI 版本：old 老版，new 新版（控制带图规格的列表/大图模式切换）
    uiVersion: {
        type: String,
        default: "old"
    }
});

const attrList = computed(() => props.modelValue.spe || []);

// 规格展示模式：list 列表模式，big 大图模式（仅新版 UI 带图规格生效）
const skuViewModeCookie = useCookie("skuViewMode", { default: () => "big" });
const viewMode = ref(skuViewModeCookie.value === "list" ? "list" : "big");
const toggleViewMode = () => {
    viewMode.value = viewMode.value === "list" ? "big" : "list";
    skuViewModeCookie.value = viewMode.value;
};
// 判断该规格组是否存在图片
const groupHasPic = (item) => item.attrList.some((value) => value.attrPic || value.attrPicThumb);
// 首个有图规格组的索引（切换按钮仅在此组展示，避免多组重复渲染切换按钮）
const firstPicGroupIndex = computed(() => attrList.value.findIndex((item) => groupHasPic(item)));

const checkedValue = ref(props.checkedValue);
// 各规格组当前选中的 attributesId，用于同名规格值区分
const checkedAttrIds = ref([]);

/**
 * 规格 attributesId 组合 ↔ SKU 映射。
 * 同名规格值导致 skuValue 重复时，按笛卡尔积顺序与 SKU 列表出现顺序一一对应。
 */
const skuSelectionMaps = computed(() => {
    const buckets = new Map();
    (props.skuList || []).forEach((skuItem) => {
        const key = String(skuItem.skuValue || "");
        if (!key) return;
        const bucket = buckets.get(key);
        if (bucket) bucket.push(skuItem);
        else buckets.set(key, [skuItem]);
    });

    const cursor = new Map();
    const consumeFromBucket = (key) => {
        const bucket = buckets.get(key);
        if (!bucket || bucket.length === 0) return undefined;
        const pos = cursor.get(key) ?? 0;
        cursor.set(key, pos + 1);
        return bucket[Math.min(pos, bucket.length - 1)];
    };

    const map = new Map();
    const attrIdsBySkuId = new Map();

    const walk = (groupIndex, selectedAttrs, tokens) => {
        if (groupIndex >= attrList.value.length) {
            if (tokens.length === 0) return;
            const sku = consumeFromBucket(tokens.join("|"));
            if (!sku) return;
            const attrIds = selectedAttrs.map((attr) => Number(attr.attributesId));
            map.set(attrIds.join("|"), sku);
            attrIdsBySkuId.set(Number(sku.skuId), attrIds);
            return;
        }
        const group = attrList.value[groupIndex];
        const list = Array.isArray(group?.attrList) ? group.attrList : [];
        if (list.length === 0) {
            walk(groupIndex + 1, selectedAttrs, tokens);
            return;
        }
        list.forEach((attr) => {
            walk(groupIndex + 1, [...selectedAttrs, attr], [...tokens, `${attr.attrName}:${attr.attrValue}`]);
        });
    };

    if (attrList.value.length > 0) walk(0, [], []);
    return { map, attrIdsBySkuId };
});

// 优先按 attributesId 组合定位 SKU，回退到 skuValue 字符串匹配
const findSkuByCheckedAttrIds = (selectedAttrIds, selectedValues) => {
    const validAttrIds = (selectedAttrIds || []).filter((id) => id != null && Number(id) > 0);
    if (validAttrIds.length > 0 && validAttrIds.length === attrList.value.length) {
        const matched = skuSelectionMaps.value.map.get(validAttrIds.join("|"));
        if (matched) return matched;
    }
    const values = selectedValues || checkedValue.value;
    return props.skuList.find((item) => item.skuValue == values.join("|"));
};

// 根据 SKU 反查各规格组应选中的 attributesId
const getCheckedAttrIdsBySku = (skuItem) => {
    if (!skuItem?.skuId) return attrList.value.map(() => null);
    const mappedIds = skuSelectionMaps.value.attrIdsBySkuId.get(Number(skuItem.skuId));
    if (mappedIds && mappedIds.length === attrList.value.length) return mappedIds;
    const tokens = String(skuItem.skuValue || "").split("|");
    return attrList.value.map((group, index) => {
        const token = tokens[index];
        const attr = (group.attrList || []).find((item) => `${item.attrName}:${item.attrValue}` === token);
        return attr?.attributesId != null ? Number(attr.attributesId) : null;
    });
};

// 由当前 checkedValue 字符串推导 checkedAttrIds
const syncCheckedAttrIdsFromValue = () => {
    checkedAttrIds.value = attrList.value.map((group, index) => {
        const token = checkedValue.value[index];
        const attr = (group.attrList || []).find((item) => `${item.attrName}:${item.attrValue}` === token);
        return attr?.attributesId != null ? Number(attr.attributesId) : null;
    });
};

const updateSelectedValue = () => {
    if (attrList.value.length > 0) {
        if (checkedValue.value.length == 0) {
            if (isOverseasUi.value) {
                // 跨境版：默认选中首个有货 sku
                let index = props.skuList.findIndex((item) => item.skuStock > 0);
                index = index > -1 ? index : 0;
                checkedValue.value = props.skuList[index].skuValue.split("|");
                checkedAttrIds.value = getCheckedAttrIdsBySku(props.skuList[index]);
            } else {
                attrList.value.forEach((item) => {
                    checkedValue.value.push(item.attrList[0].attrName + ":" + item.attrList[0].attrValue);
                });
                checkedAttrIds.value = attrList.value.map((item) => (item.attrList[0]?.attributesId != null ? Number(item.attrList[0].attributesId) : null));
            }
        } else if (checkedAttrIds.value.length !== checkedValue.value.length) {
            syncCheckedAttrIdsFromValue();
        }
        attrList.value.forEach((item, index) => {
            const checkedAttrId = checkedAttrIds.value[index];
            item.attrList.forEach((value) => {
                const attrNameValue = value.attrName + ":" + value.attrValue;
                const attrId = value.attributesId != null ? Number(value.attributesId) : null;
                // 优先按 attributesId 判断选中，避免同名规格值同时高亮
                value.checked = attrId != null && checkedAttrId != null ? checkedAttrId === attrId : checkedValue.value[index] == attrNameValue;

                const trialAttrIds = [...checkedAttrIds.value];
                const trialValues = JSON.parse(JSON.stringify(checkedValue.value));
                trialAttrIds[index] = attrId;
                trialValues[index] = attrNameValue;
                const result = findSkuByCheckedAttrIds(trialAttrIds, trialValues);
                value.skuStock = result ? result.skuStock : 0;
            });
        });
        const selectedItem = findSkuByCheckedAttrIds(checkedAttrIds.value, checkedValue.value);
        emit("change", {
            skuStock: (selectedItem && selectedItem.skuStock) || 0,
            skuId: (selectedItem && selectedItem.skuId) || 0
        });
    } else if (props.skuList.length > 0) {
        emit("change", {
            skuStock: props.skuList[0].skuStock || 0,
            skuId: props.skuList[0].skuId || 0
        });
    } else {
        emit("change", null);
    }
};
// 地址栏带 skuId 时按该 SKU 预选；按 SKU 反查 attributesId，同名规格值也能选准
const urlSku = skuUrl.urlSkuId() ? (props.skuList || []).find((item) => String(item.skuId) === skuUrl.urlSkuId()) : null;
if (urlSku && attrList.value.length > 0) {
    // 原地替换：checkedValue 与商详页共用同一数组，页签切换重挂时沿用
    checkedValue.value.splice(0, checkedValue.value.length, ...String(urlSku.skuValue || "").split("|"));
    checkedAttrIds.value = getCheckedAttrIdsBySku(urlSku);
}
updateSelectedValue();

// 优先使用本次点击的规格图；该规格无图时，再回退到其他已选规格图
const getCheckedAttrPic = (preferredAttr) => {
    const preferredPic = preferredAttr?.attrPic || preferredAttr?.attrPicThumb;
    if (preferredPic) return preferredPic;

    for (let index = 0; index < attrList.value.length; index++) {
        const list = attrList.value[index]?.attrList;
        if (!Array.isArray(list)) continue;
        const checkedAttrId = checkedAttrIds.value[index];
        // 优先按 attributesId 定位，避免同名规格值取错
        const attr =
            checkedAttrId != null
                ? list.find((value) => Number(value.attributesId) === Number(checkedAttrId))
                : list.find((value) => value.attrName + ":" + value.attrValue === checkedValue.value[index]);
        const attrPic = attr?.attrPic || attr?.attrPicThumb;
        if (attrPic) return attrPic;
    }
    return "";
};

const onChecked = (index, i) => {
    if (props.productType != "product") {
        return;
    }
    const item = attrList.value[index].attrList[i];
    checkedValue.value[index] = item.attrName + ":" + item.attrValue;
    checkedAttrIds.value[index] = item.attributesId != null ? Number(item.attributesId) : null;
    updateSelectedValue();
    writeSpecPic(getCheckedAttrPic(item));
    skuUrl.syncSkuId(findSkuByCheckedAttrIds(checkedAttrIds.value, checkedValue.value)?.skuId);
};

onMounted(() => {
    updateSelectedValue();
});
</script>
<style lang="less" scoped>
.outstock {
    color: #c5c5c5 !important;
}
.product-sku-info {
    padding: 15px 1px 0 0;

    .sku-item {
        margin-bottom: 25px;

        .tit {
            color: #999;
            height: 33px;
            line-height: 32px;
            padding-right: 10px;
        }

        .sku-tag {
            flex: 1;
            gap: 10px;
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

// 新版 UI：带图规格的列表/大图模式
.product-sku-info.new-ui {
    // 顶部分割线：与门店卡片区分隔
    border-top: 1px solid #ebebeb;
    padding: 24px 0 0 !important;

    // 新版选中态不显示右下角对勾，仅用边框/文字色标识
    .icon-gou {
        display: none !important;
    }

    .sku-item {
        margin-bottom: 12px;
    }

    .sku-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 12px;

        .tit {
            height: auto;
            line-height: 22px;
            padding-right: 0;
            color: #999;
            font-size: 14px;
            font-weight: 500;
        }

        .view-toggle {
            display: flex;
            align-items: center;
            gap: 4px;
            color: var(--general);
            font-size: 14px;
            font-weight: 500;
            cursor: pointer;
            user-select: none;

            svg {
                display: block;
            }

            &:hover {
                opacity: 0.85;
            }
        }
    }

    // 列表模式：两列卡片
    .sku-grid.list {
        display: flex;
        flex-direction: row;
        flex-wrap: wrap;

        .sku-card {
            box-sizing: border-box;
            display: flex;
            align-items: center;
            height: 40px;
            max-width: calc(100% - 8px);
            min-width: 48px;
            margin: 0 8px 12px 0;
            gap: 10px;
            padding: 0 12px 0 4px;
            border: 1px solid rgba(31, 31, 51, 0.1);
            border-radius: 8px;
            background-color: #fff;
            cursor: pointer;
            position: relative;
            transition: border-color 0.2s;

            .card-thumb {
                flex-shrink: 0;
                width: 32px;
                height: 32px;
                border-radius: 8px;
                overflow: hidden;
                background-color: #f5f5f5;

                img {
                    width: 100%;
                    height: 100%;
                    object-fit: cover;
                    display: block;
                }
            }

            .card-label {
                flex: 1;
                font-size: 14px;
                line-height: 20px;
                color: rgba(9, 9, 10, 0.92);
                display: -webkit-box;
                -webkit-line-clamp: 2;
                line-clamp: 2;
                -webkit-box-orient: vertical;
                overflow: hidden;
            }
        }
    }

    // 大图模式：自适应多列卡片
    .sku-grid.big {
        display: flex;
        flex-wrap: wrap;

        .sku-card {
            align-items: stretch;
            background-color: #fff;
            border: none;
            border-radius: 8px;
            box-sizing: border-box;
            color: rgba(9, 9, 10, 0.92);
            cursor: pointer;
            display: flex;
            flex-direction: column;
            font-weight: 400;
            height: auto;
            margin: 0 8px 12px 0;
            width: calc((100% - 40px) / 5);
            position: relative;
            overflow: hidden;

            &::after {
                content: "";
                position: absolute;
                inset: 0;
                border: 1px solid rgba(31, 31, 51, 0.1);
                border-radius: 8px;
                pointer-events: none;
                transition: border-color 0.2s;
            }

            .card-img {
                display: block;
                width: 100%;
                aspect-ratio: 1 / 1;
                overflow: hidden;
                background-color: #f5f5f5;

                img {
                    width: 100%;
                    height: 100%;
                    object-fit: cover;
                    display: block;
                }
            }

            .card-label {
                margin: 0;
                padding: 4px 8px;
                width: calc(100% - 16px);
                text-align: center;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
                word-break: keep-all;
                color: rgba(9, 9, 10, 0.92);
                font-size: 14px;
            }

            &.checked::after {
                border-width: 1.5px;
                border-color: var(--general);
            }
        }
    }

    // 选中态
    .sku-card.checked {
        border-color: var(--general) !important;

        .card-label {
            color: var(--general) !important;
        }

        .icon-gou {
            display: block !important;
        }
    }

    // 缺货态
    .sku-card.disabled {
        border-style: dashed;

        .card-label {
            color: #999 !important;
        }
    }

    .sku-card.outstock {
        .card-label {
            color: #c5c5c5 !important;
        }
    }

    // 无图规格标签
    .sku-tag {
        display: flex;
        flex-direction: row;
        flex-wrap: wrap;
        gap: 0px;

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

// 跨境版规格样式，独立类名隔离，避免与 o2o 样式互相污染
.product-sku-info-overseas {
    padding: 15px 1px 0 0;

    .sku-item {
        margin-bottom: 25px;

        .tit {
            color: #999;
            height: 33px;
            line-height: 32px;
            padding-right: 10px;
        }

        .sku-tag {
            flex: 1;
            gap: 10px;
            .tag {
                background-color: #ffffff;
                border: 1px solid #e7e7e7;
                color: #333;
                cursor: pointer;
                img {
                    width: 80px;
                    height: 80px;
                    object-fit: cover;
                    padding: 2px;
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

    .attrWidth {
        width: 100%;
        line-height: 26px;
        font-weight: bold;
        text-align: center;
        font-size: 12px;
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
    }
}
</style>
