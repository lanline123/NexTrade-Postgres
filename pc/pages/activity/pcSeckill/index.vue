<template>
    <div class="module-seckill-goods_ad-wrapper" v-if="seckillList && seckillList?.length > 0">
        <div
            :style="
                allFormat.boxPadding +
                allFormat.boxPaddingTop +
                allFormat.boxPaddingBottom +
                allFormat.innerPadding +
                allFormat.boxRadius +
                ' ' +
                frameFormat.backgroundColor +
                allFormat.backgroundColor +
                ' ' +
                frameFormat.boxRadius +
                allFormat.boxRadius
            "
            style="position: relative"
            :class="
                'module-ad-con module-goods_ad module-seckill-goods_ad ad-style__' +
                module?.style +
                ' ad-buy_btn_style__' +
                module?.buyBtnStyle +
                ' ad-goods_style__' +
                module?.goodsStyle +
                ' ad-goods_radio_style__' +
                module?.goodsRadioStyle +
                ' ad-text_align__' +
                module?.textAlign +
                ' ad-text_weight__' +
                module?.textWeight +
                ' ad-goods_name_row__' +
                module?.goodsNameRow +
                ' ad-goods_name_padding__' +
                module?.goodsNamePadding +
                ' ad-goods-title_style__' +
                module?.title?.titleStyle
            "
        >
            <div class="module-ad-content">
                <div v-if="seckillList.length > 0" class="mod_seckill">
                    <!-- 标题和倒计时一起显示隐藏 -->
                    <div
                        class="seckill-top"
                        v-if="module.title?.showTitle == 1"
                        :style="{
                            background: module.title?.titleBackground2
                                ? 'linear-gradient(to right, ' + (module.title?.titleBackground || '') + ', ' + module.title.titleBackground2 + ')'
                                : module.title?.titleBackground || '',
                            borderTopLeftRadius: module.title?.titleRadius ? module.title.titleRadius + 'px' : '',
                            borderTopRightRadius: module.title?.titleRadius ? module.title.titleRadius + 'px' : ''
                        }"
                    >
                        <div class="seckill-title-wrapper">
                            <div class="line" v-if="module.title?.titleStyle == 3"></div>
                            <el-image
                                v-if="module.title?.titleBackgroundPic?.picUrl"
                                class="image-size"
                                :src="imageFormat(module.title.titleBackgroundPic.picUrl)"
                            ></el-image>
                            <div class="seckill-title" :style="{ color: module.title?.titleColor || '#1a1a1a' }">
                                {{ module.title?.titleText ? module.title.titleText : $t("限时秒杀") }}
                            </div>
                            <div
                                class="seckill-desc"
                                v-if="module.title?.descText"
                                :style="{ color: module.title?.descColor || '#1a1a1a' }"
                                :class="[module.title?.titleStyle == 2 ? 'big-sm' : 'sm']"
                            >
                                {{ module.title.descText }}
                            </div>
                        </div>
                        <div class="seckill-top-right">
                            <!-- 倒计时：无箭头、不跳转 -->
                            <div
                                class="seckill-count-down"
                                v-if="titleMore.showCountdown && getTime > 0"
                                :style="{ color: module.title?.moreColor || '#1a1a1a' }"
                            >
                                <CountDown countdown-type="specifyTime" :specify-time="getTime">
                                    <template #default="{ days, hours, minutes, seconds }">
                                        <div class="countdown-box">
                                            <span class="countdown-text">{{ $t(countdownLabel) }}: </span>
                                            <template v-if="days && Number(days) > 0">
                                                <span class="countdown-item" :class="{ days: String(days).length > 2 }">{{ days }}</span>
                                                <span class="countdown-separator-text">{{ $t("天") }}</span>
                                            </template>
                                            <span class="countdown-item">{{ hours }}</span>
                                            <span class="countdown-separator">:</span>
                                            <span class="countdown-item">{{ minutes }}</span>
                                            <span class="countdown-separator">:</span>
                                            <span class="countdown-item">{{ seconds }}</span>
                                        </div>
                                    </template>
                                </CountDown>
                            </div>
                            <!-- 更多链接：文字 + 箭头 -->
                            <NuxtLink
                                v-if="titleMore.showLink"
                                target="_blank"
                                :to="urlFormat(module.title?.moreLink)"
                                :style="{ color: module.title?.moreColor || '#1a1a1a' }"
                                class="seckill-more-link"
                            >
                                <span>{{ $t(moreLinkText) }}</span>
                                <SvgIcon class="icon" name="arrow-right" width="16" height="16" />
                            </NuxtLink>
                        </div>
                    </div>
                    <div class="seckill_list_wrap">
                        <!-- 使用 swiper 横向滑动布局（style 6） -->
                        <div v-if="seckillList.length > 0 && useSwiperLayout" class="seckill_list">
                            <!-- 左右上下边距与商品边距一样，使用容器的 padding 实现 -->
                            <!-- 注释：之前使用 CSS 变量 --goods-padding，现在改为容器 padding -->
                            <!-- :style="{ '--goods-padding': module?.goodsPadding ? module.goodsPadding + 'px' : '0' }" -->
                            <swiper :itemList="processedSeckillList" :swiperOption="seckillProductWrap" style="width: 100%">
                                <template v-slot:default="{ item }">
                                    <!-- 注释：swiper 布局中，商品项不需要 padding，间隔由 swiper 的 spaceBetween 和容器的 padding 控制 -->
                                    <!-- padding: module?.goodsPadding ? module.goodsPadding + 'px' : '0' -->
                                    <div class="proditem goods-ad-item" style="height: 100%">
                                        <NuxtLink
                                            :title="item.productName"
                                            :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.skuSn || item.productSn })"
                                            class=""
                                            target="_blank"
                                        >
                                            <div class="seckill-item-image-wrapper">
                                                <div class="seckill-item-image">
                                                    <el-image
                                                        :src="productImage(item)"
                                                        class="pro_pic"
                                                        loading="lazy"
                                                        style="transition: opacity 0.2s linear" :alt="item.productName || ''" 
                                                    />
                                                    <img
                                                        v-if="styleMap[item.productId]?.borderImage"
                                                        class="product-style-border"
                                                        :src="styleMap[item.productId].borderImage"
                                                        alt=""
                                                    />
                                                </div>
                                                <div class="seckill-item-discount" v-if="(item as any)?.marketPrice">
                                                    {{ $t("优惠") }}{{ getDiscount(Number(item.seckillPrice), Number((item as any).marketPrice)) }}%
                                                </div>
                                            </div>
                                            <template v-if="module?.showName == 1">
                                                <NuxtLink
                                                    :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.skuSn || item.productSn })"
                                                    target="_blank"
                                                >
                                                    <div
                                                        class="seckill-item__name"
                                                        :style="{
                                                            textAlign: module?.textAlign == 1 ? 'left' : 'center',
                                                            fontWeight: module?.textWeight == 1 ? 'normal' : 'bold',
                                                            padding: module.goodsNamePadding == 1 ? '0 10px' : ''
                                                        }"
                                                    >
                                                        {{ item.productName }}
                                                    </div>
                                                </NuxtLink>
                                            </template>
                                        </NuxtLink>
                                        <template v-if="module?.showPrice == 1">
                                            <div
                                                class="price-container"
                                                :style="{
                                                    flexDirection: module?.textAlign == 1 ? 'row' : 'column',
                                                    alignItems: module?.textAlign == 1 ? 'center' : 'center',
                                                    padding: module.goodsNamePadding == 1 ? '0 10px' : ''
                                                }"
                                            >
                                                <div class="seckill-item__price">
                                                    <FormatPrice v-model="item.seckillPrice" class="price price-miaosha" />
                                                    <template v-if="(item as any)?.marketPrice && Number((item as any).marketPrice) > 0">
                                                        <FormatPrice v-model="(item as any).marketPrice" class="price price-origin" />
                                                    </template>
                                                </div>
                                                <div class="buy-container">
                                                    <div class="buy" v-if="module?.buyBtnStyle > 0">
                                                        <div
                                                            v-if="(item as any)?.seckkillData?.seckillItem && (item as any).seckkillData.seckillItem.length > 1"
                                                        >
                                                            <NuxtLink
                                                                @click.stop
                                                                :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                                                target="_blank"
                                                            >
                                                                <ProductButton :buyBtnStyle="module.buyBtnStyle"></ProductButton>
                                                            </NuxtLink>
                                                        </div>
                                                        <div v-else>
                                                            <ProductBuy
                                                                @click.stop
                                                                :isQuick="false"
                                                                :id="item.productId"
                                                                :disabled="
                                                                    ((item as any)?.isSeckill == 0 && (item as any)?.productStock == 0) ||
                                                                    ((item as any)?.isSeckill == 1 && item?.seckillStock == 0)
                                                                "
                                                                :number="1"
                                                                @callback="$emit('callback')"
                                                            >
                                                                <ProductButton :buyBtnStyle="module.buyBtnStyle"></ProductButton>
                                                            </ProductBuy>
                                                        </div>
                                                    </div>
                                                    <div class="buy" v-else>
                                                        <div
                                                            v-if="(item as any)?.seckkillData?.seckillItem && (item as any).seckkillData.seckillItem.length > 1"
                                                        >
                                                            <NuxtLink
                                                                @click.stop
                                                                :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                                                target="_blank"
                                                            >
                                                                <div class="view_detail">
                                                                    <i class="iconfont-pc icon-tianjia"></i>
                                                                </div>
                                                            </NuxtLink>
                                                        </div>
                                                        <div v-else>
                                                            <ProductBuy
                                                                @click.stop
                                                                :isQuick="false"
                                                                :id="item.productId"
                                                                :disabled="
                                                                    ((item as any)?.isSeckill == 0 && (item as any)?.productStock == 0) ||
                                                                    ((item as any)?.isSeckill == 1 && item?.seckillStock == 0)
                                                                "
                                                                :number="1"
                                                                @callback="$emit('callback')"
                                                            >
                                                                <div class="view_detail">
                                                                    <i class="iconfont-pc icon-tianjia"></i>
                                                                </div>
                                                            </ProductBuy>
                                                        </div>
                                                    </div>
                                                </div>
                                            </div>
                                        </template>
                                    </div>
                                </template>
                            </swiper>
                        </div>
                        <!-- Flex 布局（style 1, 2） -->
                        <!-- :style="{ padding: module?.goodsPadding ? module.goodsPadding + 'px' : '0' }" -->
                        <div v-else-if="seckillList.length > 0" class="goods-ad-warp">
                            <!-- 左右上下边距与商品边距一样，使用容器的 padding 实现 -->
                            <!-- 商品中间间隔为边距的两倍，使用 gap 实现 -->
                            <div class="goods-ad-con" :style="{ gap: module?.goodsPadding ? module.goodsPadding * 2 + 'px' : '0' }">
                                <!-- 注释：之前商品项有 padding，现在改为容器 padding + gap 实现 -->
                                <!-- padding: module?.goodsPadding ? module.goodsPadding + 'px' : '0' -->
                                <div
                                    v-for="item in seckillList"
                                    :key="item.productId"
                                    class="proditem goods-ad-item"
                                    :style="{
                                        width: getItemWidth
                                    }"
                                >
                                    <div class="item-content">
                                        <div class="item-con">
                                            <div class="seckill-item-image-wrapper">
                                                <div class="item-photo seckill-item-image">
                                                    <NuxtLink
                                                        :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.skuSn || item.productSn })"
                                                        class="item-image-a"
                                                        target="_blank"
                                                    >
                                                        <el-image
                                                            :src="productImage(item)"
                                                            fit="fill"
                                                            class="item-image pro_pic" :alt="item.productName || ''" 
                                                        />
                                                        <img
                                                            v-if="styleMap[item.productId]?.borderImage"
                                                            class="product-style-border"
                                                            :src="styleMap[item.productId].borderImage"
                                                            alt=""
                                                        />
                                                    </NuxtLink>
                                                </div>
                                                <div class="seckill-item-discount" v-if="(item as any)?.marketPrice">
                                                    {{ $t("优惠") }}{{ getDiscount(Number(item.seckillPrice), Number((item as any).marketPrice)) }}%
                                                </div>
                                            </div>
                                            <div class="item-info" :style="{ padding: module.goodsNamePadding == 1 ? '0 10px' : '' }">
                                                <template v-if="module?.showName == 1">
                                                    <div class="item-name seckill-item__name">
                                                        <NuxtLink
                                                            :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.skuSn || item.productSn })"
                                                            class="item-name-a"
                                                            target="_blank"
                                                        >
                                                            {{ item.productName }}
                                                        </NuxtLink>
                                                    </div>
                                                </template>
                                                <template v-if="module?.showPrice == 1">
                                                    <div
                                                        class="price-container"
                                                        :style="{
                                                            flexDirection: module?.textAlign == 1 ? 'row' : 'column',
                                                            alignItems: module?.textAlign == 1 ? 'center' : 'center'
                                                        }"
                                                    >
                                                        <div class="item-action seckill-item__price">
                                                            <div class="item-price">
                                                                <FormatPrice v-model="item.seckillPrice" class="price price-miaosha" />
                                                            </div>
                                                            <template v-if="(item as any)?.marketPrice && Number((item as any).marketPrice) > 0">
                                                                <FormatPrice v-model="(item as any).marketPrice" class="price price-origin" />
                                                            </template>
                                                        </div>
                                                        <div class="buy-container">
                                                            <div v-if="module?.buyBtnStyle > 0" class="item-buy">
                                                                <div
                                                                    v-if="
                                                                        (item as any)?.seckkillData?.seckillItem &&
                                                                        (item as any).seckkillData.seckillItem.length > 1
                                                                    "
                                                                >
                                                                    <NuxtLink
                                                                        :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                                                        target="_blank"
                                                                    >
                                                                        <ProductButton :buyBtnStyle="module.buyBtnStyle"></ProductButton>
                                                                    </NuxtLink>
                                                                </div>
                                                                <template v-else>
                                                                    <ProductBuy
                                                                        :isQuick="false"
                                                                        :id="item.productId"
                                                                        :disabled="
                                                                            ((item as any)?.isSeckill == 0 && (item as any)?.productStock == 0) ||
                                                                            ((item as any)?.isSeckill == 1 && item?.seckillStock == 0)
                                                                        "
                                                                        :number="1"
                                                                    >
                                                                        <ProductButton :buyBtnStyle="module.buyBtnStyle"></ProductButton>
                                                                    </ProductBuy>
                                                                </template>
                                                            </div>
                                                            <div v-else class="item-buy buy">
                                                                <div
                                                                    v-if="
                                                                        (item as any)?.seckkillData?.seckillItem &&
                                                                        (item as any).seckkillData.seckillItem.length > 1
                                                                    "
                                                                >
                                                                    <NuxtLink
                                                                        @click.stop
                                                                        :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                                                        target="_blank"
                                                                    >
                                                                        <div class="view_detail">
                                                                            <i class="iconfont-pc icon-tianjia"></i>
                                                                        </div>
                                                                    </NuxtLink>
                                                                </div>
                                                                <div v-else>
                                                                    <ProductBuy
                                                                        @click.stop
                                                                        :isQuick="false"
                                                                        :id="item.productId"
                                                                        :disabled="
                                                                            ((item as any)?.isSeckill == 0 && (item as any)?.productStock == 0) ||
                                                                            ((item as any)?.isSeckill == 1 && item?.seckillStock == 0)
                                                                        "
                                                                        :number="1"
                                                                        @callback="$emit('callback')"
                                                                    >
                                                                        <div class="view_detail">
                                                                            <i class="iconfont-pc icon-tianjia"></i>
                                                                        </div>
                                                                    </ProductBuy>
                                                                </div>
                                                            </div>
                                                        </div>
                                                    </div>
                                                </template>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                        <!-- 加载状态 -->
                        <div v-else class="seckill_list">
                            <div v-for="index in 5" class="proditem" style="width: 200px">
                                <div class="seckill-item-image">
                                    <HomeSkeleton></HomeSkeleton>
                                </div>
                                <div class="seckill-item__name" style="height: 30px">
                                    <HomeSkeleton></HomeSkeleton>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * PC端装修秒杀商品模块
 * - 展示限时秒杀活动商品列表，带倒计时功能
 * - 支持多种布局样式：style 1/2 为 Flex 布局，style 6 为 Swiper 横向滑动
 * - 显示商品优惠折扣百分比
 * - 商品数量不足时自动复制列表以支持无限轮播
 * - 支持快速购买（单 SKU 商品）和商品详情跳转（多 SKU 商品）
 */
