<template>
    <Title>{{ headTitle }}</Title>
    <div class="filter__box">
        <div class="sear-title">
            <h3>
                <b v-if="_filterSeleted && _filterSeleted.category">{{ _filterSeleted.category }}</b>
                <em>{{ $t("商品筛选") }}</em>
            </h3>
            <div class="st-ext">
                {{ $t("共") }}&nbsp;<span> {{ searchStore.total }} </span>&nbsp;{{ $t("个商品") }}
            </div>
        </div>
        <div class="filter">
            <div class="list" v-if="filterList.category.length > 0">
                <div class="attr-name">{{ $t("分类：") }}</div>
                <div class="attr-box">
                    <div ref="categoryLinkRef" class="category-link" :class="{ 'category-link-more': categorydShowMore }">
                        <div class="item" v-for="(item, key) in filterList.category" :key="key" @click="onCategoryClick(item.categoryId)">
                            {{ item.categoryName }}
                        </div>
                    </div>
                    <span
                        class="attrMore"
                        :class="{ attrMoreUp: categorydShowMore }"
                        @click="categorydShowMore = !categorydShowMore"
                        v-if="categoryNeedMore"
                    >
                        {{ categorydShowMore ? $t("收起") : $t("更多") }}<i class="iconfont-pc icon-xiajiantou"></i>
                    </span>
                </div>
            </div>
            <div class="list brand_fitlter" v-if="filterList.brand.length > 0 && filter.brandIds.length === 0">
                <div class="attr-name">{{ $t("品牌：") }}</div>
                <div class="attr-box">
                    <div class="category-brand-f-letter" v-if="brandShowMore">
                        <div class="leter" :class="{ cur: selectedLetter === '' }" data-word="all" @click="selectedLetter = ''">{{ $t("所有品牌") }}</div>
                        <div
                            class="leter"
                            v-for="(item, index) in filteredAlphabets"
                            :key="index"
                            @click="filterBrands(item)"
                            :class="{ cur: selectedLetter == item }"
                        >
                            {{ item }}
                        </div>
                    </div>
                    <div :class="{ 'category-brand-wrap': !brandShowMore, 'category-brand-f-wrap': brandShowMore }">
                        <template v-for="(item, key) in filteredBrands" :key="key">
                            <div
                                class="item"
                                :class="{ selected: brandIsSeleted(item.brandId) }"
                                v-if="brandShowMore || (!brandShowMore && key < 8)"
                                @click="selectBrand(item.brandId)"
                            >
                                <span class="pic_box">
                                    <img :src="imageFormat(item.brandLogo)" :alt="`${item.brandName || ''} Logo`" />
                                </span>
                                <span class="brand_name">{{ item.brandName }}</span>
                                <i></i>
                            </div>
                        </template>
                    </div>
                    <span class="attrMore" :class="{ attrMoreUp: brandShowMore }" @click="brandShowMore = !brandShowMore">
                        {{ brandShowMore ? $t("收起") : $t("更多") }}<i class="iconfont-pc icon-xiajiantou"></i>
                    </span>
                    <div
                        class="attr-multiple"
                        v-if="!brandMultiple"
                        @click="
                            brandMultiple = true;
                            brandShowMore = true;
                        "
                    >
                        <i>＋</i>
                        <div class="attr-multiple-text">{{ $t("多选") }}</div>
                    </div>
                </div>
                <div class="fc-btn-box" v-if="brandShowMore && brandMultiple">
                    <a class="fc-btn-ok" :class="{ 'fc-disable': multipleBrandIds.length === 0 }" @click="onSubmitBrandMutiple">{{ $t("确定") }}</a>
                    <a class="fc-btn-cancel" @click="onColseBrandMultiple">{{ $t("取消") }}</a>
                </div>
            </div>
            <CommonEsFilter
                v-if="commonStore.enableAttributeFilter"
                ref="esFilterRef"
                v-model:filter="filter"
                v-model:filterSeleted="_filterSeleted"
                :queryParams="queryParams"
                @change="onFilterChange"
            ></CommonEsFilter>
        </div>
        <div class="cat-sort-warp">
            <div class="cat-sort">
                <div class="flex">
                    <div class="lefter">
                        <span class="sort-link" :class="{ cur_sort: filter.sortField == '' }" :title="t('默认排序（根据相关性排序）')" @click="changeSort('')">
                            {{ $t("默认排序") }}
                        </span>
                        <span
                            class="sort-link"
                            :class="{ cur_sort: filter.sortField == 'sale' }"
                            :title="t('销量（销量从高到低）')"
                            @click="changeSort('sale')"
                        >
                            {{ $t("销量") }}
                        </span>
                        <span
                            class="sort-link"
                            :class="{ cur_sort: filter.sortField == 'time' }"
                            :title="t('上架时间（最新发布排序）')"
                            @click="changeSort('time')"
                        >
                            {{ $t("新品") }}
                        </span>
                        <span
                            class="sort-link"
                            :class="{ cur_sort: filter.sortField == 'price' }"
                            :title="filter.sortOrder == 'desc' ? t('价格（价格从高到低）') : t('价格（价格从低到高）')"
                            @click="changeSort('price')"
                        >
                            {{ $t("价格") }}
                            <span v-if="filter.sortOrder == 'desc'" class="iconfont-pc icon-xiajiantou1"></span>
                            <span v-else class="iconfont-pc icon-shangjiantou1"></span>
                        </span>
                    </div>
                    <div class="priceform">
                        <div class="form-bg">
                            <input type="text" name="pricemin" v-model.number="minPrice" :placeholder="$t('{0}最低价', [commonStore.dollarSign || '¥'])" />
                            <em> - </em>
                            <input type="text" name="pricemax" v-model.number="maxPrice" :placeholder="$t('{0}最高价', [commonStore.dollarSign || '¥'])" />
                            <a class="btn-default" draggable="false" @click="onPriceChange" rel="nofollow" href="javascript:;">{{ $t("确定") }}</a>
                        </div>
                    </div>
                </div>
                <div class="righter">
                    <div class="productsNO">
                        <div>
                            {{ $t("共") }}<em> {{ searchStore.total }} </em>{{ $t("个商品") }}
                        </div>
                    </div>
                    <div class="pageInfo">
                        <span class="page_number">{{ filter.page }}/{{ pageCount }}</span>
                        <span @click="onPrev" class="noPage" :class="{ disabled: filter.page == 1 }">&lt;</span>
                        <span @click="onNext" class="noPage" :class="{ disabled: filter.page === pageCount }">&gt;</span>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, shallowRef, onMounted, onBeforeUnmount, nextTick, watch } from "vue";
