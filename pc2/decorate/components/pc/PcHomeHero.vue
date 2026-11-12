<template>
    <div class="dec-pc-home-hero" :class="{ 'is-cate-expanded': activeSubView !== null }" :style="cssVars">
        <div class="dec-pc-home-hero__layout">
            <!-- Left -->
            <section ref="cateLeftRef" class="dec-pc-home-hero__panel dec-pc-home-hero__left">
                <div class="dec-pc-home-hero__cate-area" @mouseleave="onCateAreaLeave">
                    <div v-if="displayCategories.length" class="dec-pc-home-hero__cate-list">
                        <div
                            v-for="(item, idx) in displayCategories"
                            :key="String(item.picId ?? idx)"
                            class="dec-pc-home-hero__cate-item"
                            :class="{ 'is-active': activeSubView === idx }"
                            @mouseenter="showSubView(idx)"
                        >
                            <span class="dec-pc-home-hero__cate-icon" aria-hidden="true">
                                <i v-if="item.floorIcoFont" :class="item.floorIcoFont"></i>
                            </span>
                            <div class="dec-pc-home-hero__cate-text">
                                <div class="dec-pc-home-hero__cate-title">
                                    <template v-if="item.catFloorNames?.length">
                                        <template v-for="(cat, cIdx) in item.catFloorNames" :key="String(cat.categoryTitleName || cIdx)">
                                            <NuxtLink
                                                v-if="cat.categoryTitleName && cat.categoryTitleName !== '-'"
                                                target="_blank"
                                                class="dec-pc-home-hero__cate-link"
                                                :to="cat.link"
                                            >
                                                {{ cat.categoryTitleName }}
                                            </NuxtLink>
                                            <span
                                                v-if="
                                                    cat.categoryTitleName &&
                                                    cat.categoryTitleName !== '-' &&
                                                    cIdx < item.catFloorNames.filter((c) => c.categoryTitleName && c.categoryTitleName !== '-').length - 1
                                                "
                                                class="dec-pc-home-hero__cate-sep"
                                                >/
                                            </span>
                                        </template>
                                    </template>
                                    <template v-else>
                                        <span class="dec-pc-home-hero__cate-link" @click="handleProductCardClick(item.picLink)">
                                            {{ item.picTitle || $t("类目") }}
                                        </span>
                                    </template>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </section>

            <!-- 子类目弹层：Teleport 到 body，彻底脱离父级 overflow / z-index 限制 -->
            <Teleport to="body">
                <div
                    v-if="displayCategories.length && activeSubView !== null"
                    ref="subCataRef"
                    class="dec-pc-home-hero__subCata"
                    :class="{ 'is-positioned': hasSubCataPosition }"
                    :style="teleportSubCataStyle"
                    @mouseenter="onSubCataEnter"
                    @mouseleave="onSubCataLeave"
                >
                    <template v-for="(floor, index) in displayCategories" :key="String(floor.picId ?? index)">
                        <div class="dec-pc-home-hero__subView" :class="{ 'is-active': activeSubView === index }">
                            <div class="dec-pc-home-hero__sub-left">
                                <div class="dec-pc-home-hero__nav_cata_bd">
                                    <!-- 子级为空时退化展示一级分类卡片，避免右侧空白 -->
                                    <div class="dec-pc-home-hero__img-grid">
                                        <template v-for="cat in floor.catList || []" :key="String(cat.categoryId ?? cat.id ?? cat.categoryName ?? cat.name)">
                                            <template v-if="(cat.children || []).length">
                                                <NuxtLink
                                                    v-for="child in cat.children || []"
                                                    :key="String(child.categoryId ?? child.id ?? child.categoryName ?? child.name)"
                                                    class="dec-pc-home-hero__img-item"
                                                    target="_blank"
                                                    :to="resolveCatUrl(child, floor.picId)"
                                                >
                                                    <div class="dec-pc-home-hero__img-pic">
                                                        <img
                                                            v-if="child.categoryPic || child.pic"
                                                            :src="imageFormat(String(child.categoryPic || child.pic || ''))"
                                                            alt=""
                                                            @error="handleImgError"
                                                        />
                                                        <div v-else class="dec-pc-home-hero__img-placeholder"></div>
                                                    </div>
                                                    <div class="dec-pc-home-hero__img-name">{{ child.categoryName ?? child.name }}</div>
                                                </NuxtLink>
                                            </template>
                                            <NuxtLink
                                                v-else-if="cat.categoryPic || cat.pic || cat.categoryName || cat.name"
                                                class="dec-pc-home-hero__img-item"
                                                target="_blank"
                                                :to="resolveCatUrl(cat, floor.picId)"
                                            >
                                                <div class="dec-pc-home-hero__img-pic">
                                                    <img
                                                        v-if="cat.categoryPic || cat.pic"
                                                        :src="imageFormat(String(cat.categoryPic || cat.pic || ''))"
                                                        alt=""
                                                        @error="handleImgError"
                                                    />
                                                    <div v-else class="dec-pc-home-hero__img-placeholder"></div>
                                                </div>
                                                <div class="dec-pc-home-hero__img-name">{{ cat.categoryName ?? cat.name }}</div>
                                            </NuxtLink>
                                        </template>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </template>
                </div>
            </Teleport>

            <!-- Center -->
            <section class="dec-pc-home-hero__center">
                <div class="dec-pc-home-hero__center-top">
                    <!-- Banner -->
                    <div class="dec-pc-home-hero__banner">
                        <div v-if="banners.length" class="dec-pc-home-hero__banner-inner">
                            <Swiper :swiper-option="swiperOption" :item-list="banners">
                                <template #default="{ item }">
                                    <div class="dec-pc-home-hero__banner-slide">
                                        <div class="dec-pc-home-hero__banner-link" @click="handleProductCardClick(item.picLink)">
                                            <img
                                                class="dec-pc-home-hero__banner-img"
                                                :src="formatImage(item.picUrl)"
                                                :alt="item.picTitle || ''"
                                                @error="handleImgError"
                                            />
                                        </div>
                                    </div>
                                </template>
                            </Swiper>
                        </div>
                    </div>

                    <!-- Promo Group (Moved here, Items 0-1) -->
                    <div class="dec-pc-home-hero__promo-group">
                        <div
                            v-for="(card, idx) in promoCards"
                            :key="idx"
                            class="dec-pc-home-hero__panel dec-pc-home-hero__promo-card"
                            @click="handlePromoCardClick(card.link)"
                        >
                            <div class="dec-pc-home-hero__card-header">
                                <div class="dec-pc-home-hero__card-title" :title="card.moduleTitle || ''">{{ card.moduleTitle || $t("促销") }}</div>
                                <div v-if="card.moduleSubTitle" class="dec-pc-home-hero__card-badge" :style="{ background: getSubTitleBg(card) }">
                                    {{ card.moduleSubTitle }}
                                </div>
                            </div>

                            <div class="dec-pc-home-hero__card-products">
                                <template v-if="card.products && card.products.length">
                                    <div
                                        v-for="(p, pIdx) in card.products.slice(0, 2)"
                                        :key="pIdx"
                                        class="dec-pc-home-hero__card-product"
                                        @click.stop="handleProductClick(p)"
                                    >
                                        <div class="dec-pc-home-hero__card-img-box">
                                            <img
                                                class="dec-pc-home-hero__card-img"
                                                :src="formatImage(pickPic(p.pic))"
                                                :alt="p.productName || ''"
                                                @error="handleImgError"
                                            />
                                            <div v-if="p.label" class="dec-pc-home-hero__card-label" :style="getLabelStyle(p)">
                                                {{ p.label }}
                                            </div>
                                        </div>
                                        <div v-if="card.showPrice !== 0" class="dec-pc-home-hero__card-price">
                                            <FormatPrice :model-value="resolveCardProductPrice(p)" />
                                        </div>
                                    </div>
                                </template>
                            </div>
                        </div>
                    </div>
                </div>

                <div class="dec-pc-home-hero__center-bottom">
                    <!-- 频道卡片 (Items 2-5) -->
                    <div v-if="productCards.length" class="dec-pc-home-hero__product-grid">
                        <div v-for="(card, idx) in productCards" :key="idx" class="dec-pc-home-hero__channel-card" @click="handleProductCardClick(card.link)">
                            <div class="dec-pc-home-hero__card-header">
                                <div class="dec-pc-home-hero__card-title" :title="card.moduleTitle || ''">{{ card.moduleTitle || $t("推荐") }}</div>
                                <div v-if="card.moduleSubTitle" class="dec-pc-home-hero__card-badge" :style="{ background: getSubTitleBg(card) }">
                                    {{ card.moduleSubTitle }}
                                </div>
                            </div>

                            <div class="dec-pc-home-hero__card-products">
                                <template v-if="card.products && card.products.length">
                                    <div
                                        v-for="(p, pIdx) in card.products.slice(0, 2)"
                                        :key="pIdx"
                                        class="dec-pc-home-hero__card-product"
                                        @click.stop="handleProductClick(p)"
                                    >
                                        <div class="dec-pc-home-hero__card-img-box">
                                            <img
                                                class="dec-pc-home-hero__card-img"
                                                :src="formatImage(pickPic(p.pic))"
                                                :alt="p.productName || ''"
                                                @error="handleImgError"
                                            />
                                            <div v-if="p.label" class="dec-pc-home-hero__card-label" :style="getLabelStyle(p)">
                                                {{ p.label }}
                                            </div>
                                        </div>
                                        <div v-if="card.showPrice !== 0" class="dec-pc-home-hero__card-price">
                                            <FormatPrice :model-value="resolveCardProductPrice(p)" />
                                        </div>
                                    </div>
                                </template>
                            </div>
                        </div>
                    </div>
                </div>
            </section>

            <!-- Right -->
            <section class="dec-pc-home-hero__right">
                <!-- 1. User Panel -->
                <div class="dec-pc-home-hero__panel dec-pc-home-hero__user-card">
                    <div
                        v-if="premiumMemberConfig.show"
                        :class="['dec-pc-home-hero__premium-member', { 'is-title-only': !premiumMemberConfig.description }]"
                        :style="{ backgroundImage: `url(${premiumMemberEntry})` }"
                        :title="premiumMemberConfig.description || premiumMemberConfig.title"
                        @click="handlePremiumMemberClick"
                    >
                        <span class="dec-pc-home-hero__premium-member-title">{{ premiumMemberConfig.title }}</span>
                        <span v-if="premiumMemberConfig.description" class="dec-pc-home-hero__premium-member-desc">
                            {{ premiumMemberConfig.description }}
                        </span>
                    </div>
                    <div class="dec-pc-home-hero__user-top" :class="{ 'is-authed': isAuthed, 'has-premium-member': premiumMemberConfig.show }">
                        <img class="dec-pc-home-hero__avatar" :src="userAvatarSrc" alt="" @error="handleImgError" @click="onUserCenterClick" />
                        <div class="dec-pc-home-hero__user-info">
                            <div class="dec-pc-home-hero__user-name" :title="userDisplayName" @click="onUserCenterClick">{{ userDisplayName }}</div>
                            <div class="dec-pc-home-hero__user-links">
                                <template v-if="isAuthed">
                                    <span class="dec-pc-home-hero__user-link" @click="onAccountSettingsClick">{{ $t("账号设置") }}</span>
                                    <span class="divider"></span>
                                    <span class="dec-pc-home-hero__user-link" @click="onAddressClick">{{ $t("收货地址") }}</span>
                                </template>
                                <template v-else>
                                    <template v-if="commonStore.shopRegClosed === 0">
                                        <span class="dec-pc-home-hero__user-link" @click="onRegisterClick">{{ $t("注册") }}</span>
                                        <span class="divider"></span>
                                    </template>
                                    <span v-if="canAccessJoin" class="dec-pc-home-hero__user-link" @click="onJoiningClick">{{ $t("入驻") }}</span>
                                    <span v-else class="dec-pc-home-hero__user-link" @click="onLoginClick">{{ $t("登录") }}</span>
                                </template>
                            </div>
                        </div>
                    </div>

                    <template v-if="isAuthed">
                        <div class="dec-pc-home-hero__user-assets" :class="{ 'dec-pc-home-hero__user-assets--wide': needWideAssetCols }">
                            <div class="dec-pc-home-hero__asset-item" :title="couponFullText" @click="onCouponClick">
                                <div class="dec-pc-home-hero__asset-num">{{ couponCount }}</div>
                                <div class="dec-pc-home-hero__asset-label">{{ $t("优惠券") }}</div>
                            </div>
                            <div class="dec-pc-home-hero__asset-item" :title="balanceFullText" @click="onBalanceClick">
                                <div class="dec-pc-home-hero__asset-num">
                                    <span class="dec-pc-home-hero__asset-num-value">{{ balanceDisplay.value }}</span>
                                    <span v-if="balanceDisplay.suffix" class="dec-pc-home-hero__asset-num-suffix">{{ balanceDisplay.suffix }}</span>
                                </div>
                                <div class="dec-pc-home-hero__asset-label">{{ $t("余额") }}</div>
                            </div>
                            <div class="dec-pc-home-hero__asset-item" :title="pointsFullText" @click="onPointClick">
                                <div class="dec-pc-home-hero__asset-num">
                                    <span class="dec-pc-home-hero__asset-num-value">{{ pointsDisplay.value }}</span>
                                    <span v-if="pointsDisplay.suffix" class="dec-pc-home-hero__asset-num-suffix">{{ pointsDisplay.suffix }}</span>
                                </div>
                                <div class="dec-pc-home-hero__asset-label">{{ $t("积分") }}</div>
                            </div>
                        </div>
                    </template>
                    <template v-else>
                        <div class="dec-pc-home-hero__user-desc">
                            <div class="title">{{ rightUser.welcomeText || $t("登录后更多精彩") }}</div>
                            <div class="subtitle">{{ $t("一键登录，福利好物全拥有") }}</div>
                        </div>
                        <el-button class="dec-pc-home-hero__login-btn" type="primary" @click="onLoginClick">{{ $t("立即登录") }}</el-button>
                    </template>
                    <div class="dec-pc-home-hero__user-actions" :style="{ 'grid-template-columns': `repeat(${userActionCount}, 1fr)` }">
                        <div class="dec-pc-home-hero__action-item" @click="onMyOrdersClick">
                            <SvgIcon name="pchomeorder" width="16" height="16" />
                            <span>{{ $t("我的订单") }}</span>
                        </div>
                        <div class="dec-pc-home-hero__action-item" @click="onBrowsingHistoryClick">
                            <SvgIcon name="record" width="16" height="16" />
                            <span>{{ $t("浏览记录") }}</span>
                        </div>
                        <div class="dec-pc-home-hero__action-item" @click="onCollectProductClick">
                            <SvgIcon name="loveSolid" width="16" height="16" />
                            <span>{{ $t("心愿单") }}</span>
                        </div>
                        <div v-if="showCollectShop" class="dec-pc-home-hero__action-item" @click="onCollectShopClick">
                            <SvgIcon name="pchomeshop" width="16" height="16" />
                            <span>{{ collectShopLabel }}</span>
                        </div>
                    </div>
                </div>

                <!-- 2. Shops Panel -->
                <div class="dec-pc-home-hero__panel dec-pc-home-hero__shops">
                    <div class="dec-pc-home-hero__shops-header">
                        <div class="title">
                            {{ shopsConfig.title }}
                            <span v-if="shopsConfig.subTitle" class="hot" :style="{ background: shopsConfig.subTitleBg }">{{ shopsConfig.subTitle }}</span>
                        </div>
                        <div class="more" @click="handleProductCardClick(shopsConfig.moreLink)">
                            {{ shopsConfig.moreText }}
                            <SvgIcon name="xjt" width="9" height="9" class="more-icon" />
                        </div>
                    </div>
                    <div v-if="rightShortcuts.length" class="dec-pc-home-hero__shop-list">
                        <div
                            v-for="(sc, idx) in rightShortcuts"
                            :key="String(sc.picId ?? idx)"
                            class="dec-pc-home-hero__shop-item"
                            @click="handleProductCardClick(sc.picLink)"
                        >
                            <div class="dec-pc-home-hero__shop-img-box">
                                <img class="dec-pc-home-hero__shop-icon" :src="formatImage(pickPic(sc))" alt="" @error="handleImgError" />
                            </div>
                            <span class="dec-pc-home-hero__shop-enter" :style="{ color: shopsConfig.textColor }">{{ sc.picTitle || $t("进入店铺") }}</span>
                        </div>
                    </div>
                </div>
            </section>
        </div>
    </div>
