<template>
    <div
        class="module-groupon-goods_ad-wrapper"
        v-if="grouponItemList && grouponItemList?.length > 0"
    >
        <div
            :style="
                allFormat.boxPadding +
                allFormat.boxPaddingTop +
                allFormat.boxPaddingBottom +
                allFormat.innerPadding +
                allFormat.boxRadius +
                allFormat.backgroundColor +
                allFormat.boxRadius
            "
            style="position: relative"
            :class="
                'module-ad-con module-goods_ad module-groupon-goods_ad ad-style__' +
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
                <div v-if="grouponItemList.length > 0" class="mod_groupon">
                    <!-- 标题和查看更多一起显示隐藏 -->
                    <div
                        class="groupon-top"
                        v-if="module.title?.showTitle == 1"
                        :style="{
                            background: module.title?.titleBackground2
                                ? 'linear-gradient(to right, ' + (module.title?.titleBackground || '') + ', ' + module.title.titleBackground2 + ')'
                                : module.title?.titleBackground || '',
                            borderTopLeftRadius: module.title?.titleRadius ? module.title.titleRadius + 'px' : '',
                            borderTopRightRadius: module.title?.titleRadius ? module.title.titleRadius + 'px' : ''
                        }"
                    >
                        <div class="groupon-title-wrapper">
                            <div class="line" v-if="module.title?.titleStyle == 3"></div>
                            <el-image
                                v-if="module.title?.titleBackgroundPic?.picUrl"
                                class="image-size"
                                :src="imageFormat(module.title.titleBackgroundPic.picUrl)"
                            ></el-image>
                            <div class="groupon-title" :style="{ color: module.title?.titleColor || '#1a1a1a' }">
                                {{ module.title?.titleText ? module.title.titleText : $t("拼团") }}
                            </div>
                            <div
                                class="groupon-desc"
                                v-if="module.title?.descText"
                                :style="{ color: module.title?.descColor || '#1a1a1a' }"
                                :class="[module.title?.titleStyle == 2 ? 'big-sm' : 'sm']"
                            >
                                {{ module.title.descText }}
                            </div>
                        </div>
                        <div class="groupon-top-right">
                            <!-- 倒计时：无箭头、不跳转 -->
                            <div
                                v-if="titleMore.showCountdown && grouponTopTime > 0"
                                class="groupon-count-down"
                                :style="{ color: module.title?.moreColor || '#1a1a1a' }"
                            >
                                <CountDown countdown-type="specifyTime" :specify-time="grouponTopTime">
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
                                class="groupon-more-link"
                            >
                                <span>{{ $t(moreLinkText) }}</span>
                                <SvgIcon class="icon" name="arrow-right" width="16" height="16" />
                            </NuxtLink>
                        </div>
                    </div>
                    <div class="groupon_list_wrap">
                        <!-- 使用 swiper 横向滑动布局（style 6） -->
                        <div
                            v-if="grouponItemList.length > 0 && useSwiperLayout"
                            class="groupon_list"
                            :style="{ padding: module?.goodsPadding ? module.goodsPadding + 'px' : '0', boxSizing: 'border-box' }"
                        >
                            <!-- 左右上下边距与商品边距一样，使用容器的 padding 实现 -->
                            <!-- 注释：之前使用 CSS 变量 --goods-padding，现在改为容器 padding -->
                            <!-- :style="{ '--goods-padding': module?.goodsPadding ? module.goodsPadding + 'px' : '0' }" -->
                            <swiper :itemList="processedGrouponList" :swiperOption="grouponSwiperOption" style="width: 100%">
                                <template v-slot:default="{ item }">
                                    <!-- 注释：swiper 布局中，商品项不需要 padding，间隔由 swiper 的 spaceBetween 和容器的 padding 控制 -->
                                    <!-- padding: module?.goodsPadding ? module.goodsPadding + 'px' : '0' -->
                                    <div class="proditem goods-ad-item">
                                        <NuxtLink
                                            :title="item.productName"
                                            :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                            class=""
                                            target="_blank"
                                        >
                                            <div class="g-item-warp">
                                                <div class="pic">
                                                        <div class="groupon-item-image-wrapper">
                                                        <div class="groupon-item-image" :style="[{width: Number(module.cardStyle) === 1 ? '90%' : '100%'}]">
                                                            <el-image
                                                                :src="productImage(item)"
                                                                class="pro_pic"
                                                                loading="lazy"
                                                                fit="cover"
                                                                style="transition: opacity 0.2s linear" :alt="item.productName || ''" 
                                                            />
                                                            <img
                                                                v-if="styleMap[item.productId]?.borderImage"
                                                                class="product-style-border"
                                                                :src="styleMap[item.productId].borderImage ?? undefined"
                                                                alt=""
                                                            />
                                                        </div>
                                                    </div>
                                                    <div class="countdown" v-if="module?.cardStyle == 1">
                                                        <div class="title">{{ $t("拼团") }}</div>
                                                        <div class="countdown-time" v-if="getCountdownText(item, $t('天'))">
                                                            <div class="time">{{ isActivityStart(item) ? $t("距结束") : $t("距开始") }}</div>
                                                            <div class="text">{{ getCountdownText(item, $t("天")) }}</div>
                                                        </div>
                                                        <div class="countdown-time" v-else>
                                                            <div class="time">{{ $t("已结束") }}</div>
                                                        </div>
                                                    </div>
                                                </div>
                                                <template v-if="module?.showName == 1">
                                                    <div class="titleBox groupon-item__name" :style="[{marginBottom: Number(module?.cardStyle) === 1 ? 0 : '12px'}]">
                                                        <div
                                                            class="title product-name-text"
                                                            :data-group-label="(getMaxGroupNum(item) || 0) + $t('人团')"
                                                            :style="{
                                                                textAlign: module?.textAlign == 1 ? 'left' : 'center',
                                                                fontWeight: module?.textWeight == 1 ? 'normal' : 'bold'
                                                            }"
                                                        >
                                                            {{ item.productName }}
                                                        </div>
                                                    </div>
                                                </template>
                                            </div>
                                        </NuxtLink>
                                        <template v-if="module?.showPrice == 1">
                                            <template v-if="Number(module?.cardStyle) === 1">
                                                <div class="priceBox">
                                                    <div class="priceBoxNum">
                                                        <FormatPrice
                                                            class="minGrouponPrice"
                                                            :decimalsStyle="{ fontSize: '14px', alignSelf: 'self-end', transform: 'translateY(2px)' }"
                                                            :currencyStyle="{ fontSize: '14px', alignSelf: 'self-end', transform: 'translateY(2px)' }"
                                                            :fontStyle="{ marginTop: '2.5px' }"
                                                            v-model="item.minGrouponPrice"
                                                        ></FormatPrice>
                                                        <div class="old">
                                                            <FormatPrice
                                                                :decimalsStyle="{ fontSize: '12px', alignSelf: 'self-end' }"
                                                                :currencyStyle="{ fontSize: '12px', alignSelf: 'self-end' }"
                                                                :fontStyle="{ fontSize: '14px', color: '#999' }"
                                                                v-model="item.productPrice"
                                                                style="text-decoration: line-through"
                                                            ></FormatPrice>
                                                        </div>
                                                    </div>
                                                    <div class="text">{{ $t("拼") }}</div>
                                                </div>
                                            </template>
                                            <template v-else>
                                                <div class="groupon-item__price" :style="{ flexDirection: module?.textAlign == 1 ? 'row' : 'column' }">
                                                    <div class="price-wrapper">
                                                        <FormatPrice v-model="item.minGrouponPrice" class="price price-miaosha" />
                                                        <template v-if="item.productPrice && Number(item.productPrice) > 0">
                                                            <FormatPrice v-model="item.productPrice" class="price price-origin" />
                                                        </template>
                                                    </div>
                                                    <div class="buy" v-if="module?.buyBtnStyle > 0">
                                                        <template v-if="item.productSku && item.productSku.length > 0">
                                                            <NuxtLink :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })" target="_blank">
                                                                <ProductButton :buyBtnStyle="module.buyBtnStyle"></ProductButton>
                                                            </NuxtLink>
                                                        </template>
                                                        <template v-else>
                                                            <ProductBuy :isQuick="true" :id="item.productId" :disabled="item.productStock == 0" :number="1">
                                                                <ProductButton :buyBtnStyle="module.buyBtnStyle"></ProductButton>
                                                            </ProductBuy>
                                                        </template>
                                                    </div>
                                                    <div class="buy" v-else>
                                                        <NuxtLink :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })" target="_blank">
                                                            <div class="view_detail">
                                                                <i class="iconfont-pc icon-tianjia"></i>
                                                            </div>
                                                        </NuxtLink>
                                                    </div>
                                                </div>
                                            </template>
                                        </template>
                                    </div>
                                </template>
                            </swiper>
                        </div>
                        <!-- Flex 布局（style 1, 2） -->
                        <div
                            v-else-if="grouponItemList.length > 0"
                            class="goods-ad-warp"
                            :style="{ padding: module?.goodsPadding ? module.goodsPadding + 'px' : '0' }"
                        >
                            <!-- 左右上下边距与商品边距一样，使用容器的 padding 实现 -->
                            <!-- 商品中间间隔为边距的两倍，使用 gap 实现 -->
                            <div class="goods-ad-con" :style="{ gap: module?.goodsPadding ? module.goodsPadding * 2 + 'px' : '0' }">
                                <!-- 注释：之前商品项有 padding，现在改为容器 padding + gap 实现 -->
                                <!-- padding: module?.goodsPadding ? module.goodsPadding + 'px' : '0' -->
                                <div
                                    v-for="item in filteredGrouponList"
                                    :key="item.productId"
                                    class="proditem goods-ad-item"
                                    :style="{
                                        width: getItemWidth
                                    }"
                                >
                                    <div class="item-content">
                                        <div class="item-con">
                                            <NuxtLink
                                                :title="item.productName"
                                                :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                                class=""
                                                target="_blank"
                                            >
                                                <div class="g-item-warp">
                                                    <div class="pic">
                                                        <div class="groupon-item-image-wrapper">
                                                            <div class="groupon-item-image" :style="[{width: Number(module.cardStyle) === 1 ? '90%' : '100%'}]">
                                                                <el-image
                                                                    :src="productImage(item)"
                                                                    class="pro_pic"
                                                                    loading="lazy"
                                                                    style="transition: opacity 0.2s linear" :alt="item.productName || ''" 
                                                                />
                                                                <img
                                                                    v-if="styleMap[item.productId]?.borderImage"
                                                                    class="product-style-border"
                                                                    :src="styleMap[item.productId].borderImage ?? undefined"
                                                                    alt=""
                                                                />
                                                            </div>
                                                        </div>
                                                        <div class="countdown" v-if="Number(module?.cardStyle) === 1">
                                                            <div class="title">{{ $t("拼团") }}</div>
                                                            <div class="countdown-time" v-if="getCountdownText(item, $t('天'))">
                                                                <div class="time">{{ isActivityStart(item) ? $t("距结束") : $t("距开始") }}</div>
                                                                <div class="text">{{ getCountdownText(item, $t("天")) }}</div>
                                                            </div>
                                                            <div class="countdown-time" v-else>
                                                                <div class="time">{{ $t("已结束") }}</div>
                                                            </div>
                                                        </div>
                                                    </div>
                                                    <template v-if="module?.showName == 1">
                                                    <div class="titleBox groupon-item__name" :style="[{marginBottom: Number(module?.cardStyle) === 1 ? 0 : '12px'}]">
                                                            <div
                                                                class="title product-name-text"
                                                                :data-group-label="(getMaxGroupNum(item) || 0) + $t('人团')"
                                                                :style="{
                                                                    textAlign: module?.textAlign == 1 ? 'left' : 'center',
                                                                    fontWeight: module?.textWeight == 1 ? 'normal' : 'bold'
                                                                }"
                                                            >
                                                                {{ item.productName }}
                                                            </div>
                                                        </div>
                                                    </template>
                                                </div>
                                            </NuxtLink>
                                            <template v-if="module?.showPrice == 1">
                                                <template v-if="Number(module?.cardStyle) === 1">
                                                    <div class="priceBox">
                                                        <div class="priceBoxNum">
                                                            <FormatPrice
                                                                class="minGrouponPrice"
                                                                :decimalsStyle="{ fontSize: '14px', alignSelf: 'self-end', transform: 'translateY(2px)' }"
                                                                :currencyStyle="{ fontSize: '14px', alignSelf: 'self-end', transform: 'translateY(2px)' }"
                                                                :fontStyle="{ marginTop: '2.5px' }"
                                                                v-model="item.minGrouponPrice"
                                                            ></FormatPrice>
                                                            <div class="old">
                                                                <FormatPrice
                                                                    :decimalsStyle="{ fontSize: '12px', alignSelf: 'self-end' }"
                                                                    :currencyStyle="{ fontSize: '12px', alignSelf: 'self-end' }"
                                                                    :fontStyle="{ fontSize: '14px', color: '#999' }"
                                                                    v-model="item.productPrice"
                                                                    style="text-decoration: line-through"
                                                                ></FormatPrice>
                                                            </div>
                                                        </div>
                                                        <div class="text">{{ $t("拼") }}</div>
                                                    </div>
                                                </template>
                                                <template v-else>
                                                    <div class="groupon-item__price" :style="{ flexDirection: module?.textAlign == 1 ? 'row' : 'column' }">
                                                        <div class="price-wrapper">
                                                            <FormatPrice v-model="item.minGrouponPrice" class="price price-miaosha" />
                                                            <template v-if="item.productPrice && Number(item.productPrice) > 0">
                                                                <FormatPrice v-model="item.productPrice" class="price price-origin" />
                                                            </template>
                                                        </div>
                                                        <div class="buy" v-if="module?.buyBtnStyle > 0">
                                                            <template v-if="item.productSku && item.productSku.length > 0">
                                                                <NuxtLink
                                                                    :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                                                    target="_blank"
                                                                >
                                                                    <ProductButton :buyBtnStyle="module.buyBtnStyle"></ProductButton>
                                                                </NuxtLink>
                                                            </template>
                                                            <template v-else>
                                                                <ProductBuy :isQuick="true" :id="item.productId" :disabled="item.productStock == 0" :number="1">
                                                                    <ProductButton :buyBtnStyle="module.buyBtnStyle"></ProductButton>
                                                                </ProductBuy>
                                                            </template>
                                                        </div>
                                                        <div class="buy" v-else>
                                                            <NuxtLink :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })" target="_blank">
                                                                <div class="view_detail">
                                                                    <i class="iconfont-pc icon-tianjia"></i>
                                                                </div>
                                                            </NuxtLink>
                                                        </div>
                                                    </div>
                                                </template>
                                            </template>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                        <!-- 加载状态 -->
                        <div v-else class="groupon_list">
                            <div v-for="index in 5" class="proditem" style="width: 200px">
                                <div class="groupon-item-image">
                                    <HomeSkeleton></HomeSkeleton>
                                </div>
                                <div class="groupon-item__name" style="height: 30px">
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
import { ref, computed, onMounted, onUnmounted, reactive } from "vue";
import { getHomeGrouponList } from "@/api/home/home";
import { getCountdownFormat, imageFormat, productImage, urlFormat } from "~/utils/format";
import { resolveCountdownLabel, resolveMoreLinkTitle, resolveTitleMore } from "~/decorate/titleMore";
import { useProductStyle } from "~/hooks";
import CountDown from "~/components/common/CountDown.vue";
import HomeSkeleton from "~/components/home/Skeleton.vue";
import ProductButton from "~/components/shop/product/ProductButton.vue";
import { Buy as ProductBuy } from "~/components/product";

