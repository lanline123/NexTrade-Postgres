<template>
    <div v-if="showTopBar" class="benefit-bar-wrap topbar-shell-d2QyMjgyY3Y" :class="{ 'benefit-bar-wrap--simple': styleType === 1 }" :style="benefitBarWrapStyle">
        <div class="pc-header-container" :style="containerStyle">
            <div class="benefit-bar" :class="{ 'benefit-bar--simple': styleType === 1, 'benefit-bar--only-right': isBenefitBarOnlyRight }">
                <!-- 左侧：地区 / 会员入口（简约风） -->
                <div v-if="showStripLeft" class="benefit-bar__left" :style="topMenuTextStyle">
                    <div
                        v-if="showTopRegion"
                        class="benefit-bar__location"
                        :class="{ 'is-active': regionDropdownOpen, wide: wide }"
                        :style="topMenuTextStyle"
                        @mouseenter="regionDropdownOpen = true"
                        @mouseleave="regionDropdownOpen = false"
                    >
                        <span class="iconfont-pc icon-dizhi benefit-bar__location-ico"></span>
                        <span class="benefit-bar__region">{{ regionDisplayText }}</span>
                        <div
                            v-show="regionDropdownOpen && topRegionList.length > 0"
                            class="region-dropdown"
                            @mouseenter="regionDropdownOpen = true"
                            @mouseleave="regionDropdownOpen = false"
                        >
                            <div class="region-dropdown__list">
                                <a
                                    v-for="item in topRegionList"
                                    :key="item.regionId"
                                    class="region-dropdown__item"
                                    :class="{ selected: String(userProvinceId ?? '') === String(item.regionId) }"
                                    @click="selectRegion(item)"
                                    >{{ item.regionName }}</a
                                >
                            </div>
                        </div>
                    </div>

                    <ClientOnly v-if="showTopMemberEntry">
                        <template v-if="userStore.isAuthenticated">
                            <div class="top-link-item top-link-item--menu">
                                <el-popover
                                    trigger="hover"
                                    placement="bottom"
                                    :show-arrow="false"
                                    :teleported="false"
                                    :hide-after="300"
                                    popper-class="top-member-popper"
                                >
                                    <template #reference>
                                        <div class="top-link top-link--nav" :style="topMenuTextStyle">
                                            <NuxtLink class="top-link__name" to="/member/">{{ userDisplayName }}</NuxtLink>
                                            <span class="pc-icon pc-icon--arrow top-bar-nav__arrow"></span>
                                        </div>
                                    </template>
                                    <div class="top-member-dropdown">
                                        <div class="top-member-dropdown__profile">
                                            <div class="top-member-dropdown__avatar-wrap">
                                                <img v-if="userAvatar" class="top-member-dropdown__avatar" :src="userAvatar" alt="" />
                                                <span v-else class="top-member-dropdown__avatar top-member-dropdown__avatar--placeholder">
                                                    {{ userDisplayName.slice(0, 1) }}
                                                </span>
                                            </div>
                                            <div class="top-member-dropdown__main">
                                                <div class="top-member-dropdown__name" :title="userDisplayName">{{ userDisplayName }}</div>
                                                <div class="top-member-dropdown__actions">
                                                    <NuxtLink to="/member">{{ $t("账号管理") }}</NuxtLink>
                                                    <span class="top-member-dropdown__divider"></span>
                                                    <a @click="onLogout">{{ $t("退出") }}</a>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </el-popover>
                            </div>
                        </template>
                        <template v-else>
                            <div class="top-link-item top-link-item--auth">
                                <NuxtLink class="top-link" to="/member/login">{{ $t("你好,请登录") }}</NuxtLink>
                                <template v-if="commonStore.shopRegClosed === 0">
                                    <span class="top-link-sep"></span>
                                    <NuxtLink class="top-link" to="/member/register">{{ $t("注册") }}</NuxtLink>
                                </template>
                            </div>
                        </template>
                    </ClientOnly>
                </div>

                <!-- 中间：跨境风优惠提示（静态展示每组首条，简化轮播） -->
                <div v-if="showBenefitContent" class="benefit-bar__content" :style="topMenuTextStyle">
                    <component
                        :is="getNoticeLink(group.notices[0]) ? 'a' : 'div'"
                        v-for="(group, groupIndex) in benefitNoticeGroups"
                        :key="groupIndex"
                        class="tag-wrap"
                        :href="getNoticeLink(group.notices[0]) || undefined"
                        :target="getNoticeLink(group.notices[0]) ? '_blank' : undefined"
                    >
                        <img v-if="getNoticePic(group.notices[0])" :src="getNoticePic(group.notices[0])" class="img" alt="" />
                        <span v-else class="pc-icon pc-icon--check img"></span>
                        <div class="txt">
                            <div class="title" :style="benefitTitleStyle">{{ getNoticeTitle(group.notices[0]) }}</div>
                            <div class="sub-title" :style="benefitDescStyle">{{ getNoticeDesc(group.notices[0]) }}</div>
                        </div>
                    </component>
                </div>

                <!-- 右侧：购物车 / 消息与订单 / 站点导航 -->
                <div v-if="showStripRight" class="benefit-bar__right" :style="topMenuTextStyle">
                    <NuxtLink v-if="showTopMenuCart" to="/cart/" class="top-link top-link--cart">
                        <span class="iconfont-pc icon-shiwu-gouwuche"></span>
                        <span>{{ $t("购物车") }}{{ cartStore.cartCount > 0 ? ` ${cartStore.cartCount}` : "" }}</span>
                    </NuxtLink>

                    <div v-if="showTopMemberCenter" class="top-site-nav-wrap top-member-center">
                        <el-popover
                            v-if="memberCenterItems.length > 0"
                            trigger="hover"
                            placement="bottom"
                            :show-arrow="false"
                            :teleported="false"
                            :hide-after="300"
                            popper-class="top-site-nav-popper"
                        >
                            <template #reference>
                                <a class="top-link top-link--nav">
                                    <span class="top-bar-nav__name">{{ $t("会员中心") }}</span>
                                    <span class="pc-icon pc-icon--arrow top-bar-nav__arrow"></span>
                                </a>
                            </template>
                            <div class="top-site-nav__dropdown">
                                <NuxtLink v-for="(item, itemIndex) in memberCenterItems" :key="itemIndex" :to="item.link ? urlFormat(item.link) : '#'">
                                    {{ item.title }}
                                </NuxtLink>
                            </div>
                        </el-popover>
                        <NuxtLink v-else to="/member" class="top-link top-link--nav">
                            <span class="top-bar-nav__name">{{ $t("会员中心") }}</span>
                        </NuxtLink>
                    </div>

                    <div v-if="showTopUserLinks" class="top-links">
                        <NuxtLink v-if="showTopMessage" class="top-link top-link--icon" to="/member/userMessage/list/">
                            <span class="iconfont-pc icon-xiaoxi"></span>
                            <span>{{ $t("站内消息") }}</span>
                        </NuxtLink>
                        <NuxtLink v-if="showTopOrders" class="top-link top-link--icon" to="/member/order/list/">
                            <span class="iconfont-pc icon-dingdan"></span>
                            <span>{{ $t("我的订单") }}</span>
                        </NuxtLink>
                    </div>

                    <div v-if="effectiveShowTopSiteNav" class="top-site-nav-wrap">
                        <el-popover
                            trigger="hover"
                            placement="bottom"
                            :show-arrow="false"
                            :teleported="false"
                            :hide-after="300"
                            popper-class="top-site-nav-popper"
                        >
                            <template #reference>
                                <a class="top-link top-link--nav">
                                    <span class="top-bar-nav__name">{{ $t("站点导航") }}</span>
                                    <span class="pc-icon pc-icon--arrow top-bar-nav__arrow"></span>
                                </a>
                            </template>
                            <div class="top-site-nav__dropdown">
                                <template v-for="nav in topBarNavList" :key="nav.id ?? nav.title">
                                    <template v-if="nav.children && nav.children.length > 0">
                                        <CommonLink v-for="child in nav.children" :key="child.id ?? child.title" :item="child">{{ child.title }}</CommonLink>
                                    </template>
                                    <NuxtLink v-else :to="nav.link ? urlFormat(nav.link) : ''">{{ nav.title }}</NuxtLink>
                                </template>
                            </div>
                        </el-popover>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * PC 头部顶部条（运行时）
 * 结构与样式参考 Admin pc-header/PcHeaderPreview.vue 的 benefit-bar；
 * 简约风(styleType=1)与跨境风(styleType=2)共用：简约风展示地区/会员菜单/站点导航/购物车，
 * 跨境风展示优惠提示（benefitNoticeGroups，静态首条，简化轮播）。
 */