</template>

<script setup lang="ts">
/**
 * PC 首页焦点（pc-home-hero） - 渲染组件
 * - 三列布局：左类目 / 中轮播+频道卡 / 右会员+店铺街
 * - 一比一还原后台 Preview.vue
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import Swiper from "~/decorate/components/swiper/Swiper.vue";
import { useUserStore } from "~/store/user";
import { useCommonStore } from "~/store/common";
import { useThemeStore } from "~/store/theme";
import { formatAssetFullI18n, formatAssetNumberI18n, imageFormat, pickPic, urlFormat } from "~/utils/format";
import { buildSubCataFixedStyle } from "~/decorate/subCataPosition";
import { useLoadCss } from "~/utils/domUtils";
import { isMerchant, isOverseas, isStore } from "~/utils/util";
import { getProductsList } from "~/api/common";
import premiumMemberEntry from "~/assets/decorate/pcHome/premium-member-entry.png";
import type { CatFloorItem, CategoryNode } from "~/types/common/common.d.ts";

// 空图占位
const emptyImgBg =
    "data:image/svg+xml;utf8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='200' height='200'%3E%3Crect fill='%23f2f3f5' width='200' height='200' rx='8'/%3E%3C/svg%3E";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

const { locale, t } = useI18n();
const moduleConfig = computed(() => props.module || {});
const commonStore = useCommonStore();
const themeStore = useThemeStore();
const collectShopLabel = computed(() => (isStore() ? t("关注门店") : t("关注店铺")));
// 「关注店铺/门店」入口：标准 / o2o / 跨境三端通用，不可只绑 isMerchant()
const showCollectShop = computed(() => isMerchant() || isStore() || isOverseas());
// 「入驻」入口：仅多商户与 o2o 门店可入驻，其余版本未登录时该位展示「登录」
const canAccessJoin = computed(() => isMerchant() || isStore());
// 会员操作区列数：带「关注店铺/门店」时 4 项，否则 3 项
const userActionCount = computed(() => (showCollectShop.value ? 4 : 3));
const generalColor = computed(() => ((themeStore.themeColor as Record<string, string>)["--general"] || "").trim() || "#e1251b");

// 卡位配置类型（用于模板类型推断，避免 card 被推断为 {}）
interface HeroCard {
    link?: string | Record<string, unknown>;
    moduleTitle?: string;
    moduleSubTitle?: string;
    moduleSubTitleBg?: string;
    moduleSubTitleBgColorA?: string;
    moduleSubTitleBgColorB?: string;
    moduleSubTitleBgType?: string;
    showPrice?: number;
    products?: CardProduct[];
    productSource?: string;
    productGroupId?: string;
}

// 加载接口分类数据（在 setup 阶段调用以支持 SSR）
commonStore.ensureAllCate();

// 分类 icon 依赖后端动态 @font-face（icoDefinedCss）；首页 header 在 index 页不渲染 AllCate 不会加载，这里 client 端兜底注入
onMounted(() => {
    watch(
        () => commonStore.catIcoCss,
        (css) => {
            if (css) useLoadCss(css);
        },
        { immediate: true }
    );
    if (import.meta.client) {
        window.addEventListener("scroll", onViewportChange, { passive: true });
        window.addEventListener("resize", onViewportChange, { passive: true });
    }
});

const activeSubView = ref<number | null>(null);
// 左侧分类面板 DOM 引用，用于计算弹层 fixed 视口定位
const cateLeftRef = ref<HTMLElement | null>(null);
// 弹层 DOM 引用
const subCataRef = ref<HTMLElement | null>(null);
// 弹层定位样式
const subCataStyle = ref<Record<string, string>>({});
const hasSubCataPosition = computed(() => subCataStyle.value.position === "fixed");
const teleportSubCataStyle = computed<Record<string, string>>(() => ({
    ...subCataStyle.value,
    "--general": generalColor.value
}));
// 延迟关闭定时器，避免鼠标从分类移到弹层的间隙导致闪烁
let hideTimer: ReturnType<typeof setTimeout> | null = null;
let positionFrameId = 0;

/**
 * 紧贴左侧分类面板右边缘、顶部对齐（fixed + 视口坐标）
 */
