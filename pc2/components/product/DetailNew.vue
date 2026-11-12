<template>
    <!--
      新版 UI 商品详情内容区
      参考京东全球购详情页布局：买家评价 / 为你推荐 / 进店逛逛 / 商品详情参数 / 商品咨询 / 售后服务
    -->
    <div class="detail-new">
        <!-- 顶部锚点 Tab 栏 -->
        <div class="detail-tabs sticky-element">
            <ul class="tab-list">
                <li v-for="tab in tabs" :key="tab.anchor" class="tab-item" :class="{ current: activeTab === tab.anchor }" @click="scrollToAnchor(tab.anchor)">
                    {{ tab.label }}
                </li>
            </ul>
        </div>
        <div class="detail-content">
            <!-- 买家评价 -->
            <section class="block" id="pingjia-new" v-if="commentEnabled">
                <div class="block-head">
                    <h3 class="block-title">{{ $t("买家评价") }}({{ commentCountText }})</h3>
                    <a class="block-action praise" href="javascript:;" @click="openCommentDialog" v-if="(rankDetail?.total || 0) > 0">
                        {{ $t("好评率") }}{{ goodPercentText }}<el-icon><ArrowRight /></el-icon>
                    </a>
                </div>
                <div class="comment-preview" v-if="previewList.length">
                    <div class="comment-item" v-for="item in previewList" :key="item.commentId">
                        <div class="comment-user">
                            <img class="avatar" :src="imageFormat(item.avatar || '')" :alt="item.username || ''" />
                            <span class="nickname">{{ item.username }}</span>
                        </div>
                        <div class="comment-content">{{ item.content }}</div>
                        <div class="comment-pics" v-if="item.showPics && item.showPics.length">
                            <div class="comment-pic" v-for="(photo, pIndex) in item.showPics" :key="pIndex">
                                <el-image
                                    :src="imageFormat(photo.picThumb)"
                                    fit="cover"
                                    loading="lazy"
                                    :preview-src-list="item.showPics.map((p) => imageFormat(p.picUrl))"
                                    :initial-index="pIndex"
                                />
                            </div>
                        </div>
                    </div>
                </div>
                <div class="comment-empty" v-else-if="!commentLoading">「{{ $t("暂无评价") }}」</div>
                <div class="comment-more" v-if="(rankDetail?.total || 0) >= 3">
                    <button type="button" class="more-btn" @click="openCommentDialog">
                        {{ $t("全部评价") }}<el-icon><ArrowRight /></el-icon>
                    </button>
                </div>
            </section>

            <!-- 为你推荐 -->
            <section class="block" id="recommend-new" v-if="recommendList.length">
                <div class="block-head">
                    <h3 class="block-title">{{ $t("为你推荐") }}</h3>
                </div>
                <div class="recommend" :class="{ 'has-left': canScrollLeft, 'has-right': canScrollRight }">
                    <button class="rec-arrow rec-arrow-left" type="button" v-show="canScrollLeft" @click="scrollRec(-1)">
                        <el-icon><ArrowLeft /></el-icon>
                    </button>
                    <div class="rec-track" ref="recTrackRef" @scroll="updateRecArrows">
                        <div class="rec-card" v-for="item in recommendList" :key="item.productId">
                            <NuxtLink class="rec-pic" :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })" target="_blank">
                                <el-image :src="productImage(item)" lazy :alt="item.productName || ''" />
                            </NuxtLink>
                            <div class="rec-body">
                                <NuxtLink class="rec-name" :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })" target="_blank">
                                    {{ item.productName }}
                                </NuxtLink>
                                <div class="rec-foot">
                                    <span class="rec-price"
                                        ><i>{{ dollarSign }}</i
                                        >{{ item.productPrice }}</span
                                    >
                                    <NuxtLink class="rec-add" :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })" target="_blank">
                                        <el-icon><Plus /></el-icon>
                                    </NuxtLink>
                                </div>
                            </div>
                        </div>
                    </div>
                    <button class="rec-arrow rec-arrow-right" type="button" v-show="canScrollRight" @click="scrollRec(1)">
                        <el-icon><ArrowRight /></el-icon>
                    </button>
                </div>
            </section>

            <!-- 进店逛逛 -->
            <section
                class="block"
                id="shop-new"
                v-if="(isMerchant() || isStore()) && shopInfo && shopInfo.shopId"
            >
                <div class="block-head">
                    <h3 class="block-title">{{ $t("进店逛逛") }}</h3>
                </div>
                <div class="shop-card">
                    <NuxtLink class="shop-logo" :to="shopHref">
                        <img :src="imageFormat(shopInfo.shopLogo)" :alt="shopInfo.shopTitle || ''" />
                    </NuxtLink>
                    <div class="shop-main">
                        <div class="shop-name-row">
                            <span class="shop-tag" v-if="shopRankName">{{ shopRankName }}</span>
                            <NuxtLink class="shop-name" :to="shopHref">{{ shopInfo.shopTitle }}</NuxtLink>
                            <el-rate v-model="rating" disabled allow-half size="small" />
                            <span class="rate-score">{{ scoreText }}</span>
                        </div>
                    </div>
                    <div class="shop-scores">
                        <div class="score-item">
                            <span class="score-label">{{ $t("商品评价") }}</span>
                            <span class="score-value">{{ scoreLevelText }}</span>
                        </div>
                    </div>
                    <div class="shop-actions">
                        <CommonService class="shop-action-item" :shopId="shopInfo.shopId" :phone="shopInfo.kefuPhone" :isIndex="true">
                            <button type="button" class="shop-btn"><span class="iconfont-pc icon-kefu"></span>{{ $t("联系客服") }}</button>
                        </CommonService>
                        <NuxtLink class="shop-action-item shop-btn" :to="shopHref"><span class="iconfont-pc icon-guanli"></span>{{ $t("进店逛逛") }}</NuxtLink>
                    </div>
                </div>
            </section>

            <!-- 商品详情（参数表 + 富文本介绍） -->
            <section class="block" id="param-new" v-if="paramRows.length || descArr.length || isPaidProduct">
                <div class="block-head">
                    <h3 class="block-title">{{ $t("商品详情") }}</h3>
                </div>
                <ul class="param-cards" v-if="paramCards.length">
                    <li class="param-card" v-for="(row, index) in paramCards" :key="index">
                        <span class="param-card-value">{{ row.value }}</span>
                        <span class="param-card-label">{{ row.label }}</span>
                    </li>
                </ul>
                <ul class="param-grid" v-if="paramRest.length">
                    <li class="param-item" v-for="(row, index) in paramRest" :key="index">
                        <span class="param-label">{{ row.label }}</span>
                        <span class="param-value">{{ row.value }}</span>
                    </li>
                </ul>
                <div class="goods-description detail-content-c29wMjI4Mmc" v-if="isPaidProduct">
                    <template v-if="product.isBuy && product.paidContent">
                        <template v-for="(item, index) in product.paidContent" :key="index">
                            <img v-if="item.type == 'pic'" class="desc-img" :src="imageFormat(item.pic || '')" :alt="product.productName || ''" />
                            <ClientOnly>
                                <div v-if="item.type == 'text'" class="desc-text" v-html="formatRichText(item.html || '')"></div>
                            </ClientOnly>
                            <div class="desc-video-item" v-if="item.type == 'video' && item.videoUrl">
                                <video
                                    :src="imageFormat(item.videoUrl)"
                                    :poster="item.cover ? imageFormat(item.cover) : undefined"
                                    controls
                                    preload="metadata"
                                    style="width: 100%; display: block"
                                ></video>
                            </div>
                        </template>
                    </template>
                    <img v-else class="desc-img" src="~/assets/images/product/paidcontent-bg.png" alt="" />
                </div>
                <div class="goods-description" v-if="descArr.length">
                    <template v-for="(item, index) in descArr" :key="index">
                        <img v-if="item.type == 'pic'" class="desc-img" :src="imageFormat(item.pic || '')" :alt="product.productName || ''" />
                        <ClientOnly>
                            <div v-if="item.type == 'text'" class="desc-text" v-html="formatRichText(item.html)"></div>
                        </ClientOnly>
                        <div class="desc-video-item" v-if="item.type == 'video' && item.videoUrl">
                            <video
                                :src="imageFormat(item.videoUrl)"
                                :poster="item.cover ? imageFormat(item.cover) : undefined"
                                controls
                                preload="metadata"
                                style="width: 100%; display: block"
                            ></video>
                        </div>
                    </template>
                </div>
            </section>

            <!-- 商品咨询 -->
            <section class="block" id="consult-new">
                <div class="block-head">
                    <h3 class="block-title">{{ $t("商品咨询") }}({{ consultationTotal }})</h3>
                </div>
                <ProductConsultation :product="product" :productStock="productStock" :productNumber="productNumber"></ProductConsultation>
            </section>

            <!-- 售后服务 -->
            <section class="block" id="service-new" v-if="afterSalesFlag">
                <div class="block-head">
                    <h3 class="block-title">{{ $t("售后服务") }}</h3>
                </div>
                <ClientOnly>
                    <ProductAfterSaleService></ProductAfterSaleService>
                </ClientOnly>
            </section>
        </div>

        <!-- 全部评价弹窗 -->
        <el-dialog v-model="commentDialogVisible" :title="$t('商品评价')" width="920px" top="6vh" class="comment-dialog" append-to-body>
            <div class="app" :style="themeStore.themeColor">
                <ProductComment v-if="commentDialogVisible" :modelValue="productId"></ProductComment>
            </div>
        </el-dialog>
    </div>
