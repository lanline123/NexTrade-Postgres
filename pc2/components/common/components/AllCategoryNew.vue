<template>
    <div class="category-wrap" :class="{ white: isWhite }">
        <div ref="triggerRef" class="category-trigger" @mouseenter="handleMouseEnter" @mouseleave="handleMouseLeave">
            <div class="bar-wrap">
                <div class="content">
                    <div class="icon" aria-hidden="true">
                        <span class="pc-icon pc-icon--menu"></span>
                    </div>
                    <div class="txt">{{ $t("分类") }}</div>
                    <div class="icon ml">
                        <span class="pc-icon pc-icon--arrow arrow" :class="{ rotate: visible }"></span>
                    </div>
                </div>
            </div>
            <div v-show="visible" class="category-dropdown" :style="dropdownStyle" @mouseenter="handleDropdownEnter" @mouseleave="handleDropdownLeave">
                <div class="popover-cate-content">
                    <div v-if="displayCategories.length === 0" style="padding: 20px; text-align: center">{{ $t("暂无分类数据") }}</div>
                    <div v-else class="category-content-wrapper">
                        <div class="left-wrap">
                            <div
                                v-for="item in displayCategories"
                                :key="item.displayId"
                                class="cate_menu_item"
                                :class="{ active: String(currentLeftActiveId) === String(item.displayId) }"
                                @mouseenter="handleLeftMove(item.displayId)"
                            >
                                <span class="cate_menu_icon" aria-hidden="true">
                                    <img
                                        v-if="item.useImage === 1 && item.imageUrl"
                                        class="cate_menu_icon_img"
                                        :src="imageFormat(item.imageUrl)"
                                        :alt="getFloorImageAlt(item)"
                                    />
                                    <span v-else-if="item.floorIcoFont" class="floor-ico" :class="item.floorIcoFont"></span>
                                    <img
                                        v-else-if="item.picThumb || item.picUrl"
                                        class="cate_menu_icon_img"
                                        :src="productImage(item)"
                                        :alt="getFloorImageAlt(item)"
                                    />
                                </span>
                                <div class="cate_menu_text">
                                    <span class="cate_menu_text_inner">
                                        <template v-for="(cat, catIndex) in item.catFloorNames" :key="String(cat.categoryTitleName || catIndex)">
                                            <NuxtLink v-if="cat.categoryTitleName != '-'" target="_blank" class="cate_menu_lk" :to="cat.link" @click.stop>
                                                {{ cat.categoryTitleName }}
                                            </NuxtLink>
                                            <span
                                                v-if="
                                                    cat.categoryTitleName != '-' &&
                                                    catIndex < item.catFloorNames.filter((c) => c.categoryTitleName && c.categoryTitleName !== '-').length - 1
                                                "
                                                class="cate_menu_line"
                                            >
                                                /
                                            </span>
                                        </template>
                                    </span>
                                </div>
                            </div>
                        </div>
                        <div ref="rightBoxRef" class="right-wrap" @scroll="handeleRightScroll">
                            <template v-for="item in rightDisplayCategories" :key="item.displayId">
                                <div ref="secondItemRef" class="cate-item-wrap" :data-id="item.displayId">
                                    <div class="cate-title-row">
                                        <template
                                            v-for="(floorName, floorNameIndex) in item.catFloorNames"
                                            :key="String(floorName.categoryTitleName || floorNameIndex)"
                                        >
                                            <span class="cate-title">{{ floorName.categoryTitleName }}</span>
                                            <span v-if="floorNameIndex < item.catFloorNames.length - 1" class="cate-menu-line">、</span>
                                        </template>
                                    </div>
                                    <div class="second-class-container">
                                        <template v-for="cat in item.catList || []" :key="String(cat.categoryId ?? cat.id ?? cat.categoryName ?? cat.name)">
                                            <template v-if="(cat.children || []).length">
                                                <NuxtLink
                                                    v-for="second in cat.children || []"
                                                    :key="String(second.categoryId ?? second.id ?? second.categoryName ?? second.name)"
                                                    target="_blank"
                                                    class="second-class-item"
                                                    :class="{ 'no-img': !(second?.categoryPic || second?.pic) }"
                                                    :to="resolveCatUrl(second, item.catFloorId)"
                                                >
                                                    <div v-if="second?.categoryPic || second?.pic" class="item-pic">
                                                        <img :src="imageFormat(String(second?.categoryPic || second?.pic || ''))" :alt="getCategoryName(second)" />
                                                    </div>
                                                    <div class="item-name">{{ second.categoryName ?? second.name ?? "" }}</div>
                                                </NuxtLink>
                                            </template>
                                            <NuxtLink
                                                v-else-if="cat?.categoryPic || cat?.pic || cat?.categoryName || cat?.name"
                                                target="_blank"
                                                class="second-class-item"
                                                :class="{ 'no-img': !(cat?.categoryPic || cat?.pic) }"
                                                :to="resolveCatUrl(cat, item.catFloorId)"
                                            >
                                                <div v-if="cat?.categoryPic || cat?.pic" class="item-pic">
                                                    <img :src="imageFormat(String(cat?.categoryPic || cat?.pic || ''))" :alt="getCategoryName(cat)" />
                                                </div>
                                                <div class="item-name">{{ cat.categoryName ?? cat.name ?? "" }}</div>
                                            </NuxtLink>
                                        </template>
                                    </div>
                                </div>
                            </template>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
