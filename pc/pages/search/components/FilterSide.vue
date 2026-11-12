<template>
    <div class="filter-container">
        <div class="filter-box">
            <div class="name">{{ $t("筛选") }}</div>
        </div>
        <div v-if="showPriceRange" class="price-box" :class="{ border: filterList.brand.length > 0 }">
            <div class="name">{{ $t("价格区间") }}</div>
            <div class="price-container">
                <div class="price-txt">
                    <div class="min">
                        <FormatPrice
                            :model-value="price[0]"
                            :price-style="{ alignItems: 'baseline' }"
                            :currency-style="{ fontSize: '14px', color: '#1a1a1a' }"
                            :font-style="{ fontSize: '14px', color: '#1a1a1a' }"
                            :decimals-style="{ fontSize: '0px', color: '#1a1a1a' }"
                        ></FormatPrice>
                    </div>
                    <div class="max">
                        <FormatPrice
                            :model-value="price[1]"
                            :price-style="{ alignItems: 'baseline' }"
                            :currency-style="{ fontSize: '14px', color: '#1a1a1a' }"
                            :font-style="{ fontSize: '14px', color: '#1a1a1a' }"
                            :decimals-style="{ fontSize: '0px', color: '#1a1a1a' }"
                        ></FormatPrice>
                    </div>
                </div>
                <div class="price-slider">
                    <el-slider v-model="price" range :min="0" :max="sliderMaxSafe" :show-tooltip="false" @change="handleSliderChange" />
                    <el-button type="primary" class="btn" @click="onPriceChange">{{ $t("确定") }}</el-button>
                </div>
            </div>
        </div>
        <el-collapse v-if="filterList.brand.length > 0" v-model="brandCollapseActive">
            <el-collapse-item :title="$t('品牌')" :name="1">
                <template #icon="{ isActive }">
                    <div class="flex align-center">
                        <SvgIcon name="arrow" width="24px" height="24px" class="arrow-icon" :class="{ isActive }" />
                    </div>
                </template>
                <div class="brand-box">
                    <div class="tag-container" :class="{ show: isShowMore }">
                        <el-checkbox-group v-model="multipleBrandIds" @change="selectBrand">
                            <template v-for="(item, index) in filterList.brand" :key="index">
                                <el-checkbox :label="item.brandName" :value="String(item.id)" />
                            </template>
                        </el-checkbox-group>
                    </div>
                    <div v-if="filterList.brand.length > 5" class="show-more" @click="isShowMore = !isShowMore">
                        {{ isShowMore ? $t("收起") : $t("更多") }}
                        <SvgIcon name="arrow" width="14px" height="14px" class="icon" :class="{ rotate: isShowMore }" />
                    </div>
                </div>
            </el-collapse-item>
        </el-collapse>
        <el-collapse v-if="filterList.attrs.length > 0" v-model="collapseActive">
            <el-collapse-item v-for="(attr, index) in filterList.attrs" :key="attr.name" :title="$t(attr.name)" :name="index + 1">
                <template #icon="{ isActive }">
                    <div class="flex align-center">
                        <SvgIcon name="arrow" width="24px" height="24px" class="arrow-icon" :class="{ isActive }" />
                    </div>
                </template>
                <div class="brand-box">
                    <div class="tag-container1">
                        <template v-for="(item, valueIndex) in attr.values" :key="`${attr.name}-${valueIndex}`">
                            <el-checkbox v-model="item.checked" :label="$t(item.value)" :value="item.value" @change="changeAttr" />
                        </template>
                    </div>
                </div>
            </el-collapse-item>
        </el-collapse>
    </div>
</template>

<script lang="ts" setup>
import { computed, ref, watch, type PropType, type Ref } from "vue";

import { useSearchFilterData } from "./useSearchFilterData";
import { buildSelectedAttrs } from "./searchFilterUtils";
import type { SearchFilter, filterSeleted, QueryParams } from "./types.d";