// 类型定义
import type { GrouponItem, GrouponItemListParams } from "~/types/home/groupon";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

const { frame } = props.module;

if(props.module.style == 1) {
    props.module.style = 3
}

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

const grouponItemList = ref<GrouponItem[]>([]);
const { styleMap } = useProductStyle(grouponItemList);
const time = ref(60 * 60 * 1000);
const now = ref(Date.now());
let countdownTimer: number | null = null;

const filterParams = reactive<GrouponItemListParams>({
    //初始化用于查询的参数
    page: 1,
    size: 20,
    sortField: "",
    keyword: "",
    sortOrder: "",
    grouponPromotionId: 0
});

const className = computed(() => {
    const style = typeof props.module.style === "string" ? Number(props.module.style) : props.module.style;
    switch (style) {
        case 1:
            return "big";
        case 2:
            return "small flex-end";
        case 6:
            return "list flex-end";
        default:
            return "";
    }
});

const groupPirceStyle = computed(() => {
    if (props.module.style != 3) {
        return {
            fontSize: "16px"
        };
    } else {
        return {
            fontSize: "14px"
        };
    }
});

const groupCurrencyStyle = computed(() => {
    if (props.module.style != 3) {
        return {
            fontSize: "12px"
        };
    } else {
        return {
            fontSize: "11px"
        };
    }
});