import { ref, computed, onMounted } from "vue";
import { getHomeSeckill } from "@/api/home/home";
import type { SeckillFilterState } from "~/types/home/seckill";
import CountDown from "~/components/common/CountDown.vue";
import { imageFormat, productImage, urlFormat } from "~/utils/format";
import { resolveCountdownLabel, resolveMoreLinkTitle, resolveTitleMore } from "~/decorate/titleMore";
import { useProductStyle } from "~/hooks";
import ProductBuy from "~/components/product/Buy.vue";
import HomeSkeleton from "~/components/home/Skeleton.vue";
import ProductButton from "~/components/shop/product/ProductButton.vue";
import type { Box } from "@element-plus/icons-vue";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

const { frame } = props.module;

// 格式化 frame 配置
const frameFormat = computed(() => {
    const frameData = frame ?? {};
    return {
        boxPadding: `padding-left: ${frameData.boxPadding || 0}px; padding-right: ${frameData.boxPadding || 0}px;`,
        boxPaddingTop: `padding-top: ${frameData.boxPaddingTop || 0}px;`,
        boxPaddingBottom: `padding-bottom: ${frameData.boxPaddingBottom || 0}px;`,
        backgroundColor: frameData.backgroundColor ? `background-color: ${frameData.backgroundColor};` : "",
        boxRadius: frameData.boxRadius ? `border-radius: ${frameData.boxRadius}px;` : ""
    };
});

