<template>
    <div
        ref="productBox"
        class="goods-list__box"
        :class="{ 'goods-list__box--loading': isLoading, 'goods-list__box--empty': productList.length === 0 && !isLoading }"
    >
        <div class="goods-list-c">
            <div class="wrap">
                <template v-for="(item, index) in productList" :key="index">
                    <SearchProductCard
                        :group-data="getGroupData(String(item.id ?? ''))"
                        :activity-info="getActivityInfo(String(item.id ?? ''))"
                        :product="item"
                        :search-config="searchConfig"
                        :fixed-name-two-lines="fixedNameTwoLines"
                    />
                </template>

                <div v-if="productList.length === 0 && !isLoading" class="search-empty">
                    <tig-icon name="shop" :size="50" />
                    <template v-if="isImageSearch">
                        <span class="search-empty__text" role="status">{{ $t("没有找到相似的商品") }}</span>
                        <div v-if="hasRouteFilter" class="search-empty__actions">
                            <button type="button" @click="imageSearchContext?.clearFilters()">{{ $t("清除筛选") }}</button>
                        </div>
                    </template>
                    <span v-else>
                        {{
                            $t("{0}没有搜索到相关结果", [
                                filterSeleted?.keyword || filterSeleted?.category || filterSeleted?.brand
                                    ? ((filterSeleted?.keyword || "") + " " + (filterSeleted?.category || "") + " " + (filterSeleted?.brand || "")).trim()
                                    : ""
                            ])
                        }}</span
                    >
                </div>
            </div>
        </div>
        <div v-if="isLoading" class="goods-list-loading">
            <div class="goods-list-loading__spinner"></div>
        </div>
    </div>
    <div class="pagination">
        <Pagination
            v-if="searchStore.total > 0 && productList.length > 0"
            v-model:page="filter.page"
            :size="size"
            :total="searchStore.total"
            @callback="pageChange"
        />
    </div>
</template>
<script lang="ts" setup>
// ===================== Vue 核心 =====================
import { inject, ref, type PropType } from "vue";

// ===================== 组件导入 =====================
import { Pagination } from "~/components/list";
import SearchProductCard from "./SearchProductCard.vue";

// ===================== API 导入 =====================
import { searchApi } from "./api";
import { message } from "ant-design-vue";
import { isImageSearchDisabled, resolveImageSearchErrorMessage } from "~/api/search/imageSearch";
import { isSilentError } from "~/utils/request";

// ===================== 类型导入 =====================
import type { QueryParams, SearchFilter, filterSeleted as FilterSelected, SearchProductItem } from "./types.d";

// ===================== Store =====================
import { useSearchStore } from "~/store/search";

// ===================== Props & Emits =====================
defineProps({
    filterSeleted: {
        type: Object as PropType<FilterSelected>
    },
    searchConfig: {
        type: Object as PropType<Record<string, unknown>>,
        default: () => ({})
    },
    fixedNameTwoLines: {
        type: Boolean,
        default: false
    }
});

const emit = defineEmits(["change", "pageChange", "listLength", "loadFinished"]);

// ===================== 响应式状态 =====================
const searchStore = useSearchStore();
const size = ref<number>(30);
const filter = defineModel<SearchFilter>("filter") as Ref<SearchFilter>;
const queryParams = defineModel<QueryParams>("queryParams", { default: () => ({}) });
const isLoading = ref<boolean>(false);
const productList = ref<SearchProductItem[]>([]);
const FilterLoading = defineModel("FilterLoading", { default: false });
const productBox: Ref<HTMLElement | null> = ref(null);
const requestSerial = ref(0);
const queuedRequestSignature = ref<string | null>(null);
const { getActivityInfo, fetchPromotionList, clearPromotionList } = usePromotionActivity();

// ===================== 图片搜索结果 =====================
const { t } = useI18n();
const route = useRoute();
/** 图片结果页提供的操作：清除筛选 */
const imageSearchContext = inject<{
    clearFilters: () => void;
} | null>("imageSearchContext", null);
const isImageSearch = computed(() => Boolean(queryParams.value?.searchToken));
const hasRouteFilter = computed(
    () => Boolean(route.query.cat || route.query.brand || route.query.min || route.query.max) || (filter.value?.attrs?.length ?? 0) > 0
);

const currentLocationParams = computed(() => {
    return {};
});

const searchRequestParams = computed(() => ({
    ...(queryParams.value ?? {}),
    attrs: filter.value?.attrs ?? [],
    size: size.value,
    ...currentLocationParams.value
}));
const searchRequestSignature = computed(() => JSON.stringify(searchRequestParams.value));

