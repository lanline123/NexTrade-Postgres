<template>
    <div class="top-search" :class="{ white: props.isWhite }">
        <div class="search-form search-shell-dWh4ZzIyODI" :class="{ 'search-form-focus': searchFocus || keyword != '' }">
            <div class="acea-row row-middle">
                <div v-if="showSearchTypeDropdown" ref="typeSelectRef" class="search-type-select" :class="{ open: typeDropdownOpen }">
                    <div class="search-type-menu" :class="{ open: typeDropdownOpen }" :data-longest-label="$t(longestSearchTypeLabel)">
                        <div
                            v-for="(opt, idx) in searchTypeOptions"
                            :key="opt.value"
                            class="search-type-item"
                            :class="{ active: opt.value === searchType }"
                            @click.stop="onSearchTypeItemClick(idx, opt.value)"
                        >
                            <span class="search-type-label">{{ $t(opt.label) }}</span>
                            <ArrowIcon v-if="idx === 0" width="14" height="14" class="arrow" />
                        </div>
                    </div>
                </div>
                <input
                    v-model="keyword"
                    type="text"
                    autocomplete="off"
                    :placeholder="$t('请输入关键词')"
                    class="search-input"
                    name="keywords"
                    @focus="onSearchFocus"
                    @blur="onSearchBlur"
                    @keyup="onSearchKeyup"
                    @keydown.enter="onSearchSubmit"
                />
                <ImageSearchEntry v-if="searchType !== 'shop'" />
                <div type="submit" class="search-submit" :style="submitStyle" @click="onSearchSubmit">
                    <SearchIcon width="18px" height="18px" />
                </div>
            </div>
        </div>
        <div v-if="props.enableGuess && guessBoxShow" ref="guessBox" class="search-guess-box">
            <div v-for="(item, index) in guessKeywordList" :key="index" class="search-guess-item">
                <button v-if="props.submitMode === 'emit'" type="button" class="serlink" @click="onSelectGuessKeyword(item.keyword ?? '')">
                    <span class="name">{{ item.keyword }}</span>
                </button>
                <NuxtLink v-else :to="guessLinkUrl(item.keyword ?? '')" class="serlink">
                    <span class="name">{{ item.keyword }}</span>
                </NuxtLink>
            </div>
        </div>
    </div>
</template>
<script setup lang="ts">
// ===================== Vue 核心 =====================
import { computed, onMounted, onUnmounted, ref, watch } from "vue";

// ===================== 图标（内联 svg，SSR 友好；等价 vortmall 的 svg?component / vort-icon） =====================
import SearchIcon from "~/components/icons/SearchIcon.vue";
import ArrowIcon from "~/components/icons/ArrowIcon.vue";
import ImageSearchEntry from "~/components/imageSearch/ImageSearchEntry.vue";

// ===================== API / Store =====================
import { getSearchGuessData } from "~/api/common";
import { useCommonStore } from "~/store/common";
import { isMerchant, isO2o } from "~/utils/util";

// ===================== 类型 =====================
type SearchType = "product" | "shop";

interface SearchTypeOption {
    label: string;
    value: SearchType;
}

// ===================== Props & Emits =====================
const props = withDefaults(
    defineProps<{
        isWhite?: boolean;
        modelValue?: string;
        submitMode?: "navigate" | "emit";
        enableGuess?: boolean;
        defaultSearchType?: SearchType;
        /** 头部装修自定义搜索色，直接写到按钮上，避免被组件内部样式盖掉 */
        accentColor?: string;
    }>(),
    {
        isWhite: false,
        modelValue: undefined,
        submitMode: "navigate",
        enableGuess: true,
        defaultSearchType: "product",
        accentColor: ""
    }
);

const emit = defineEmits<{
    (e: "update:modelValue" | "submit", value: string): void;
}>();

const submitStyle = computed(() => {
    const color = String(props.accentColor || "").trim();
    if (!color) return undefined;
    return { backgroundColor: color };
});

// ===================== 响应式状态 =====================
const route = useRoute();
const commonStore = useCommonStore();
const searchFocus = ref<boolean>(false);
const keyword = ref<string>("");
const committedKeyword = ref<string>("");
const guessKeywordList = ref<Array<{ keyword?: string }>>([]);
const guessBox = ref<HTMLElement | null>(null);
const guessBoxShow = ref<boolean>(false);

let searchGuessDisabled = false;
let setTime: ReturnType<typeof setTimeout> | null = null;

// ===================== 搜索类型下拉 =====================
const searchType = ref<SearchType>(props.defaultSearchType);
const typeDropdownOpen = ref(false);
const typeSelectRef = ref<HTMLElement | null>(null);

// NexTrade 用 indexSearchShop 表示首页是否开启“搜店铺”
const showSearchTypeDropdown = computed(() => (isMerchant() || isO2o()) && commonStore.indexSearchShop === 1);

// o2o（门店）版下“店铺”即“门店”，文案与跳转地址均需区分
const shopLabel = computed(() => (isO2o() ? "门店" : "店铺"));