// 格式化 all 配置
const allFormat = computed(() => {
    return {
        backgroundColor: props.module.backgroundColor ? `background-color: ${props.module.backgroundColor};` : "",
        boxPadding: `margin-left: ${props.module.boxPadding || 0}px; margin-right: ${props.module.boxPadding || 0}px;`,
        boxPaddingBottom: `margin-bottom: ${props.module.boxPaddingBottom || 0}px;`,
        boxPaddingTop: `margin-top: ${props.module.boxPaddingTop || 0}px;`,
        boxRadius: props.module.boxRadius ? `border-radius: ${props.module.boxRadius}px;` : "",
        innerPadding: props.module.innerPadding ? `padding: ${props.module.innerPadding}px;` : "",
        goodsPadding: props.module.goodsPadding ? `padding: ${props.module.goodsPadding}px;` : ""
    };
});

const getDiscount = (currentPrice: number, originalPrice: number) => {
    if (originalPrice === 0 || currentPrice >= originalPrice) {
        return 0;
    }

    const discountPercentage = ((originalPrice - currentPrice) / originalPrice) * 100;

    return Math.floor(discountPercentage * 10) / 10;
};

// 判断是否使用 swiper 布局
const useSwiperLayout = computed(() => {
    const style = typeof props.module?.style === "string" ? Number(props.module.style) : props.module?.style;
    // style 6 使用 swiper 横向滑动，style 1 和 2 使用 flex 布局
    return style === 6;
});