import { getSearchFilter } from "@/api/search/search";
import { urlFormat, imageFormat } from "@/utils/format";
import type { filterSeleted, SearchFilter, SearchFilterListResult } from "@/types/search/search.d";
import { useCommonStore } from "@/store/common";
import { useSearchStore } from "@/store/search";
const searchStore = useSearchStore();
const commonStore = useCommonStore();
const headTitle = ref("");
const headDescription = ref();
const headKeywords = ref();
const { t } = useI18n();

const props = defineProps({
    isSearchPage: {
        type: Boolean,
        default: false
    },
    total: {
        type: Number,
        default: 0
    },
    queryParams: {
        type: Object,
        default: {}
    },
    pageType: {
        type: String,
        default: "search"
    },
    attrList: {
        type: Array,
        default: () => []
    }
});
const emit = defineEmits(["change", "changeAttr"]);
const attrsList = ref<any>(props.attrList);
const filter = defineModel<SearchFilter>("filter") as Ref<SearchFilter>;
const filterSeleted = defineModel("filterSeleted");
const _filterSeleted = ref<filterSeleted>({
    category: "",
    brand: "",
    attrs: []
});
const filterList = ref<SearchFilterListResult>({
    brand: [],
    category: []
});
const categorydShowMore = ref(false);
const categoryLinkRef = ref<HTMLElement | null>(null);
const categoryNeedMore = ref(false);
let categoryResizeObserver: ResizeObserver | null = null;