// ===================== Vue 核心 =====================
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";

// ===================== 工具函数 =====================
import { imageFormat, urlFormat } from "~/utils/format";
import { useLoadCss } from "~/utils/domUtils";

// ===================== Store =====================
import { useCommonStore } from "~/store/common";

// ===================== 类型 =====================
type CategoryNode = Record<string, any> & {
    categoryId?: string | number;
    id?: string | number;
    categoryName?: string;
    name?: string;
    categoryLink?: string;
    categoryPic?: string;
    pic?: string;
    seoLink?: string;
    children?: CategoryNode[] | null;
};

type CatFloorItem = Record<string, any> & {
    catList?: CategoryNode[];
    catFloorId?: string | number;
    catFloorNames?: Array<Record<string, unknown>>;
    floorIcoFont?: string;
    imageUrl?: string;
    useImage?: number;
};

// ===================== Props & Emits =====================
defineProps({
    isWhite: {
        type: Boolean,
        default: false
    }
});

// ===================== 响应式状态 =====================
const commonStore = useCommonStore();
const visible = ref<boolean>(false);
const secondItemRef = ref<HTMLElement[]>();
const rightBoxRef = ref<HTMLElement>();
const activeId = ref<string>("");
const triggerRef = ref<HTMLElement | null>(null);
const dropdownTopPx = ref<number>(0);

const SHOW_DELAY_MS = 300;

let showTimer: ReturnType<typeof setTimeout> | null = null;
let leaveTimer: ReturnType<typeof setTimeout> | null = null;