import { computed, onMounted, ref } from "vue";
import type { PcHeaderConfig, PcHeaderNoticeItem, PcHeaderNoticeGroup } from "~/types/decorate/pcHeader";
import { imageFormat, productImage, urlFormat } from "~/utils/format";
import { useCommonStore } from "~/store/common";
import { useCartStore } from "~/store/cart";
import { useUserStore } from "~/store/user";
import { getTopRegion, getUserRegion } from "~/api/common";
import CommonLink from "~/components/common/Link.vue";

defineOptions({ name: "PcHeaderTopBarRuntime" });

const props = withDefaults(
    defineProps<{
        styleType: 1 | 2;
        isWhite: boolean;
        config: PcHeaderConfig;
        /** 容器宽度模式：false=原有宽度(1600/1264/38)，true=加宽(1760/1056/80) */
        wide?: boolean;
    }>(),
    {
        wide: false
    }
);

const { t } = useI18n();
const commonStore = useCommonStore();
const cartStore = useCartStore();
const userStore = useUserStore();
const userProvinceId = useCookie<string | number | null>("userProvinceId", { default: () => null });

const regionDropdownOpen = ref(false);

const showBenefitBar = computed(() => (props.config?.showBenefitBar ?? 1) !== 0);
const showTopMenuBar = computed(() => (props.config?.showTopMenuBar ?? 1) !== 0);
// 地区选择独立受自身开关控制，不受右侧菜单总开关影响
const showTopRegion = computed(() => (props.config?.showTopRegion ?? 1) !== 0);
const showTopUserMenu = computed(() => (props.config?.showTopUserMenu ?? 1) !== 0);
const showTopMemberCenterEnabled = computed(() => (props.config?.showTopMemberCenter ?? 1) !== 0);
const showTopMemberName = computed(() => (props.config?.showTopMemberName ?? 1) !== 0);
const showTopMessage = computed(() => (props.config?.showTopMessage ?? 1) !== 0);
const showTopOrders = computed(() => (props.config?.showTopOrders ?? 1) !== 0);
const showTopSiteNav = computed(() => (props.config?.showTopSiteNav ?? 1) !== 0);
const showTopMenuCart = computed(() => (props.config?.topMenuCartShow ?? 1) !== 0);