const CATEGORY_COLLAPSED_HEIGHT = 28; // 对应 .category-link 的默认 height: 28px
const updateCategoryNeedMore = () => {
    const el = categoryLinkRef.value;
    if (!el) return;
    const need = el.scrollHeight > CATEGORY_COLLAPSED_HEIGHT + 1;
    categoryNeedMore.value = need;
    if (!need) categorydShowMore.value = false;
};

watch(
    () => filterList.value.category,
    async () => {
        await nextTick();
        updateCategoryNeedMore();
    },
    { deep: true }
);

onMounted(async () => {
    await nextTick();
    updateCategoryNeedMore();

    const el = categoryLinkRef.value;
    if (el && typeof ResizeObserver !== "undefined") {
        categoryResizeObserver = new ResizeObserver(() => updateCategoryNeedMore());
        categoryResizeObserver.observe(el);
    } else if (typeof window !== "undefined") {
        window.addEventListener("resize", updateCategoryNeedMore);
    }
});

onBeforeUnmount(() => {
    if (categoryResizeObserver) {
        categoryResizeObserver.disconnect();
        categoryResizeObserver = null;
    }
    if (typeof window !== "undefined") {
        window.removeEventListener("resize", updateCategoryNeedMore);
    }
});
type PriceInput = number | null | "";
const minPrice = ref<PriceInput>(filter.value.minPrice ?? null);
const maxPrice = ref<PriceInput>(filter.value.maxPrice ?? null);
const _getSearchFilter = async (type = "") => {
    try {
        const result = await getSearchFilter({ ...props.queryParams, page_type: props.pageType, attrs: attrsList.value });
        filterList.value.brand = result.filter.brand;
        filterList.value.category = result.filter.category;
        _filterSeleted.value = result.filterSelected;
        headTitle.value = "";
        if (_filterSeleted.value.category) {
            headTitle.value += _filterSeleted.value.category + " - ";
        }
        if (filter.value.keyword) {
            headTitle.value += filter.value.keyword + " -  ";
        }
        if (_filterSeleted.value.brand) {
            headTitle.value += _filterSeleted.value.brand + " -  ";
        }
        headTitle.value += t("商品搜索") + commonStore.pageTitle;
        filterSeleted.value = result.filterSelected;
    } catch (error) {
    } finally {
    }
};
const pageCount = computed(() => {
    return searchStore.total ? Math.ceil(Number(searchStore.total) / 25) : 1;
});

watch(
    () => filter.value,
    () => {
        _getSearchFilter();
    },
    { deep: true, immediate: true }
);
watch(
    () => props.attrList,
    () => {
        attrsList.value = props.attrList;
        _getSearchFilter();
    },
    { deep: true, immediate: true }
);

const onFilterChange = (attrs: any) => {
    filter.value.page = 1;
    attrsList.value = attrs;
    _getSearchFilter()
    emit("changeAttr", attrs);
};