/*转时间搓 */
const dateStringToTimestamp = (dateString: any) => {
    if (!dateString) return 0;
    const dateParts = dateString.split(" ");
    const [year, month, day] = dateParts[0].split("-");
    const [hour, minute, second] = dateParts[1] ? dateParts[1].split(":") : ["0", "0", "0"];
    const date = new Date(year, month - 1, day, hour, minute, second);
    return date.getTime();
};

/**
 * 是否活动开始
 * @param item 团购活动信息
 */
const isActivityStart = (item: GrouponItem) => {
    const startTime = dateStringToTimestamp(item.startTime);
    return startTime < new Date().getTime();
};

/**
 * 获取时间文本
 */
const getTimeText = (item: GrouponItem) => {
    // 如果活动呢已经开始了
    if (isActivityStart(item)) {
        return props.module.style == 1 ? "距离结束仅剩" : "距结束";
    } else {
        return props.module.style == 1 ? "距离开始仅剩" : "距开始";
    }
};

/**
 * 倒计时文本（依赖 now，保证每秒刷新）
 */
const getCountdownText = (item: GrouponItem, dayText: string) => {
    void now.value;
    const targetTime = isActivityStart(item) ? dateStringToTimestamp(item.endTime) : dateStringToTimestamp(item.startTime);
    return getCountdownFormat(targetTime, dayText);
};

/**
 * 拿到成团人数
 * @param item
 */
const getMaxGroupNum = (item: GrouponItem): number => {
    if (!item?.groupNum) return 0;
    try {
        const list = JSON.parse(item.groupNum);
        if (!Array.isArray(list) || list.length === 0) return 0;
        const max = Math.max(...list.map((x: any) => Number(x?.groupNum) || 0));
        return Number.isFinite(max) ? max : 0;
    } catch {
        return 0;
    }
};

/**
 * 团是否结束了
 */
const isActivityEnd = (item: GrouponItem) => {
    // 不是待成团的状态，拼团结束了
    const endTime = dateStringToTimestamp(item.endTime);
    return endTime < new Date().getTime();
};

// ==================== 标题右侧区（更多链接 / 倒计时） ====================
const titleMore = computed(() => resolveTitleMore(props.module?.title, "link"));
const countdownLabel = computed(() => resolveCountdownLabel(props.module?.title, "距结束"));
const moreLinkText = computed(() => resolveMoreLinkTitle(props.module?.title, "查看更多"));

/** 顶部倒计时：取列表中最早的结束时间 */
const grouponTopTime = computed(() => {
    const endTimes = grouponItemList.value
        .filter((item) => !isActivityEnd(item))
        .map((item) => dateStringToTimestamp(item.endTime))
        .filter((t: number) => t > 0);
    return endTimes.length > 0 ? Math.min(...endTimes) : 0;
});

// 判断是否使用 swiper 布局
const useSwiperLayout = computed(() => {
    const style = typeof props.module?.style === "string" ? Number(props.module.style) : props.module?.style;
    // style 6 使用 swiper 横向滑动，style 2 和 3 使用 flex 布局
    return style === 6;
});