</template>

<script lang="ts" setup>
import { computed, ref, onMounted, onUnmounted, nextTick } from "vue";
import { ArrowRight, ArrowLeft, Plus } from "@element-plus/icons-vue";
import { imageFormat, urlFormat, ratingFormat, productImage } from "@/utils/format";
import { formatRichText, isMerchant, isPro, isStore } from "@/utils/util";
import { getCommentList, getProductLikeList } from "~/api/product/product";
import { useCommonStore } from "@/store/common";
import { useThemeStore } from "@/store/theme";
import type { CommentItem, ProductItem } from "~/types/product/product.d";

const props = defineProps({
    product: { type: Object, default: () => ({}) },
    attrList: { type: Object, default: () => ({ normal: [] }) },
    rankDetail: { type: Object, default: () => ({}) },
    consultationTotal: { type: Number, default: 0 },
    shopInfo: { type: Object, default: () => ({}) },
    productId: { type: Number, default: 0 },
    descArr: { type: Array as () => any[], default: () => [] },
    productStock: { type: Number, default: 0 },
    productNumber: { type: Number, default: 1 },
    afterSalesFlag: { type: Boolean, default: true }
});

const { t } = useI18n();
const commonStore = useCommonStore();
const themeStore = useThemeStore();
const dollarSign = computed(() => commonStore.dollarSign || "¥");
// D1146：商品评价总开关，关闭时买家评价区块与锚点 Tab 一并隐藏
const commentEnabled = computed(() => commonStore.productCommentEnabled === 1);