const topRegionList = computed<Array<{ regionId: string | number; regionName: string }>>(() =>
    Array.isArray(commonStore.topRegion) ? (commonStore.topRegion as Array<{ regionId: string | number; regionName: string }>) : []
);
const regionDisplayText = computed(() => {
    const selected = topRegionList.value.find((item) => String(item.regionId) === String(userProvinceId.value ?? ""));
    return String(selected?.regionName || props.config?.topRegionText || t("全国"));
});

const topBarNavList = computed<Array<Record<string, any>>>(() =>
    Array.isArray(commonStore.topBarNav) ? (commonStore.topBarNav as Array<Record<string, any>>) : []
);
/** 会员中心菜单默认项，与后台头部装修的默认配置保持一致 */
function getDefaultMemberCenterItems() {
    return [
        { title: t("我的足迹"), link: "/member/browsingHistory/list" },
        { title: t("我的优惠券"), link: "/member/coupon/list" }
    ];
}
const memberCenterItems = computed(() => {
    const configured = props.config?.memberCenterItems;
    if (Array.isArray(configured)) {
        return configured
            .map((item) => {
                const link = item?.picLink?.link || item?.link || "";
                const isCouponLink = String(link).replace(/\/$/, "") === "/member/coupon/list";
                return {
                    title: isCouponLink ? t("我的优惠券") : item?.picTitle || item?.title || "",
                    link
                };
            })
            .filter((item) => item.title);
    }
    // 未下发菜单配置时使用默认项，避免与后台设置面板展示不一致
    return getDefaultMemberCenterItems();
});
const effectiveShowTopSiteNav = computed(() => showTopSiteNav.value && topBarNavList.value.length > 0);