// 处理商品列表，当商品数量不足时复制列表以支持无限轮播
const processedSeckillList = computed(() => {
    const style = typeof props.module?.style === "string" ? Number(props.module.style) : props.module?.style;
    if (style !== 6 || !useSwiperLayout.value) {
        // 非 swiper 布局，直接返回原列表
        return seckillList.value;
    }

    const slidesPerView = 5;
    const minSlidesForLoop = slidesPerView * 2;

    // 如果商品数量足够，直接返回
    if (seckillList.value.length >= minSlidesForLoop) {
        return seckillList.value;
    }

    // 如果商品数量不足，复制列表以满足 loop 的要求
    // 复制足够的次数以确保至少有 minSlidesForLoop 个商品
    const timesToDuplicate = Math.ceil(minSlidesForLoop / seckillList.value.length);
    const duplicatedList: SeckillFilterState[] = [];
    for (let i = 0; i < timesToDuplicate; i++) {
        duplicatedList.push(...seckillList.value);
    }

    return duplicatedList;
});

// 根据 style 获取 swiper 配置
const getSwiperConfig = computed(() => {
    const style = typeof props.module?.style === "string" ? Number(props.module.style) : props.module?.style;
    // style 6 使用横向滑动，显示5个商品，样式与 style 2 保持一致
    const slidesPerView = 5;
    // 商品中间间隔为边距的两倍
    const spaceBetween = props.module?.goodsPadding ? Number(props.module.goodsPadding) * 2 : 0;

    // 使用处理后的商品列表来判断是否可以启用 loop
    // processedSeckillList 已经处理了商品数量不足的情况，确保有足够的商品
    const canLoop = style === 6 && processedSeckillList.value.length >= slidesPerView * 2;

    return {
        autoplay: {
            delay: 5000,
            disableOnInteraction: false,
            pauseOnMouseEnter: true
        },
        slidesPerGroup: 1, // 每次滑动1个商品
        slidesPerView: slidesPerView,
        loopFillGroupWithBlank: true,
        spaceBetween: spaceBetween, // 间隔由商品项的 padding 控制
        pagination: false,
        navigation: true,
        effect: "slide",
        loop: canLoop,
        watchOverflow: false, // 取消根据商品数量自动禁用按钮的判断
        // 当启用 loop 时，添加额外的 slides 以确保无限轮播流畅
        loopAdditionalSlides: canLoop ? slidesPerView : 0
    };
});

const seckillProductWrap = computed(() => getSwiperConfig.value);

// 获取商品项宽度（非 swiper 布局时使用，仅 style 1 和 2）
const getItemWidth = computed(() => {
    const style = typeof props.module?.style === "string" ? Number(props.module.style) : props.module?.style;
    // 商品中间间隔为边距的两倍，使用 gap 实现
    // 左右上下边距与商品边距一样，使用 padding 实现
    // gap 是商品之间的间隔，需要在宽度计算中减去相应的 gap
    const gap = props.module?.goodsPadding ? Number(props.module.goodsPadding) * 2 : 0; // gap 是边距的两倍

    switch (style) {
        case 1: {
            // 一行4个，每个25%
            // 4个商品之间有3个间隔，每个间隔 = gap (2 * goodsPadding)
            // 每个商品的宽度 = (100% - 3 * gap) / 4 = 25% - (3 * gap) / 4
            if (gap > 0) {
                const totalGap = (gap * 3) / 4;
                return `calc(25% - ${totalGap}px)`;
            }
            return "25%";
        }
        case 2: {
            // 一行5个，每个20%
            // 5个商品之间有4个间隔，每个间隔 = gap (2 * goodsPadding)
            // 每个商品的宽度 = (100% - 4 * gap) / 5 = 20% - (4 * gap) / 5
            if (gap > 0) {
                const totalGap = (gap * 4) / 5;
                return `calc(20% - ${totalGap}px)`;
            }
            return "20%";
        }
        case 6:
            return "248px"; // style 6 使用 swiper 布局，不会使用此宽度
        default:
            return "25%"; // 默认一行4个
    }
});

const seckillList = ref<SeckillFilterState[]>([]);
const { styleMap } = useProductStyle(seckillList);
const getData = async () => {
    try {
        const result = await getHomeSeckill();
        seckillList.value = result.records;
        console.log(result);
    } catch (error) {
        console.error(error);
    }
};

onMounted(() => {
    getData();
});

const getTime = computed(() => {
    if (seckillList.value && seckillList.value.length > 0) {
        const firstItem = seckillList.value[0] as any;
        if (firstItem?.seckkillData?.seckillEndTime) {
            return new Date(firstItem.seckkillData.seckillEndTime).getTime();
        }
        if (firstItem?.seckillEndTime) {
            return new Date(firstItem.seckillEndTime).getTime();
        }
    }
    return 0;
});

// ==================== 标题右侧区（更多链接 / 倒计时） ====================
const titleMore = computed(() => resolveTitleMore(props.module?.title, "seckillCustom"));
const countdownLabel = computed(() => resolveCountdownLabel(props.module?.title, "距结束"));
const moreLinkText = computed(() => resolveMoreLinkTitle(props.module?.title, "查看更多"));
</script>
<style lang="less" scoped>
// ========== 基础样式 ==========
.module-seckill-goods_ad-wrapper {
    box-sizing: border-box;
}