/* ---------------- 门店 ---------------- */
const shopHref = computed(() => {
    const shopId = props.shopInfo?.shopId;
    if (!shopId) return "";
    // 非 o2o：进入多商户店铺首页
    return urlFormat({ path: "shop", id: shopId, seoLink: (props.shopInfo as any)?.seoLink });
});
const rating = computed(() => Number(ratingFormat(props.shopInfo?.shopStar || props.shopInfo?.productScore || 5)));
const scoreText = computed(() => ratingFormat(props.shopInfo?.shopStar || props.shopInfo?.productScore || 5));
const scoreLevelText = computed(() => {
    const score = Number(scoreText.value);
    if (score >= 4.5) return t("高");
    if (score >= 3) return t("中");
    return t("低");
});
const shopRankName = computed(() => props.shopInfo?.shopRankName || props.shopInfo?.rankName || "");

// o2o 显示「门店」，非 o2o 显示「店铺」
const shopLabel = computed(() => {
    return t("店铺");
});

/* ---------------- 商品参数表 ---------------- */
const paramRows = computed(() => {
    const rows: { label: string; value: string }[] = [];
    const normal = props.attrList?.normal || [];
    normal.forEach((item: any) => {
        const value = (item.attrList || [])
            .map((a: any) => a.attrValue)
            .filter(Boolean)
            .join(" ");
        if (item.attrName && value) rows.push({ label: item.attrName, value });
    });
    return rows;
});
// 前六个参数以卡片形式展示，其余沿用行布局
const paramCards = computed(() => paramRows.value.slice(0, 6));
const paramRest = computed(() => paramRows.value.slice(6));
const isPaidProduct = computed(() => isPro() && Number(props.product?.productType) === 4);

