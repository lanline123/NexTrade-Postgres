<template>
    <CommonPageHeader />
    <ClientOnly>
        <Search v-if="activeToken" :key="`${activeToken}-${resetKey}`" page-type="/search/image" :search-token="activeToken">
            <template #top>
                <div class="image-search-summary">
                    <div class="image-search-summary__title">{{ $t("搜索原图") }}</div>
                    <div class="image-search-summary__thumb">
                        <el-image
                            v-if="imageUrl"
                            :src="imageUrl"
                            :preview-src-list="[imageUrl]"
                            :alt="$t('搜索使用的图片')"
                            fit="contain"
                            preview-teleported
                            hide-on-click-modal
                        />
                        <CameraIcon v-else class="image-search-summary__placeholder" />
                    </div>
                </div>
            </template>
        </Search>

        <!-- 开关读取中 / 检索中 / 功能已关闭正在退出：与商品搜索页首次加载一致 -->
        <div v-else-if="!ready || researching || !pageAvailable" class="image-search-loading">
            <div class="container image-search-loading__body">
                <SearchSkeleton />
            </div>
        </div>

        <!-- 没有图片，或编号因登录身份变化作废且原图不在：与商品搜索页无结果一致 -->
        <div v-else class="image-search-state">
            <div class="image-search-empty">
                <tig-icon name="shop" :size="50" />
                <span class="image-search-empty__text">{{ $t("{0}没有搜索到相关结果", [""]) }}</span>
            </div>
        </div>

        <template #fallback>
            <div class="image-search-loading">
                <div class="container image-search-loading__body">
                    <SearchSkeleton />
                </div>
            </div>
        </template>
    </ClientOnly>
    <CommonPageFooter :mt30="false" />
</template>

<script lang="ts" setup>
/**
 * 图片搜索结果页
 * - 入口弹层只负责选图；本页上传检索，取得图搜编号后写到地址上并展示结果
 * - 列表、筛选接口返回图搜编号失效时按没有数据展示，不自动重新检索
 * - 商品列表、排序、筛选和分页复用商品搜索页，查询范围限定在本次图片召回的相似商品
 * - 页头搜索框提交文字后按原路径进入普通文字结果，不做图文组合
 */
import { computed, onBeforeUnmount, onMounted, provide, ref, watch } from "vue";
import Search from "~/pages/search/components/search.vue";
import SearchSkeleton from "~/pages/search/components/SearchSkeleton.vue";
import CameraIcon from "~/components/icons/CameraIcon.vue";
import { message } from "ant-design-vue";
import { isImageSearchAborted, isImageSearchDisabled, resolveImageSearchErrorMessage, searchByImage } from "~/api/search/imageSearch";
import { useImageSearchStore } from "~/store/imageSearch";
import { useUserStore } from "~/store/user";
import { isSilentError } from "~/utils/request";

const RESULT_PATH = "/search/image";

const route = useRoute();
const router = useRouter();
const { t } = useI18n();
const imageSearchStore = useImageSearchStore();
const userStore = useUserStore();

const ready = ref(false);
const researching = ref(false);
/** 当前编号因登录身份变化而作废 */
const expired = ref(false);
const resetKey = ref(0);
let controller: AbortController | null = null;
/** 正在检索的图片，同一张图不重复发起 */
let researchingSource: File | null = null;
// 每次检索递增，被取代的旧请求不再生效
let researchSerial = 0;

/** 地址上的图搜编号 */
const routeToken = computed(() => {
    const raw = route.query.searchToken;
    const value = typeof raw === "string" ? raw : "";
    return /^[a-f0-9]{32}$/.test(value) ? value : "";
});

/** 店铺内搜索的店铺 ID，地址上没有时为全站；0 为平台自营 */
const routeShopId = computed(() => {
    const raw = route.query.shopId;
    const value = typeof raw === "string" ? raw : "";
    return /^\d+$/.test(value) ? Number(value) : undefined;
});

/** 本次搜索范围对应的开关：店铺内搜索还要看店铺开关 */
const pageAvailable = computed(() => imageSearchStore.isAvailable(routeShopId.value !== undefined));

// 图搜接口返回功能关闭时，按本页的搜索范围关闭对应开关
watch(
    routeShopId,
    (shopId) => {
        imageSearchStore.resultInShop = shopId !== undefined;
    },
    { immediate: true }
);

/** 当前用于展示结果的图搜编号；开关未开、重新搜索中或编号作废时为空 */
const activeToken = computed(() => {
    if (!ready.value || !pageAvailable.value || researching.value || expired.value) return "";
    return routeToken.value;
});

/** 地址上的编号对应的图片：优先本地原图，刷新后原图不在内存时用保存的缩小图 */
const imageUrl = computed(() => {
    if (imageSearchStore.query?.searchToken !== routeToken.value) return "";
    return imageSearchStore.previewUrl || imageSearchStore.query?.thumb || "";
});

/** 是否还留着这次查询的原图，可用来重新搜索 */
const canResearch = computed(() => Boolean(imageSearchStore.file) && imageSearchStore.query?.searchToken === routeToken.value);

/** 图片已选定但还没有检索：地址上没有编号，由本页发起 */
const hasPending = computed(() => !routeToken.value && canResearch.value);

/** 不带筛选和页码的结果页地址，保留店铺范围和装修预览参数 */
const cleanTarget = (searchToken: string) => {
    const previewId = Number(route.query.previewId) > 0 ? String(route.query.previewId) : "";
    const shopId = routeShopId.value !== undefined ? String(routeShopId.value) : "";
    return {
        path: RESULT_PATH,
        query: { ...(searchToken ? { searchToken } : {}), ...(shopId ? { shopId } : {}), ...(previewId ? { previewId } : {}) }
    };
};