const cloneRequestParams = (): Record<string, unknown> => {
    return JSON.parse(JSON.stringify(searchRequestParams.value));
};

// ===================== 数据获取方法 =====================
const getSearchProduct = async (): Promise<void> => {
    const requestParams = cloneRequestParams();
    const requestSignature = JSON.stringify(requestParams);
    if (isLoading.value) {
        queuedRequestSignature.value = requestSignature;
        return;
    }
    ++requestSerial.value;
    queuedRequestSignature.value = null;
    try {
        isLoading.value = true;
        FilterLoading.value = true;
        productList.value = [];
        clearPromotionList();
        const result = await searchApi.getProductList(requestParams);
        productList.value = result.records;
        searchStore.total = result.total ?? 0;
        productList.value.forEach((item) => {
            item.number = 1;
        });

        emit("listLength", productList.value.length);

        const products = result.records.map((item: any) => ({ productId: item.id }));
        // 兜底：商品列表为空时不请求促销，避免接口报「商品列表不能为空」
        if (products.length > 0) {
            const shopId = queryParams.value?.shopId ? String(queryParams.value.shopId) : undefined;
            void fetchPromotionList(products, { from: "list", shopId });
        }
    } catch (error) {
        console.error("[CatGoodsList.getSearchProduct] 获取搜索商品列表失败:", error);
        if (requestParams.searchToken) {
            // 与项目接口报错的处理一致：提示原因，列表按空展示；功能关闭由结果页静默退出
            // 登录失效已跳转登录页，不再提示
            if (!isImageSearchDisabled(error) && !isSilentError(error)) message.error(resolveImageSearchErrorMessage(error, t));
            searchStore.total = 0;
        }
    } finally {
        isLoading.value = false;
        FilterLoading.value = false;
        emit("loadFinished");
        if (queuedRequestSignature.value && queuedRequestSignature.value !== requestSignature) {
            void getSearchProduct();
        }
    }
};

// ===================== 工具方法 =====================
const getGroupData = (productId: string): Record<string, unknown> => {
    const activityInfo = getActivityInfo(productId) as Array<Record<string, unknown>>;
    return activityInfo.find((item) => item.type == 7) || {};
};

// ===================== 事件处理方法 =====================
const pageChange = (): void => {
    emit("change", filter.value.page);
    emit("pageChange");
};