/* ---------------- 为你推荐 ---------------- */
const recommendList = ref<ProductItem[]>([]);
const getRecommend = async () => {
    try {
        const result = await getProductLikeList({ page: 1, size: 10 });
        recommendList.value = (result || []).filter((item: any) => item.productId !== props.productId);
    } catch (error) {
        console.error(error);
    }
};

const recTrackRef = ref<HTMLElement | null>(null);
const canScrollLeft = ref(false);
const canScrollRight = ref(false);
const updateRecArrows = () => {
    const el = recTrackRef.value;
    if (!el) return;
    canScrollLeft.value = el.scrollLeft > 4;
    canScrollRight.value = el.scrollLeft + el.clientWidth < el.scrollWidth - 4;
};
const scrollRec = (dir: number) => {
    const el = recTrackRef.value;
    if (!el) return;
    el.scrollBy({ left: dir * el.clientWidth * 0.8, behavior: "smooth" });
};

/* ---------------- 买家评价 ---------------- */
const previewList = ref<CommentItem[]>([]);
const commentLoading = ref(false);
const commentDialogVisible = ref(false);
const commentCountText = computed(() => {
    const total = props.rankDetail?.total || 0;
    return total > 99 ? "99+" : String(total);
});
const goodPercentText = computed(() => {
    const percent = props.rankDetail?.goodPercent ?? 0;
    return `${percent}%`;
});
const getCommentPreview = async () => {
    // D1146：评价功能关闭时不再请求评价预览
    if (!props.productId || !commentEnabled.value) return;
    commentLoading.value = true;
    try {
        const result: any = await getCommentList(props.productId, { type: 1, page: 1 });
        previewList.value = (result.records || []).slice(0, 3);
    } catch (error) {
        console.error(error);
    } finally {
        commentLoading.value = false;
    }
};
const openCommentDialog = () => {
    commentDialogVisible.value = true;
};

/* ---------------- 顶部锚点 Tab ---------------- */
const ANCHOR_OFFSET = 64;
const tabs = computed(() => {
    const list: { label: string; anchor: string }[] = [];
    // D1146：评价功能关闭时摘掉「大家评」锚点，避免点击滚到不存在的区块
    if (commentEnabled.value) list.push({ label: t("大家评"), anchor: "pingjia-new" });
    if (recommendList.value.length) list.push({ label: t("推荐"), anchor: "recommend-new" });
    if ((isMerchant() || isStore()) && props.shopInfo?.shopId) {
        list.push({ label: shopLabel.value, anchor: "shop-new" });
    }
    if (paramRows.value.length || props.descArr.length || isPaidProduct.value) list.push({ label: t("商品详情"), anchor: "param-new" });
    list.push({ label: t("商品咨询"), anchor: "consult-new" });
    if (props.afterSalesFlag) list.push({ label: t("售后保障"), anchor: "service-new" });
    return list;
});
const activeTab = ref(tabs.value[0]?.anchor ?? "");

const scrollToAnchor = (anchor: string) => {
    const el = document.getElementById(anchor);
    if (!el) return;
    const top = el.getBoundingClientRect().top + window.scrollY - ANCHOR_OFFSET;
    window.scrollTo({ top, behavior: "smooth" });
};

const onScroll = () => {
    let current = tabs.value[0]?.anchor || "pingjia-new";
    for (const tab of tabs.value) {
        const el = document.getElementById(tab.anchor);
        if (el && el.getBoundingClientRect().top - ANCHOR_OFFSET <= 1) {
            current = tab.anchor;
        }
    }
    activeTab.value = current;
};

onMounted(async () => {
    getCommentPreview();
    await getRecommend();
    await nextTick();
    updateRecArrows();
    window.addEventListener("scroll", onScroll, { passive: true });
    onScroll();
});

onUnmounted(() => {
    window.removeEventListener("scroll", onScroll);
});
</script>