// 分类相关
const onCategoryClick = (categoryId: number) => {
    filter.value.categoryId = categoryId;
    filter.value.page = 1;
    emit("change");
};
// 品牌相关
const filteredBrands = computed(() => {
    if (selectedLetter.value) {
        return filterList.value.brand.filter((brand) => brand.firstWord === selectedLetter.value);
    }
    return filterList.value.brand;
});
const brandShowMore = ref(false);
const brandMultiple = ref(false);
const filteredAlphabets = computed(() => {
    const alphabets = [...new Set(filterList.value.brand.map((brand) => brand.firstWord))];
    alphabets.sort();
    return alphabets;
});
const multipleBrandIds = ref<number[]>([]);
const selectBrand = (brandId: number) => {
    if (brandMultiple.value === true) {
        const index = multipleBrandIds.value.indexOf(brandId);
        if (index > -1) {
            multipleBrandIds.value.splice(index, 1); // 如果存在
        } else {
            multipleBrandIds.value.push(brandId); // 如果不存在
        }
    } else {
        filter.value.brandIds = [brandId];
        filter.value.page = 1;
        emit("change");
    }
};
const brandIsSeleted = (brandId: number) => {
    const index = multipleBrandIds.value.indexOf(brandId);
    if (index > -1) {
        return true;
    } else {
        return false;
    }
};
const onColseBrandMultiple = () => {
    brandShowMore.value = false;
    brandMultiple.value = false;
    multipleBrandIds.value = filter.value.brandIds;
};
const onSubmitBrandMutiple = () => {
    filter.value.brandIds = multipleBrandIds.value;
    filter.value.page = 1;
    brandShowMore.value = false;
    brandMultiple.value = false;
    emit("change");
};
const selectedLetter = ref<string>("");
const filterBrands = (letter: string): void => {
    selectedLetter.value = letter;
};