// 公共样式：图片过渡效果
.pro_pic,
.item-image {
    transition:
        transform 0.4s ease,
        -webkit-transform 0.4s ease,
        -moz-transform 0.4s ease,
        -o-transform 0.4s ease;
}
// ========== 秒杀模块 ==========
.mod_seckill {
    display: flex;
    flex-direction: column;
    width: 100%;
    border-radius: 8px;
    overflow: hidden;

    // 商品列表容器
    // 顶部区域（标题和倒计时）
    .seckill-top {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 10px 0;
        box-sizing: border-box;
        margin-bottom: 10px;
        .seckill-title-wrapper {
            display: flex;
            align-items: baseline;
            gap: 4px;
            .line {
                width: 4px;
                height: 16px;
                background-color: #fff;
                border-radius: 8px;
            }
            .image-size {
                width: 30px;
                height: 30px;
                margin-left: 10px;
            }
            .seckill-title {
                font-size: 16px;
                font-weight: bold;
                color: #1a1a1a;
                margin-right: 5px;
            }
            .seckill-desc {
                font-size: 12px;
                color: #1a1a1a;
                &.big-sm {
                    font-size: 16px;
                }
            }
        }
        .seckill-top-right {
            display: flex;
            align-items: center;
            gap: 12px;
        }
        // 文字颜色由 title.moreColor 内联下发，子元素继承
        .seckill-count-down {
            display: flex;
            align-items: center;
            font-size: 18px;
            color: #1a1a1a;
        }
        .seckill-more-link {
            display: flex;
            align-items: center;
            font-size: 14px;
            color: #1a1a1a;
            text-decoration: none;
            cursor: pointer;

            .icon {
                margin-left: 4px;
                color: inherit;
            }

            &:hover {
                color: var(--main-bg) !important;
            }
        }
    }

    // 商品列表容器
    .seckill_list_wrap {
        position: relative;
        z-index: 1;
        flex: 1;
        height: 100%;
        //padding: 0 10px;
        padding-bottom: 10px;
    }

    .seckill_list {
        position: relative;
        z-index: 1;
        height: 100%;
        overflow: hidden;
        width: 100%;

        :deep(.swiper) {
            width: 100%;
            padding: 0;
        }

        :deep(.swiper-slide) {
            // 宽度由 swiper 根据 slidesPerView 和 spaceBetween 自动计算
            flex-shrink: 0;
            height: auto;
            box-sizing: border-box;
        }

        .proditem {
            box-sizing: border-box;
            position: relative;
            z-index: 1;
            width: 100%; // 宽度填满 slide
            background-color: #ffffff;

            a {
                display: block;
            }

            .seckill-item-image {
                aspect-ratio: 1 / 1;
            }

            .seckill-item-discount {
                margin: 5px;
                width: fit-content;
                height: 20px;
                line-height: 20px;
                color: #fff;
                text-align: center;
                border-radius: 2px;
                background-color: #ee0a24;
                padding: 0px 6px;
                font-size: 14px;
                box-sizing: border-box;
            }

            &:hover {
                .seckill-item-image {
                    display: flex;
                    justify-content: center;
                    overflow: hidden;

                    .pro_pic {
                        filter: brightness(105%);
                    }
                }
            }
        }

        .pro_pic {
            width: 100%;
            height: 100%;
            object-fit: cover;
        }

        .seckill-item__name {
            height: 20px;
            margin-bottom: 12px;
            overflow: hidden;
            white-space: nowrap;
            text-overflow: ellipsis;
            color: #1a1a1a;
            font-size: 16px;
            font-weight: 400;
            line-height: 18px;
            cursor: pointer;
            padding: 0 5px;
            &:hover {
                color: #ee0a24;
            }
        }

        .seckill-item__price {
            display: flex;
            align-items: flex-start;

            .buy {
                margin-left: auto;

                .view_detail {
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    width: 32px;
                    height: 32px;
                    border-radius: 50%;
                    background: var(--primary-light-9);
                    color: #ee0a24;
                    cursor: pointer;

                    &:hover {
                        color: var(--primary-dark-2);
                    }

                    &.disabled {
                        opacity: 0.5;
                        cursor: not-allowed;
                    }
                }
            }
        }
    }
}

// 倒计时盒子（文字颜色继承外层，由 title.moreColor 控制）
.countdown-box {
    display: flex;
    align-items: center;
    .countdown-separator-text {
        font-size: 18px;
        color: inherit;
    }

    .countdown-separator {
        color: inherit;
        margin: 0 1px;
    }

    .countdown-text {
        font-size: 18px;
        font-weight: 400;
        line-height: 24px;
        color: inherit;
        text-transform: capitalize;
    }

    .countdown-item {
        display: inline-block;
        padding: 2px 0;
        width: 32px;
        text-align: center;
        background-color: #1a1a1a;
        color: #fff;
        border-radius: 4px;

        &.days {
            width: auto;
            min-width: 32px;
            padding: 0 5px;
        }
    }

    .iconfont-pc {
        color: inherit;
        font-size: 14px;
        padding-left: 4px;
    }

    span {
        margin: 0 1px;
    }
}

// 商品图片
.seckill-item-image {
    width: 100%;
    display: flex;
    justify-content: center;
    overflow: hidden;
    background-color: transparent;
    aspect-ratio: 1 / 1;
    position: relative;
}

.product-style-border {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    z-index: 5;
    pointer-events: none;
    object-fit: fill;
}

// 商品折扣标签
.seckill-item-discount {
    margin: 5px;
    width: fit-content;
    height: 20px;
    line-height: 16px;
    color: #fff;
    text-align: center;
    border-radius: 2px;
    background-color: #ee0a24;
    padding: 0px 6px;
    font-size: 14px;
    box-sizing: border-box;
}

// 商品名称
.seckill-item__name {
    height: 20px;
    margin-bottom: 12px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: #1a1a1a;
    font-size: 16px;
    font-weight: 400;
    line-height: 18px;
    cursor: pointer;
    padding: 0 5px;

    &:hover {
        color: #ee0a24;
    }
}

.price-container {
    display: flex;
    align-items: center;
    margin-top: 10px;
    padding: 0 5px;
    margin-bottom: 10px;
}

.seckill-item__price .price-miaosha {
    align-items: baseline;
    justify-content: center;
    margin-right: 8px;
    color: #1a1a1a;

    :deep(.price-row) {
        align-items: baseline !important;
    }

    :deep(.num) {
        font-size: 24px;
        color: #1a1a1a;
        font-weight: 600;
    }

    :deep(.util),
    :deep(.decimal) {
        color: #1a1a1a;
        font-size: 16px;
    }
}

.seckill-item__price .price-origin {
    align-items: baseline;
    justify-content: center;
    height: 19px;
    color: #888b94 !important;

    :deep(.price-row) {
        line-height: 22px;
        font-weight: 500;
    }

    :deep(.num),
    :deep(.util) {
        color: #888b94 !important;
    }

    :deep(.num) {
        font-size: 14px;
    }
}

.price-origin {
    box-sizing: border-box;
}

:deep(.swiper) {
    box-sizing: border-box;
    // padding: 0 12px;
}

:deep(.swiper-wrapper) {
    display: flex;
}

.module-seckill-goods_ad {
    :deep(.swiper-button-prev),
    :deep(.swiper-button-next) {
        box-sizing: border-box;
        background-color: rgba(0, 0, 0, 0.122);
        width: 40px;
        height: 48px;
        line-height: 35px;
        z-index: 2;
        outline: none;
        transition: background-color 0.2s ease;

        &:hover {
            background-color: rgba(0, 0, 0, 0.239);
        }

        &:after {
            font-family: "iconfont-pc";
            content: "\e70d";
            color: #fff;
            font-size: 14px;
        }

        &.swiper-button-disabled {
            pointer-events: fill;
        }
    }

    :deep(.swiper-button-prev) {
        left: 0;
        padding-right: 7px;
        border-radius: 0 24px 24px 0 !important;

        &:after {
            transform: rotate(180deg);
        }
    }

    :deep(.swiper-button-next) {
        right: 0;
        padding-left: 7px;
        border-radius: 24px 0 0 24px !important;
    }
}

// ========== 商品广告样式 ==========
.cap-seckill-goods__tag {
    display: flex;
    align-items: center;
    justify-content: space-between;

    &.flex-end {
        justify-content: flex-end;
    }
}