function updateSubCataPosition(): void {
    if (!cateLeftRef.value || !import.meta.client) return;
    const style = buildSubCataFixedStyle(cateLeftRef.value.getBoundingClientRect());
    // 无效矩形时保留已有坐标，避免写回 (0,0) 导致弹层飞到左上角
    if (!style) return;
    subCataStyle.value = style;
}

function scheduleSubCataPosition(): void {
    if (!import.meta.client) return;
    nextTick(() => {
        updateSubCataPosition();
        if (positionFrameId) cancelAnimationFrame(positionFrameId);
        positionFrameId = requestAnimationFrame(() => {
            positionFrameId = 0;
            updateSubCataPosition();
        });
    });
}

function onViewportChange(): void {
    if (activeSubView.value === null) return;
    updateSubCataPosition();
}

function buildCategoryUrl(categoryId?: string | number, pId?: string | number, seoLink?: string): string {
    return urlFormat({ path: "list", seoLink, id: categoryId, pId });
}

function clearHideTimer(): void {
    if (hideTimer) {
        clearTimeout(hideTimer);
        hideTimer = null;
    }
}

const showSubView = (index: number) => {
    clearHideTimer();
    activeSubView.value = index;
    updateSubCataPosition();
    scheduleSubCataPosition();
};

/**
 * 鼠标离开左侧分类区域时，延迟关闭弹层（给用户移入弹层的时间）
 */
function onCateAreaLeave(): void {
    clearHideTimer();
    hideTimer = setTimeout(() => {
        activeSubView.value = null;
        subCataStyle.value = {};
    }, 100);
}

/**
 * 鼠标进入弹层时取消关闭
 */
function onSubCataEnter(): void {
    clearHideTimer();
}

/**
 * 鼠标离开弹层时关闭
 */
function onSubCataLeave(): void {
    clearHideTimer();
    hideTimer = setTimeout(() => {
        activeSubView.value = null;
        subCataStyle.value = {};
    }, 100);
}

onBeforeUnmount(() => {
    clearHideTimer();
    if (positionFrameId) cancelAnimationFrame(positionFrameId);
    if (!import.meta.client) return;
    window.removeEventListener("scroll", onViewportChange);
    window.removeEventListener("resize", onViewportChange);
});

/**
 * 解析分类链接：如果 cat 有后台配置的自定义 categoryLink 则优先使用，否则走默认分类列表
 */
function resolveCatUrl(cat: CategoryNode, pId?: string | number): string {
    const cl = cat.categoryLink;
    if (cl) {
        // categoryLink 是路径字符串，如 "cart"、"member/order/list"、"list"
        if (String(cl).trim() === "list") {
            return buildCategoryUrl(cat.categoryId ?? cat.id, pId, cat.seoLink);
        }
        return cl.startsWith("/") ? cl : `/${cl}`;
    }
    return buildCategoryUrl(cat.categoryId ?? cat.id, pId, cat.seoLink);
}

/**
 * 解析楼层标题链接：优先使用后台配置的 categoryLink；当值为 list 时按当前楼层默认分类页组装
 */
function resolveFloorTitleLink(
    categoryLink: string,
    payload: {
        seoLink?: string;
        categoryId?: string | number;
        catFloorId?: string | number;
    }
): string {
    const link = String(categoryLink || "").trim();
    if (!link) {
        return buildCategoryUrl(payload.categoryId, payload.catFloorId, payload.seoLink);
    }
    if (link === "list") {
        return buildCategoryUrl(payload.categoryId, payload.catFloorId, payload.seoLink);
    }
    if (/^(https?:)?\/\//.test(link) || link.startsWith("/") || link.startsWith("#") || link.startsWith("mailto:") || link.startsWith("tel:")) {
        return link;
    }
    return `/${link}`;
}

/**
 * 解析可选链接：为空时返回空字符串，配合模板移除 href，避免空链接跳转
 */