// 获取商品项宽度（非 swiper 布局时使用，style 2 和 3）
const getItemWidth = computed(() => {
    const style = typeof props.module?.style === "string" ? Number(props.module.style) : props.module?.style;
    // 商品中间间隔为边距的两倍，使用 gap 实现
    // 左右上下边距与商品边距一样，使用 padding 实现
    // gap 是商品之间的间隔，需要在宽度计算中减去相应的 gap
    const gap = props.module?.goodsPadding ? Number(props.module.goodsPadding) * 2 : 0; // gap 是边距的两倍

    switch (style) {
        case 2: {
            // 一行4个，每个25%
            // 4个商品之间有3个间隔，每个间隔 = gap (2 * goodsPadding)
            // 每个商品的宽度 = (100% - 3 * gap) / 4 = 25% - (3 * gap) / 4
            if (gap > 0) {
                const totalGap = (gap * 3) / 4;
                return `calc(25% - ${totalGap}px)`;
            }
            return "25%";
        }
        case 3: {
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

// 过滤掉已结束的活动
const filteredGrouponList = computed(() => {
    return grouponItemList.value.filter((item: GrouponItem) => !isActivityEnd(item));
});

// 处理商品列表，当商品数量不足时复制列表以支持无限轮播
const processedGrouponList = computed(() => {
    const style = typeof props.module?.style === "string" ? Number(props.module.style) : props.module?.style;
    if (style !== 6 || !useSwiperLayout.value) {
        // 非 swiper 布局，直接返回过滤后的列表
        return filteredGrouponList.value;
    }

    const slidesPerView = 5;
    const minSlidesForLoop = slidesPerView * 2;

    // 如果商品数量足够，直接返回
    if (filteredGrouponList.value.length >= minSlidesForLoop) {
        return filteredGrouponList.value;
    }

    // 如果商品数量不足，复制列表以满足 loop 的要求
    // 复制足够的次数以确保至少有 minSlidesForLoop 个商品
    const timesToDuplicate = Math.ceil(minSlidesForLoop / filteredGrouponList.value.length);
    const duplicatedList: GrouponItem[] = [];
    for (let i = 0; i < timesToDuplicate; i++) {
        duplicatedList.push(...filteredGrouponList.value);
    }

    return duplicatedList;
});

// 根据 style 获取 swiper 配置
const getSwiperConfig = computed(() => {
    const style = typeof props.module?.style === "string" ? Number(props.module.style) : props.module?.style;
    // style 6 使用横向滑动，显示5个商品
    const slidesPerView = 5;
    // 商品中间间隔为边距的两倍
    const spaceBetween = props.module?.goodsPadding ? Number(props.module.goodsPadding) * 2 : 0;

    // 使用处理后的商品列表来判断是否可以启用 loop
    // processedGrouponList 已经处理了商品数量不足的情况，确保有足够的商品
    const canLoop = style === 6 && processedGrouponList.value.length >= slidesPerView * 2;

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

const grouponSwiperOption = computed(() => getSwiperConfig.value);

const getData = async () => {
    try {
        const result = await getHomeGrouponList(filterParams);
        // 过滤掉拼团活动结束的
        if (result && result.records) {
            result.records = result.records.filter((item: GrouponItem) => {
                return !isActivityEnd(item);
            });
            grouponItemList.value = result.records;
        }
    } catch (error) {
        console.error(error);
    }
};

onMounted(() => {
    if (props.module.grouponActivityItem) {
        filterParams.grouponPromotionId = props.module.grouponActivityItem.grouponPromotionId;
        // 获取更多商品数据，由CSS布局控制一行显示几个
        // style 2 一行4个，style 3 一行5个，但需要获取更多商品以便换行显示
        if (props.module.style == 2 || props.module.style == 3) {
            filterParams.size = 20; // 获取更多商品，支持多行显示
        } else if (props.module.style == 6) {
            filterParams.size = 15; // style 6 使用 swiper，显示5个商品
        } else if (props.module.style < 6) {
            filterParams.size = Number(props.module.style);
        }
        getData();
    }
    // 倒计时刷新
    if (countdownTimer) {
        clearInterval(countdownTimer);
        countdownTimer = null;
    }
    countdownTimer = window.setInterval(() => {
        now.value = Date.now();
    }, 1000);
});

onUnmounted(() => {
    if (countdownTimer) {
        clearInterval(countdownTimer);
        countdownTimer = null;
    }
});
</script>
<style lang="less" scoped>
.mod_groupon {
    display: flex;
    flex-direction: column;
    width: 100%;
    overflow: hidden;
    .groupon-top {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 10px 0;
        margin-bottom: 10px;
        box-sizing: border-box;
        .groupon-title-wrapper {
            display: flex;
            align-items: baseline;
            gap: 4px;
            .line {
                width: 4px;
                height: 16px;
                background-color: #1a1a1a;
                border-radius: 8px;
            }
            .image-size {
                width: 30px;
                height: 30px;
                margin-left: 10px;
            }
            .groupon-title {
                font-size: 16px;
                font-weight: bold;
                color: #1a1a1a;
                margin-right: 5px;
            }
            .groupon-desc {
                font-size: 12px;
                color: #1a1a1a;
                &.big-sm {
                    font-size: 16px;
                }
            }
        }
        .groupon-top-right {
            display: flex;
            align-items: center;
            gap: 12px;
        }
        .groupon-more-link {
            display: flex;
            align-items: center;
            font-size: 14px;
            color: #1a1a1a;
            text-decoration: none;
            .icon {
                margin-left: 4px;
                color: inherit;
            }
            &:hover {
                color: var(--main-bg) !important;
            }
        }
        // 顶部倒计时（文字颜色由 title.moreColor 控制，子元素继承）
        .groupon-count-down {
            display: flex;
            align-items: center;
            font-size: 18px;
            color: #1a1a1a;

            .countdown-box {
                display: flex;
                align-items: center;

                span {
                    margin: 0 1px;
                }
            }

            .countdown-text,
            .countdown-separator-text {
                font-size: 18px;
                color: inherit;
            }

            .countdown-separator {
                color: inherit;
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
        }
    }
}

.groupon-card {
    background: #fff;
    border: 1px solid rgba(64, 112, 255, 0.35);
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    height: 100%;
    transition:
        border-color 0.2s ease,
        box-shadow 0.2s ease,
        transform 0.2s ease;
    box-shadow: 0 6px 16px rgba(15, 35, 95, 0.05);
}

.groupon-card:hover {
    border-color: var(--main-bg);
    box-shadow: 0 12px 30px rgba(15, 35, 95, 0.12);
    transform: translateY(-4px);
}

.groupon-card > a {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.groupon-card .groupon-item-image {
    width: 100%;
    border-radius: 14px;
    overflow: hidden;
    aspect-ratio: 3 / 4;
    background: #f8f8f8;
    align-items: center;
    justify-content: center;
}

.groupon-card .groupon-item-image :deep(.el-image) {
    width: 100%;
    height: 100%;
    object-fit: cover;
}

.groupon-card .groupon-item__name {
    margin-bottom: 4px;
}

.groupon-card .groupon-item__name .item-name-a {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #2a3145;
    font-size: 16px;
    font-weight: 600;
    line-height: 22px;
}

.groupon-card .group-num-text {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 44px;
    padding: 0 10px;
    height: 24px;
    border-radius: 2px;
    border: 1px solid var(--main-bg);
    background: transparent;
    color: var(--main-bg);
    font-size: 12px;
    font-weight: 500;
}

.groupon-card .product-desc {
    margin-top: 4px;
    font-size: 13px;
    color: #999;
}

.groupon-card .groupon-item__price {
    margin-top: 12px;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 8px;
}

.groupon-card .groupon-item__price .item-price {
    display: flex;
    align-items: baseline;
    gap: 8px;
}

.groupon-card .groupon-item__price .price-miaosha {
    margin-right: 0;
    :deep(.num) {
        color: var(--main-bg);
        font-size: 26px;
        font-weight: 700;
    }
    :deep(.util),
    :deep(.decimal) {
        color: var(--main-bg);
        font-size: 16px;
    }
}

.groupon-card .groupon-item__price .price-origin {
    align-items: baseline;
    justify-content: center;
    height: 19px;
    color: #888b94;
    text-decoration: line-through;
    :deep(.price-row) {
        line-height: 22px;
        font-weight: 500;
    }
    :deep(.num) {
        font-size: 14px;
        color: #888b94;
    }
    :deep(.util) {
        color: #888b94;
    }
}

.groupon-card .item-buy {
    display: flex;
    align-items: center;
}

.groupon-card .view_detail {
    background: #eef3ff;
    color: var(--main-bg);
}

.mod_groupon .groupon_list_wrap {
    position: relative;
    z-index: 1;
    flex: 1;
    height: 100%;
    width: 100%;
}

.mod_groupon .groupon_list {
    position: relative;
    z-index: 1;
    height: 100%;
    overflow: hidden;
    width: 100%;
}

.mod_groupon .groupon_list :deep(.swiper) {
    width: 100%;
    padding: 0;
}

.mod_groupon .groupon_list :deep(.swiper-slide) {
    // 宽度由 swiper 根据 slidesPerView 和 spaceBetween 自动计算
    flex-shrink: 0;
    height: auto;
    box-sizing: border-box;
}

.mod_groupon .groupon_list .proditem {
    box-sizing: border-box;
    position: relative;
    z-index: 1;
    width: 100%; // 宽度填满 slide
    background-color: #ffffff;
    padding-bottom: 10px;
    &:hover {
        .groupon-item-image {
            display: flex;
            justify-content: center;
            overflow: hidden;
            .pro_pic {
                filter: brightness(105%);
            }
        }
    }
}

.mod_groupon .groupon_list .proditem a {
    display: block;
}

.groupon-item-image {
    width: 100%;
    margin: 0 auto;
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

.mod_groupon .groupon_list .groupon-item-image {
    aspect-ratio: 1 / 1; // 设置固定宽高比，确保所有商品卡片高度一致
}
.mod_groupon .groupon_list .pro_pic {
    transition:
        transform 0.4s ease,
        -webkit-transform 0.4s ease,
        -moz-transform 0.4s ease,
        -o-transform 0.4s ease;
    width: 100%;
    height: 100%;
    object-fit: cover; // 确保图片填满容器
}

.mod_groupon .groupon_list .groupon-item__name {
    height: auto;
    min-height: 20px;
    margin-top: 10px;
    margin-bottom: 12px;
    // padding: 0 14px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: #1a1a1a;
    font-size: 16px;
    font-weight: 400;
    line-height: 18px;
    cursor: pointer;
    display: flex;
    align-items: center;
    flex-wrap: nowrap;
    gap: 4px;
    &:hover {
        color: var(--main-bg);
    }
    .group-num {
        flex-shrink: 0;
        display: inline-flex;
        align-items: baseline;
        margin-right: 4px;
        .group-num-text {
            color: var(--main-bg) !important;
            background-color: transparent;
            border: 1px solid var(--main-bg);
            padding: 2px 4.5px;
            border-radius: 2px;
            font-size: 12px;
            line-height: 12px;
            font-weight: normal;
            white-space: nowrap;
            box-sizing: border-box;
            display: inline-flex;
        }
    }
    .product-name-text {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        flex: 1;
        min-width: 0;
        position: relative;
        &::before {
            content: attr(data-group-label);
            display: inline-block;
            color: var(--main-bg) !important;
            background-color: transparent;
            border: 1px solid var(--main-bg);
            padding: 2px 4.5px;
            border-radius: 2px;
            font-size: 12px;
            line-height: 12px;
            font-weight: normal;
            white-space: nowrap;
            box-sizing: border-box;
            margin-right: 4px;
            margin-top: -3px;
            vertical-align: middle;
            flex-shrink: 0;
        }
    }
}
.ad-goods_name_row__2 .mod_groupon .groupon_list .groupon-item__name {
    min-height: 56px;
    .product-name-text {
        white-space: normal;
        display: -webkit-box;
        display: -moz-box;
        -webkit-line-clamp: 2;
        -moz-line-clamp: 2;
        line-clamp: 2;
        -webkit-box-orient: vertical;
        -moz-box-orient: vertical;
        line-height: 28px;
        height: 56px;
        overflow: hidden;
        text-overflow: ellipsis;
        word-break: break-word;
        position: relative;
        padding-left: 0;
        text-indent: 0;
        &::before {
            content: attr(data-group-label);
            display: inline-block;
            color: var(--main-bg) !important;
            background-color: transparent;
            border: 1px solid var(--main-bg);
            padding: 2px 4.5px;
            border-radius: 2px;
            font-size: 12px;
            line-height: 12px;
            font-weight: normal;
            white-space: nowrap;
            box-sizing: border-box;
            margin-right: 4px;
            margin-top: -3px;
            vertical-align: middle;
            flex-shrink: 0;
        }
    }
}
.mod_groupon .groupon_list .groupon-item__price {
    display: flex;
    align-items: center;
    // padding: 0 14px;
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
            color: var(--main-bg);
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

.groupon-item__price .price-miaosha {
    align-items: baseline;
    justify-content: center;
    margin-right: 8px;
    color: var(--main-bg);
    line-height: 22px;

    :deep(.price-row) {
        align-items: baseline !important;
    }

    :deep(.num) {
        font-size: 24px;
        color: var(--main-bg);
        font-weight: 600;
    }

    :deep(.util) {
        color: var(--main-bg);
        font-size: 16px;
    }
    :deep(.decimal) {
        color: var(--main-bg);
        font-size: 16px;
    }
}

.groupon-item__price .price-origin {
    align-items: baseline;
    justify-content: center;
    height: 19px;
    color: #888b94;
    text-decoration: line-through;
    :deep(.price-row) {
        line-height: 22px;
        font-weight: 500;
    }
    :deep(.num) {
        font-size: 14px;
        color: #888b94;
    }

    :deep(.util) {
        color: #888b94;
    }
}

.price-origin {
    // padding: 0 14px;
    box-sizing: border-box;
    text-decoration: line-through;
    margin-top: 5px;
}

:deep(.swiper) {
    box-sizing: border-box;
    // padding: 0 12px;
}

:deep(.swiper-wrapper) {
    display: flex;
}

.module-groupon-goods_ad-wrapper {
    width: 100%;
    box-sizing: border-box;
}
.module-groupon-goods_ad:deep(.swiper-button-prev) {
    box-sizing: border-box;
    left: 0;
    background-color: rgba(0, 0, 0, 0.122);
    width: 40px;
    height: 48px;
    padding-right: 7px;
    line-height: 35px;
    border-radius: 0 24px 24px 0 !important;
    z-index: 2;
    outline: none;
    transition: background-color 0.2s ease;
    &:hover {
        background-color: rgba(0, 0, 0, 0.239);
    }
}

.module-groupon-goods_ad:deep(.swiper-button-next) {
    box-sizing: border-box;
    right: 0;
    background-color: rgba(0, 0, 0, 0.122);
    width: 40px;
    height: 48px;
    padding-left: 7px;
    line-height: 35px;
    z-index: 2;
    border-radius: 24px 0 0 24px !important;
    outline: none;
    transition: background-color 0.2s ease;
    &:hover {
        background-color: rgba(0, 0, 0, 0.239);
    }
}

.module-groupon-goods_ad:deep(.swiper-button-prev:after),
.module-groupon-goods_ad:deep(.swiper-button-next:after) {
    font-family: "iconfont-pc";
    content: "\e70d";
    color: #fff;
    font-size: 14px;
}

.module-groupon-goods_ad:deep(.swiper-button-prev:after) {
    transform: rotate(180deg);
}

.module-groupon-goods_ad:deep(.swiper-button-prev.swiper-button-disabled),
.module-groupon-goods_ad:deep(.swiper-button-next.swiper-button-disabled) {
    pointer-events: fill;
}

.cap-seckill-goods__tag {
    display: flex;
    align-items: center;
    justify-content: space-between;
    &.flex-end {
        justify-content: flex-end;
    }
}
/*商品类型*/
.module-goods_ad .module-ad-content {
    overflow: hidden;
    z-index: 1;
    position: relative;
}
.goods-ad-warp .goods-ad-item {
    /* width 由内联样式或具体样式类控制，不在此处设置固定值 */
    position: relative;
    overflow: hidden;
    display: inline-block;
    transition: opacity 0.4s ease-in-out;
    box-sizing: border-box;
    background-color: #ffffff;
    // padding-bottom: 10px;

    .ad-goods_radio_style__2 & {
        background-color: transparent;
    }

    &:hover {
        .groupon-item-image {
            display: flex;
            justify-content: center;
            overflow: hidden;
            .pro_pic {
                filter: brightness(105%);
            }
        }
    }
}
.goods-ad-warp .goods-ad-con {
    display: flex;
    flex-wrap: wrap;
}
.goods-ad-warp .goods-ad-item .item-content {
    background-color: transparent;
}
.goods-ad-warp .goods-ad-item .item-con {
    background-color: #ffffff;
    padding-bottom: 10px;
}
.groupon-item-image-wrapper {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
}
.mod_groupon .groupon_list .proditem .groupon-item-image-wrapper {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
}

/* ========== 复用 JoiningGoodsList 的结构样式（不影响外层装修与横向滚动） ========== */
.g-item-warp {
    display: flex;
    flex-direction: column;
    width: 100%;
}
.g-item-warp .pic {
    width: 100%;
}
.g-item-warp .countdown {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: linear-gradient(90deg, #f86e30, #e91817);
    height: 34px;
    padding: 0 10px;
    box-sizing: border-box;
    font-size: 10px;
    color: #ffffff;
    font-family: Menlo;
}
.goods-ad-item .countdown,
.mod_groupon .groupon_list .proditem .countdown {
    background: linear-gradient(90deg, #f86e30, #e91817);
}
.g-item-warp .countdown .title {
    flex: 0 0 auto;
    font-weight: bold;
    font-size: 16px;
    color: #ffffff;
}
.g-item-warp .countdown .countdown-time {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex: 1;
    gap: 6px;
    white-space: nowrap;
}
.g-item-warp .countdown .countdown-time .text {
    /* 解决 macOS 下数字宽度不一致导致的左右抖动（如 1 -> 0） */
    font-variant-numeric: tabular-nums;
    font-feature-settings: "tnum";
    font-family: Menlo;
}
.g-item-warp .titleBox {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-top: 11px;
    margin-bottom: 12px;
    width: 100%;
}
.g-item-warp .titleBox .tag {
    font-weight: 500;
    font-size: 12px;
    color: var(--general);
    min-width: 40px;
    height: 20px;
    background: var(--ump-tag-bg);
    border-radius: 4px;
    line-height: 20px;
    text-align: center;
    flex-shrink: 0;
    padding: 0 5px;
    box-sizing: border-box;
}
.g-item-warp .titleBox .title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.ad-goods_name_row__2 .g-item-warp .titleBox .title {
    white-space: normal;
    display: -webkit-box;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
}

/* 让“人团”标签保持原来有背景的视觉（但布局使用 product-name-text::before） */
.g-item-warp .titleBox .product-name-text::before {
    color: var(--general) !important;
    background: var(--ump-tag-bg) !important;
    border: 0 !important;
    border-radius: 4px;
    min-width: 40px;
    height: 20px;
    line-height: 16px !important;
    padding: 0 5px;
    box-sizing: border-box;
    text-align: center;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-weight: 500;
    font-size: 12px;
    margin-right: 5px;
    margin-top: 0;
    vertical-align: middle;
}

.goods-ad-item .priceBox,
.mod_groupon .groupon_list .proditem .priceBox {
    width: 100%;
    height: 35px;
    background: #fff2f3;
    border-radius: 3px;
    margin-top: 18px;
    display: flex;
    align-items: center;
    position: relative;
}
.goods-ad-item .priceBox:before,
.mod_groupon .groupon_list .proditem .priceBox:before {
    content: "";
    position: absolute;
    left: -23px;
    top: -11px;
    width: calc(100% + 46px);
    height: 1px;
    // background: #f6f6f6;
}
.goods-ad-item .priceBox .priceBoxNum,
.mod_groupon .groupon_list .proditem .priceBox .priceBoxNum {
    display: flex;
    align-items: flex-end;
}
.goods-ad-item .priceBox .priceBoxNum .old,
.mod_groupon .groupon_list .proditem .priceBox .priceBoxNum .old {
    margin-left: 7px;
    display: flex;
    align-items: flex-end;
    font-weight: 500;
    font-size: 12px;
    color: #9a9a9a;
    text-decoration-line: line-through;
}
.goods-ad-item .priceBox .priceBoxNum .minGrouponPrice,
.mod_groupon .groupon_list .proditem .priceBox .priceBoxNum .minGrouponPrice {
    color: var(--price);
    font-weight: 500;
    font-size: 22px;
    padding-left: 9px;
    height: 26px;
}
.goods-ad-item .priceBox .text,
.mod_groupon .groupon_list .proditem .priceBox .text {
    padding: 0 20px;
    height: 30px;
    background: #ee0a25;
    border-radius: 3px;
    font-weight: 600;
    font-size: 20px;
    color: #ffffff;
    line-height: 30px;
    margin-left: auto;
    clip-path: polygon(20% 0, 100% 0, 100% 100%, 0 100%);
    text-align: center;
}
.goods-ad-warp .goods-ad-item .groupon-item-image {
    width: 100%;
    display: flex;
    justify-content: center;
    overflow: hidden;
    background-color: transparent;
    aspect-ratio: 1 / 1; // 设置固定宽高比，确保所有商品卡片高度一致
}
.goods-ad-warp .goods-ad-item .groupon-item-image .pro_pic {
    width: 100%;
    height: 100%;
    object-fit: cover; // 确保图片填满容器
    transition:
        transform 0.4s ease,
        -webkit-transform 0.4s ease,
        -moz-transform 0.4s ease,
        -o-transform 0.4s ease;
}
.mod_groupon .groupon_list .pro_pic {
    width: 100%;
    height: 100%;
    object-fit: contain;
}
.goods-ad-warp .goods-ad-item .item-info .count-down {
    font-size: 12px;
    font-weight: normal;
    clear: both;
}
.goods-ad-warp .goods-ad-item .item-info .count-down i {
    font-size: 18px;
    padding-right: 5px;
}
.goods-ad-warp .goods-ad-item .item-info .count-down em {
    font-size: 16px;
    font-weight: normal;
    color: var(--general);
    padding: 0 2px;
}
.goods-ad-warp .goods-ad-item .groupon-item__name {
    height: auto;
    min-height: 20px;
    margin-top: 10px;
    margin-bottom: 12px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: #1a1a1a;
    font-size: 16px;
    font-weight: 400;
    line-height: 18px;
    cursor: pointer;
    display: flex;
    align-items: center;
    flex-wrap: nowrap;
    gap: 4px;
    &:hover {
        color: var(--main-bg);
    }
    .group-num {
        flex-shrink: 0;
        display: inline-flex;
        align-items: baseline;
        margin-right: 4px;
        .group-num-text {
            color: var(--main-bg) !important;
            background-color: transparent;
            border: 1px solid var(--main-bg);
            padding: 2px 4.5px;
            border-radius: 2px;
            font-size: 12px;
            line-height: 12px;
            font-weight: normal;
            white-space: nowrap;
            box-sizing: border-box;
            display: inline-flex;
        }
    }
    .product-name-text {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        flex: 1;
        min-width: 0;
        position: relative;
        &::before {
            content: attr(data-group-label);
            display: inline-block;
            color: var(--main-bg) !important;
            background-color: transparent;
            border: 1px solid var(--main-bg);
            padding: 2px 4.5px;
            border-radius: 2px;
            font-size: 12px;
            line-height: 12px;
            font-weight: normal;
            white-space: nowrap;
            box-sizing: border-box;
            margin-right: 4px;
            margin-top: -3px;
            vertical-align: middle;
            flex-shrink: 0;
        }
    }
}
.ad-goods_name_row__2 .goods-ad-warp .goods-ad-item .groupon-item__name {
    min-height: 56px;
    .product-name-text {
        white-space: normal;
        display: -webkit-box;
        display: -moz-box;
        -webkit-line-clamp: 2;
        -moz-line-clamp: 2;
        line-clamp: 2;
        -webkit-box-orient: vertical;
        -moz-box-orient: vertical;
        line-height: 28px;
        height: 56px;
        overflow: hidden;
        text-overflow: ellipsis;
        word-break: break-word;
        position: relative;
        padding-left: 0;
        text-indent: 0;
        &::before {
            content: attr(data-group-label);
            display: inline-block;
            color: var(--main-bg) !important;
            background-color: transparent;
            border: 1px solid var(--main-bg);
            padding: 2px 4.5px;
            border-radius: 2px;
            font-size: 12px;
            line-height: 12px;
            font-weight: normal;
            white-space: nowrap;
            box-sizing: border-box;
            margin-right: 4px;
            margin-top: -3px;
            vertical-align: middle;
            flex-shrink: 0;
        }
    }
}
.goods-ad-warp .goods-ad-item .item-info .item-name .item-name-a .item-name-link {
    color: #2a3145;
    text-decoration: none;
}
.goods-ad-warp .goods-ad-item .item-info .item-name .item-brief {
    line-height: 20px;
    height: 20px;
    overflow: hidden;
    display: block;
    color: #aaa;
    overflow: hidden;
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
.goods-ad-warp .goods-ad-item .groupon-item__price {
    display: flex;
    align-items: flex-start;
    // padding: 0 14px;
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
            color: var(--main-bg);
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
.goods-ad-warp .goods-ad-item .groupon-item__price + .price-origin {
    align-items: baseline;
    justify-content: center;
    height: 19px;
    color: #888b94;
    text-decoration: line-through;
    margin-top: 5px;

    :deep(.price-row) {
        line-height: 22px;
        font-weight: 500;
    }
    :deep(.num) {
        font-size: 14px;
        color: #888b94;
    }

    :deep(.util) {
        color: #888b94;
    }
}
.ad-style__1 .goods-ad-warp .goods-ad-item .item-info .item-name:not(.groupon-item__name) .item-name-a {
    font-size: 14px;
}
.ad-style__2 .goods-ad-warp .goods-ad-item .item-info .item-name:not(.groupon-item__name) .item-name-a,
.ad-style__6 .goods-ad-warp .goods-ad-item .item-info .item-name:not(.groupon-item__name) .item-name-a {
    font-size: 12px;
    font-weight: bold;
}
.ad-style__2 .goods-ad-warp .goods-ad-con {
    display: flex;
    flex-wrap: wrap;
}
/* .ad-style__2 的宽度由内联样式动态计算，一行4个，每个25% */
.ad-style__3 .goods-ad-warp .goods-ad-con {
    display: flex;
    flex-wrap: wrap;
}
/* .ad-style__3 的宽度由内联样式动态计算，一行5个，每个20% */
.ad-style__3 .goods-ad-warp .goods-ad-item .item-info .item-buy {
    display: none;
}
.ad-style__4 .goods-ad-warp .goods-ad-con {
    display: flex;
    flex-wrap: wrap;
}
.ad-style__4 .goods-ad-warp .goods-ad-con .goods-ad-item {
    width: 100%;
}
.ad-style__4 .goods-ad-warp .goods-ad-con .goods-ad-item:nth-child(3n + 2) {
    width: 50%;
}
.ad-style__4 .goods-ad-warp .goods-ad-con .goods-ad-item:nth-child(3n + 3) {
    width: 50%;
}
.ad-style__6 .goods-ad-warp .goods-ad-con {
    display: flex;
    flex-wrap: wrap;
}
.ad-style__6 .goods-ad-warp .goods-ad-con .goods-ad-item {
    width: 100%;
}
.ad-style__6 .goods-ad-warp .goods-ad-item .item-content {
    display: flex;
    flex-wrap: nowrap;
}
.ad-style__6 .goods-ad-warp .goods-ad-item .item-content .item-con {
    display: flex;
    flex-wrap: nowrap;
    flex: 1;
}
.ad-style__6 .goods-ad-warp .goods-ad-item .item-photo {
    width: 40%;
}
.ad-style__6 .goods-ad-warp .goods-ad-item .item-info {
    width: 60%;
    text-align: left;
}
.ad-style__6 .goods-ad-warp .goods-ad-item .item-action {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    padding-bottom: 0;
}
.ad-style__6 .goods-ad-warp {
    overflow: hidden;
}
.ad-style__6 .goods-ad-warp .goods-ad-con {
    display: flex;
    flex-wrap: nowrap;
    overflow-x: scroll;
    margin-bottom: -20px;
    padding-bottom: 20px !important;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: none; /* Firefox */
    -ms-overflow-style: none; /* IE and Edge */

    &::-webkit-scrollbar {
        display: none; /* Chrome, Safari, Opera */
    }
}
.ad-style__6 .goods-ad-warp .goods-ad-con .goods-ad-item {
    width: 40%;
    flex: none;
}
.ad-style__7 .goods-ad-warp {
    overflow: hidden;
    padding-bottom: 25px;
}
.ad-style__7 .goods-ad-warp .goods-ad-con {
    display: flex;
    flex-wrap: nowrap;
}
.ad-style__7 .goods-ad-warp .goods-ad-con {
    display: block;
    flex-wrap: nowrap;
    padding: 0 !important;
    height: 100%;
}
.ad-style__7 .goods-ad-warp swiper {
    height: 200px;
    position: relative;
    overflow: hidden;
}
.ad-style__7 .goods-ad-warp .goods-ad-con .goods-ad-item {
    width: 33.333%;
}
.ad-style__7 .goods-ad-warp .goods-ad-item .item-info .item-buy {
    display: none;
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
    -webkit-transform: scale(0.5);
    transform: scale(0.5);
    -webkit-transform-origin: 0 0;
    transform-origin: 0 0;
    content: "";
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
    -webkit-transform: scale(0.5);
    transform: scale(0.5);
    -webkit-transform-origin: 0 0;
    transform-origin: 0 0;
    content: "";
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
// 标准样式 (ad-goods_style__1) - 默认白底，无需额外样式

// 投影样式 (ad-goods_style__2)
.ad-goods_style__2 {
    .goods-ad-warp {
        .goods-ad-con {
            .goods-ad-item {
                box-shadow: 0 2px 4px rgba(0, 0, 0, 0.06) !important;
                .item-con {
                    box-shadow: none;
                }
            }
        }
    }
    .mod_groupon .groupon_list .proditem {
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.06) !important;
    }
}

// 描边样式 (ad-goods_style__3)
.ad-goods_style__3 {
    .goods-ad-warp {
        .goods-ad-con {
            .goods-ad-item {
                border: 1px solid rgba(50, 50, 51, 0.1) !important;
                .item-con {
                    border: none;
                }
            }
        }
    }
    .mod_groupon .groupon_list .proditem {
        border: 1px solid rgba(50, 50, 51, 0.1) !important;
    }
}

// 透明样式 (ad-goods_style__4)
.ad-goods_style__4 {
    .mod_groupon .groupon_list .groupon-item__name {
        padding: 0 !important;
    }
    .mod_groupon .groupon_list .groupon-item__price {
        padding: 0 !important;
    }
    .goods-ad-warp {
        .goods-ad-item {
            background: transparent !important;
            .item-content,
            .item-con {
                background: transparent !important;
            }
            .groupon-item-image-wrapper {
                background: transparent !important;
            }
            .groupon-item__name,
            .groupon-item__price,
            .price-origin {
                background: transparent !important;
            }
        }
    }
    .mod_groupon .groupon_list .proditem {
        background: transparent !important;
        .groupon-item-image-wrapper {
            background: transparent !important;
        }
        .groupon-item__name,
        .groupon-item__price,
        .price-origin {
            background: transparent !important;
        }
    }
    // 圆角样式组合时也保持透明
    &.ad-goods_radio_style__2 {
        .goods-ad-warp .goods-ad-con .goods-ad-item {
            background: transparent !important;
            .groupon-item__name,
            .groupon-item__price,
            .price-origin {
                background: transparent !important;
            }
        }
        .mod_groupon .groupon_list .proditem {
            background: transparent !important;
            .groupon-item__name,
            .groupon-item__price,
            .price-origin {
                background: transparent !important;
            }
        }
    }
}
.ad-goods_radio_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item {
    background-color: #ffffff;
    border-radius: 6px;
    overflow: hidden;
}
.ad-goods_radio_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item .item-content {
    border-radius: 6px;
    overflow: hidden;
}
.ad-goods_radio_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item .item-con {
    border-radius: 6px;
    overflow: hidden;
}
.ad-goods_radio_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item .groupon-item-image-wrapper {
    background-color: transparent;
}
.ad-goods_radio_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item .groupon-item-image {
    border-radius: 6px 6px 0 0;
    overflow: hidden;
}
.ad-goods_radio_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item .groupon-item__name,
.ad-goods_radio_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item .groupon-item__price,
.ad-goods_radio_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item .price-origin {
    background-color: #ffffff;
}
.ad-goods_radio_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item .groupon-item__price,
.ad-goods_radio_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item .price-origin {
    border-radius: 0 0 6px 6px;
}
/* 横向滚动布局（style 6）的圆角配置 */
.ad-goods_radio_style__2 .groupon_list .proditem {
    background-color: #ffffff;
    border-radius: 6px;
    overflow: hidden;
}
.ad-goods_radio_style__2 .groupon_list .proditem .groupon-item-image-wrapper {
    background-color: transparent;
}
.ad-goods_radio_style__2 .groupon_list .proditem .groupon-item-image {
    border-radius: 6px 6px 0 0;
    overflow: hidden;
}
.ad-goods_radio_style__2 .groupon_list .proditem .groupon-item__name {
    background-color: #ffffff;
}
.ad-goods_radio_style__2 .groupon_list .proditem .groupon-item__price {
    background-color: #ffffff;
    border-radius: 0 0 6px 6px;
}
.ad-goods_radio_style__2 .groupon_list .proditem .price-origin {
    background-color: #ffffff;
    border-radius: 0 0 6px 6px;
}
.ad-text_align__2 .goods-ad-warp .goods-ad-con .goods-ad-item {
    text-align: center;
}
.ad-text_align__2 .goods-ad-warp .goods-ad-item .item-info .item-buy {
    height: auto;
    padding-bottom: 15px;
}
.ad-text_align__2 .goods-ad-warp .goods-ad-item .item-info .item-action {
    flex-wrap: wrap;
    justify-content: normal;
}
.ad-text_align__2 .goods-ad-warp .goods-ad-item .item-info .item-action .item-price {
    width: 100%;
}
.ad-text_align__2 .goods-ad-warp .goods-ad-item .item-info .item-action .item-buy {
    width: 100%;
}
.ad-text_weight__2 .goods-ad-warp .goods-ad-item .item-info .item-name .item-name-a {
    font-weight: bold;
}
.ad-goods_name_row__1 .goods-ad-warp .goods-ad-item .item-info .item-name .item-name-a {
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
.ad-goods_name_row__2 .goods-ad-warp .goods-ad-item .item-info .item-name .item-name-a {
    line-height: 20px;
    height: 40px;
    text-overflow: ellipsis;
    display: -webkit-box;
    display: -moz-box;
    -webkit-line-clamp: 2;
    -moz-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    -moz-box-orient: vertical;
    overflow: hidden;
}
.ad-goods_name_padding__0 .goods-ad-warp .goods-ad-item .groupon-item__name {
    margin-left: 0;
    margin-right: 0;
}
.ad-goods_name_padding__0 .goods-ad-warp .goods-ad-item .groupon-item__price {
    padding-left: 0;
    padding-right: 0;
}
.ad-goods_name_padding__1 .goods-ad-warp .goods-ad-item .price-origin {
    padding-left: 1.5px;
    padding-right: 10px;
}
.ad-goods_name_padding__0 .groupon_list .groupon-item__name {
    margin-left: 0 !important;
    margin-right: 0 !important;
}
.ad-goods_name_padding__0 .groupon_list .groupon-item__price {
    padding-left: 0 !important;
    padding-right: 0 !important;
}
.ad-goods_name_padding__0 .groupon_list .price-origin {
    padding-left: 0 !important;
    padding-right: 0 !important;
}
//.ad-goods_name_padding__1 .groupon_list .groupon-item__name {
//    margin-left: 10px !important;
//    margin-right: 10px !important;
//}
.ad-goods_name_padding__1 .groupon_list .groupon-item__price {
    padding-left: 10px !important;
    padding-right: 10px !important;
}
.ad-goods_name_padding__1 .groupon_list .price-origin {
    padding-left: 10px !important;
    padding-right: 10px !important;
}
.ad-goods-title_style__3 .image-ad-title {
    padding-bottom: 28px !important;
}
.ad-goods-title_style__3 .goods-ad-warp {
    border-top-left-radius: 15px;
    margin-top: -25px !important;
    overflow: hidden;
    background: #fff;
    padding-top: 10px;
}

/*拼团*/
.module-seckill-goods_ad .cap-seckill-goods__tag {
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
}
.module-seckill-goods_ad .cap-seckill-goods__tag .cap-seckill-goods__tag-title {
    font-weight: 700;
    font-size: 20px;
    float: left;
}
.module-seckill-goods_ad .cap-seckill-goods__tag .cap-seckill-goods__countdown rich-text {
    font-weight: bold;
}
.module-seckill-goods_ad .cap-seckill-goods__tag .cap-seckill-goods__countdown rich-text {
    font-size: 12px;
    text-align: center;
}
.module-seckill-goods_ad .cap-seckill-goods__tag.big {
    height: 40px;
    line-height: 40px;
}
.module-seckill-goods_ad .cap-seckill-goods__tag.big .cap-seckill-goods__tag-countdown {
    float: right;
    height: 40px;
    font-size: 12px;
    line-height: 16px;
}
.module-seckill-goods_ad .cap-seckill-goods__tag.big .cap-seckill-goods__tag-countdown-text {
    display: block;
    height: 16px;
    margin: 5px 0 0;
    line-height: 16px;
    text-align: left;
    opacity: 0.6;
}
.module-seckill-goods_ad .cap-seckill-goods__tag.big .cap-seckill-goods__countdown {
    margin-top: 0;
}
.module-seckill-goods_ad .cap-seckill-goods__tag.big .cap-seckill-goods__countdown text {
    padding: 0 2px;
}
.module-seckill-goods_ad .cap-seckill-goods__tag.list .cap-seckill-goods__tag-countdown,
.module-seckill-goods_ad .cap-seckill-goods__tag.small .cap-seckill-goods__tag-countdown {
    font-size: 12px;
    display: -webkit-box;
    display: -webkit-flex;
    display: flex;
    white-space: nowrap;
    -webkit-box-pack: justify;
    -webkit-justify-content: space-between;
    justify-content: space-between;
    height: auto;
}
.module-seckill-goods_ad .cap-seckill-goods__tag.list,
.module-seckill-goods_ad .cap-seckill-goods__tag.small {
    height: auto;
    padding: 5px 10px;
}
.module-seckill-goods_ad .cap-seckill-goods__tag.list .cap-seckill-goods__tag-countdown-text,
.module-seckill-goods_ad .cap-seckill-goods__tag.small .cap-seckill-goods__tag-countdown-text {
    display: block;
    height: 24px;
    line-height: 24px;
}

.item-info .item-name {
    .group-num {
        flex-shrink: 0;
        float: left;
        display: inline-flex;
        align-items: baseline;
        margin-right: 5px;
        padding-top: 1.5px;
    }

    .group-num-text {
        color: var(--main-bg) !important;
        background-color: transparent;
        border: 1px solid var(--main-bg);
        padding: 2px 4.5px;
        border-radius: 2px;
        font-size: 12px;
        line-height: 12px;
        font-weight: normal;
        white-space: nowrap;
        box-sizing: border-box;
        display: inline-flex;
    }
}

.product-desc {
    margin-top: 2px;
}

.source_price {
    color: #ccc;
    font-size: 12px !important;
    margin-left: 4px;
    text-decoration: line-through;
    margin-top: 5px;
}

.ad-style__1 {
    .countdown {
        position: relative;
        display: flex;
        flex-direction: column;
        line-height: 15px;
        align-items: flex-start;

        .countdown__title {
            font-size: 12px;
            color: rgb(229 229 229 / 70%);
            padding-left: 3.5px;
        }
    }
}

.ad-style__2,
.ad-style__6 {
    .cap-seckill-goods__tag.list {
        padding: 0 5px;
    }

    .countdown {
        position: relative;
        display: flex;
        align-items: center;
        justify-content: space-between;
        flex: 1;
        white-space: nowrap;
        margin: 0;

        .countdown__title {
            font-size: 11px;
            color: rgb(229 229 229 / 70%);
        }
    }
}

.item-action {
    margin-top: 5px;
}

.ad-style__6 .goods-ad-item .item-content {
    min-height: 150px;
}

/* ========== 购买按钮样式 ========== */
.price-wrapper {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
}

// 购买按钮通用样式
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
        color: var(--main-bg);
        cursor: pointer;
        &:hover {
            color: var(--primary-dark-2);
        }
        &.disabled {
            opacity: 0.5;
            cursor: not-allowed;
        }
    }
    .button-ico {
        display: flex;
        justify-content: center;
        align-items: center;
        cursor: pointer;

        .iconfont-pc {
            font-size: 24px;
            color: var(--main-bg);
            transition: color 0.2s ease;

            &:hover {
                color: var(--primary-dark-2);
            }
        }

        .el-button {
            font-size: 12px;
        }
    }
}

// 隐藏购买按钮
.ad-buy_btn_style__0 {
    .goods-ad-warp .goods-ad-item .buy,
    .mod_groupon .groupon_list .proditem .buy {
        display: none;
    }
}

// 样式1-4：图标按钮样式
.ad-buy_btn_style__1,
.ad-buy_btn_style__2,
.ad-buy_btn_style__3,
.ad-buy_btn_style__4 {
    .goods-ad-warp .goods-ad-item .buy,
    .mod_groupon .groupon_list .proditem .buy {
        :deep(.button-ico) {
            .iconfont-pc {
                color: var(--main-bg);
                font-size: 24px;
            }
        }
    }
}

// 样式5和7：边框按钮样式（预留扩展）
// .ad-buy_btn_style__5,
// .ad-buy_btn_style__7 {}

// 样式6和8：圆角填充按钮样式（预留扩展）
// .ad-buy_btn_style__6,
// .ad-buy_btn_style__8 {}

/* 居中对齐时的价格和购买按钮样式 - flex布局 */
.ad-text_align__2 .goods-ad-warp .goods-ad-item .groupon-item__price {
    align-items: center;
    justify-content: center;
}
.ad-text_align__2 .goods-ad-warp .goods-ad-item .groupon-item__price > .buy {
    margin-left: 0;
    margin-top: 8px;
}
/* 居中对齐时的价格和购买按钮样式 - swiper布局 */
.ad-text_align__2 .groupon_list .proditem .groupon-item__price {
    align-items: center;
    justify-content: center;
}
.ad-text_align__2 .groupon_list .proditem .groupon-item__price .buy {
    margin-left: 0;
    margin-top: 8px;
}
</style>