const userDisplayName = computed(() => {
    const u = userStore.userInfo as Record<string, any>;
    return String(u?.nickname || u?.dimUsername || t("会员"));
});
const userAvatar = computed(() => imageFormat(String((userStore.userInfo as Record<string, any>)?.avatar || commonStore.defaultAvatar || "")));

function getNoticePic(notice?: PcHeaderNoticeItem | null): string {
    return productImage(notice);
}
function getNoticeTitle(notice?: PcHeaderNoticeItem | null): string {
    return String(notice?.picTitle || "");
}
function getNoticeDesc(notice?: PcHeaderNoticeItem | null): string {
    return String(notice?.picDesc || "");
}
function getNoticeLink(notice?: PcHeaderNoticeItem | null): string {
    const link = notice?.picLink?.link;
    return link ? urlFormat(link) : "";
}
function hasNoticeContent(notice?: PcHeaderNoticeItem | null): boolean {
    return Boolean(getNoticePic(notice) || getNoticeTitle(notice) || getNoticeDesc(notice));
}
const benefitNoticeGroups = computed(() => {
    const raw = Array.isArray(props.config?.benefitNoticeGroups) ? (props.config!.benefitNoticeGroups as PcHeaderNoticeGroup[]) : [];
    return raw
        .map((g) => ({ ...g, notices: (Array.isArray(g?.notices) ? g.notices! : []).filter(hasNoticeContent) }))
        .filter((g) => (g.notices?.length ?? 0) > 0);
});
const showBenefitContent = computed(() => props.styleType === 2 && showBenefitBar.value && benefitNoticeGroups.value.length > 0);

const showTopMemberEntry = computed(() => props.styleType === 1 && showTopMemberName.value);
const showTopUserLinks = computed(
    () => props.styleType === 1 && showTopMenuBar.value && showTopUserMenu.value && (showTopMessage.value || showTopOrders.value)
);
const showTopMemberCenter = computed(() => props.styleType === 1 && showTopMenuBar.value && showTopUserMenu.value && showTopMemberCenterEnabled.value);
const showStripLeft = computed(() => props.styleType === 1 && (showTopRegion.value || showTopMemberEntry.value));
const showStripRight = computed(() => {
    if (props.styleType === 2) return effectiveShowTopSiteNav.value || showTopMenuCart.value;
    return effectiveShowTopSiteNav.value || showTopMenuCart.value || showTopMemberCenter.value || showTopUserLinks.value;
});
const showTopBar = computed(() => {
    if (!showBenefitBar.value) return false;
    return showStripLeft.value || showStripRight.value || showBenefitContent.value;
});
const isBenefitBarOnlyRight = computed(() => showStripRight.value && !showStripLeft.value && !showBenefitContent.value);