// ===================== 分类链接解析 =====================
const DIRECT_CATEGORY_LINK = /^(?:(?:https?:)?\/\/|\/|#|mailto:|tel:)/i;

function buildCategoryUrl(seoLink?: string, categoryId?: string | number, pId?: string | number): string {
    return urlFormat({ path: "list", seoLink, id: categoryId, pId });
}

function formatCustomCategoryLink(link: string): string {
    return urlFormat(DIRECT_CATEGORY_LINK.test(link) ? link : `/${link}`);
}

function resolveCatUrl(cat: CategoryNode, pId?: string | number): string {
    const categoryLink = String(cat.categoryLink || "").trim();
    if (categoryLink) {
        if (categoryLink === "list") {
            return buildCategoryUrl(cat.seoLink, cat.categoryId ?? cat.id, pId);
        }
        return formatCustomCategoryLink(categoryLink);
    }
    return buildCategoryUrl(cat.seoLink, cat.categoryId ?? cat.id, pId);
}

function resolveFloorTitleLink(categoryLink: string, payload: { seoLink?: string; categoryId?: string | number; catFloorId?: string | number }): string {
    const link = String(categoryLink || "").trim();
    if (!link || link === "list") {
        return buildCategoryUrl(payload.seoLink, payload.categoryId, payload.catFloorId);
    }
    return formatCustomCategoryLink(link);
}

function getCategoryName(cat?: CategoryNode | null): string {
    return String(cat?.categoryName ?? cat?.name ?? "").trim();
}

function getFloorImageAlt(item: { catFloorNames?: Array<{ categoryTitleName?: string }> }): string {
    return (item.catFloorNames ?? [])
        .map((cat) => String(cat.categoryTitleName ?? "").trim())
        .filter(Boolean)
        .join("/");
}

// ===================== 分类数据 =====================
const displayCategories = computed(() => {
    if (!commonStore.allCate || !Array.isArray(commonStore.allCate)) return [];

    const toId = (value: unknown): string | number | "" => (typeof value === "string" || typeof value === "number" ? value : "");

    return commonStore.allCate
        .map((floor: CatFloorItem, floorIdx: number) => {
            const catList: CategoryNode[] = Array.isArray(floor.catList) ? floor.catList : [];
            const catFloorId = toId(floor["catFloorId"]);
            const displayId = `${String(catFloorId || "floor")}-${floorIdx}`;
            const firstCat = catList.find((cat) => Boolean(cat.categoryId ?? cat.id) && (cat.categoryName ?? cat.name) !== "-");
            const categoryId = (firstCat?.categoryId ?? firstCat?.id ?? catFloorId) || "";
            const seoLink = firstCat?.seoLink ?? "";
            const picFromApi = typeof firstCat?.["pic"] === "string" ? (firstCat["pic"] as string) : "";
            const pic = (typeof firstCat?.categoryPic === "string" ? firstCat.categoryPic : "") || picFromApi;
            const imageUrl = typeof floor["imageUrl"] === "string" ? (floor["imageUrl"] as string) : "";
            const useImage = Number(floor["useImage"] ?? (imageUrl ? 1 : 0));
            const floorIcoFont = typeof floor["floorIcoFont"] === "string" ? (floor["floorIcoFont"] as string) : "";
            const rawFloorNameList = Array.isArray(floor["catFloorNames"]) ? (floor["catFloorNames"] as Array<Record<string, unknown>>) : [];
            const catFloorNames =
                rawFloorNameList.length > 0
                    ? rawFloorNameList
                          .map((item) => ({
                              categoryTitleName: String(item["categoryTitleName"] ?? "").trim(),
                              link: resolveFloorTitleLink(String(item["categoryLink"] ?? "").trim(), { seoLink, categoryId, catFloorId })
                          }))
                          .filter((item) => item.categoryTitleName && item.categoryTitleName !== "-")
                    : catList
                          .map((cat) => ({
                              categoryTitleName: String(cat.categoryName ?? cat.name ?? "").trim(),
                              link: resolveCatUrl(cat, catFloorId)
                          }))
                          .filter((item) => item.categoryTitleName && item.categoryTitleName !== "-");

            return {
                displayId,
                catFloorId,
                catFloorNames,
                picThumb: pic,
                picUrl: pic,
                imageUrl,
                useImage,
                floorIcoFont,
                catList
            };
        })
        .filter((item: { catList: CategoryNode[] }) => item.catList.length > 0);
});

// ===================== 事件处理方法 =====================
function clearShowTimer(): void {
    if (!showTimer) return;
    clearTimeout(showTimer);
    showTimer = null;
}

function clearLeaveTimer(): void {
    if (!leaveTimer) return;
    clearTimeout(leaveTimer);
    leaveTimer = null;
}

const handleMouseEnter = (): void => {
    clearLeaveTimer();
    if (visible.value || showTimer) return;
    showTimer = setTimeout(() => {
        visible.value = true;
        activeId.value = displayCategories.value?.[0]?.displayId ?? "";
        scheduleDropdownTopSync();
        showTimer = null;
    }, SHOW_DELAY_MS);
};

const handleMouseLeave = (): void => {
    clearShowTimer();
    leaveTimer = setTimeout(() => {
        visible.value = false;
    }, 150);
};

const handleDropdownEnter = (): void => {
    clearLeaveTimer();
    scheduleDropdownTopSync();
};

const handleDropdownLeave = (): void => {
    clearShowTimer();
    leaveTimer = setTimeout(() => {
        visible.value = false;
    }, 150);
};

function syncDropdownTop(): void {
    if (!import.meta.client) return;
    const trigger = triggerRef.value;
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    dropdownTopPx.value = Math.max(0, Math.round(rect.bottom));
}

function scheduleDropdownTopSync(): void {
    if (!import.meta.client) return;
    nextTick(() => syncDropdownTop());
}

const dropdownStyle = computed(() => {
    if (!visible.value) return {};
    return {
        top: `${dropdownTopPx.value}px`
    } as Record<string, string>;
});

const currentLeftActiveId = computed(() => activeId.value);
const rightDisplayCategories = computed(() => {
    if (!activeId.value) return displayCategories.value;
    const current = displayCategories.value.find((item: { displayId: string }) => item.displayId === activeId.value);
    return current ? [current] : displayCategories.value;
});

const RIGHT_SCROLL_EDGE_PADDING = 20;

const handleLeftMove = (id: string): void => {
    activeId.value = id;
    nextTick(() => {
        const rightBox = rightBoxRef.value;
        if (rightBox) rightBox.scrollTop = 0;
    });
};

const handeleRightScroll = (): void => {
    if (!rightBoxRef.value) return;
    const scrollBoxRect = rightBoxRef.value.getBoundingClientRect();
    let matchedId: string | null = null;
    let minRelativeTop = Number.POSITIVE_INFINITY;

    secondItemRef.value?.forEach((item: HTMLElement) => {
        const itemRect = item.getBoundingClientRect();
        const relativeTop = itemRect.top - scrollBoxRect.top - RIGHT_SCROLL_EDGE_PADDING;
        if (relativeTop >= 0 && relativeTop < 50 && relativeTop < minRelativeTop) {
            minRelativeTop = relativeTop;
            matchedId = item.dataset.id ?? "";
        }
    });

    if (matchedId !== null) {
        activeId.value = matchedId;
    }
};

// ===================== 初始化 =====================
onMounted(async () => {
    await commonStore.ensureAllCate();
    // 分类楼层字体图标（floorIcoFont）依赖后端动态 @font-face；非首页头部也需主动加载一次
    if (import.meta.client && commonStore.catIcoCss) {
        useLoadCss(commonStore.catIcoCss);
    }
    if (import.meta.client) {
        window.addEventListener("scroll", syncDropdownTop, { passive: true });
        window.addEventListener("resize", syncDropdownTop, { passive: true });
    }
});

watch(
    () => commonStore.catIcoCss,
    (css) => {
        if (import.meta.client && css) useLoadCss(css);
    }
);

watch(
    displayCategories,
    (list) => {
        if (!list.length) {
            activeId.value = "";
            return;
        }
        const exists = list.some((item) => item.displayId === activeId.value);
        if (!exists) {
            activeId.value = list[0]?.displayId ?? "";
        }
    },
    { immediate: true }
);

onBeforeUnmount(() => {
    clearShowTimer();
    clearLeaveTimer();
    if (import.meta.client) {
        window.removeEventListener("scroll", syncDropdownTop);
        window.removeEventListener("resize", syncDropdownTop);
    }
});
</script>

<style lang="less" scoped>
/* 与 Admin pc-header 预览共用同一套 mask 图标（overseas-cat / overseas-arrowdown，14×14） */
@import "~/decorate/components/pc/pcHeaderRuntime/_pcHeaderPreviewIcons.less";

.category-wrap {
    height: 100%;
    position: relative;
    .category-trigger {
        height: 100%;
        position: relative;
    }
    .category-dropdown {
        position: fixed;
        top: 0;
        left: 50%;
        transform: translateX(-55%);
        z-index: 5000;
        width: 1070px;
        max-width: 1866px;
        background: #fff;
        border: 1px solid #f2f2f2;
        border-radius: 10px;
        box-shadow: 0 6px 18px rgba(0, 0, 0, 0.12);
        overflow: hidden;
    }
    .bar-wrap {
        cursor: pointer;
        display: flex;
        align-items: center;
        height: 100%;
        .content {
            display: flex;
            height: 28px;
            &:hover {
                .icon {
                    color: #fffc;
                }
                .txt {
                    color: #fffc;
                    font-weight: 500;
                    text-transform: capitalize;
                }
            }
            .icon {
                align-self: center;
                color: #fff;
                /* 消除 inline-block 基线空隙；固定为 Admin 实测的 14×16 */
                display: inline-flex;
                align-items: center;
                justify-content: center;
                width: 14px;
                height: 16px;
                line-height: 1;
                font-size: 0;

                &.ml {
                    margin-left: 4px;
                    /* 箭头图标在画板中偏上，整体稍微下移一点以对齐文字视觉中心 */
                    margin-top: 4px;
                }

                .pc-icon {
                    display: block;
                }

                .arrow {
                    transition: transform 0.2s;
                    /* 修正 mask svg 图标本身未居中导致的旋转偏移问题 */
                    transform-origin: 50% 36%;

                    &.rotate {
                        transform: rotate(180deg);
                    }
                }
            }
            .txt {
                align-self: center;
                margin-left: 8px;
                font-size: 14px;
                /* 14px 字号 + 浏览器 normal ≈ 20px；显式写出，避免随全局 html font-size 漂移 */
                line-height: 20px;
                color: #fff;
                font-weight: 500;
                text-transform: capitalize;
            }
        }
    }
    &.white {
        .bar-wrap {
            .content {
                &:hover {
                    .icon {
                        color: var(--primary-dark-2);
                    }
                    .txt {
                        color: var(--primary-dark-2);
                    }
                }
                .icon {
                    color: #1a1a1a;
                }
                .txt {
                    color: #1a1a1a;
                }
            }
        }
    }
}
.popover-cate-content {
    position: relative;
    display: flex;
    column-gap: 25px;
    height: 600px;
    padding: 20px;
    box-sizing: border-box;

    .category-content-wrapper {
        display: flex;
        flex: 1 1 auto;
        width: 100%;
        min-width: 0;
        flex-wrap: nowrap;
    }

    .left-wrap {
        position: relative;
        width: 252px;
        flex: 0 0 252px;
        overflow-y: auto;
        overflow-x: hidden;
        padding-right: 20px;
        &::-webkit-scrollbar {
            width: 0;
            height: 0;
        }
        .cate_menu_item {
            display: flex;
            align-items: center;
            gap: 6px;
            cursor: pointer;
            box-sizing: border-box;
            width: 252px;
            min-width: 0;
            height: 32px;
            line-height: 32px;
            padding: 0 10px;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            font-size: 16px;
            font-weight: 700;
            color: #1a1a1a;
            line-height: 18px;
            &:hover {
                background: #f2f2f2;
                color: #1a1a1a;
            }
            &.active {
                background: #f2f2f2;
                color: #1a1a1a;
            }
            .cate_menu_icon {
                width: 18px;
                height: 18px;
                flex-shrink: 0;
                display: inline-flex;
                align-items: center;
                justify-content: center;
                color: #333333;
            }
            .cate_menu_icon_img {
                width: 18px;
                height: 18px;
                object-fit: contain;
                display: block;
            }
            .cate_menu_icon .floor-ico {
                font-size: 18px;
                line-height: 1;
                display: inline-block;
                // 字体图标不继承父级 .cate_menu_item 的 700，避免被合成加粗，与 dec-pc-home-hero 一致
                font-weight: normal;
            }
            .cate_menu_text {
                display: flex;
                align-items: center;
                flex: 1;
                min-width: 0;
                overflow: hidden;
            }
            .cate_menu_text_inner {
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
                height: 30px;
                line-height: 30px;
            }
            .cate_menu_lk {
                font-size: 14px;
                color: #1a1a1a;
                font-weight: 400;
                cursor: pointer;
                transition: color 0.2s ease;
                padding: 9px 4px;
                &:hover {
                    color: var(--general);
                }
            }
            .cate_menu_line {
                font-size: 12px;
                color: #666;
            }
        }
        &::after {
            display: block;
            position: absolute;
            top: 0;
            left: 268px;
            content: "";
            width: 1px;
            height: 100%;
            background: #f0f1f2;
        }
    }
    .right-wrap {
        position: absolute;
        top: 20px;
        right: 20px;
        bottom: 20px;
        left: 312px;
        flex: 1 1 auto;
        min-width: 0;
        padding: 20px 0 20px 14px;
        box-sizing: border-box;
        scroll-padding-block: 20px;
        overflow-y: auto;
        &::-webkit-scrollbar {
            width: 0;
            height: 0;
        }
        .cate-item-wrap {
            margin-bottom: 20px;

            .cate-title-row {
                display: flex;
                flex-wrap: wrap;
                align-items: center;
                margin-bottom: 4px;
            }
            .cate-title {
                font-size: 16px;
                font-weight: 700;
                color: #1a1a1a;
            }
            .cate-menu-line {
                font-size: 16px;
                font-weight: 700;
                color: #1a1a1a;
            }

            /* ========== 模式 1 + 图文模式（原始图文布局） ========== */
            .second-class-container {
                display: flex;
                gap: 12px;
                margin-top: 12px;
                flex-wrap: wrap;
                .second-class-item {
                    box-sizing: border-box;
                    cursor: pointer;
                    width: 90px;
                    height: 130px;
                    margin-bottom: 8px;
                    padding: 4px 4px 6px 4px;
                    border-radius: 4px;
                    border: 1px solid transparent;
                    &.no-img {
                        align-self: center;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        height: 50px !important;
                        background: #f5f7f8;
                        .item-name {
                            margin: 6px;
                        }
                        &:hover {
                            background: #fff;
                        }
                    }
                    .cate_menu_lk {
                        display: flex;
                        flex-direction: column;
                        align-items: center;
                    }
                    &:hover {
                        border: 1px solid var(--main-bg);
                        .item-name {
                            color: var(--main-bg);
                        }
                    }
                    .item-pic {
                        width: 82px;
                        height: 82px;
                        border-radius: 4px;
                        overflow: hidden;
                        img {
                            width: 100%;
                            height: 100%;
                            object-fit: contain;
                        }
                    }
                    .item-name {
                        width: 100%;
                        text-align: center;
                        font-size: 12px;
                        color: #1a1a1a;
                        margin: 0 4px 6px;
                        margin-top: 4px;
                        overflow: hidden;
                        text-overflow: ellipsis;
                        display: -webkit-box;
                        line-clamp: 2;
                        -webkit-line-clamp: 2;
                        -webkit-box-orient: vertical;
                        word-break: break-word;
                    }
                }
            }
        }
    }
}
</style>