<style lang="less" scoped>
.detail-new {
    width: 100%;

    /* ---------------- 顶部锚点 Tab 栏 ---------------- */
    .detail-tabs {
        background: #fff;
        border-bottom: 1px solid rgba(194, 196, 204, 0.6);

        .tab-list {
            display: flex;
            align-items: center;
            height: 50px;
            list-style: none;
            margin: 0;
            padding: 0;
        }

        .tab-item {
            position: relative;
            margin: 0 14px;
            font-size: 16px;
            color: #000;
            cursor: pointer;
            font-weight: 600;
            transition: color 0.2s ease;

            &:hover {
                color: var(--main-bg);
            }

            &.current {
                color: var(--general);

                &::after {
                    content: "";
                    position: absolute;
                    left: 0;
                    right: 0;
                    bottom: -15px;
                    width: 100%;
                    height: 2px;
                    background: var(--general);
                }
            }
        }
    }

    .block {
        padding: 24px 0;
        border-bottom: 1px solid #f0f0f0;
        scroll-margin-top: 64px;

        &:last-child {
            border-bottom: none;
        }

        :deep(.promInfo) {
            border-radius: 6px;
            .span1 {
                font-size: 12px;
                color: #191919;
                line-height: 18px;
            }
        }
    }

    .block-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 24px;

        .block-title {
            display: flex;
            align-items: baseline;
            font-size: 20px;
            font-weight: 600;
            color: #1a1a1a;

            .count {
                margin-left: 4px;
                font-size: 14px;
                font-weight: 400;
                font-style: normal;
                color: #9ba0a8;
            }
        }

        .block-action {
            display: inline-flex;
            align-items: center;
            gap: 2px;
            font-size: 14px;
            color: #9ba0a8;
            text-decoration: none;
            transition: color 0.3s ease;

            &:hover {
                color: var(--main-bg);
            }

            &.praise {
                color: var(--general);
                font-weight: 600;

                &:hover {
                    opacity: 0.85;
                    color: var(--general);
                }
            }
        }
    }

    .detail-content {
        padding: 0 14px;
    }

    /* ---------------- 买家评价 ---------------- */
    .comment-preview {
        .comment-item {
            margin-bottom: 24px;

            .comment-user {
                display: flex;
                align-items: center;
                gap: 8px;
                margin-bottom: 16px;

                .avatar {
                    width: 32px;
                    height: 32px;
                    border-radius: 50%;
                    object-fit: cover;
                    background: #f2f2f2;
                }
                .nickname {
                    font-size: 14px;
                    color: #828794;
                }
            }

            .comment-content {
                font-size: 14px;
                color: #191919;
                line-height: 18px;
            }

            .comment-pics {
                display: flex;
                flex-wrap: wrap;
                gap: 8px;
                margin-top: 16px;

                .comment-pic {
                    width: 118px;
                    height: 118px;
                    cursor: pointer;
                    border-radius: 6px;
                    overflow: hidden;

                    :deep(.el-image) {
                        display: block;
                        width: 118px;
                        height: 118px;
                        border-radius: 6px;
                        overflow: hidden;

                        .el-image__inner {
                            border-radius: 6px;
                        }
                    }
                }
            }
        }
    }

    .comment-empty {
        padding: 30px 0;
        text-align: center;
        color: #9ba0a8;
    }

    .comment-more {
        display: flex;
        justify-content: center;
        margin-top: 20px;

        .more-btn {
            display: flex;
            align-items: center;
            justify-content: center;
            width: 128px;
            height: 40px;
            font-size: 16px;
            color: #1a1a1a;
            cursor: pointer;
            border-radius: 6px;
            border: 0.5px solid #888b93;
            margin: 0 auto 0;
            background: #fff;
            transition:
                color 0.3s ease,
                border-color 0.3s ease;

            &:hover {
                color: var(--main-bg);
                border-color: var(--main-bg);
            }

            :deep(.el-icon) {
                font-size: 18px;
            }

            :deep(svg) {
                margin-left: 4px;
                stroke: currentColor;
                stroke-width: 50;
            }
        }
    }

    /* ---------------- 为你推荐 ---------------- */
    .recommend {
        position: relative;

        &.has-left {
            padding-left: 38px;
        }

        &.has-right {
            padding-right: 38px;
        }

        .rec-track {
            display: flex;
            overflow-x: auto;
            scroll-behavior: smooth;
            padding-bottom: 4px;
            scrollbar-width: none;

            &::-webkit-scrollbar {
                display: none;
            }
        }

        .rec-card {
            flex: 0 0 auto;
            width: 238px;
            height: 116px;
            background: #f9fafc;
            border-radius: 6px;
            margin-right: 12px;
            display: flex;
            padding: 8px;
            box-sizing: border-box;
            transition: border-color 0.3s ease;

            &:hover {
                border-color: var(--main-bg);
            }

            .rec-pic {
                flex-shrink: 0;
                display: flex;
                align-items: center;
                justify-content: center;
                width: 100px;
                height: 100px;
                overflow: hidden;
                background: #fff;
                border-radius: 4px;
                border: 1px solid rgba(0, 0, 0, 0.03);

                :deep(.el-image),
                :deep(img) {
                    width: 85px;
                    height: 85px;
                    object-fit: cover;
                }
            }

            .rec-body {
                position: relative;
                display: flex;
                flex-direction: column;
                margin-left: 6px;
                min-width: 0;
                flex: 1;
            }

            .rec-name {
                font-size: 14px;
                color: #1a1a1a;
                line-height: 20px;
                margin-bottom: 12px;
                overflow: hidden;
                cursor: pointer;
                word-break: break-all;
                height: 37px;
                text-decoration: none;
                display: -webkit-box;
                -webkit-line-clamp: 2;
                -webkit-box-orient: vertical;

                &:hover {
                    color: var(--main-bg);
                }
            }

            .rec-foot {
                display: flex;
                align-items: flex-end;
                justify-content: space-between;

                .rec-price {
                    color: var(--general);
                    font-size: 18px;

                    i {
                        font-size: 14px;
                        font-style: normal;
                    }
                }

                .rec-add {
                    position: absolute;
                    right: 0;
                    bottom: 0;
                    display: inline-flex;
                    align-items: center;
                    justify-content: center;
                    width: 24px;
                    height: 24px;
                    border-radius: 50%;
                    background: var(--general);
                    color: #fff;
                    text-decoration: none;
                    font-size: 16px;

                    &:hover {
                        color: #fff !important;
                    }
                }
            }
        }

        .rec-arrow {
            position: absolute;
            top: 50%;
            transform: translateY(-50%);
            z-index: 2;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            width: 30px;
            height: 116px;
            box-sizing: border-box;
            border: 0.5px solid rgba(0, 0, 0, 0.06);
            border-radius: 4px;
            background: rgba(255, 255, 255, 0.95);
            box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
            color: #5e5e5e;
            cursor: pointer;
            user-select: none;

            &.rec-arrow-left {
                left: 0;
                font-size: 18px;
            }
            &.rec-arrow-right {
                right: 0;
                font-size: 18px;
            }
        }
    }

    /* ---------------- 进店逛逛 ---------------- */
    .shop-card {
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 24px;

        .shop-logo {
            flex-shrink: 0;
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 6px;
            width: 64px;
            text-decoration: none;
            border: 0.5px solid rgba(0, 0, 0, 0.06);
            border-radius: 4px;
            overflow: hidden;

            img {
                width: 100%;
                height: 100%;
                object-fit: contain;
            }
        }

        .shop-main {
            margin-left: -12px;
            flex-shrink: 0;
            min-width: 0;

            .shop-name-row {
                display: flex;
                align-items: center;
                gap: 8px;

                .shop-tag {
                    flex-shrink: 0;
                    padding: 2px 7px;
                    font-size: 12px;
                    line-height: 16px;
                    color: #fff;
                    background: #2b2b35;
                    border-radius: 4px;
                    white-space: nowrap;
                }

                .shop-name {
                    font-size: 16px;
                    font-weight: 700;
                    color: #1a1a1a;
                    text-decoration: none;
                    white-space: nowrap;

                    &:hover {
                        color: var(--general);
                    }
                }

                .rate-score {
                    color: var(--general);
                    font-size: 14px;
                    font-weight: 700;
                }
            }
        }

        .shop-scores {
            display: flex;
            align-items: center;
            justify-content: flex-start;
            flex: 1;
            gap: 48px;

            .score-item {
                display: flex;
                flex-direction: column;
                align-items: center;
                gap: 8px;

                .score-label {
                    font-size: 14px;
                    color: #888b94;
                }
                .score-value {
                    font-size: 14px;
                    color: var(--general);
                }
            }
        }

        .shop-actions {
            display: flex;
            gap: 12px;
            flex-shrink: 0;

            .shop-btn {
                display: inline-flex;
                align-items: center;
                justify-content: center;
                gap: 6px;
                border: 0.5px solid #888b93;
                width: 98px;
                height: 36px;
                border-radius: 4px;
                background: #fff;
                color: rgba(9, 9, 10, 0.92);
                font-size: 14px;
                text-decoration: none;
                cursor: pointer;
                box-sizing: border-box;
                white-space: nowrap;
                transition:
                    color 0.3s ease,
                    border-color 0.3s ease;

                .iconfont-pc {
                    font-size: 16px;
                }

                &:hover {
                    color: var(--main-bg);
                    border-color: var(--main-bg);
                }
            }
        }

        /* 小于 1420px 时按钮整行下移到底部 */
        @media (max-width: 1420px) {
            row-gap: 16px;

            .shop-scores {
                justify-content: flex-end;
            }

            .shop-actions {
                flex: 0 0 100%;
                width: 100%;

                .shop-action-item {
                    flex: 1;
                    min-width: 0;
                }

                .shop-btn {
                    width: 100%;
                }
            }
        }
    }

    /* ---------------- 商品详情参数 ---------------- */
    .param-cards {
        background: #f5f6fa;
        border-radius: 8px;
        box-sizing: border-box;
        padding: 32px 24px;
        margin-bottom: 12px;
        list-style: none;
        display: flex;
        flex-wrap: wrap;

        .param-card {
            flex: 0 0 33.333%;
            max-width: 33.333%;
            box-sizing: border-box;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            gap: 12px;
            text-align: center;

            &:nth-child(n + 4) {
                margin-top: 32px;
            }

            .param-card-value {
                max-width: 100%;
                font-size: 20px;
                color: #1a1a1a;
                font-weight: 600;
                line-height: 20px;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
            }

            .param-card-label {
                max-width: 100%;
                font-size: 16px;
                line-height: 16px;
                color: #3d414d;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
            }
        }
    }

    .param-grid {
        display: grid;
        grid-template-columns: repeat(2, 1fr);
        column-gap: 24px;

        .param-item {
            display: flex;
            align-items: center;
            border-bottom: 1px solid #f5f5f5;
            font-size: 14px;
            height: 48px;

            .param-label {
                flex-shrink: 0;
                width: 142px;
                padding: 0 16px;
                color: #1a1a1a;
                font-size: 14px;
                font-weight: 600;
                box-sizing: border-box;
                overflow: hidden;
                text-overflow: ellipsis;
                display: -webkit-box;
                -webkit-line-clamp: 2;
                line-clamp: 2;
                -webkit-box-orient: vertical;
                word-break: break-all;
            }
            .param-value {
                padding: 0 24px;
                color: #1a1a1a;
                font-size: 14px;
                word-break: break-all;
                overflow: hidden;
                text-overflow: ellipsis;
                display: -webkit-box;
                -webkit-line-clamp: 2;
                line-clamp: 2;
                -webkit-box-orient: vertical;
            }
        }
    }

    .goods-description {
        margin-top: 24px;

        .desc-img {
            display: block;
            width: 100%;
        }

        .desc-text {
            line-height: 1.8;
            color: #333;

            :deep(img) {
                max-width: 100%;
            }
        }

        .desc-video-item {
            margin: 10px 0;
        }
    }
}
</style>

<style lang="less">
/* 全部评价弹窗圆角（append-to-body 渲染到 body 外，需非 scoped 样式命中） */
.comment-dialog.el-dialog {
    border-radius: 12px;
    overflow: hidden;
    max-height: 950px;
    display: flex;
    flex-direction: column;

    .el-dialog__body {
        flex: 1;
        min-height: 0;
        overflow-y: auto;
    }
}
</style>