.module-goods_ad {
    .module-ad-content {
        overflow: hidden;
        z-index: 1;
        position: relative;
    }
}

.seckill-item-image-wrapper {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
}
.goods-ad-warp {
    .goods-ad-con {
        display: flex;
        flex-wrap: wrap;
    }

    .goods-ad-item {
        /* width 由内联样式或具体样式类控制，不在此处设置固定值 */
        position: relative;
        overflow: hidden;
        display: inline-block;
        transition: opacity 0.4s ease-in-out;
        box-sizing: border-box;
        background-color: #ffffff;

        .item-content {
            background-color: transparent;
        }

        .item-con {
            background-color: #ffffff;
        }

        .item-photo.seckill-item-image {
            background: #ffffff;
            text-align: center;
            position: relative;
            width: 100%;
            display: flex;
            justify-content: center;
            overflow: hidden;
            aspect-ratio: 1 / 1;
            background-color: transparent;

            .item-image,
            .item-image.pro_pic {
                height: 100%;
                width: 100%;
                object-fit: cover;
            }
        }

        .seckill-item-discount {
            margin: 12px 5px;
            width: fit-content;
            height: 20px;
            line-height: 20px;
            color: #fff;
            text-align: center;
            border-radius: 2px;
            background-color: var(--main-bg);
            padding: 0px 6px;
            font-size: 14px;
        }

        .item-info {
            background: #ffffff;
            position: relative;
            padding: 5px;

            .item-name.seckill-item__name {
                margin: 0;
                margin-top: 0;
                display: block;

                .item-name-a {
                    line-height: 18px;
                    height: 20px;
                    overflow: hidden;
                    display: block;
                }
            }

            .item-action.seckill-item__price {
                margin-top: 0;
                vertical-align: middle;
                padding: 0;
                position: relative;
                display: flex;
                justify-content: space-between;
                align-items: center;

                + .price-origin {
                    text-decoration: line-through;
                }
            }

            .price-container {
                display: flex;
                align-items: center;
            }

            .item-buy {
                justify-content: center;
                height: 30px;
                display: flex;
                align-items: center;

                .buy-btn {
                    div {
                        font-size: 12px;
                    }
                }

                .iconfont-h5 {
                    color: var(--general);
                    font-size: 18px;

                    &::before {
                        content: "\e611";
                    }
                }
            }
        }

        &:hover {
            .item-photo.seckill-item-image,
            .seckill-item-image {
                display: flex;
                justify-content: center;
                overflow: hidden;

                .pro_pic,
                .item-image {
                    filter: brightness(105%);
                }
            }
        }
    }
}

.seckill-item-image-wrapper {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
}

.goods-ad-warp .goods-ad-item .item-info {
    .item-name.seckill-item__name {
        margin: 0;
        margin-top: 0;
        display: block;

        .item-name-a {
            line-height: 18px;
            height: 20px;
            overflow: hidden;
            display: block;
        }
    }

    .item-name {
        .item-brief {
            line-height: 20px;
            height: 20px;
            overflow: hidden;
            display: block;
            color: #aaa;
            text-overflow: ellipsis;
            display: -webkit-box;
            display: -moz-box;
            -webkit-line-clamp: 1;
            -moz-line-clamp: 1;
            line-clamp: 1;
            -webkit-box-orient: vertical;
            -moz-box-orient: vertical;
            font-size: 12px;
        }
    }

    .item-action.seckill-item__price {
        .price-miaosha {
            align-items: baseline;
            justify-content: center;
            margin-right: 8px;
            color: #1a1a1a;

            :deep(.price-row) {
                align-items: baseline !important;
            }

            :deep(.num) {
                font-size: 24px;
                color: #1a1a1a;
                font-weight: 600;
            }

            :deep(.util),
            :deep(.decimal) {
                color: #1a1a1a;
                font-size: 16px;
            }
        }

        + .price-origin {
            align-items: baseline;
            justify-content: center;
            height: 19px;
            color: #888b94;

            :deep(.price-row) {
                line-height: 22px;
                font-weight: 500;
            }

            :deep(.num) {
                font-size: 16px;
                color: #888b94;
            }

            :deep(.util) {
                color: #888b94;
            }
        }
    }
}
.goods-ad-warp .goods-ad-item .item-info .item-buy {
    justify-content: center;
    height: 30px;
    display: flex;
    align-items: center;

    .buy-btn {
        div {
            font-size: 12px;
        }
    }

    .iconfont-h5 {
        color: var(--general);
        font-size: 18px;

        &::before {
            content: "\e611";
        }
    }
}
.ad-text_align__1 {
    .goods-ad-warp .goods-ad-item .item-info .price-container {
        flex-direction: row;
        justify-content: space-between;
        align-items: flex-start;

        .item-action.seckill-item__price {
            flex-direction: column;
            align-items: flex-start;
        }

        .buy-container {
            margin-left: auto;
            align-self: flex-start;
        }
    }

    .mod_seckill .seckill_list .price-container {
        flex-direction: row;
        justify-content: space-between;
        align-items: flex-start;

        .seckill-item__price {
            flex-direction: column;
            align-items: flex-start;
        }

        .buy-container {
            margin-left: auto;
            align-self: flex-start;
        }
    }
}

.ad-text_align__2 {
    .goods-ad-warp .goods-ad-item .item-info .price-container {
        flex-direction: column;
        align-items: center;

        .item-action.seckill-item__price {
            flex-direction: column;
            align-items: center;

            .item-price {
                align-self: center;
            }

            .price-origin {
                align-self: center;
            }
        }

        .buy-container {
            margin-top: 8px;
            display: flex;
            justify-content: center;
        }
    }

    .mod_seckill .seckill_list .price-container {
        flex-direction: column;
        align-items: center;

        .seckill-item__price {
            flex-direction: column;
            align-items: center;

            .price-miaosha {
                align-self: center;
            }

            .price-origin {
                align-self: center;
            }
        }

        .buy-container {
            margin-top: 8px;
            display: flex;
            justify-content: center;
        }
    }
}