// 排序
const changeSort = (sort: string) => {
    if (sort === "") {
        filter.value.sortField = "";
        filter.value.sortOrder = "";
    } else if (sort === "sale") {
        filter.value.sortField = "sale";
        filter.value.sortOrder = "";
    } else if (sort === "time") {
        filter.value.sortField = "time";
        filter.value.sortOrder = "";
    } else if (sort === "price") {
        filter.value.sortField = "price";
        filter.value.sortOrder = filter.value.sortOrder === "asc" ? "desc" : "asc";
    }
    filter.value.page = 1;
    emit("change");
};
const onPrev = () => {
    if (filter.value.page > 1) {
        filter.value.page--;
        emit("change", filter.value.page);
    }
};
const onNext = () => {
    if (filter.value.page < pageCount.value) {
        filter.value.page++;
        emit("change", filter.value.page);
    }
};
// 价格
const onPriceChange = () => {
    const rawMin = minPrice.value === "" || minPrice.value == null ? null : Number(minPrice.value);
    const rawMax = maxPrice.value === "" || maxPrice.value == null ? null : Number(maxPrice.value);
    const normalizedMinPrice = rawMin !== null && Number.isFinite(rawMin) ? rawMin : null;
    const normalizedMaxPrice = rawMax !== null && Number.isFinite(rawMax) ? rawMax : null;
    if (normalizedMinPrice == null || normalizedMaxPrice == null || normalizedMinPrice > normalizedMaxPrice || normalizedMaxPrice === 0) {
        return message.error(t("请输入正确的价格范围"));
    }
    filter.value.minPrice = normalizedMinPrice;
    filter.value.maxPrice = normalizedMaxPrice;
    emit("change");
};
const clearMultipleBrandIds = () => {
    multipleBrandIds.value.length = 0;
};
defineExpose({
    clearMultipleBrandIds
});
</script>
<style lang="less" scoped>
.sear-title {
    background: #f8f8f8 none repeat scroll 0 0;
    height: 34px;
    line-height: 34px;
    overflow: hidden;
    margin-top: 10px;
    display: flex;
    align-items: center;
    h3 {
        padding-left: 10px;
        font-weight: bold;
        font-size: 14px;
        b {
            color: var(--general);
            font-weight: bold;
            margin-right: 5px;
        }
        em {
            font-weight: bold;
        }
    }
    .st-ext {
        padding-left: 20px;
    }
}
.filter .list {
    position: relative;
    overflow: hidden;
    padding: 6px 0;
    height: 100%;
    border-bottom: 1px solid #dddddd;
    line-height: 20px;
    transition: height 0.3s ease 0s;
}
.filter .attr-name {
    position: absolute;
    top: 9px;
    left: 10px;
    width: 67px;
    font-weight: normal;
    font-size: 12px;
    line-height: 23px;
}
.filter .attr-box {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    overflow: hidden;
    padding: 0 130px 0 83px;
    align-items: center;
}
.filter .attr-box .pic_box {
    display: inline-flex;
    width: 110px;
    height: 45px;
    text-align: center;
    box-sizing: border-box;
    padding: 4px 6px;
    align-items: center;
    justify-content: center;
    img {
        width: 100%;
        height: auto;
        object-fit: contain;
        max-height: 100%;
    }
}
.filter .attr-box .category-link {
    overflow: hidden;
    margin: 0 10px 2px 0;
    height: 28px;
    line-height: 28px;
    .item {
        padding: 0;
        cursor: pointer;
    }
}
.filter .attr-box .category-link-more {
    height: 100%;
}
.filter .attr-box .item {
    display: inline-block;
    margin-right: 10px;
    padding: 0 8px;
    color: #767676;
    white-space: nowrap;
}
.filter .attr-box .item:hover {
    color: var(--general);
}
.attrMore {
    position: absolute;
    top: 3px;
    right: 60px;
    padding: 0 20px 0 0;
    cursor: pointer;
    i {
        margin-left: 3px;
        font-size: 12px;
    }
}
.attrMoreUp {
    i {
        transform: rotate(180deg);
        display: inline-block;
    }
}
.attr-multiple {
    position: absolute;
    top: 3px;
    right: 10px;
    min-width: 45px;
    max-width: 60px;
    min-height: 18px;
    border: 1px solid #dad9d9;
    text-align: center;
    font-weight: normal;
    line-height: 18px;
    cursor: pointer;
    display: flex;
    align-items: center;

    i {
        color: #aaaaaa;
        font-size: 16px;
    }

    .attr-multiple-text {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
}
.priceform {
    display: inline-flex;
    .form-bg {
        padding: 0 15px;
        width: 280px;
    }
    input {
        padding: 3px;
        width: 52px;
        height: 18px;
        border: 1px solid #eaeaea;
        vertical-align: top;
        font-size: 12px;
        line-height: 18px;
    }
    input:hover {
        border-color: var(--general);
    }
}
.btn-default {
    display: inline-block;
    margin-left: 3px;
    padding: 5px 13px 5px;
    height: 14px;
    border: 1px solid #ddd;
    border-radius: 2px;
    background: #f7f7f7;
    background-color: #f7f7f7;
    background-image: -moz-linear-gradient(to bottom, #f7f7f7, #f2f2f2);
    background-image: -webkit-linear-gradient(to bottom, #f7f7f7, #f2f2f2);
    background-image: -o-linear-gradient(to bottom, #f7f7f7, #f2f2f2);
    background-image: linear-gradient(to bottom, #f7f7f7, #f2f2f2);
    background-repeat: repeat-x;
    color: #666;
    text-align: center;
    text-decoration: none;
    line-height: 14px;
    cursor: pointer;
}

.btn-default:hover {
    box-shadow: 0 1px 1px rgba(0, 1, 1, 0.08);
    color: #666;
    text-decoration: none;
    cursor: pointer;
}
.filter .brand_fitlter .attr-box {
    margin-top: 10px;
}
.filter .brand_fitlter .attr-box .item {
    display: inline-block;
    margin-right: 8px;
    margin-left: 0;
    border: 1px solid #eeeeee;
    padding: 0;
    width: 110px;
    height: 45px;
    overflow: hidden;
    margin-bottom: 8px;
    position: relative;
    &.selected {
        border-color: var(--general);
    }
    &.selected i {
        //background: url("/assets/images/product/attr_bg.gif") no-repeat right bottom;
        display: block;
        height: 14px;
        overflow: hidden;
        position: absolute;
        right: 0px;
        bottom: 0;
        width: 14px;
        text-indent: -999px;
    }
}
.filter .brand_fitlter .attr-box .item img {
    height: 45px;
    margin: 0 auto;
    display: block;
    max-width: 110px;
}
.filter .brand_fitlter .attr-box .item {
    background: none;
}
.filter .brand_fitlter .attr-box .item:hover {
    border-color: var(--general);
}
.filter .brand_fitlter .attr-box .item i {
    display: none;
}
.filter .attr-box .brand_name {
    width: 110px;
    height: 45px;
    background: #fff;
    position: absolute;
    top: 0px;
    display: none;
    cursor: pointer;
    text-align: center;
    line-height: 45px;
    color: var(--general);
}
.filter .brand_fitlter .attr-box .item:hover .brand_name {
    display: block;
}
.category-brand-f-letter {
    width: 100%;
    color: var(--general);
    display: flex;
    flex-wrap: wrap;
    margin-bottom: 20px;
    .leter {
        border: 1px solid #fff;
        cursor: pointer;
        height: 22px;
        line-height: 22px;
        margin-right: 5px;
        padding: 0 8px;
    }
}
.category-brand-wrap {
    height: 55px;
}
.category-brand-f-wrap {
    width: 100%;
    display: flex;
    flex-wrap: wrap;
}
.category-brand-f-letter .cur {
    border: 1px solid var(--general);
    color: var(--general);
}
.fc-btn-box {
    line-height: 20px;
    padding: 13px 0 10px;
    display: flex;
    justify-content: center;
}
.fc-btn-ok {
    overflow: hidden;
    padding: 0 9px;
    height: 20px;
    background-color: var(--general);
    border: 1px solid var(--general);
    color: var(--main-text);
    vertical-align: middle;
    line-height: 20px;
    margin-right: 3px;
}
.fc-btn-ok:hover {
    color: var(--main-text) !important;
}
.fc-disable,
.fc-disable:hover {
    border-color: #e6e6e6;
    background: #f8f8f8 none repeat scroll 0 0;
    color: #ccc;
}
.fc-btn-cancel {
    overflow: hidden;
    padding: 0 9px;
    height: 20px;
    border: 1px solid #ddd;
    background: #fff none repeat scroll 0 0;
    color: #333;
    vertical-align: middle;
    line-height: 20px;
}

// 排序
.cat-sort-warp {
    margin: 10px 0;
}
.cat-sort {
    height: 32px;
    background: #f8f8f8;
    padding: 5px 0;
    padding-right: 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    .lefter {
        background: #fff none repeat scroll 0 0;
        border: 1px solid #eee;
        height: 24px;
        margin-left: 8px;
        display: flex;
        align-items: center;
        .sort-link {
            padding: 0 20px;
            font-size: 12px;
            position: relative;
            height: 24px;
            display: flex;
            cursor: pointer;
            align-items: center;

            span {
                height: 24px;
                line-height: 24px;
            }

            .iconfont-pc {
                font-size: 10px;
                margin-left: 3px;
            }
        }
        .sort-link:hover {
            color: var(--general);
        }
        .cur_sort {
            background-color: var(--general);
            color: var(--main-text);
        }
        .cur_sort:hover {
            color: white;
        }
    }
    .righter {
        display: flex;
        align-items: center;
        .colour_scan {
            margin-right: 20px;
        }
        .productsNO {
            display: flex;
            align-items: center;
            em {
                color: var(--general);
                margin: 0 3px;
                font-weight: 700;
            }
        }
        .pageInfo {
            display: flex;
            align-items: center;
            color: #999;
            margin-left: 20px;
            .page_number {
                color: #999;
                font-size: 14px;
                padding-right: 10px;
            }
            .noPage {
                display: flex;
                font-size: 14px;
                background-color: #ffffff;
                border: 1px solid #cacaca;
                color: #767676;
                height: 24px;
                text-align: center;
                width: 30px;
                margin-left: -1px;
                align-items: center;
                justify-content: center;
                &:hover {
                    color: var(--general);
                }
            }
            .disabled {
                color: #ddd !important;
            }
        }
    }
}
</style>
