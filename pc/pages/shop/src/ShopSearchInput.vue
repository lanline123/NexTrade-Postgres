<template>
    <div class="top-search">
        <div class="search-form" :class="{ 'search-form-focus': searchFocus || keyword != '' }">
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
            <!-- 店铺页的相机只搜本店 -->
            <ImageSearchEntry :shop-id="imageSearchShopId" />
            <div class="search-buttons">
                <button class="search-btn shop-search" @click="onShopSearch">
                    {{ $t("搜本店") }}
                </button>
                <button class="search-btn site-search" @click="onSiteSearch">
                    {{ $t("搜全站") }}
                </button>
            </div>
        </div>
        <div ref="guessBox" class="search-guess-box" v-if="guessBoxShow">
            <ul class="search-guess-ul">
                <li v-for="(item, index) in guessKeywordList" :key="index">
                    <NuxtLink :to="'/search/?keyword=' + item.keyword" class="serlink">
                        <span class="name">{{ item.keyword }}</span>
                    </NuxtLink>
                </li>
            </ul>
        </div>
    </div>
</template>
<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getSearchGuessData } from "~/api/common";
import ImageSearchEntry from "~/components/imageSearch/ImageSearchEntry.vue";
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
if (routeKeyword.value && route.path === "/search/") {
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
// 关键词 URL 编码修复
const onSearchSubmit = () => {
    location.href = "/search/?keyword=" + encodeURIComponent(keyword.value);
};

/** 当前店铺 ID，取不到时相机按全站搜索 */
const imageSearchShopId = computed(() => {
    const shopId = Number(route.params.id || route.path.split("/").pop());
    return Number.isInteger(shopId) && shopId > 0 ? shopId : undefined;
});

const onShopSearch = () => {
    const shopId = route.params.id || route.path.split("/").pop();
    location.href = `/shop/search/${shopId}?keyword=${encodeURIComponent(keyword.value)}`;
};

const onSiteSearch = () => {
    location.href = "/search/?keyword=" + encodeURIComponent(keyword.value);
};

const guessBox = ref<HTMLElement | null>(null);

const guessBoxShow = ref(false);
const handleOutsideClick = (event: MouseEvent) => {
    if (guessBox.value && !guessBox.value.contains(event.target as Node)) {
        guessBoxShow.value = false;
    }
};

onMounted(() => {
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
    width: 700px;

    .search-form-focus .search-input {
        color: #333 !important;
    }

    .search-form {
        display: flex;
        align-items: center;
        background-color: #fff;
        border: 1px solid var(--general);
        border-radius: 8px;
        overflow: hidden;
        height: 40px;
        width: 100%;
        box-sizing: border-box;

        .search-input {
            flex: 1;
            background-color: #fff;
            border: none;
            color: #333;
            height: 100%;
            line-height: 36px;
            padding: 0 15px;
            outline: none;
            font-size: 14px;

            &::placeholder {
                color: #999;
            }
        }

        .search-buttons {
            display: flex;
            align-items: center;
            padding: 3px 2px;
            gap: 3px;
            height: 100%;

            .search-btn {
                border: none;
                cursor: pointer;
                height: 90%;
                padding: 0 20px;
                font-size: 14px;
                font-weight: bold;
                transition: all 0.3s;
                outline: none;
                border-radius: 5px;

                &.shop-search {
                    background-color: var(--vice-bg);
                    color: var(--general);

                    &:hover {
                        background-color: #ffe0e0;
                    }
                }

                &.site-search {
                    background-color: var(--general);
                    color: #fff;
                    border-left: 1px solid var(--general);

                    &:hover {
                        background-color: var(--general);
                    }
                }
            }
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