.ad-goods_name_row__2 {
    .mod_seckill .seckill_list .seckill-item__name {
        height: 40px;
        white-space: normal;
        display: -webkit-box;
        display: -moz-box;
        -webkit-line-clamp: 2;
        -moz-line-clamp: 2;
        line-clamp: 2;
        -webkit-box-orient: vertical;
        -moz-box-orient: vertical;
        line-height: 20px;
    }

    .goods-ad-warp .goods-ad-item .item-info .item-name.seckill-item__name {
        height: 40px;
        white-space: normal;
        display: -webkit-box;
        display: -moz-box;
        -webkit-line-clamp: 2;
        -moz-line-clamp: 2;
        line-clamp: 2;
        -webkit-box-orient: vertical;
        -moz-box-orient: vertical;
        line-height: 20px;

        .item-name-a {
            height: 40px;
            white-space: normal;
            display: -webkit-box;
            display: -moz-box;
            -webkit-line-clamp: 2;
            -moz-line-clamp: 2;
            line-clamp: 2;
            -webkit-box-orient: vertical;
            -moz-box-orient: vertical;
            line-height: 20px;
            overflow: hidden;
        }
    }
}
// ========== 广告样式变体 ==========
.ad-style__1 {
    .goods-ad-warp {
        .goods-ad-con {
            display: flex;
            flex-wrap: wrap;
        }

        .goods-ad-item .item-info .item-name:not(.seckill-item__name) .item-name-a {
            font-size: 14px;
        }
    }
    /* .ad-style__1 的宽度由内联样式动态计算，考虑gap */
}

.ad-style__2 {
    .goods-ad-warp {
        .goods-ad-con {
            display: flex;
            flex-wrap: wrap;
        }

        .goods-ad-item .item-info .item-name:not(.seckill-item__name) .item-name-a {
            font-size: 12px;
            font-weight: bold;
        }
    }
    /* .ad-style__2 的宽度由内联样式动态计算，考虑gap */
}

.ad-style__3 {
    .goods-ad-warp .goods-ad-item .item-info .item-buy {
        display: none;
    }
}

.ad-style__4 {
    .goods-ad-warp {
        .goods-ad-con {
            display: flex;
            flex-wrap: wrap;

            .goods-ad-item {
                width: 100%;

                &:nth-child(3n + 2),
                &:nth-child(3n + 3) {
                    width: 50%;
                }
            }
        }
    }
}

.ad-style__6 {
    .goods-ad-warp {
        .goods-ad-con {
            display: flex;
            flex-wrap: wrap;

            .goods-ad-item {
                width: 100%;
            }
        }

        .goods-ad-item {
            .item-content {
                display: flex;
                flex-wrap: nowrap;

                .item-con {
                    display: flex;
                    flex-wrap: nowrap;
                    flex: 1;
                }
            }

            .item-photo {
                width: 40%;
            }

            .item-info {
                width: 60%;
                text-align: left;

                .item-name:not(.seckill-item__name) .item-name-a {
                    font-size: 12px;
                    font-weight: bold;
                }
            }

            .item-action {
                position: absolute;
                bottom: 0;
                left: 0;
                right: 0;
                padding-bottom: 0;
            }
        }
    }
    /* style 6 使用 swiper 横向滑动，不需要在 flex 布局中定义宽度 */
}

.ad-style__7 {
    .goods-ad-warp {
        overflow: hidden;
        padding-bottom: 25px;

        .goods-ad-con {
            display: block;
            flex-wrap: nowrap;
            padding: 0 !important;
            height: 100%;

            .goods-ad-item {
                width: 33.333%;
            }
        }

        .goods-ad-item .item-info .item-buy {
            display: none;
        }

        swiper {
            height: 200px;
            position: relative;
            overflow: hidden;
        }

        .swiper-pagination-con {
            position: absolute;
            bottom: 15px;
            display: flex;
            justify-content: center;
            align-items: center;
            width: 100%;
        }

        .swiper-pagination {
            display: inline-block;
            width: 12px;
            border-radius: 0;
            height: 2px;
            margin: 0 1px;
            background: #000;
            opacity: 0.2;

            &.active {
                opacity: 1;
                background-color: #fff;
            }
        }
    }
}
.ad-style__7 .goods-ad-warp .swiper-pagination-con {
    position: absolute;
    bottom: 15px;
    display: flex;
    justify-content: center;
    align-items: center;
    width: 100%;
}
.ad-style__7 .goods-ad-warp .swiper-pagination {
    display: inline-block;
    width: 12px;
    border-radius: 0;
    height: 2px;
    margin: 0 1px;
    background: #000;
    opacity: 0.2;
}
.ad-style__7 .goods-ad-warp .swiper-pagination.active {
    opacity: 1;
    background-color: #fff;
}
.ad-buy_btn_style__0 .goods-ad-warp .goods-ad-item .item-info .item-buy {
    display: none;
}
.ad-buy_btn_style__1 .goods-ad-warp .goods-ad-item .item-info .item-buy .iconfont-h5::before {
    content: "\e611";
    font-size: 18px;
}
.ad-buy_btn_style__2 .goods-ad-warp .goods-ad-item .item-info .item-buy .iconfont-h5::before {
    content: "\e664";
    font-size: 20px;
}
.ad-buy_btn_style__3 .goods-ad-warp .goods-ad-item .item-info .item-buy .iconfont-h5::before {
    content: "\e74a";
    font-size: 18px;
}
.ad-buy_btn_style__4 .goods-ad-warp .goods-ad-item .item-info .item-buy .iconfont-h5::before {
    content: "\e620";
    font-size: 18px;
}
.ad-buy_btn_style__5 .goods-ad-warp .goods-ad-item .item-info .item-buy .buy-btn div {
    color: var(--general);
    height: 22px;
    line-height: 22px;
    padding: 0 8px;
    display: inline-block;
    position: relative;
    border-radius: 2px;
    border: 0 solid var(--general);
}
.ad-buy_btn_style__5 .goods-ad-warp .goods-ad-item .item-info .item-buy .buy-btn div:before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    box-sizing: border-box;
    width: 200%;
    height: 200%;
    transform: scale(0.5);
    transform-origin: 0 0;
    pointer-events: none;
    border: 1px solid #f44;
    border-radius: 2px;
}
.ad-buy_btn_style__6 .goods-ad-warp .goods-ad-item .item-info .item-buy .buy-btn div {
    background: var(--general);
    color: #fff;
    height: 22px;
    line-height: 22px;
    padding: 0 10px;
    display: inline-block;
    position: relative;
    border-radius: 22px;
}
.ad-buy_btn_style__7 .goods-ad-warp .goods-ad-item .item-info .item-buy .buy-btn div {
    color: var(--general);
    height: 22px;
    line-height: 22px;
    padding: 0 8px;
    display: inline-block;
    position: relative;
    border-radius: 2px;
    border: 0 solid var(--general);
}
.ad-buy_btn_style__7 .goods-ad-warp .goods-ad-item .item-info .item-buy .buy-btn div:before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    box-sizing: border-box;
    width: 200%;
    height: 200%;
    transform: scale(0.5);
    transform-origin: 0 0;
    pointer-events: none;
    border: 1px solid #f44;
    border-radius: 2px;
}
.ad-buy_btn_style__8 .goods-ad-warp .goods-ad-item .item-info .item-buy .buy-btn div {
    background: var(--general);
    color: #fff;
    height: 22px;
    line-height: 22px;
    padding: 0 10px;
    display: inline-block;
    position: relative;
    border-radius: 22px;
}
// ========== 商品样式变体 ==========
.ad-goods_style__2 {
    .goods-ad-warp {
        .goods-ad-con {
            .goods-ad-item {
                box-shadow: 0 2px 4px rgba(0, 0, 0, 0.06) !important;
            }
        }
    }
    .mod_seckill .seckill_list .proditem {
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.06) !important;
    }
}