const props = defineProps({
    queryParams: {
        type: Object as PropType<QueryParams>,
        default: () => ({})
    },
    pageType: {
        type: String,
        default: "search"
    },
    showPriceRange: {
        type: Boolean,
        default: true
    }
});

const emit = defineEmits(["change"]);

const filter = defineModel<SearchFilter>("filter", { default: () => ({ brandIds: [], attrs: [] }) }) as Ref<SearchFilter>;
const filterSeletedModel = defineModel<filterSeleted | undefined>("filterSeleted");
const filterLoading = defineModel<boolean>("filterLoading", { default: false });

const route = useRoute();

const { filterList, filterSeleted, sliderMax, maxPrice, loading, fetchFilter, fetchEsFilter } = useSearchFilterData();

// ===================== 本地展示态 =====================
const brandCollapseActive = ref([1]);
const collapseActive = ref([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
const isShowMore = ref(false);
const price = ref<number[]>([0, 0]);
const multipleBrandIds = ref<string[]>([...(filter.value.brandIds ?? [])]);

const sliderMaxSafe = computed(() => (sliderMax.value > 0 ? sliderMax.value : 1));

// ===================== 事件处理 =====================
const handleSliderChange = (value: number | number[]): void => {
    if (Array.isArray(value)) {
        price.value = [...value];
    }
};

const onPriceChange = (): void => {
    filter.value.minPrice = price.value[0] ?? 0;
    filter.value.maxPrice = price.value[1] ?? 0;
    filter.value.page = 1;
    emit("change");
};

const selectBrand = (): void => {
    filter.value.brandIds = [...multipleBrandIds.value];
    filter.value.page = 1;
    emit("change");
};

const changeAttr = (): void => {
    filter.value.attrs = buildSelectedAttrs(filterList.attrs) as unknown as SearchFilter["attrs"];
    filter.value.page = 1;
    emit("change");
};

// ===================== 同步与副作用 =====================
watch(loading, (value) => {
    filterLoading.value = value;
});

watch(filterSeleted, (value) => {
    filterSeletedModel.value = value;
});

// 已选品牌回填（路由/父级变更时）
watch(
    () => filter.value.brandIds,
    (ids) => {
        multipleBrandIds.value = [...(ids ?? [])];
    }
);

// 筛选项只在搜索上下文变化时拉取，不因品牌/价格/排序重拉
watch(
    () => [props.queryParams?.keyword, props.queryParams?.cat, props.queryParams?.couponId, props.pageType, props.queryParams?.searchToken] as const,
    async () => {
        await fetchFilter({
            keyword: props.queryParams?.keyword,
            cat: props.queryParams?.cat,
            couponId: props.queryParams?.couponId,
            searchToken: props.queryParams?.searchToken,
            page_type: props.pageType
        });
        // 对齐跨境：getFilter 成功后写入 filter.maxPrice，点价格排序时 URL 才会带 max
        const routeMin = route.query.min ? Number(route.query.min) : 0;
        const routeMax = route.query.max ? Number(route.query.max) : 0;
        price.value = [routeMin, routeMax > 0 ? routeMax : maxPrice.value];
        filter.value.minPrice = routeMin > 0 ? routeMin : 0;
        filter.value.maxPrice = routeMax > 0 ? routeMax : maxPrice.value || 0;
    },
    { immediate: true }
);

watch(
    () => [filter.value.keyword, props.queryParams?.cat, props.queryParams?.searchToken] as const,
    () => {
        fetchEsFilter({
            keyword: props.queryParams?.keyword ?? filter.value.keyword,
            cat: props.queryParams?.cat,
            searchToken: props.queryParams?.searchToken
        });
    },
    { immediate: true }
);
</script>

<style lang="less" scoped>
.filter-container {
    box-sizing: border-box;
    width: 240px;
    border: 1px solid #e0e1e5;
    background: #fff;
    border-radius: 8px;
    padding: 12px;
    align-self: flex-start;
    position: sticky;
    top: 32px;
    max-height: calc(100vh - 32px - 72px);
    overflow: auto;

    :deep(.el-checkbox__inner) {
        border-width: 2px;
        border-radius: 2.5px;
        height: 18px;
        width: 18px;

        &:hover {
            border-color: #1a1a1a;
        }
    }

    :deep(.el-checkbox__input.is-checked .el-checkbox__inner) {
        background-color: #1a1a1a;
        border-color: #1a1a1a;
    }

    :deep(.el-checkbox__inner::after) {
        width: 4px;
        height: 8px;
        border-width: 2px;
    }

    .filter-box {
        display: flex;
        justify-content: space-between;
        margin-top: 14px;
        padding-bottom: 10px;
        border-bottom: 1.5px solid #f0f1f2;

        .name {
            font-size: 16px;
            color: #1a1a1a;
            font-weight: 700;
        }
    }

    .price-box {
        margin-top: 16px;
        padding-bottom: 10px;

        &.border {
            border-bottom: 1.5px solid #f0f1f2;
        }

        .name {
            font-size: 16px;
            color: #1a1a1a;
            font-weight: 700;
        }

        .price-container {
            margin-top: 28px;

            .price-txt {
                display: flex;
                justify-content: space-between;
                padding-right: 36px;
            }

            .price-slider {
                display: flex;
                justify-content: space-between;
                align-items: center;
                padding-left: 10px;

                .btn {
                    background: #1a1a1a;
                    border-color: #1a1a1a;
                    width: 31px;
                    height: 26px;
                    margin-left: 15px;
                    font-size: 12px;
                }

                :deep(.el-slider__button) {
                    border-color: #1a1a1a;
                }

                :deep(.el-slider__bar) {
                    background: #1a1a1a;
                }
            }
        }
    }

    .brand-box {
        margin-top: 16px;
        border-bottom: 1.5px solid #f0f1f2;
        padding-bottom: 15px;

        .name {
            font-size: 16px;
            color: #1a1a1a;
            font-weight: 700;
            text-transform: capitalize;
        }

        .tag-container {
            display: flex;
            flex-wrap: nowrap;
            margin-top: 16px;
            max-height: 165px;
            overflow: hidden;

            &.show {
                height: fit-content;
                max-height: fit-content;
                overflow-y: auto;
            }
        }

        .tag-container1 {
            margin-top: 16px;
            margin-bottom: 10px;
        }

        .show-more {
            cursor: pointer;
            display: flex;
            align-items: center;
            margin-top: 16px;
            font-size: 14px;
            color: #1a1a1a;

            .icon {
                color: #888b94;
                transition: all 0.2s;
                margin-left: 4px;
                margin-top: -4px;

                &.rotate {
                    transform: rotate(180deg);
                }
            }
        }
    }

    :deep(.el-checkbox) {
        display: flex;
        align-items: center;
        height: 21px;
        margin-bottom: 15px;

        &:last-child {
            margin-bottom: 0;
        }

        .el-checkbox__label {
            font-size: 14px;
            color: #1a1a1a;
            line-height: 21px;
            width: 100%;
            overflow: hidden;
            text-overflow: ellipsis;
        }

        &.is-checked {
            .el-checkbox__label {
                font-size: 14px;
                color: #1a1a1a;
                width: 100%;
                overflow: hidden;
                text-overflow: ellipsis;
            }
        }
    }

    :deep(.el-collapse-item__title) {
        flex: none;
        font-size: 16px;
        color: #1a1a1a;
        font-weight: 700;
        text-transform: capitalize;
    }

    :deep(.el-collapse-item__content) {
        padding-bottom: 0;
    }

    :deep(.el-collapse-item__wrap) {
        border-bottom: none;
    }

    :deep(.el-collapse-item__header) {
        border-bottom: none;
    }

    :deep(.el-collapse) {
        border-top: none;
    }

    .arrow-icon {
        transition: transform 0.3s ease;
        transform-origin: center;

        &.isActive {
            transform: rotate(180deg);
        }
    }
}
</style>