const searchTypeOptions = computed<SearchTypeOption[]>(() => {
    const current = searchType.value;
    const other: SearchType = current === "product" ? "shop" : "product";
    const labelOf = (t: SearchType) => (t === "shop" ? shopLabel.value : "商品");
    return [
        { label: labelOf(current), value: current },
        { label: labelOf(other), value: other }
    ];
});

const longestSearchTypeLabel = computed(() => {
    const a = "商品";
    const b = shopLabel.value;
    return a.length >= b.length ? a : b;
});

const onSearchTypeItemClick = (idx: number, type: SearchType): void => {
    if (idx === 0) {
        typeDropdownOpen.value = !typeDropdownOpen.value;
        return;
    }
    searchType.value = type;
    typeDropdownOpen.value = false;
};

const appendPreviewId = (url: string): string => {
    const pid = route.query.previewId ? String(route.query.previewId) : "";
    if (!pid || Number(pid) <= 0) return url;
    return url + (url.includes("?") ? "&" : "?") + "previewId=" + encodeURIComponent(pid);
};

const getSearchUrl = (kw: string): string => {
    const encodedKeyword = encodeURIComponent(kw);
    if (searchType.value === "shop" && showSearchTypeDropdown.value) {
        return appendPreviewId(isO2o() ? "/store/search?keyword=" + encodedKeyword : "/shop/list?keyword=" + encodedKeyword);
    }
    return appendPreviewId("/search/?keyword=" + encodedKeyword);
};

const guessLinkUrl = (kw: string): string => getSearchUrl(kw);

// ===================== 页面搜索元配置 =====================
interface SearchMetaConfig {
    syncKeyword?: boolean;
    defaultSearchType?: SearchType;
}

const searchMeta = computed<SearchMetaConfig>(() => (route.meta?.search as SearchMetaConfig | undefined) ?? {});
// o2o 门店搜索页：无需依赖页面 meta 即默认选中“门店”
const isStoreSearchPage = computed(() => isO2o() && route.path === "/store/search");
const shouldSyncKeyword = computed(() => route.path === "/search/" || isStoreSearchPage.value || searchMeta.value.syncKeyword === true);

const syncKeywordFromRoute = (): void => {
    if (!shouldSyncKeyword.value) return;

    const kw = String(route.query.keyword ?? "");
    committedKeyword.value = kw;
    keyword.value = kw;

    if (route.path === "/search/") {
        searchType.value = "product";
    } else if (isStoreSearchPage.value && showSearchTypeDropdown.value) {
        searchType.value = "shop";
    } else if (searchMeta.value.defaultSearchType && showSearchTypeDropdown.value) {
        searchType.value = searchMeta.value.defaultSearchType;
    }

    if (!kw) {
        guessBoxShow.value = false;
    }
};

if (shouldSyncKeyword.value) {
    syncKeywordFromRoute();
}

// ===================== 数据获取方法 =====================
const onSearchKeyup = (): void => {
    if (!props.enableGuess) {
        guessBoxShow.value = false;
        return;
    }
    if (searchGuessDisabled) {
        return;
    }
    if (setTime) clearTimeout(setTime);
    searchGuessDisabled = true;
    setTime = setTimeout(async () => {
        try {
            const result = await getSearchGuessData(keyword.value);
            guessKeywordList.value = Array.isArray(result) ? result : [];
            guessBoxShow.value = guessKeywordList.value.length > 0;
        } catch (error) {
            console.error("[DefSearchNew.onSearchKeyup] 搜索关键词失败:", error);
        } finally {
            searchGuessDisabled = false;
        }
    }, 300);
};

// ===================== 事件处理方法 =====================
const onSearchFocus = (): void => {
    searchFocus.value = true;
};

const onSearchBlur = (): void => {
    searchFocus.value = false;
};

const onSearchSubmit = (): void => {
    const nextKeyword = keyword.value.trim();
    keyword.value = nextKeyword;
    committedKeyword.value = nextKeyword;
    guessBoxShow.value = false;

    if (props.submitMode === "emit") {
        emit("update:modelValue", nextKeyword);
        emit("submit", nextKeyword);
        return;
    }

    location.href = getSearchUrl(nextKeyword);
};

const onSelectGuessKeyword = (selectedKeyword: string): void => {
    keyword.value = String(selectedKeyword ?? "");
    onSearchSubmit();
};

const handleOutsideClick = (event: MouseEvent): void => {
    const el = event.target as Node;
    if (typeDropdownOpen.value && typeSelectRef.value && !typeSelectRef.value.contains(el)) {
        typeDropdownOpen.value = false;
    }
    if (guessBox.value && !guessBox.value.contains(el)) {
        guessBoxShow.value = false;
    }
};

// ===================== 侦听器 =====================
watch(
    () => [route.path, route.query.keyword],
    () => {
        syncKeywordFromRoute();
    }
);