.ad-goods_style__3 {
    .goods-ad-warp {
        .goods-ad-con {
            .goods-ad-item {
                border: 1px solid rgba(50, 50, 51, 0.1) !important;
            }
        }
    }
    .mod_seckill .seckill_list .proditem {
        border: 1px solid rgba(50, 50, 51, 0.1) !important;
    }
}

.ad-goods_style__4 {
    .mod_seckill .seckill_list .seckill-item__name {
        padding: 0 !important;
    }
    .mod_seckill .seckill_list .price-container {
        padding: 0 !important;
    }
    .goods-ad-warp {
        .goods-ad-item {
            background: transparent !important;
            .item-content,
            .item-con,
            .item-info,
            .item-photo,
            .item-photo.seckill-item-image {
                background: transparent !important;
            }
            .seckill-item-discount {
                margin: 5px 0px !important;
            }
            .item-info {
                padding: 0 !important;
            }
        }
    }
    .proditem {
        background: transparent !important;
        .item-content,
        .item-con,
        .item-info,
        .item-photo,
        .item-photo.seckill-item-image {
            background: transparent !important;
        }
        .seckill-item-discount {
            margin: 5px 0px !important;
        }
        .item-info {
            padding: 0 !important;
        }
    }
}
// ========== 商品圆角样式 ==========
.ad-goods_radio_style__2 {
    .goods-ad-item {
        border-radius: 6px;
        overflow: hidden;
    }

    .goods-ad-warp {
        .goods-ad-con {
            .goods-ad-item {
                .item-con {
                    border-radius: 6px;

                    .item-photo {
                        border-radius: 6px 6px 0 0;
                        overflow: hidden;
                    }

                    .item-info {
                        border-radius: 0 0 6px 6px;
                    }
                }
            }
        }
    }
}
// ========== 文本对齐样式 ==========
.ad-text_align__2 {
    .goods-ad-warp {
        .goods-ad-con {
            .goods-ad-item {
                text-align: center;
            }
        }

        .goods-ad-item {
            .item-info {
                .item-buy {
                    height: auto;
                    padding-bottom: 15px;
                }

                .item-action {
                    flex-wrap: wrap;
                    justify-content: normal;

                    .item-price,
                    .item-buy {
                        width: 100%;
                    }
                }
            }
        }
    }
}

// ========== 文本粗细样式 ==========
.ad-text_weight__2 {
    .goods-ad-warp {
        .goods-ad-item {
            .item-info {
                .item-name {
                    .item-name-a {
                        font-weight: bold;
                    }
                }
            }
        }
    }
}

.item-name {
    .item-name-a {
        line-height: 20px;
        height: 20px;
        text-overflow: ellipsis;
        display: -webkit-box;
        display: -moz-box;
        -webkit-line-clamp: 1;
        -moz-line-clamp: 1;
        line-clamp: 1;
        -webkit-box-orient: vertical;
        -moz-box-orient: vertical;
    }
}

// ========== 商品名称行数 ==========
.ad-goods_name_row__1 {
    .goods-ad-warp {
        .goods-ad-item {
            .item-info {
                .item-name {
                    .item-name-a {
                        line-height: 20px;
                        height: 20px;
                        text-overflow: ellipsis;
                        display: -webkit-box;
                        display: -moz-box;
                        -webkit-line-clamp: 1;
                        -moz-line-clamp: 1;
                        line-clamp: 1;
                        -webkit-box-orient: vertical;
                        -moz-box-orient: vertical;
                    }
                }
            }
        }
    }
}
// ========== 商品名称内边距 ==========
.ad-goods_name_padding__0 {
    .goods-ad-warp {
        .goods-ad-item {
            .item-info {
                .item-name {
                    margin-left: 0;
                    margin-right: 0;
                }

                .item-action {
                    padding-left: 0;
                    padding-right: 0;
                }
            }
        }
    }
}

// ========== 商品标题样式 ==========
.ad-goods-title_style__3 {
    .image-ad-title {
        padding-bottom: 28px !important;
    }
    .goods-ad-warp {
        border-top-left-radius: 15px;
        margin-top: -25px !important;
        overflow: hidden;
        background: #fff;
        padding-top: 10px;
    }
}

// 秒杀标签相关
.module-seckill-goods_ad {
    .cap-seckill-goods__tag {
        position: absolute;
        bottom: 0;
        left: 0;
        box-sizing: border-box;
        width: 100%;
        height: 24px;
        padding: 0 12px;
        color: #fff;
        line-height: 24px;
        background: -webkit-linear-gradient(left, #fd3e20, #fc6340);
        background: linear-gradient(90deg, #fd3e20, #fc6340);
        background: linear-gradient(to right, var(--ump-start-bg, #f86f30), var(--ump-end-bg, #e91717));
        color: var(--ump-main-text, #ffffff);

        .cap-seckill-goods__tag-title {
            font-weight: 700;
            font-size: 20px;
            float: left;
        }

        .cap-seckill-goods__countdown {
            rich-text {
                font-weight: bold;
                font-size: 12px;
                text-align: center;
            }
        }

        &.big {
            height: 40px;
            line-height: 40px;

            .cap-seckill-goods__tag-countdown {
                float: right;
                height: 40px;
                font-size: 12px;
                line-height: 16px;
            }

            .cap-seckill-goods__tag-countdown-text {
                display: block;
                height: 16px;
                margin: 5px 0 0;
                line-height: 16px;
                text-align: left;
                opacity: 0.6;
            }

            .cap-seckill-goods__countdown {
                margin-top: 0;

                text {
                    padding: 0 2px;
                }
            }
        }

        &.list,
        &.small {
            height: auto;
            padding: 5px 10px;

            .cap-seckill-goods__tag-countdown {
                font-size: 12px;
                display: flex;
                white-space: nowrap;
                justify-content: space-between;
                height: auto;
            }

            .cap-seckill-goods__tag-countdown-text {
                display: block;
                height: 24px;
                line-height: 24px;
            }
        }
    }
}
</style>
