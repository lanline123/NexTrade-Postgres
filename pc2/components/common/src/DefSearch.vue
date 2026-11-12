<template>
    <div class="top-search">
        <div class="search-form search-shell-dWh4ZzIyODI" :class="{ 'search-form-focus': searchFocus || keyword != '' }">
            <div class="acea-row row-middle">
                <div
                    v-if="shopSearchEnabled"
                    ref="typeSelectRef"
                    class="search-type-select"
                    :class="{ open: typeDropdownOpen }"
                    @click.stop="toggleTypeDropdown"
                >
                    <span class="search-type-text">{{ $t(searchTypeLabel) }}</span>
                    <i class="iconfont-pc icon-xiajiantou search-type-arrow"></i>

                    <div v-if="typeDropdownOpen" class="search-type-menu" @click.stop="">
                        <div class="search-type-item" :class="{ active: searchType === 'product' }" @click="setSearchType('product')">
                            {{ $t("商品") }}
                        </div>
                        <div class="search-type-item" :class="{ active: searchType === 'shop' }" @click="setSearchType('shop')">
                            {{ $t("门店") }}
                        </div>
                    </div>
                </div>
                <input
                    type="text"
                    autocomplete="off"
                    v-model="keyword"
                    :placeholder="$t('搜索')"
                    class="search-input"
                    name="keywords"
                    @focus="onSearchFocus"
                    @focusout="onSearchFocusout"
                    @keyup="onSearchKeyup"
                    @keydown.enter="onSearchSubmit"
                />
                <ImageSearchEntry v-if="searchType !== 'shop'" />
                <div type="submit" class="search-submit iconfont-pc icon-sousuo" @click="onSearchSubmit"></div>
            </div>
        </div>
        <div ref="guessBox" class="search-guess-box" v-if="guessBoxShow">
            <ul class="search-guess-ul">
                <li v-for="(item, index) in guessKeywordList" :key="index">
                    <NuxtLink :to="guessLink(item.keyword)" class="serlink">
                        <span class="name">{{ item.keyword }}</span>
                    </NuxtLink>
                </li>
            </ul>
        </div>
    </div>
</template>
<script setup lang="ts">
import type { PropType } from "vue";
import { computed, onMounted, ref } from "vue";
import { getSearchGuessData } from "~/api/common";
import { isMerchant, isO2o } from "~/utils/util";
import ImageSearchEntry from "~/components/imageSearch/ImageSearchEntry.vue";

const props = defineProps({
    /** 首页是否开启“搜门店”：1开启，0关闭 */
    indexSearchShop: {
        type: Number,
        default: 0
    },
    /**
     * 默认搜索类型
     * - product: 搜商品
     * - shop: 搜门店（仅在 indexSearchShop=1 时生效）
     */
    defaultSearchType: {
        type: String,
        default: "product"
    }
});

const shopSearchEnabled = computed(() => (isMerchant() || isO2o()) && props.indexSearchShop === 1);
const searchType = ref<string>("product");
const searchTypeLabel = computed(() => (searchType.value === "shop" ? "门店" : "商品"));
const typeDropdownOpen = ref(false);
const typeSelectRef = ref<HTMLElement | null>(null);
const toggleTypeDropdown = () => {
    typeDropdownOpen.value = !typeDropdownOpen.value;
};

const setSearchType = (type: string) => {
    searchType.value = type;
    typeDropdownOpen.value = false;
};

const guessLink = (kw: string) => {
    if (shopSearchEnabled.value && searchType.value === "shop") {
        return "/store/search?keyword=" + encodeURIComponent(kw);
    }
    return "/search/?keyword=" + encodeURIComponent(kw);
};

const guessKeywordList = ref<any>([]);

let searchGuessDisabled = false;
let setTime: any = null;
const onSearchKeyup = () => {
    if (searchGuessDisabled) {
        return false;
    }
    clearTimeout(setTime);
    searchGuessDisabled = true;
    setTime = setTimeout(async () => {
        try {
            const result = await getSearchGuessData(keyword.value);
            guessKeywordList.value = result;
            guessBoxShow.value = guessKeywordList.value.length > 0;
        } catch (error) {
        } finally {
            searchGuessDisabled = false;
        }
    }, 300);
};
const route = useRoute();
const routeKeyword = ref(route.query.keyword) as Ref<string>;
const searchFocus = ref(false);
const keyword = ref<string>("");
const _keyword = keyword.value;
if (routeKeyword.value && (route.path.toLowerCase() === "/search/" || route.path.toLowerCase() === "/store/search")) {
    keyword.value = routeKeyword.value;
}

watch(
    () => route.query,
    () => {
        if (!route.query.keyword) {
            keyword.value = "";
        }
    },
    { deep: true }
);