watch(
    () => props.modelValue,
    (value) => {
        if (shouldSyncKeyword.value) return;
        const nextKeyword = String(value ?? "");
        if (keyword.value !== nextKeyword) {
            keyword.value = nextKeyword;
        }
        if (!searchFocus.value) {
            committedKeyword.value = nextKeyword;
        }
    },
    { immediate: true }
);

watch(
    () => showSearchTypeDropdown.value,
    (enabled) => {
        if (!enabled) {
            searchType.value = "product";
            typeDropdownOpen.value = false;
        }
    },
    { immediate: true }
);

watch(keyword, (value) => {
    if (props.modelValue !== undefined && value !== props.modelValue) {
        emit("update:modelValue", value);
    }
});

// ===================== 生命周期 =====================
onMounted(() => {
    window.addEventListener("click", handleOutsideClick);
});

onUnmounted(() => {
    window.removeEventListener("click", handleOutsideClick);
});
</script>
<style scoped lang="less">
.top-search {
    position: relative;
    border-radius: 7px;
    width: 100%;
    border: 1px solid #fff;
    padding: 1px;
    background: #fff;
    box-sizing: border-box;
    &:hover {
        border: 1px solid #1a1a1a;
    }

    &.white {
        border: 1px solid #c1c1c1;
        &:hover {
            border: 1px solid var(--primary-dark-2);
        }
    }

    .search-form-focus .search-input {
        color: #333 !important;
    }

    .search-form {
        display: flex;
        align-items: center;
        background-color: #fff;
        border-radius: 0;
        overflow: visible;
        height: 37px;
        width: 100%;
        border-radius: 8px;

        .search-type-select {
            position: relative;
            height: 37px;
            display: inline-flex;
            align-items: center;
            border-radius: 8px;
            user-select: none;
            cursor: pointer;
            background-color: #fff;
            color: #333;

            .search-type-menu {
                width: max-content;
                height: 37px;
                position: relative;
                display: flex;
                flex-direction: column;

                &::after {
                    content: attr(data-longest-label);
                    display: block;
                    height: 0;
                    overflow: hidden;
                    visibility: hidden;
                    white-space: nowrap;
                    font-size: 14px;
                    padding: 0 calc(12px + 14px + 4px) 0 12px;
                }

                &:not(.open) {
                    .search-type-item:nth-child(n + 2) {
                        display: none;
                    }
                }

                &.open {
                    z-index: 8996;

                    &::before {
                        content: "";
                        position: absolute;
                        left: 0;
                        top: 0;
                        width: 100%;
                        height: calc(37px * 2);
                        background: #fff;
                        border-radius: 8px;
                        box-shadow: 0 6px 18px rgba(0, 0, 0, 0.12);
                        z-index: 0;
                    }

                    .search-type-item:nth-child(2) {
                        display: flex;
                        position: absolute;
                        left: 0;
                        top: 37px;
                        z-index: 1;
                    }
                }

                .search-type-item {
                    height: 37px;
                    font-size: 14px;
                    color: #333;
                    display: flex;
                    align-items: center;
                    justify-content: flex-start;
                    padding-left: 16px;
                    border-radius: 6px;
                    gap: 4px;
                    white-space: nowrap;
                    width: 100%;
                    box-sizing: border-box;
                    border: 1px solid transparent;
                    position: relative;
                    z-index: 1;

                    .arrow {
                        flex: 0 0 auto;
                    }
                }
            }
        }

        .row-middle {
            flex: 1;
            display: flex;
            justify-content: end;
            align-items: center;
        }

        .search-input {
            flex: 1;
            background-color: #fff;
            border: medium none;
            color: #bbbbbb;
            height: 26px;
            line-height: 26px;
            padding: 6px 10px;
            outline: medium none;
            border-radius: 8px;
            &::placeholder {
                font-size: 14px;
                color: #9ca3af;
            }
        }

        .search-submit {
            width: 58px;
            height: 32px;
            margin-right: 3px;
            background: var(--pc-header-search-accent, #1a1a1a);
            border: medium none;
            border-radius: 4px;
            color: #ffffff;
            cursor: pointer;
            text-align: center;
            font-weight: normal;
            font-size: 22px;
            line-height: 32px;
            display: flex;
            align-items: center;
            justify-content: center;
        }
    }

    .search-guess-box {
        background: #fff none repeat scroll 0 0;
        left: 0px;
        position: absolute;
        top: 50px;
        width: calc(100% - 32px);
        height: fit-content;
        overflow-y: auto;
        max-height: calc(100vh - 150px);
        border-radius: 4px;
        z-index: 8995;
        box-shadow:
            0 6px 56px 0 #0000000a,
            0 12px 35px 0px #0000000f;
        padding: 8px 16px;

        .search-guess-item {
            line-height: 30px;
            height: 30px;

            .serlink {
                display: flex;
                justify-content: left;
                align-items: center;
                width: 100%;
                height: 33px;
                padding: 0;
                border: 0;
                background: transparent;
                cursor: pointer;

                .name {
                    width: 260px;
                    height: 30px;
                    overflow: hidden;
                    display: inline-block;
                }
            }
        }
    }
}
</style>