const containerStyle = computed(() => ({
    maxWidth: props.wide ? "1760px" : "var(--pc-home-max-width, 1600px)",
    minWidth: props.wide ? "1056px" : "var(--pc-home-min-width, 1264px)",
    width: "100%",
    margin: "0 auto",
    paddingLeft: props.wide ? "80px" : "var(--pc-home-padding-x, 38px)",
    paddingRight: props.wide ? "80px" : "var(--pc-home-padding-x, 38px)"
}));
const benefitBarWrapStyle = computed(() => {
    const bg = String(props.config?.benefitBarBackgroundColor || "");
    return bg ? { backgroundColor: bg } : {};
});
const topMenuTextStyle = computed(() => {
    // 文字色用 color；悬浮色用 CSS 变量，避免 inline color 压过 :hover
    const styles: Record<string, string> = {};
    const color = String(props.config?.topMenuTextColor || "#ffffffd6");
    const hoverColor = String(props.config?.topMenuTextHoverColor || "#ffffff");
    if (color) styles.color = color;
    if (hoverColor) styles["--pc-header-top-menu-hover-color"] = hoverColor;
    return styles;
});
// benefit 主/副标题取色优先级：各自的设置项 > 顶部菜单文字颜色 > 内置默认
const benefitTitleStyle = computed(() => ({ color: String(props.config?.benefitTitleColor || props.config?.topMenuTextColor || "#ffffff") }));
const benefitDescStyle = computed(() => ({ color: String(props.config?.benefitDescColor || props.config?.topMenuTextColor || "#ffffffcc") }));

function selectRegion(item: { regionId: string | number; regionName: string }) {
    userProvinceId.value = item.regionId;
    regionDropdownOpen.value = false;
}
function onLogout() {
    userStore.loginOut();
}

async function ensureRegionData() {
    try {
        if (!Array.isArray(commonStore.topRegion) || commonStore.topRegion.length === 0) {
            const result: any = await getTopRegion();
            commonStore.topRegion = (Array.isArray(result) ? result : (result?.regionList ?? [])) as typeof commonStore.topRegion;
        }
        if (!userProvinceId.value) {
            const result: any = await getUserRegion();
            const regionId = result?.regionId ?? result?.data?.regionId ?? null;
            if (regionId != null) userProvinceId.value = regionId;
        }
    } catch (error) {
        console.error("[PcHeaderTopBarRuntime.ensureRegionData] 加载地区失败:", error);
    }
}
async function loadCartCount() {
    if (!userStore.isAuthenticated) {
        cartStore.resetCart();
        return;
    }
    try {
        await cartStore.changeCartList();
    } catch (error) {
        console.error("[PcHeaderTopBarRuntime.loadCartCount] 获取购物车数量失败:", error);
    }
}

commonStore.loadNav();
userStore.validateUserInfoOnce();

onMounted(() => {
    if (props.styleType === 1 && showTopRegion.value) ensureRegionData();
    loadCartCount();
});
</script>

<style lang="less" scoped>
@import "../_pcHeaderPreviewIcons.less";

.benefit-bar-wrap {
    height: 40px;
    line-height: 40px;
    min-width: var(--pc-home-min-width, 1264px);
    width: 100%;
    background: #1a1a1a;
    box-sizing: border-box;
    padding: 0;
    overflow: visible;
    /* 高于下方 header 主体，遮住底部分割阴影向上渗出的 blur */
    position: relative;
    z-index: 2;
}

/* 容器对齐新版首页主体 home-new-warp(max1600/min1264/padding38)，简约/跨境统一，使顶部条与下方 logo/搜索左右对齐 */
.pc-header-container {
    margin: 0 auto;
    width: 100%;
    min-width: var(--pc-home-min-width, 1264px);
    max-width: var(--pc-home-max-width, 1600px);
    padding: 0 var(--pc-home-padding-x, 38px);
    box-sizing: border-box;
}

.benefit-bar {
    display: flex;
    align-items: center;
    height: 40px;
    gap: 40px;

    &.benefit-bar--simple {
        justify-content: space-between;
        gap: 0;
    }

    &.benefit-bar--only-right {
        justify-content: flex-end;
        gap: 0;
    }
}