const onSearchFocus = () => {
    searchFocus.value = true;
    if (keyword.value === _keyword) {
        keyword.value = "";
    }
};
const onSearchFocusout = () => {
    if (keyword.value === "") {
        keyword.value = _keyword;
    }
    searchFocus.value = false;
};
const onSearchSubmit = () => {
    if (shopSearchEnabled.value && searchType.value === "shop") {
        location.href = "/store/search?keyword=" + encodeURIComponent(keyword.value);
        return;
    }
    location.href = "/search/?keyword=" + encodeURIComponent(keyword.value);
};

const guessBox = ref<HTMLElement | null>(null);

const guessBoxShow = ref(false);
const handleOutsideClick = (event: MouseEvent) => {
    if (typeDropdownOpen.value && typeSelectRef.value && !typeSelectRef.value.contains(event.target as Node)) {
        typeDropdownOpen.value = false;
    }
    if (guessBox.value && !guessBox.value.contains(event.target as Node)) {
        guessBoxShow.value = false;
    }
};

onMounted(() => {
    searchType.value = shopSearchEnabled.value ? props.defaultSearchType : "product";
    window.addEventListener("click", handleOutsideClick);
});

onUnmounted(() => {
    window.removeEventListener("click", handleOutsideClick);
});
</script>
<style scoped lang="less">
.acea-row {
    display: flex;
    flex-wrap: wrap;
}

.acea-row.row-middle {
    align-items: center;
}

.acea-row.row-right {
    justify-content: flex-end;
}

.acea-row.row-center-wrapper {
    align-items: center;
    justify-content: center;
}

.acea-row.row-between-wrapper {
    align-items: center;
    justify-content: space-between;
}
.top-search {
    position: relative;
    width: 100%;
    max-width: 700px;
    min-width: 200px;

    .search-form-focus .search-input {
        color: #333 !important;
    }

    .search-form {
        background-color: #f2f2f2;
        border-radius: 0;
        // overflow: hidden;
        height: 38px;

        .search-type-select {
            position: relative;
            height: 38px;
            display: inline-flex;
            align-items: center;
            padding: 0 10px 0 12px;
            user-select: none;
            cursor: pointer;
            background-color: #f2f2f2;
            color: #333;

            .search-type-text {
                font-size: 12px;
                line-height: 1;
                white-space: nowrap;
            }

            .search-type-arrow {
                font-size: 10px;
                margin-left: 6px;
                color: #999;
                transform: scale(0.9);
            }

            .search-type-menu {
                position: absolute;
                left: 0;
                top: 40px;
                width: 62px;
                box-sizing: border-box;
                background: #fff;
                box-shadow: 0 6px 18px rgba(0, 0, 0, 0.12);
                padding: 4px;
                z-index: 8996;

                .search-type-item {
                    height: 16px;
                    padding: 5px;
                    font-size: 12px;
                    color: #333;
                    display: flex;
                    align-items: center;
                    justify-content: center;

                    &.active {
                        color: var(--general);
                    }
                }
            }
        }

        .search-input {
            flex: 1;
            background-color: #f2f2f2;
            border: medium none;
            color: #bbbbbb;
            height: 26px;
            line-height: 26px;
            padding: 6px 10px;
            outline: medium none;
            width: auto;
            min-width: 0;
        }

        .search-submit {
            background: var(--main-bg);
            border: medium none;
            color: #ffffff;
            cursor: pointer;
            height: 38px;
            text-align: center;
            width: 50px;
            font-weight: normal;
            font-size: 22px;
            line-height: 36px;
        }
    }

    .search-hotkey {
        padding: 8px 0px;
        padding-left: 3px;
        height: 22px;
        line-height: 22px;
        overflow: hidden;

        a {
            margin-right: 15px;
            font-size: 12px;
            color: #999;

            b {
                color: var(--general);
            }
        }

        a:hover {
            color: var(--general);
        }
    }

    .search-guess-box {
        background: #fff none repeat scroll 0 0;
        box-shadow: 1px 2px 1px rgba(0, 0, 0, 0.2);
        border: 1px solid #ccc;
        left: 0px;
        position: absolute;
        top: 39px;
        width: 426px;
        z-index: 8995;

        li {
            line-height: 30px;
            height: 30px;
            padding: 0 10px;

            .serlink {
                display: flex;
                justify-content: left;
                align-items: center;

                .name {
                    width: 260px;
                    height: 30px;
                    overflow: hidden;
                    display: inline-block;
                }

                .conut {
                    width: 100px;
                    text-align: right;
                    height: 30px;
                    overflow: hidden;
                    display: inline-block;
                }
            }
        }
    }
}
</style>