// ===================== 侦听器 =====================
watch(
    () => searchRequestSignature.value,
    () => {
        getSearchProduct();
    },
    { immediate: true }
);
</script>
<style lang="less" scoped>
.promotion-box {
    display: flex;
    align-items: center;
    height: 20px;
    padding-top: 5px;

    .activity {
        max-width: 100px;
        background-color: var(--general);
        color: var(--main-text);
        padding: 0 8px;
        border-radius: 2px;
        padding-bottom: 2px;
        margin-right: 10px;
    }

    .promotion {
        display: flex;
        background-color: var(--tag-bg);
        color: var(--tag-text);
        border-radius: 2px;
        padding-bottom: 2px;

        .ticket-type {
            padding: 0 6px;
            max-width: 50px;
            position: relative;

            .ticket-line {
                position: absolute;
                display: inline-block;
                height: 100%;
                opacity: 0.9;
                width: 2px;
                top: 0;
                right: 0px;
                transform: scaleY(0.7);
                background: linear-gradient(
                    to bottom,
                    var(--ump-border, #c9c9ff) 20%,
                    var(--ump-border, #c9c9ff) 20%,
                    var(--ump-border, #c9c9ff) 20%,
                    rgba(255, 255, 255, 0) 20%,
                    rgba(255, 255, 255, 0) 35%,
                    rgba(136, 76, 255, 1) 35%,
                    var(--ump-border, #c9c9ff) 35%,
                    var(--ump-border, #c9c9ff) 35%,
                    var(--ump-border, #c9c9ff) 60%,
                    rgba(255, 255, 255, 0) 60%,
                    rgba(255, 255, 255, 0) 55%,
                    rgba(255, 255, 255, 0) 75%,
                    var(--ump-border, #c9c9ff) 75%,
                    var(--ump-border, #c9c9ff) 75%,
                    var(--ump-border, #c9c9ff) 100%
                );
            }
        }
        .ticket-content {
            padding: 0 4px;
            max-width: 90px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
    }
}

.shop-info {
    line-height: 18px;
    height: 18px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;

    .shop-name {
        color: #999;
    }
}

.goods-list-c {
    width: 100%;
    overflow: hidden;
    .wrap {
        display: grid;
        grid-template-columns: repeat(var(--search-grid-columns, 5), minmax(0, 1fr));
        width: 100%;
        gap: 12px 16px;
        // min-height: 400px;
        .item {
            border: 1px solid #fff;
            padding: 18px 9px 20px;
            transition: border-color 0.1s ease 0s;
            margin: 0 5px 14px 0;
            position: relative;
            width: 214px;
            background: #fff;
            .pic {
                width: 214px;
                height: 214px;
                margin-bottom: 10px;

                a {
                    height: 100%;
                    width: 100%;
                }

                .ease {
                    height: 100%;
                    width: 100%;
                }
            }
            .info {
                :deep(.price) {
                    height: 27px;
                    line-height: 27px;
                    color: var(--price);
                    .num {
                        font-size: 18px;
                        font-weight: normal;
                    }
                    .util {
                        font-size: 12px;
                        margin-bottom: -2px;
                    }
                }
                .name {
                    height: 40px;
                    line-height: 20px;
                    word-break: break-all;
                    display: -webkit-box;
                    -webkit-line-clamp: 2;
                    -webkit-box-orient: vertical;
                    overflow: hidden;
                    a {
                        color: #666;
                    }
                }
                .brief {
                    height: 20px;
                    line-height: 20px;
                    overflow: hidden;
                    .desc {
                        color: #ff4040;
                    }
                }
            }
        }
        .item:hover {
            border-color: #e9e9e9;
            box-shadow: 0 0 2px 2px #f8f8f8;
        }
    }
}

.goods-list__box {
    position: relative;
}

.goods-list__box--empty {
    min-height: 620px;
}

.goods-list__box--loading {
    min-height: 620px;
}

.goods-list-loading {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 620px;
    background: #fff;
}

.goods-list-loading__spinner {
    width: 40px;
    height: 40px;
    border: 3px solid rgba(255, 77, 79, 0.15);
    border-top-color: var(--general);
    border-radius: 50%;
    animation: goods-list-loading-spin 0.8s linear infinite;
}

@keyframes goods-list-loading-spin {
    from {
        transform: rotate(0deg);
    }

    to {
        transform: rotate(360deg);
    }
}

.item_cart_num {
    display: flex;
    align-items: center;
    padding-top: 10px;
    .shopping_num {
        display: flex;
        align-items: center;
        margin-right: 8px;
        input {
            background: url("/assets/images/category/search.png") no-repeat scroll 0px -310px;
            background-position: 0 -310px;
            border: 0 none;
            color: #555;
            padding: 0;
            height: 28px;
            line-height: 28px;
            text-align: center;
            width: 30px;
        }
        span {
            height: 28px;
            margin-left: 2px;
            width: 16px;
        }
        a {
            background: url("/assets/images/category/search.png") no-repeat scroll;
            cursor: pointer;
            display: block;
            height: 14px;
            line-height: 14px;
            overflow: hidden;
            text-indent: -99em;
            width: 16px;
        }
        .add {
            background-position: -17px -281px;
        }
        .decrease {
            background-position: -17px -295px;
        }
        .decrease.disabled {
            background-position: 0 -295px;
        }
    }
    .shopping_act {
        height: 28px;
        overflow: hidden;
        display: flex;
        align-items: center;
    }
}
.search-empty {
    grid-column: 1 / -1;
    min-height: 620px;
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    width: 100%;
    color: #c2c4cc;
    font-size: 14px;
    box-sizing: border-box;
}

.search-empty__text {
    margin-top: 12px;
    max-width: 480px;
    text-align: center;
    word-break: break-word;
}

.search-empty__actions {
    display: flex;
    gap: 12px;
    margin-top: 16px;

    button {
        min-width: 96px;
        height: 34px;
        padding: 0 18px;
        border: 1px solid #d9d9d9;
        border-radius: 6px;
        background: #fff;
        color: #1f1f1f;
        font-size: 14px;
        cursor: pointer;

        &:hover,
        &:focus-visible {
            border-color: var(--general);
            color: var(--general);
        }
    }
}

// .search-empty .bd {
//     background: url("/assets/images/common/empty.gif") no-repeat scroll 56px 45px;
//     padding: 45px 50px 45px 200px;
// }

// .search-empty h4 {
//     font-size: 14px;
//     padding-bottom: 10px;
// }

// .search-empty p {
//     color: #666666;
//     margin-bottom: 5px;
// }
.pagination {
    display: flex;
    justify-content: end;
    margin-top: 10px;
    margin-bottom: 20px;
}
</style>