.benefit-bar__left {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 18px;
    min-width: 0;

    .top-link {
        display: flex;
        align-items: center;
        color: inherit;
        cursor: pointer;
        line-height: 40px;
        user-select: none;
        gap: 6px;
        text-decoration: none;

        &:hover {
            color: var(--pc-header-top-menu-hover-color, #ffffff) !important;
        }

        &:hover .top-link__name {
            color: var(--pc-header-top-menu-hover-color, inherit) !important;
        }
    }

    .top-link-item {
        position: relative;
        display: inline-flex;
        align-items: center;
        gap: 6px;
        height: 40px;
    }

    .top-link__name {
        max-width: 128px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: inherit;
        font-size: 12px;
        text-decoration: none;
        line-height: 40px;
    }

    .top-link-sep {
        width: 1px;
        height: 12px;
        background: currentColor;
        opacity: 0.4;
    }

    .top-bar-nav__arrow {
        width: 12px;
        height: 12px;
        opacity: 1;
        margin-left: 2px;
        margin-top: 2px;
        align-self: center;
        transition: transform ease 0.25s;
        /* 修正 mask svg 图标本身未居中导致的旋转偏移问题 */
        transform-origin: 50% 36%;
    }

    .top-link--nav:hover .top-bar-nav__arrow {
        transform: rotate(180deg);
    }

    .top-site-nav__dropdown {
        display: flex;
        flex-direction: column;

        :deep(a) {
            display: block;
            padding: 12px 20px;
            font-size: 14px;
            line-height: 20px;
            color: #333 !important;
            text-decoration: none;
            white-space: nowrap;
            border-radius: 6px;
            background: #fff;
            cursor: pointer;

            &:hover {
                background: #f8f8f8;
                color: #333 !important;
            }
        }
    }

    .top-member-dropdown {
        /* 英文等长文案下「账号管理 | 退出」会超出固定宽度并被弹层裁掉，改为按内容自适应 */
        width: max-content;
        min-width: 184px;
        max-width: 320px;
        padding: 16px;
        box-sizing: content-box;
        color: #222;
        line-height: 1.4;
    }

    .top-member-dropdown__profile {
        display: flex;
        align-items: center;
        min-height: 48px;
    }

    .top-member-dropdown__avatar-wrap {
        position: relative;
        flex: 0 0 48px;
        width: 48px;
        height: 48px;
        margin-right: 12px;
    }

    .top-member-dropdown__avatar {
        display: block;
        width: 48px;
        height: 48px;
        border: 1px solid #555;
        border-radius: 50%;
        object-fit: contain;
        box-sizing: border-box;
    }

    .top-member-dropdown__avatar--placeholder {
        display: flex;
        align-items: center;
        justify-content: center;
        background: #f2f2f2;
        color: #999;
        font-size: 18px;
    }

    .top-member-dropdown__main {
        min-width: 0;
        flex: 1;
    }

    .top-member-dropdown__name {
        overflow: hidden;
        color: #222;
        font-size: 14px;
        font-weight: 600;
        line-height: 22px;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .top-member-dropdown__actions {
        display: flex;
        align-items: center;
        margin-top: 8px;
        font-size: 12px;
        line-height: 16px;

        a {
            color: #333 !important;
            text-decoration: none;
            white-space: nowrap;

            &:hover {
                color: var(--general) !important;
            }
        }
    }

    .top-member-dropdown__divider {
        width: 1px;
        height: 12px;
        margin: 0 12px;
        background: #d9d9d9;
    }
}

.benefit-bar__location {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: #ffffffd6;
    font-size: 12px;
    font-weight: 600;
    min-width: 0;
    cursor: pointer;
    position: relative;
    height: 40px;
    padding: 0 10px;
    border-left: 1px solid transparent;
    border-right: 1px solid transparent;

    .benefit-bar__location-ico {
        font-size: 14px;
        color: inherit;
    }

    /* 对齐 vortmall：hover 展开地区时入口高亮为白底 */
    &.is-active {
        color: var(--general);
        background: #fff;
        border-left-color: #dbdbdb;
        border-right-color: #dbdbdb;
    }
    &.wide {
        padding-left: 0;
    }
}

.benefit-bar__region {
    max-width: 240px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.region-dropdown {
    position: absolute;
    left: 0;
    top: 40px;
    width: 300px;
    background: #fff;
    border: 1px solid #dbdbdb;
    border-top: 0;
    box-shadow: 1px 2px 1px rgba(0, 0, 0, 0.1);
    z-index: 4001;
    padding: 10px;
    line-height: 24px;
    cursor: default;

    .region-dropdown__list {
        display: flex;
        flex-wrap: wrap;
        overflow: hidden;
    }

    .region-dropdown__item {
        width: 60px;
        padding: 2px 8px;
        color: #333;
        cursor: pointer;
        text-decoration: none;
        box-sizing: border-box;

        &:hover {
            background-color: #f4f4f4;
        }

        &.selected {
            background-color: var(--general);
            color: var(--main-text) !important;
        }
    }
}

.benefit-bar__content {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 40px;
    color: #fff;
}

.tag-wrap {
    display: flex;
    align-items: center;
    color: inherit;
    text-decoration: none;

    .img {
        width: 14px;
        height: 14px;
    }

    .txt {
        display: flex;
        flex-direction: row;
        align-items: baseline;
        line-height: 1;
        margin-left: 4px;

        .title {
            font-size: 12px;
            font-weight: 700;
            margin-right: 6px;
            white-space: nowrap;
        }

        .sub-title {
            font-size: 12px;
            font-weight: 500;
            white-space: nowrap;
            color: #ffffffcc;
        }
    }
}

.benefit-bar__right {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 18px;
    white-space: nowrap;
    color: #ffffffd6;

    .top-links {
        display: inline-flex;
        align-items: center;
        gap: 18px;
    }

    .top-link {
        display: flex;
        align-items: center;
        color: inherit;
        cursor: pointer;
        line-height: 40px;
        user-select: none;
        gap: 6px;
        text-decoration: none;

        &:hover {
            color: var(--pc-header-top-menu-hover-color, #ffffff) !important;
        }

        .iconfont-pc {
            font-size: 15px;
            line-height: 1;
        }
    }

    .top-link--cart .iconfont-pc {
        font-size: 16px;
    }

    .top-link-item {
        position: relative;
        display: inline-flex;
        align-items: center;
        gap: 6px;
        height: 40px;
    }

    .top-link-item--auth {
        gap: 6px;
    }

    .top-link__name {
        max-width: 160px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: inherit;
        text-decoration: none;
        line-height: 40px;
    }

    .top-link-sep {
        width: 1px;
        height: 12px;
        background: currentColor;
        opacity: 0.4;
    }

    .top-bar-nav__name {
        font-size: 12px;
        line-height: 40px;
        white-space: nowrap;
    }

    .top-bar-nav__arrow {
        width: 12px;
        height: 12px;
        opacity: 1;
        margin-left: 2px;
        margin-top: 2px;
        align-self: center;
        transition: transform ease 0.25s;
        /* 修正 mask svg 图标本身未居中导致的旋转偏移问题 */
        transform-origin: 50% 36%;
    }

    .top-site-nav-wrap {
        display: inline-flex;
        align-items: center;
        height: 40px;
    }

    .top-link--nav:hover .top-bar-nav__arrow {
        transform: rotate(180deg);
    }

    /* 会员下拉 / 站点导航下拉共用（对齐 el-popover 圆角卡片样式） */
    .top-site-nav__dropdown {
        display: flex;
        flex-direction: column;

        :deep(a) {
            display: block;
            padding: 12px 20px;
            font-size: 14px;
            line-height: 20px;
            color: #333 !important;
            text-decoration: none;
            white-space: nowrap;
            border-radius: 6px;
            background: #fff;
            cursor: pointer;

            &:hover {
                background: #f8f8f8;
                color: #333 !important;
            }
        }
    }
}
</style>

<style lang="less">
/* el-popover 弹层在组件内渲染(teleported=false)，容器根 .el-popper 不带 scoped 属性，需单独重置默认内边距/圆角 */
.top-site-nav-popper.el-popover.el-popper {
    width: auto !important;
    min-width: 0 !important;
    padding: 6px;
    border: 1px solid #ebebeb;
    border-radius: 8px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.12);
    white-space: nowrap;
}

.top-member-popper.el-popover.el-popper {
    width: auto !important;
    max-width: none !important;
    min-width: 0 !important;
    box-sizing: content-box;
    padding: 0;
    border: 1px solid #eeeeee;
    border-radius: 16px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.12);
    overflow: hidden;
}
</style>