// ===================== 操作 =====================
/**
 * 上传本地图片检索：首次检索和重新搜索共用；原图已不在本地时不检索，由页头相机重新发起
 */
const research = async (): Promise<boolean> => {
    const source = imageSearchStore.file;
    if (!source || !canResearch.value) return false;
    if (researching.value && researchingSource === source) return false;
    controller?.abort();
    const serial = ++researchSerial;
    researchingSource = source;
    researching.value = true;
    controller = new AbortController();
    try {
        const result = await searchByImage(source, controller.signal, routeShopId.value);
        if (serial !== researchSerial) return false;
        // 缩小图生成期间可能已离开页面或换了图，写入前和写入后都要确认仍是当前检索
        const committed = await imageSearchStore.commit(result.searchToken, source, () => serial === researchSerial);
        if (!committed || serial !== researchSerial) return false;
        expired.value = false;
        await navigateTo(cleanTarget(result.searchToken), { replace: true });
        return true;
    } catch (error) {
        // 离开页面时中止的请求不再提示；功能关闭由开关监听静默退出；登录失效已跳转登录页，不再提示也不返回上一页
        if (serial !== researchSerial || isImageSearchAborted(error) || isImageSearchDisabled(error) || isSilentError(error)) return false;
        // 与项目接口报错的处理一致：只提示原因，不单独展示失败页，回到发起搜索的页面
        message.error(resolveImageSearchErrorMessage(error, t));
        leave();
        return false;
    } finally {
        if (serial === researchSerial) {
            controller = null;
            researchingSource = null;
            researching.value = false;
        }
    }
};

/** 退出结果页：回到发起搜索的页面；没有上一页时替换为首页，返回键不再回到本页 */
const leave = (): void => {
    controller?.abort();
    if (window.history.state?.back) router.back();
    else void navigateTo("/", { replace: true });
};

const clearFilters = async (): Promise<void> => {
    await navigateTo(cleanTarget(routeToken.value));
    resetKey.value++;
};

provide("imageSearchContext", {
    clearFilters
});

/** 登录、退出或切换账号后旧查询作废；图片还在时自动重新搜索一次 */
const handleIdentityChange = (): void => {
    if (!routeToken.value || researching.value) return;
    if (imageSearchStore.isOwnedByCurrentIdentity(routeToken.value)) return;
    if (canResearch.value && pageAvailable.value) {
        void research();
    } else {
        expired.value = true;
    }
};

/** 地址上没有编号时检索刚选定的图片，有编号时核对它是否仍属于当前身份 */
const syncQuery = (): void => {
    if (!ready.value) return;
    if (routeToken.value) {
        handleIdentityChange();
    } else if (hasPending.value && pageAvailable.value) {
        void research();
    }
};

watch(() => userStore.token, syncQuery);

// 进入时读到已关闭，或任一图搜接口返回功能关闭
watch(
    () => ready.value && !pageAvailable.value,
    (disabled) => {
        if (disabled) leave();
    }
);

// 换了一张图或回到另一次查询时，清掉上一张图的失效状态
watch(routeToken, () => {
    expired.value = false;
    syncQuery();
});

// 已经停在没有编号的结果页时再选图，地址不变，靠图片变化发起检索
watch(
    () => imageSearchStore.file,
    () => {
        syncQuery();
    }
);

// ===================== 生命周期 =====================
onMounted(async () => {
    await Promise.all([imageSearchStore.restore(), imageSearchStore.fetchCapabilities()]);
    ready.value = true;
    syncQuery();
});

onBeforeUnmount(() => {
    // 作废进行中的检索：上传已完成、正在生成缩小图的任务也不再写入和跳转
    researchSerial++;
    controller?.abort();
    imageSearchStore.resultInShop = false;
});

useSeo({
    title: computed(() => t("图片搜索")),
    robots: "noindex,nofollow",
    canonical: RESULT_PATH
});
</script>

<style lang="less" scoped>
.image-search-summary {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    width: 100%;
    max-width: var(--pc-home-max-width, 1600px);
    min-width: var(--pc-home-min-width, 1264px);
    margin: 0 auto 12px;
    padding: 12px var(--pc-home-padding-x, 38px) 0;
    box-sizing: border-box;

    &__thumb {
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;
        width: 72px;
        height: 72px;
        border: 1px solid #f0f0f0;
        border-radius: 8px;
        background: #fafafa;
        overflow: hidden;

        :deep(.el-image) {
            width: 100%;
            height: 100%;
            cursor: zoom-in;
        }
    }

    &__placeholder {
        width: 28px;
        height: 28px;
        color: #bfbfbf;
    }

    &__title {
        color: #1f1f1f;
        font-size: 16px;
        font-weight: 600;
        line-height: 24px;
    }
}

// 对齐商品搜索页：页面上边距 8px、首屏最小高度
.image-search-loading {
    padding: 8px 0 30px;
    background: #fff;

    &__body {
        min-height: calc(100vh - 120px);
    }
}

// 与商品搜索页 .search-empty 一致
.image-search-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: #c2c4cc;
    font-size: 14px;

    &__text {
        margin-top: 12px;
    }
}

.image-search-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 560px;
    padding: 40px 16px;
    box-sizing: border-box;
    background: #fff;
}
</style>