function resolveOptionalLink(rawLink?: string | Record<string, unknown> | null): string {
    let linkInput: string | Record<string, unknown> | undefined | null = rawLink;
    if (rawLink && typeof rawLink === "object" && !Array.isArray(rawLink)) {
        const linkData = rawLink.data;
        if ((rawLink.id === undefined || rawLink.id === null || rawLink.id === "") && linkData && typeof linkData === "object" && !Array.isArray(linkData)) {
            const dataId = (linkData as Record<string, unknown>).id;
            if (dataId !== undefined && dataId !== null && dataId !== "") {
                linkInput = {
                    ...rawLink,
                    id: dataId
                };
            }
        }
    }
    const link = urlFormat(linkInput).replace(
        /^\/member\/premiumMember\/index(?=\/|\?|#|$)/,
        "/member/premiumMember"
    );
    if (!link) return "";
    if (/^(https?:)?\/\//.test(link) || link.startsWith("/") || link.startsWith("#") || link.startsWith("mailto:") || link.startsWith("tel:")) {
        return link;
    }
    return `/${link}`;
}

/**
 * 点击促销卡片时跳转配置链接；未配置链接则不跳转
 */
function handlePromoCardClick(rawLink?: string | Record<string, unknown> | null): void {
    const link = resolveOptionalLink(rawLink);
    if (!link || !import.meta.client) return;
    window.open(link, "_blank", "noopener,noreferrer");
}

/**
 * 点击频道卡片时跳转配置链接；未配置链接则不跳转
 */
function handleProductCardClick(rawLink?: string | Record<string, unknown> | null): void {
    const link = resolveOptionalLink(rawLink);
    if (!link || !import.meta.client) return;
    window.open(link, "_blank", "noopener,noreferrer");
}

/**
 * 解析商品详情链接：优先使用商品 sn，没有时用 productId 兜底
 */
function resolveProductLink(product: CardProduct): string {
    const productSn = String(product.productSn || "");
    const legacyProductId = String(product.productId || "");
    return urlFormat({
        path: "product",
        seoLink: product.seoLink,
        sn: productSn || legacyProductId
    });
}

/**
 * 点击卡位商品时跳转商品详情，并阻止外层卡片链接触发
 */
function handleProductClick(product: CardProduct): void {
    const link = resolveProductLink(product);
    if (!link || !import.meta.client) return;
    window.open(link, "_blank", "noopener,noreferrer");
}

// 从接口获取的分类数据（转换为组件所需格式）
const displayCategories = computed(() => {
    if (!commonStore.allCate || !Array.isArray(commonStore.allCate)) return [];

    const result: Array<{
        picId: string | number;
        picLink: { link: string };
        picThumb: string;
        picUrl: string;
        picTitle: string;
        catFloorNames?: Array<{ categoryTitleName: string; link: string }>;
        categoryId: string | number;
        seoLink: string;
        useImage?: number;
        imageUrl?: string;
        floorIcoFont?: string;
        catList?: CategoryNode[];
    }> = [];

    // 遍历所有楼层：左侧每一项对应一个楼层（catFloor）
    const toId = (v: unknown): string | number | "" => (typeof v === "string" || typeof v === "number" ? v : "");

    commonStore.allCate.forEach((floor: CatFloorItem, floorIdx: number) => {
        const catList: CategoryNode[] = Array.isArray(floor.catList) ? floor.catList : [];
        if (!catList.length) return;

        // 标题优先用楼层配置的 catFloorNames（与后台配置一致）
        const rawFloorNameList = Array.isArray(floor["catFloorNames"]) ? (floor["catFloorNames"] as Array<Record<string, unknown>>) : [];
        const floorNames = rawFloorNameList
            .map((n) => String(n["categoryTitleName"] ?? "").trim())
            .filter((n) => n.length > 0 && n !== "-")
            .slice(0, 3);

        const floorNameRaw = String(floor["catFloorName"] ?? "");
        const title =
            floorNames.length > 0
                ? floorNames.join(" / ")
                : floorNameRaw
                      .replace(/[，,、]/g, " / ")
                      .replace(/\s*\/\s*/g, " / ")
                      .trim();

        // 入口链接：兜底取该楼层第一个有效一级类目
        const firstCat = catList.find((c) => Boolean(c.categoryId ?? c.id) && (c.categoryName ?? c.name) !== "-");
        const catFloorId = toId(floor["catFloorId"]);
        const categoryId = (firstCat?.categoryId ?? firstCat?.id ?? catFloorId) || "";
        const seoLink = firstCat?.seoLink ?? "";

        // 兜底图标：一级类目的 pic（接口字段）或 categoryPic（兼容字段）
        const picFromApi = typeof firstCat?.["pic"] === "string" ? (firstCat["pic"] as string) : "";
        const pic = (typeof firstCat?.categoryPic === "string" ? firstCat.categoryPic : "") || picFromApi;
        const imageUrl = typeof floor["imageUrl"] === "string" ? (floor["imageUrl"] as string) : "";
        const useImage = Number(floor["useImage"] ?? (imageUrl ? 1 : 0));
        const floorIcoFont = typeof floor["floorIcoFont"] === "string" ? (floor["floorIcoFont"] as string) : undefined;

        result.push({
            picId: catFloorId || categoryId || floorIdx,
            picLink: {
                // 优先使用第一个一级类目的自定义链接；否则走默认分类列表
                link: firstCat ? resolveCatUrl(firstCat, catFloorId) : buildCategoryUrl(categoryId, catFloorId, seoLink)
            },
            picThumb: pic,
            picUrl: pic,
            picTitle: title || t("类目"),
            catFloorNames:
                rawFloorNameList.length > 0
                    ? rawFloorNameList
                          .map((n) => {
                              const categoryTitleName = String(n["categoryTitleName"] ?? "").trim();
                              const categoryLink = String(n["categoryLink"] ?? "").trim();
                              return {
                                  categoryTitleName,
                                  link: resolveFloorTitleLink(categoryLink, {
                                      seoLink,
                                      categoryId,
                                      catFloorId
                                  })
                              };
                          })
                          .filter((n) => n.categoryTitleName && n.categoryTitleName !== "-")
                    : catList
                          .map((c) => ({
                              categoryTitleName: String(c.categoryName ?? c.name ?? "").trim(),
                              link: resolveCatUrl(c, catFloorId)
                          }))
                          .filter((n) => n.categoryTitleName && n.categoryTitleName !== "-"),
            categoryId,
            seoLink,
            useImage,
            imageUrl,
            floorIcoFont,
            catList
        });
    });

    return result;
});

const banners = computed(() => moduleConfig.value.center?.banners || []);

// 模块化卡片 (6个)
const allModules = computed<HeroCard[]>(() => {
    const raw = (moduleConfig.value.center?.modules || []) as HeroCard[];
    const list = [...raw];
    while (list.length < 6) list.push({});
    return list.slice(0, 6);
});

// ==================== 商品分组数据获取（对齐后台 Preview.vue 逻辑） ====================
// 卡位商品项类型
interface CardProduct {
    productId?: string | number;
    productSn?: string;
    seoLink?: string;
    productName?: string;
    pic?: { picUrl?: string; picThumb?: string; picLarge?: string };
    productPrice?: string | number;
    price?: string;
    label?: string;
    labelBg?: string;
    labelBgColorA?: string;
    labelBgColorB?: string;
    labelBgType?: string;
}

// 存储每个卡位通过商品分组获取到的商品数据
const groupProductsMap = ref<Record<number, CardProduct[]>>({});

/**
 * 根据商品分组ID获取商品数据
 */
async function fetchGroupProducts(moduleIndex: number, productGroupId: string) {
    try {
        const result = await getProductsList({
            productGroupId,
            size: 2,
            page: 1
        });
        if (result.records && result.records.length > 0) {
            groupProductsMap.value = {
                ...groupProductsMap.value,
                [moduleIndex]: result.records.map(
                    (item) =>
                        ({
                            productId: item.productId,
                            productSn: String(item.productSn || ""),
                            seoLink: String(item.seoLink || ""),
                            productName: item.productName || "",
                            pic: {
                                picUrl: item.picUrl || "",
                                picThumb: item.picThumb || "",
                                picLarge: item.picLarge || ""
                            },
                            productPrice: item.productPrice ?? 0,
                            price: item.productPrice ? String(item.productPrice) : "0"
                        }) as CardProduct
                )
            };
        } else {
            groupProductsMap.value = {
                ...groupProductsMap.value,
                [moduleIndex]: []
            };
        }
    } catch {
        groupProductsMap.value = {
            ...groupProductsMap.value,
            [moduleIndex]: []
        };
    }
}

/**
 * 统一营销卡片价格来源：
 * - 优先使用 productPrice（数值，支持币种换算）
 */
function resolveCardProductPrice(product: CardProduct): string | number {
    const productPrice = (product as CardProduct)?.productPrice;
    if (productPrice !== undefined && productPrice !== null && String(productPrice).trim() !== "") {
        return productPrice;
    }
    const legacyPrice = String((product as CardProduct)?.price ?? "").trim();
    if (!legacyPrice) return 0;
    const normalized = legacyPrice.replace(/[^\d.-]/g, "");
    return normalized || 0;
}

// 监听模块配置变化，当卡位使用商品分组时自动获取商品
watch(
    () =>
        allModules.value.map((m: HeroCard) => ({
            productSource: m.productSource,
            productGroupId: m.productGroupId
        })),
    (newVal) => {
        newVal.forEach((item, idx) => {
            if (item.productSource === "productGroup" && item.productGroupId) {
                fetchGroupProducts(idx, item.productGroupId);
            }
        });
    },
    { immediate: true, deep: true }
);

// 增强后的模块列表：商品分组卡位使用从 API 获取的商品数据
const enhancedModules = computed<HeroCard[]>(() => {
    return allModules.value.map((card: HeroCard, idx: number) => {
        if (card.productSource === "productGroup" && groupProductsMap.value[idx]?.length) {
            return {
                ...card,
                products: groupProductsMap.value[idx]
            };
        }
        return card;
    });
});

// 前2个是 Banner 右侧推荐位
const promoCards = computed<HeroCard[]>(() => enhancedModules.value.slice(0, 2));

// 后4个是 下方商品位
const productCards = computed<HeroCard[]>(() => enhancedModules.value.slice(2, 6));

const rightUser = computed(() => {
    const user = moduleConfig.value.right?.user || {
        avatar: undefined,
        nickname: t("欢迎来{0}！", [commonStore.shopName]),
        welcomeText: t("登录后更多精彩")
    };
    return {
        ...user,
        nickname: user.nickname ? t(user.nickname) : t("欢迎来{0}！", [commonStore.shopName]),
        welcomeText: user.welcomeText ? t(user.welcomeText) : user.welcomeText
    };
});
const rightShortcuts = computed(() => moduleConfig.value.right?.shortcuts || []);
const premiumMemberConfig = computed(() => ({
    // 装修配置在不同接口版本中可能返回 boolean 或 0/1 字段，统一按启用状态处理
    show: [true, 1, "1", "true"].includes(moduleConfig.value.right?.premiumMember?.show),
    title: moduleConfig.value.right?.premiumMember?.title || t("会员"),
    description: moduleConfig.value.right?.premiumMember?.description || "",
    link: moduleConfig.value.right?.premiumMember?.link || {}
}));

function handlePremiumMemberClick(): void {
    if (premiumMemberConfig.value.link?.link) {
        handleProductCardClick(premiumMemberConfig.value.link);
        return;
    }
}

// 店铺街配置
const shopsConfig = computed(() => {
    const right = moduleConfig.value.right;
    return {
        title: right?.shopsTitle || t("店铺街"),
        subTitle: right?.shopsSubTitle || "",
        subTitleBg: right?.shopsSubTitleBg || "#FF4E4E",
        moreText: right?.shopsMoreText ? t(right.shopsMoreText) : t("查看全部"),
        moreLink: right?.shopsMoreLink || right?.shopsMoreLink?.link || "",
        textColor: right?.shopsTextColor || "#a06a38"
    };
});

const cssVars = computed(() => {
    const frame = moduleConfig.value.frame;
    const style = moduleConfig.value.style;
    const left = moduleConfig.value.left;
    const right = moduleConfig.value.right;

    // 计算用户卡片渐变背景
    let userCardGradient = "none";
    if (right?.userCardGradientColorA && right?.userCardGradientColorB) {
        const direction = gradientDirectionMap[right.userCardGradientType || "upDown"] || "to bottom";
        userCardGradient = `linear-gradient(${direction}, ${right.userCardGradientColorA}, ${right.userCardGradientColorB})`;
    }

    return {
        "--dec-box-padding": `${frame?.boxPadding ?? 0}px`,
        "--dec-box-padding-top": `${frame?.boxPaddingTop ?? 0}px`,
        "--dec-box-padding-bottom": `${frame?.boxPaddingBottom ?? 0}px`,
        "--dec-bg-color": frame?.backgroundColor || "transparent",
        "--dec-box-radius": `${frame?.boxRadius ?? 0}px`,

        "--dec-height": `${style?.height ?? 336}px`,
        "--dec-gap": `${style?.gap ?? 16}px`,
        "--dec-left-width": `${style?.leftWidth ?? 220}px`,
        "--dec-right-width": `${style?.rightWidth ?? 260}px`,

        "--dec-panel-bg": style?.panelBackgroundColor ?? "#ffffff",
        "--dec-panel-radius": `${style?.panelRadius ?? 8}px`,
        "--dec-left-bg": left?.backgroundColor ?? "#ffffff",
        "--dec-left-text-color": left?.textColor ?? "#333333",
        "--dec-user-card-bg": right?.userCardBackground || style?.panelBackgroundColor || "#ffffff",
        "--dec-user-card-gradient": userCardGradient,

        "--dec-title-size": `${style?.titleFontSize ?? 14}px`,
        "--dec-title-color": style?.titleColor ?? "#333333",
        "--dec-muted-color": style?.mutedColor ?? "#999999",
        "--dec-price-color": style?.priceColor ?? "#e1251b",
        "--general": generalColor.value,

        "--dec-banner-radius": `${style?.bannerRadius ?? 12}px`,
        "--dec-card-radius": `${style?.cardRadius ?? 12}px`
    };
});

const swiperOption = computed(() => ({
    autoplay: {
        delay: 4500,
        disableOnInteraction: false,
        pauseOnMouseEnter: true
    },
    pagination: {
        clickable: true,
        enabled: true
    },
    loop: true,
    navigation: false
}));

const userStore = useUserStore();
const route = useRoute();

// 登录态：优先使用真实用户信息（头像/昵称/资产），否则回退装修配置
type DecorateAvatar = { picThumb?: string; picUrl?: string };
type DecorateRightUser = { avatar?: DecorateAvatar; nickname?: string; welcomeText?: string };

// 展示层“已登录”以 userId 为准（仅 token 存在但已失效时，避免误显示为已登录）
const isAuthed = computed(() => userStore.userInfoLoaded && Boolean(userStore.userInfo?.userId));
const userDisplayName = computed(() => {
    if (isAuthed.value) return userStore.userInfo.nickname || userStore.userInfo.username || t("会员");
    return rightUser.value.nickname || t("欢迎来{0}！", [commonStore.shopName]);
});
const decorateUserAvatarUrl = computed(() => {
    const u = rightUser.value as unknown as DecorateRightUser;
    return pickPic(u.avatar);
});
const userAvatarSrc = computed(() => {
    if (isAuthed.value) return formatImage(userStore.userInfo.avatar);
    if (commonStore.defaultAvatar) return formatImage(commonStore.defaultAvatar);
    return formatImage(decorateUserAvatarUrl.value);
});
const couponRawCount = computed(() => (isAuthed.value ? Number(userStore.userInfo.coupon || 0) : 0));
const pointsRawCount = computed(() => (isAuthed.value ? Number(userStore.userInfo.points || 0) : 0));
const balanceRawValue = computed(() => (isAuthed.value ? Number(userStore.userInfo.balance ?? 0) : 0));
const currentLocale = computed(() => locale.value || "zh-CN");

const couponCount = computed(() => formatAssetNumberI18n(couponRawCount.value, { locale: currentLocale.value }));
const pointsCount = computed(() => formatAssetNumberI18n(pointsRawCount.value, { locale: currentLocale.value }));
const balanceText = computed(() => formatAssetNumberI18n(balanceRawValue.value, { locale: currentLocale.value, money: true }));
const couponFullText = computed(() => formatAssetFullI18n(couponRawCount.value, { locale: currentLocale.value }));
const pointsFullText = computed(() => formatAssetFullI18n(pointsRawCount.value, { locale: currentLocale.value }));
const balanceFullText = computed(() => formatAssetFullI18n(balanceRawValue.value, { locale: currentLocale.value, money: true }));
const pointsDisplay = computed(() => splitWanSuffix(pointsCount.value));
const balanceDisplay = computed(() => splitWanSuffix(balanceText.value));
// 余额/积分出现缩写单位或数字偏长时，加宽这两列；否则保持三列等宽
const needWideAssetCols = computed(() => {
    const balanceWide = !!balanceDisplay.value.suffix || balanceDisplay.value.value.length >= 7;
    const pointsWide = !!pointsDisplay.value.suffix || pointsDisplay.value.value.length >= 7;
    return balanceWide || pointsWide;
});

// 监听登录态：顶栏只看 token 会立刻变“会员”，这里需在拿到 token 后补拉 userInfo，否则会员卡仍停在未登录
watch(
    () => userStore.isAuthenticated,
    (authed) => {
        if (authed) {
            userStore.validateUserInfoOnce();
        }
    },
    { immediate: true }
);

async function toLogin(returnUrl?: string): Promise<void> {
    await navigateTo({
        path: "/member/login",
        query: {
            returnUrl: returnUrl || route.fullPath || "/"
        }
    });
}

async function authedNavigateTo(path: string): Promise<void> {
    if (!userStore.isAuthenticated) {
        await toLogin(path);
        return;
    }
    await navigateTo(path);
}

async function onRegisterClick(): Promise<void> {
    await navigateTo("/member/register");
}

function splitWanSuffix(text: string): { value: string; suffix: string } {
    const match = text.match(/^(.*?)([万亿KMB])$/);
    return match ? { value: match[1], suffix: match[2] } : { value: text, suffix: "" };
}

async function onJoiningClick(): Promise<void> {
    await navigateTo("/join/intro");
}

async function onLoginClick(): Promise<void> {
    if (userStore.isAuthenticated) {
        await navigateTo("/member");
        return;
    }
    await toLogin(route.fullPath || "/");
}

async function onMyOrdersClick(): Promise<void> {
    await authedNavigateTo("/member/order/list");
}

async function onBrowsingHistoryClick(): Promise<void> {
    await authedNavigateTo("/member/browsingHistory/list");
}

async function onCollectProductClick(): Promise<void> {
    await authedNavigateTo("/member/collectProduct/list");
}

async function onCollectShopClick(): Promise<void> {
    await authedNavigateTo("/member/collectShop/list");
}

async function onUserCenterClick(): Promise<void> {
    await authedNavigateTo("/member");
}

async function onAccountSettingsClick(): Promise<void> {
    await authedNavigateTo("/member");
}

async function onAddressClick(): Promise<void> {
    await authedNavigateTo("/member/address/list");
}

async function onCouponClick(): Promise<void> {
    await authedNavigateTo("/member/coupon/list");
}

async function onBalanceClick(): Promise<void> {
    await authedNavigateTo("/member/account/detail");
}

async function onPointClick(): Promise<void> {
    await authedNavigateTo("/member/point/list");
}

function formatImage(url?: string): string {
    if (!url) return emptyImgBg;
    // 相对路径（如 demo 商品图 "img/item/demo/xxx.jpeg"）需要拼接 OSS/CDN 前缀，
    // imageFormat 内部会判断是否已经是完整 URL
    return imageFormat(url) || emptyImgBg;
}

function handleImgError(e: Event) {
    const img = e.target as HTMLImageElement | null;
    if (!img) return;
    if (img.src === emptyImgBg) return;
    img.onerror = null;
    img.src = emptyImgBg;
}

// 渐变方向映射
const gradientDirectionMap: Record<string, string> = {
    upDown: "to bottom",
    leftRight: "to right",
    diagonal: "to bottom right"
};

// 默认子标题背景渐变色（与 Settings.vue 保持一致）
const DEFAULT_SUBTITLE_BG_COLOR_A = "#FF4E4E";
const DEFAULT_SUBTITLE_BG_COLOR_B = "#E1251B";

// 默认标签背景渐变色（与 Settings.vue 保持一致）
const DEFAULT_LABEL_BG_COLOR_A = "#FF4E4E";
const DEFAULT_LABEL_BG_COLOR_B = "#E1251B";

// 获取子标题背景色（支持渐变）
function getSubTitleBg(card: HeroCard): string {
    // 优先使用新的渐变字段
    const colorA = card.moduleSubTitleBgColorA;
    const colorB = card.moduleSubTitleBgColorB;
    const type = card.moduleSubTitleBgType || "leftRight";

    if (colorA && colorB) {
        const direction = gradientDirectionMap[type] || "to right";
        return `linear-gradient(${direction}, ${colorA}, ${colorB})`;
    }

    if (card.moduleSubTitleBg) {
        return card.moduleSubTitleBg;
    }

    // 默认渐变色
    return `linear-gradient(to right, ${DEFAULT_SUBTITLE_BG_COLOR_A}, ${DEFAULT_SUBTITLE_BG_COLOR_B})`;
}

// 获取标签背景样式（支持渐变）
function getLabelStyle(product: CardProduct): Record<string, string | undefined> {
    const colorA = product.labelBgColorA;
    const colorB = product.labelBgColorB;
    const type = product.labelBgType || "leftRight";

    let background: string;

    if (colorA && colorB) {
        const direction = gradientDirectionMap[type] || "to right";
        background = `linear-gradient(${direction}, ${colorA}, ${colorB})`;
    } else if (product.labelBg) {
        background = product.labelBg;
    } else {
        // 默认渐变色
        background = `linear-gradient(to right, ${DEFAULT_LABEL_BG_COLOR_A}, ${DEFAULT_LABEL_BG_COLOR_B})`;
    }

    // 背景恒为深色渐变（自定义或默认红渐变），文字统一用白色，避免漏配背景色时回退成红底红字
    return {
        background,
        color: "#fff"
    };
}
</script>

<style lang="less" scoped>
.dec-pc-home-hero {
    width: 100%;
    box-sizing: border-box;
    background: var(--dec-bg-color, transparent);
    border-radius: var(--dec-box-radius, 0);
    padding-left: var(--dec-box-padding, 0);
    padding-right: var(--dec-box-padding, 0);
    padding-top: var(--dec-box-padding-top, 0);
    padding-bottom: var(--dec-box-padding-bottom, 0);
}

.dec-pc-home-hero.is-cate-expanded {
    position: relative;
    z-index: 1000;
}

.dec-pc-home-hero,
.dec-pc-home-hero *,
.dec-pc-home-hero *::before,
.dec-pc-home-hero *::after {
    box-sizing: border-box;
}

.dec-pc-home-hero__layout {
    width: 100%;
    height: var(--dec-height, 336px);
    display: grid;
    grid-template-columns: 252px minmax(0, 1fr) 252px;
    gap: var(--dec-gap, 16px);
    align-items: stretch;
    overflow: visible;
    position: relative;
}

/* Common Panel Style */
.dec-pc-home-hero__panel {
    background: var(--dec-panel-bg, #ffffff);
    border-radius: var(--dec-panel-radius, 8px);
    box-shadow: var(--dec-panel-shadow, none);
    overflow: hidden;
    display: flex;
    flex-direction: column;
}

/* Left: Categories */
.dec-pc-home-hero__left {
    min-height: 0;
    height: 100%;
    position: relative;
    z-index: 20;
    overflow: hidden;

    /* 保留最原始的 hover 溢出行为，确保下方被裁切内容可见 */
    &:hover {
        overflow: visible;
        z-index: 30;
    }
}

.dec-pc-home-hero__cate-area {
    position: relative;
    width: 100%;
    height: 100%;
    z-index: 999;
}

.dec-pc-home-hero__cate-list {
    padding-top: 11px;
    padding-bottom: 11px;
    overflow: hidden;
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    box-sizing: border-box;
    background: var(--dec-left-bg, #ffffff);
    border-radius: var(--dec-panel-radius, 8px);
}

.dec-pc-home-hero__cate-area:hover .dec-pc-home-hero__cate-list {
    overflow: visible;
    height: auto;
    min-height: 100%;
}

.dec-pc-home-hero__cate-item {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    min-height: 28px;
    max-height: 28px;
    line-height: 28px;
    padding: 0 16px;
    color: inherit;
    cursor: pointer;
    user-select: none;
    transition: background 0.2s;
    margin-top: 1px;
    overflow: hidden;
    flex-shrink: 0;

    &:hover {
        background: #f2f2f2;
    }

    &:first-child {
        margin-top: 0;
    }
}

.dec-pc-home-hero__cate-item.is-active {
    background: #f2f2f2;
}

.dec-pc-home-hero__cate-link {
    font-size: 14px;
    color: var(--dec-left-text-color, #1a1a1a);
    font-weight: 400;
    cursor: pointer;
    transition: color 0.2s ease;
    padding: 8px 4px;

    &:hover {
        color: var(--general);
    }
}

.dec-pc-home-hero__cate-sep {
    font-size: 12px;
    color: var(--dec-left-text-color, #666);
    line-height: inherit;
}

.dec-pc-home-hero__cate-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--dec-left-text-color, #333333);
}

.dec-pc-home-hero__cate-icon-img {
    width: 18px;
    height: 18px;
    object-fit: contain;
    display: block;
}

.dec-pc-home-hero__cate-icon :deep(i) {
    font-size: 18px;
    line-height: 1;
    display: inline-block;
}

.dec-pc-home-hero__cate-text {
    min-width: 0;
    flex: 1;
    line-height: 30px;
    height: 28px;
}

.dec-pc-home-hero__cate-title {
    font-size: 14px;
    color: var(--dec-left-text-color, #1a1a1a);
    font-weight: 400;
    line-height: 30px;
    height: 28px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* 子类目弹层：Teleport 到 body，fixed 视口定位由 JS 写入；未定位前隐藏避免闪到左上角 */
.dec-pc-home-hero__subCata {
    visibility: hidden;
    pointer-events: none;
    background: transparent;
    border-radius: var(--dec-panel-radius, 8px);
    border: none;
    box-shadow: none;
    overflow-y: visible;

    &.is-positioned {
        visibility: visible;
        pointer-events: auto;
    }
}

.dec-pc-home-hero__subView {
    background-color: #fff;
    border: 1px solid var(--general);
    border-radius: var(--dec-panel-radius, 8px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    display: none;
    max-height: 630px;
    min-height: 436px;
    padding: 14px 0;
    box-sizing: border-box;
    overflow: hidden;
    position: relative;
    width: 742px;
    z-index: 1;
}

.dec-pc-home-hero__subView.is-active {
    display: block;
}

.dec-pc-home-hero__sub-left {
    min-height: 0;
    height: 100%;
    position: relative;
    width: 100%;
    background: #fff;
    border-right: 1px solid #f2f2f2;
    overflow: hidden;
}

.dec-pc-home-hero__nav_cata_bd {
    padding: 0 14px;
    height: 100%;
    overflow-y: auto;
    box-sizing: border-box;

    &::-webkit-scrollbar {
        width: 6px;
    }

    &::-webkit-scrollbar-thumb {
        background: #d9d9d9;
        border-radius: 999px;
    }

    &::-webkit-scrollbar-track {
        background: transparent;
    }
}

.dec-pc-home-hero__sub-item {
    position: relative;
    font-size: 12px;
}

.dec-pc-home-hero__sub-title {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-weight: 600;
    height: 39px;
    line-height: 38px;
    padding-right: 9px;
    text-align: left;
    white-space: nowrap;

    a {
        color: #1a1a1a;

        &:hover {
            color: var(--general);
        }
    }

    i {
        font-family: "宋体", serif;
        font-style: normal;
        color: #666;
    }
}

.dec-pc-home-hero__sub-names {
    border-bottom: 1px solid #f2f2f2;
    display: block;
    line-height: 24px;
    padding: 7px 0;

    a {
        display: inline-block;
        color: #1a1a1a;
        margin: 0 9px;
        font-size: 14px;

        &:hover {
            color: var(--general);
        }
    }
}

/* ========== 模式 1 + 文字模式：扁平文字列表 ========== */
.dec-pc-home-hero__flat-text-item {
    padding: 8px 14px;
    // border-bottom: 1px solid #f5f5f5;

    &:last-child {
        border-bottom: none;
    }

    a {
        font-size: 14px;
        color: #1a1a1a;
        font-weight: 600;
        transition: color 0.2s;

        &:hover {
            color: var(--general);
        }
    }
}

/* ========== 图文模式共用样式 ========== */
.dec-pc-home-hero__img-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    padding: 8px 0;
}

.dec-pc-home-hero__img-item {
    width: 96px;
    min-height: 96px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    border-radius: 10px;
    border: 1px solid #f0f0f0;
    background: #f5f5f5;
    padding: 8px 6px;
    text-decoration: none;
    // box-sizing: border-box;
    transition:
        border-color 0.2s,
        background-color 0.2s;

    &:hover {
        border-color: var(--general, #e1251b);
        background: #ffffff;

        .dec-pc-home-hero__img-pic {
            background: #f5f5f5;
        }

        .dec-pc-home-hero__img-name {
            color: var(--general, #e1251b);
        }
    }
}

.dec-pc-home-hero__img-pic {
    width: 72px;
    height: 72px;
    border-radius: 4px;
    overflow: hidden;
    background: transparent;

    img {
        width: 100%;
        height: 100%;
        object-fit: contain;
    }
}

.dec-pc-home-hero__img-placeholder {
    width: 100%;
    height: 100%;
    background: #f0f0f0;
}

.dec-pc-home-hero__img-name {
    width: 100%;
    text-align: center;
    font-size: 13px;
    color: #333;
    margin-top: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    word-break: break-word;
}

/* ========== 模式 2 + 图文模式：分组图文 ========== */
.dec-pc-home-hero__img-section {
    margin-bottom: 16px;

    &:last-child {
        margin-bottom: 0;
    }
}

.dec-pc-home-hero__img-section-title {
    font-size: 14px;
    font-weight: 600;
    color: #1a1a1a;
    margin-bottom: 8px;
    padding-bottom: 4px;
    // border-bottom: 1px solid #f0f0f0;
}

/* Center: Banner + Grid */
.dec-pc-home-hero__center {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--dec-gap, 16px);
    height: 100%;
    overflow: hidden;
    position: relative;
    z-index: 1;
}

.dec-pc-home-hero__center-top {
    flex: 1;
    min-height: 0;
    width: 100%;
    display: flex;
    gap: var(--dec-gap, 16px);
    overflow: hidden;
}

.dec-pc-home-hero__banner {
    flex: 1;
    height: 100%;
    border-radius: var(--dec-panel-radius, 8px);
    overflow: hidden;
    background: var(--dec-panel-bg, #ffffff);
    box-shadow: var(--dec-panel-shadow, none);
    min-width: 0;
    position: relative;
    /* Ensure banner does not grow indefinitely */
    flex-shrink: 1;
}

.dec-pc-home-hero__banner-inner {
    width: 100%;
    height: 100%;
    display: block;

    // Swiper 容器需要固定高度，否则会按内容自适应导致分页指示器被吞没
    :deep(.swiper) {
        width: 100%;
        height: 100%;
    }
}

.dec-pc-home-hero__banner-slide,
.dec-pc-home-hero__banner-link,
.dec-pc-home-hero__banner-img {
    width: 100%;
    height: 100%;
    display: block;
}

.dec-pc-home-hero__banner-img {
    object-fit: cover;
    // 窄屏裁剪时左对齐，优先保留左侧主文案，仅从右侧裁切
    object-position: left center;
    background: #f2f3f5;
}

/* Promo Group (Next to Banner) */
.dec-pc-home-hero__promo-group {
    flex: 1;
    display: flex;
    gap: var(--dec-gap, 16px);
    min-width: 0;
    min-height: 0;
    /* Ensure height matches banner by taking up parent height */
    height: 100%;
    flex-shrink: 1;
    overflow: hidden;
}

.dec-pc-home-hero__promo-card {
    flex: 1;
    height: 100%;
    padding: 0;
    text-decoration: none;
    color: inherit;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    border-radius: var(--dec-panel-radius, 8px);
    background: var(--dec-panel-bg, #ffffff);
    cursor: pointer;
}

/* Unified Card Header Style */
.dec-pc-home-hero__card-header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 12px;
    margin: 12px 0 10px;
    flex-shrink: 0;
    overflow: hidden;
}

.dec-pc-home-hero__card-title {
    font-size: 14px;
    font-weight: 700;
    color: #333;
    line-height: 16px;
    height: 16px;
    flex-shrink: 0;
    white-space: nowrap;
}

.dec-pc-home-hero__card-badge {
    display: inline-block;
    font-size: 10px;
    color: #fff;
    background: #e1251b;
    padding: 1px 4px;
    border-radius: 2px;
    line-height: 1.4;
    flex-shrink: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    vertical-align: middle;

    &.is-orange {
        background: linear-gradient(90deg, #ff9f00, #ff6b00);
    }
    &.is-green {
        background: #00b578;
    }
}

/* Unified Card Products Style */
.dec-pc-home-hero__card-products {
    flex: 1;
    display: flex;
    gap: 4px;
    min-height: 0;
    overflow: hidden;
    padding-bottom: 8px;
}

.dec-pc-home-hero__card-product {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    cursor: pointer;
}

.dec-pc-home-hero__card-img-box {
    width: 100%;
    flex: 1;
    min-height: 0;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
}

.dec-pc-home-hero__card-img {
    width: 86px;
    height: 86px;
    object-fit: contain;
    margin: auto;
}

.dec-pc-home-hero__card-label {
    position: absolute;
    bottom: 0;
    left: 50%;
    transform: translateX(-50%);
    font-size: 10px;
    color: #e1251b;
    background: rgba(255, 255, 255, 0.9);
    padding: 1px 4px;
    border-radius: 3px 3px 0 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 90%;
    box-shadow: 0 -2px 4px rgba(0, 0, 0, 0.05);
    line-height: 1.6;
}

.dec-pc-home-hero__card-price {
    font-size: 16px;
    color: #e1251b;
    font-weight: 700;
    line-height: 1.2;
    flex-shrink: 0;
    font-family: Arial, Helvetica, sans-serif;
    margin-top: 4px;
}

/* Center Bottom: Channel Grid */
.dec-pc-home-hero__center-bottom {
    flex: 1;
    min-height: 0;
    overflow: hidden;
}

.dec-pc-home-hero__product-grid {
    height: 100%;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--dec-gap, 16px);
}

.dec-pc-home-hero__channel-card {
    background: var(--dec-panel-bg, #ffffff);
    border-radius: var(--dec-panel-radius, 8px);
    box-shadow: var(--dec-panel-shadow, none);
    overflow: hidden;
    text-decoration: none;
    color: inherit;
    display: flex;
    flex-direction: column;
    padding: 0;
    transition: transform 0.2s;
    min-height: 0;
    cursor: pointer;
}

/* Right Column */
.dec-pc-home-hero__right {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
    overflow: hidden;
    position: relative;
    z-index: 1;
}

/* User Card */
.dec-pc-home-hero__user-card {
    position: relative;
    padding: 16px 12px;
    height: 220px;
    flex-shrink: 0;
    align-items: stretch;
    text-align: left;
    overflow: hidden;
    background: var(--dec-user-card-bg, #fff5ee); /* 浅橘粉背景 */
    background-image: var(--dec-user-card-gradient, none);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    box-sizing: border-box;
}

/* 付费会员入口固定在右上角，不改变会员卡原有内容布局 */
.dec-pc-home-hero__premium-member {
    position: absolute;
    top: 0;
    right: 0;
    z-index: 2;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0;
    width: 86px;
    height: 41px;
    padding: 5px 12px 5px 20px;
    box-sizing: border-box;
    color: #fff;
    background-repeat: no-repeat;
    background-position: center;
    background-size: 100% 100%;
    text-decoration: none;
    cursor: pointer;
}

.dec-pc-home-hero__premium-member > span {
    position: relative;
    top: -5px;
}

.dec-pc-home-hero__premium-member-title {
    display: block;
    flex-shrink: 0;
    width: 100%;
    min-width: 0;
    max-width: 54px;
    text-align: center;
    overflow: hidden;
    font-size: 11px;
    font-weight: 400;
    line-height: 16px;
    white-space: nowrap;
    text-overflow: ellipsis;
}

.dec-pc-home-hero__premium-member-desc {
    display: block;
    min-width: 0;
    width: 100%;
    overflow: hidden;
    max-width: 54px;
    font-size: 8px;
    line-height: 12px;
    text-align: center;
    white-space: nowrap;
    text-overflow: ellipsis;
    opacity: 0.85;
}

.dec-pc-home-hero__premium-member.is-title-only {
    right: 0;
    padding-left: 20px;
    transform: none;
}

.dec-pc-home-hero__premium-member.is-title-only .dec-pc-home-hero__premium-member-title {
    max-width: 60px;
}

.dec-pc-home-hero__user-top {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    margin-bottom: 15px;
    flex-shrink: 0;

    &.is-authed {
        margin-bottom: 0;
    }
}

.dec-pc-home-hero__avatar {
    width: 48px;
    height: 48px;
    flex-shrink: 0;
    border-radius: 50%;
    object-fit: cover;
    background: #fff;
    border: 3px solid #fff;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.05);
    cursor: pointer;
}

.dec-pc-home-hero__user-info {
    flex: 1;
    min-width: 0;
    text-align: left;
}

.dec-pc-home-hero__user-name {
    font-size: 16px;
    font-weight: 700;
    line-height: 1.2;
    color: #333;
    padding-bottom: 12.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
}

/* 会员入口只覆盖用户卡片右上角，预留空间仅作用于用户名，避免账号链接被挤到换行 */
.dec-pc-home-hero__user-top.has-premium-member .dec-pc-home-hero__user-name {
    padding-right: 74px;
    box-sizing: border-box;
}

.dec-pc-home-hero__user-links {
    font-size: 12px;
    color: #666;
    display: flex;
    align-items: center;
    gap: 8px;
    line-height: 1.2;
    white-space: nowrap;

    .divider {
        width: 1px;
        height: 10px;
        background-color: #ddd;
    }

    span:not(.divider) {
        cursor: pointer;
        position: relative;

        &::after {
            content: "";
            position: absolute;
            top: -8px;
            bottom: -8px;
            left: -4px;
            right: -4px;
        }

        &:hover {
            color: #e1251b;
        }
    }
}

.dec-pc-home-hero__user-desc {
    width: 100%;
    text-align: center;
    margin-bottom: 8px;
    flex-shrink: 0;

    .title {
        font-size: 15px;
        font-weight: 700;
        line-height: 1;
        color: #333;
        margin-bottom: 7px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .subtitle {
        font-size: 12px;
        line-height: 1;
        color: #999;
        transform: scale(0.9);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
}

.dec-pc-home-hero__user-assets {
    width: 100%;
    background: #fff;
    border-radius: 8px;
    padding: 10px 0;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0;
    flex: 1;
    flex-shrink: 0;
    margin: 12px 0 10px;
    font-size: 16px;

    &--wide {
        grid-template-columns: 0.8fr 1.15fr 1.15fr;
    }
}

.dec-pc-home-hero__asset-item {
    text-align: center;
    cursor: pointer;
    user-select: none;
    transition: color 0.2s;
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 0 4px;
    box-sizing: border-box;
    min-width: 0;

    &:hover {
        color: var(--dec-price-color, #e1251b);
        .dec-pc-home-hero__asset-num,
        .dec-pc-home-hero__asset-label {
            color: var(--dec-price-color, #e1251b);
        }
    }
}

.dec-pc-home-hero__asset-num {
    display: inline-flex;
    align-items: baseline;
    justify-content: center;
    gap: 1px;
    font-size: 16px;
    font-weight: 700;
    line-height: 1.2;
    color: #333;
    transition: color 0.2s;
    max-width: 100%;
    overflow: hidden;
    white-space: nowrap;
}

.dec-pc-home-hero__asset-num-value {
    line-height: 1.2;
    min-width: 0;
}

.dec-pc-home-hero__asset-num-suffix {
    flex-shrink: 0;
    font-size: 11px;
    line-height: 1.2;
}

.dec-pc-home-hero__asset-label {
    margin-top: 6px;
    font-size: 12px;
    color: #1a1a1a;
    transition: color 0.2s;
}

.dec-pc-home-hero__login-btn {
    appearance: none;
    border: 0;
    background: var(--main-bg);
    color: var(--main-text);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    width: 100%;
    height: 36px;
    line-height: 36px;
    padding: 0;
    margin-bottom: 12px;
    flex-shrink: 0;
    font-size: 15px;
    font-weight: 700;
    text-align: center;
    white-space: nowrap;
    cursor: pointer;
    transition: background-color 0.2s ease;
    border-radius: 6px;

    &:hover {
        background: var(--primary-dark-2, var(--main-bg));
    }
}
.dec-pc-home-hero__user-actions {
    width: 100%;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 2px;
    flex-shrink: 0;
    margin-top: auto;
    text-align: center;
}

.dec-pc-home-hero__action-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    gap: 10px;
    font-size: 12px;
    color: #333;
    cursor: pointer;
    transition: color 0.2s;
    line-height: 1.2;
    min-width: 0;
    overflow: visible;
    padding: 0;

    span {
        display: block;
        width: 100%;
        line-height: 1;
        padding: 0 2px;
        box-sizing: border-box;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    &:hover {
        color: #e1251b;
    }
}

/* Shops Panel */
.dec-pc-home-hero__shops {
    flex: 1;
    min-height: 0;
    padding: 0;
    overflow: hidden;
    background: var(--dec-panel-bg, #ffffff);
    border-radius: var(--dec-panel-radius, 8px);
    display: flex;
    flex-direction: column;
}

.dec-pc-home-hero__shops-header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 12px;
    margin: 5px 0 5px;
    flex-shrink: 0;

    .title {
        font-size: 14px;
        font-weight: 700;
        color: #333;
        line-height: 1.6;
        height: 16px;
        display: flex;
        align-items: center;
        gap: 4px;
        flex: none;
    }
    .hot {
        display: inline-block;
        font-size: 10px;
        color: #fff;
        background: #e1251b;
        padding: 1px 4px;
        border-radius: 2px;
        line-height: 12px;
        font-weight: normal;
    }
    .more {
        margin-left: auto;
        font-size: 12px;
        color: #999;
        text-decoration: none;
        display: flex;
        align-items: center;
        gap: 2px;
        cursor: pointer;
        line-height: 1.6;

        &:hover {
            color: #e1251b;
        }

        .more-icon {
            transform: rotate(-90deg);
            display: inline-block;
        }
    }
}

.dec-pc-home-hero__shop-list {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
    padding: 4px 12px 0px;
    flex: 1;
}

.dec-pc-home-hero__shop-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    text-decoration: none;
    cursor: pointer;
    min-width: 0;
}

.dec-pc-home-hero__shop-img-box {
    width: 44px;
    height: 44px;
    border-radius: 8px;
    background: #ffffff;
    border: 1px solid #f2f2f2;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    transition: all 0.2s;
    overflow: hidden;
    flex-shrink: 0;
}

.dec-pc-home-hero__shop-icon {
    width: 100%;
    height: 100%;
    object-fit: contain;
}

.dec-pc-home-hero__shop-enter {
    font-size: 12px;
    color: #a06a38;
    white-space: nowrap;
    width: 100%;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
    text-align: center;
    line-height: 1.6;
}

/* Swiper overrides */
.dec-pc-home-hero__banner {
    :deep(.swiper-pagination) {
        text-align: left;
        padding-left: 12px;
        bottom: 12px;
        display: flex;
        align-items: center;
        gap: 6px;
    }
    :deep(.swiper-pagination-bullet) {
        width: 8px;
        height: 8px;
        background: #fff;
        opacity: 0.4;
        transition: all 0.2s;
        margin: 0 !important;
    }
    :deep(.swiper-pagination-bullet-active) {
        opacity: 1;
        width: 24px;
        border-radius: 4px;
        background: #fff;
    }
    /* Hide nav buttons as per design */
    :deep(.swiper-button-prev),
    :deep(.swiper-button-next) {
        display: none;
    }
}

/* ==================== Responsive Styles ==================== */

/* Container query for better responsiveness in editor */
.dec-pc-home-hero {
    container-type: inline-size;
    container-name: hero;
}

/*
   Logic:
   - Above 1350px: Show Full Layout (2 Promo Modules, 2 Products per Card)
   - Below 1350px (Target 1200px): Show Compact Layout (1 Promo Module, 1 Product per Card)
*/

@container hero (max-width: 1350px) {
    /* Layout Adjustments for 1200px - keep fixed widths */
    .dec-pc-home-hero__layout {
        grid-template-columns: 252px minmax(0, 1fr) 252px;
    }

    /* 1. Promo Group: Show both modules to align with 4-column grid below (Banner 2, Promo 1+1) */
    .dec-pc-home-hero__promo-group .dec-pc-home-hero__promo-card:nth-child(2) {
        display: flex;
    }

    /* 2. Center Bottom: Show only 1 product per card (hide the 2nd one) */
    .dec-pc-home-hero__center-bottom .dec-pc-home-hero__card-product:nth-child(2),
    .dec-pc-home-hero__promo-group .dec-pc-home-hero__card-product:nth-child(2) {
        display: none;
    }

    /* Adjust the remaining product to look good (bigger image) */
    .dec-pc-home-hero__center-bottom .dec-pc-home-hero__card-product,
    .dec-pc-home-hero__promo-group .dec-pc-home-hero__card-product {
        /* Add some padding to frame the single product nicely */
        padding: 0 10px;
        justify-content: center;

        .dec-pc-home-hero__card-img {
            width: 86px;
            height: 86px;
            object-fit: contain;
            margin: auto;
        }
    }
}

/* Small screens (1100px and below) - 提前隐藏标签，因为频道卡片已经很挤了 */
@container hero (max-width: 1100px) {
    .dec-pc-home-hero__card-badge {
        display: none !important;
    }
}

/* Small screens (900px and below) - further compacting based on the single-item layout */
@container hero (max-width: 900px) {
    .dec-pc-home-hero__layout {
        grid-template-columns: 200px minmax(0, 1fr) 200px;
        gap: 12px;
    }

    /* Change ratio to 3:1 when one promo card is hidden, so the remaining one stays 1-column wide */
    .dec-pc-home-hero__banner {
        flex: 3;
    }
    .dec-pc-home-hero__promo-group {
        flex: 1;
    }

    .dec-pc-home-hero__cate-item {
        padding: 5px 12px;
        gap: 8px;
    }

    .dec-pc-home-hero__cate-icon {
        width: 16px;
        height: 16px;
    }

    .dec-pc-home-hero__cate-title {
        font-size: 12px;
    }

    /* Ensure single-item logic persists */
    .dec-pc-home-hero__promo-group .dec-pc-home-hero__promo-card:nth-child(2) {
        display: none;
    }
    .dec-pc-home-hero__center-bottom .dec-pc-home-hero__card-product:nth-child(2) {
        display: none;
    }

    .dec-pc-home-hero__promo-group {
        gap: 12px;
    }

    .dec-pc-home-hero__promo-card {
        padding: 8px;
    }

    .dec-pc-home-hero__card-header {
        margin-bottom: 6px;
    }

    .dec-pc-home-hero__card-title {
        font-size: 12px;
    }

    .dec-pc-home-hero__card-badge {
        font-size: 9px;
        padding: 1px 3px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        flex-shrink: 1;
    }

    .dec-pc-home-hero__card-price {
        font-size: 14px;
    }

    .dec-pc-home-hero__product-grid {
        gap: 12px;
    }

    .dec-pc-home-hero__channel-card {
        padding: 0;
    }

    .dec-pc-home-hero__right {
        gap: 12px;
    }

    .dec-pc-home-hero__user-card {
        padding: 10px;
    }

    .dec-pc-home-hero__avatar {
        width: 36px;
        height: 36px;
    }

    .dec-pc-home-hero__user-name {
        font-size: 13px;
        line-height: 1.2;
    }

    .dec-pc-home-hero__user-links {
        font-size: 11px;
    }

    .dec-pc-home-hero__login-btn {
        height: 28px;
        line-height: 28px;
        font-size: 12px;
        padding: 0;
        margin-bottom: 6px;
    }

    .dec-pc-home-hero__action-item {
        font-size: 9px;
    }

    .dec-pc-home-hero__shops {
        padding: 8px 10px;
        height: auto;
        min-height: 0;
    }

    .dec-pc-home-hero__shops-header {
        margin-bottom: 8px;
        padding: 0;
        .title {
            font-size: 12px;
        }
        .hot {
            display: none;
        }
        .more {
            font-size: 11px;
        }
    }

    .dec-pc-home-hero__shop-list {
        grid-template-columns: repeat(3, 1fr);
        gap: 8px;
        padding: 0;
    }

    .dec-pc-home-hero__shop-item {
        padding: 0;
    }

    .dec-pc-home-hero__shop-img-box {
        width: 40px;
        height: 40px;
    }
}

/* Extra small screens (< 700px) */
@container hero (max-width: 700px) {
    .dec-pc-home-hero__layout {
        grid-template-columns: 160px minmax(0, 1fr) 160px;
        gap: 8px;
    }

    .dec-pc-home-hero__cate-list {
        padding: 4px 0;
    }

    .dec-pc-home-hero__cate-item {
        padding: 3px 8px;
        gap: 4px;
    }

    .dec-pc-home-hero__cate-icon {
        width: 12px;
        height: 12px;
    }

    .dec-pc-home-hero__cate-title {
        font-size: 10px;
        line-height: 14px;
    }

    .dec-pc-home-hero__card-header {
        margin-bottom: 3px;
    }

    .dec-pc-home-hero__card-title {
        font-size: 10px;
        flex: 1;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .dec-pc-home-hero__card-badge {
        display: none !important;
    }

    .dec-pc-home-hero__card-label {
        font-size: 7px;
        padding: 0 2px;
    }

    .dec-pc-home-hero__card-price {
        font-size: 12px;
    }

    .dec-pc-home-hero__product-grid {
        gap: 8px;
    }

    .dec-pc-home-hero__channel-card {
        padding: 0;
        border-radius: 8px;
    }

    .dec-pc-home-hero__right {
        gap: 8px;
    }

    .dec-pc-home-hero__user-card {
        padding: 6px;
        gap: 4px;
    }

    .dec-pc-home-hero__avatar {
        width: 28px;
        height: 28px;
    }

    .dec-pc-home-hero__user-name {
        font-size: 11px;
        margin-bottom: 2px;
        line-height: 16px;
    }

    .dec-pc-home-hero__user-links {
        font-size: 9px;
        gap: 4px;
    }

    .dec-pc-home-hero__login-btn {
        height: 24px;
        line-height: 24px;
        font-size: 10px;
        padding: 0;
        margin-bottom: 3px;
    }

    .dec-pc-home-hero__user-actions {
        padding-top: 0;
    }

    .dec-pc-home-hero__action-item {
        font-size: 7px;
    }

    .dec-pc-home-hero__shops {
        padding: 4px 6px;
    }

    .dec-pc-home-hero__shops-header {
        margin-bottom: 4px;
        .title {
            font-size: 10px;
        }
        .hot {
            display: none;
        }
        .more {
            font-size: 9px;
        }
    }

    .dec-pc-home-hero__shop-list {
        grid-template-columns: repeat(2, 1fr);
        gap: 4px;
        padding: 0;
    }

    .dec-pc-home-hero__shop-item {
        padding: 0;
    }

    .dec-pc-home-hero__shop-img-box {
        width: 32px;
        height: 32px;
        padding: 2px;
    }

    .dec-pc-home-hero__banner {
        :deep(.swiper-pagination) {
            padding-left: 8px;
            bottom: 8px;
        }
        :deep(.swiper-pagination-bullet) {
            width: 4px;
            height: 4px;
            margin: 0 1px;
        }
        :deep(.swiper-pagination-bullet-active) {
            width: 10px;
        }
    }
}
</style>
