<template>
    <div v-if="seckillList && seckillList?.length > 0" class="module-seckill-goods_ad-wrapper">
        <div
            :style="
                frameFormat.boxPadding +
                frameFormat.boxPaddingTop +
                frameFormat.boxPaddingBottom +
                frameFormat.contentPaddingX +
                allFormat.boxRadius +
                ' ' +
                defaultBgStyle +
                frameFormat.backgroundColor +
                allFormat.backgroundColor +
                ' ' +
                frameFormat.boxRadius +
                allFormat.boxRadius
            "
            style="position: relative"
            :class="
                'module-ad-con module-goods_ad module-seckill-goods_ad ad-style__' +
                (module?.style ?? 6) +
                ' ad-buy_btn_style__' +
                resolvedBuyBtnStyle +
                ' ad-goods_style__' +
                module?.goodsStyle +
                ' ad-goods_radio_style__' +
                module?.goodsRadioStyle +
                ' ad-text_align__' +
                (module?.textAlign ?? 1) +
                ' ad-text_weight__' +
                module?.textWeight +
                ' ad-goods_name_row__' +
                module?.goodsNameRow +
                ' ad-goods_name_padding__' +
                module?.goodsNamePadding +
                ' ad-goods-title_style__' +
                titleCfg.titleStyle
            "
        >
            <div class="module-ad-content">
                <div v-if="seckillList.length > 0" class="mod_seckill">
                    <!-- 标题和倒计时一起显示隐藏 -->
                    <div
                        v-if="titleCfg.showTitle == 1"
                        class="seckill-top"
                        :style="{
                            background: titleCfg.titleBackground2
                                ? 'linear-gradient(to right, ' + (titleCfg.titleBackground || '') + ', ' + titleCfg.titleBackground2 + ')'
                                : titleCfg.titleBackground || '',
                            borderTopLeftRadius: titleCfg.titleRadius ? titleCfg.titleRadius + 'px' : '',
                            borderTopRightRadius: titleCfg.titleRadius ? titleCfg.titleRadius + 'px' : '',
                            paddingTop: (titleCfg.titlePaddingTop ?? 20) + 'px',
                            paddingBottom: (titleCfg.titlePaddingBottom ?? 20) + 'px'
                        }"
                    >
                        <div class="seckill-title-wrapper">
                            <div v-if="titleCfg.titleStyle == 3" class="line"></div>
                            <el-image
                                v-if="titleCfg.titleBackgroundPic?.picUrl"
                                class="image-size"
                                :src="imageFormat(titleCfg.titleBackgroundPic.picUrl)"
                                alt=""
                            ></el-image>
                            <div class="seckill-title" :style="{ color: titleCfg.titleColor || '#1a1a1a' }">
                                {{ titleCfg.titleText ? titleCfg.titleText : $t("限时秒杀") }}
                            </div>
                            <div
                                v-if="titleCfg.descText"
                                class="seckill-desc"
                                :style="{ color: titleCfg.descColor || '#1a1a1a' }"
                                :class="[titleCfg.titleStyle == 2 ? 'big-sm' : 'sm']"
                            >
                                {{ $t(titleCfg.descText) }}
                            </div>
                        </div>
                        <div v-if="titleMore.visible" class="seckill-top-right" :style="{ color: titleCfg.moreColor || '#1a1a1a' }">
                            <!-- 倒计时：无箭头、不跳转 -->
                            <div v-if="titleMore.showCountdown && getTime > 0" class="seckill-count-down">
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
                                :to="seckillMoreLink"
                                class="seckill-more-link"
                                :style="{ color: titleCfg.moreColor || '#1a1a1a' }"
                            >
                                <span>{{ $t(moreLinkText) }}</span>
                                <SvgIcon class="icon" name="arrow-right" width="16" height="16" />
                            </NuxtLink>
                        </div>
                    </div>
                    <div class="seckill_list_wrap">
                        <!-- 使用 swiper 横向滑动布局（style 6） -->
                        <div v-if="seckillList.length > 0 && useSwiperLayout" class="seckill_list">
                            <swiper :item-list="seckillList" :swiper-option="seckillProductWrap" style="width: 100%">
                                <template #default="{ item }">
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
                                                        fit="contain"
                                                        class="pro_pic"
                                                        loading="lazy"
                                                        style="transition: opacity 0.2s linear"
                                                        :alt="item.productName"
                                                    >
                                                        <template #placeholder>
                                                            <div class="seckill-img-placeholder"></div>
                                                        </template>
                                                        <template #error>
                                                            <div class="seckill-img-placeholder"></div>
                                                        </template>
                                                    </el-image>
                                                </div>
                                                <div v-if="item?.productPrice || item?.marketPrice" class="seckill-item-discount">
                                                    {{
                                                        $t("优惠{0}%", [getDiscount(Number(item.seckillPrice), Number(item.productPrice ?? item.marketPrice))])
                                                    }}
                                                </div>
                                            </div>
                                            <template v-if="isShowName">
                                                <NuxtLink
                                                    :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.skuSn || item.productSn })"
                                                    target="_blank"
                                                >
                                                    <div
                                                        class="seckill-item__name"
                                                        :style="{
                                                            textAlign: textAlignNumber === 1 ? 'left' : 'center',
                                                            fontWeight: module?.textWeight == 1 ? 'normal' : 'bold',
                                                            padding: module.goodsNamePadding == 1 ? '0 10px' : ''
                                                        }"
                                                    >
                                                        {{ item.productName }}
                                                    </div>
                                                </NuxtLink>
                                            </template>
                                        </NuxtLink>
                                        <template v-if="isShowPrice">
                                            <div
                                                class="price-container"
                                                :style="{
                                                    flexDirection: textAlignNumber === 1 ? 'row' : 'column',
                                                    alignItems: textAlignNumber === 1 ? 'flex-start' : 'center',
                                                    padding: module.goodsNamePadding == 1 ? '0 10px' : ''
                                                }"
                                            >
                                                <div class="seckill-item__price">
                                                    <div class="price-stack">
                                                        <FormatPrice v-model="item.seckillPrice" class="price price-miaosha" />
                                                        <template
                                                            v-if="
                                                                (item?.productPrice && Number(item.productPrice) > 0) ||
                                                                (item?.marketPrice && Number(item.marketPrice) > 0)
                                                            "
                                                        >
                                                            <FormatPrice :model-value="item.productPrice ?? item.marketPrice" class="price price-origin" />
                                                        </template>
                                                    </div>
                                                </div>
                                                <div class="buy-container">
                                                    <div class="buy">
                                                        <div v-if="isReserveProduct(item) || hasMultiSeckillSku(item)">
                                                            <NuxtLink
                                                                :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                                                target="_blank"
                                                                @click.stop
                                                            >
                                                                <template v-if="useRoundSvgBuyBtn">
                                                                    <div class="view_detail">
                                                                        <SvgIcon name="add-bold" width="32" height="32" />
                                                                    </div>
                                                                </template>
                                                                <template v-else>
                                                                    <ProductButton :buy-btn-style="resolvedBuyBtnStyle"></ProductButton>
                                                                </template>
                                                            </NuxtLink>
                                                        </div>
                                                        <div v-else>
                                                            <ProductBuy
                                                                :id="item.productId"
                                                                :sku-id="item?.skuId ? String(item.skuId) : undefined"
                                                                :is-quick="false"
                                                                :disabled="
                                                                    (item?.isSeckill == 0 && item?.productStock == 0) ||
                                                                    (item?.isSeckill == 1 && item?.seckillStock == 0)
                                                                "
                                                                :number="1"
                                                                @click.stop
                                                                @callback="$emit('callback')"
                                                            >
                                                                <div
                                                                    class="view_detail"
                                                                    :class="{
                                                                        disabled:
                                                                            (item?.isSeckill == 0 && item?.productStock == 0) ||
                                                                            (item?.isSeckill == 1 && item?.seckillStock == 0)
                                                                    }"
                                                                >
                                                                    <template v-if="useRoundSvgBuyBtn">
                                                                        <SvgIcon name="add-bold" width="32" height="32" />
                                                                    </template>
                                                                    <template v-else>
                                                                        <ProductButton :buy-btn-style="resolvedBuyBtnStyle"></ProductButton>
                                                                    </template>
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
                        <div v-else-if="seckillList.length > 0" class="goods-ad-warp">
                            <div class="goods-ad-con" :style="{ gap: goodsGapNumber + 'px' }">
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
                                                            fit="contain"
                                                            class="item-image pro_pic"
                                                            :alt="item.productName"
                                                        >
                                                            <template #placeholder>
                                                                <div class="seckill-img-placeholder"></div>
                                                            </template>
                                                            <template #error>
                                                                <div class="seckill-img-placeholder"></div>
                                                            </template>
                                                        </el-image>
                                                    </NuxtLink>
                                                </div>
                                                <div v-if="item?.productPrice || item?.marketPrice" class="seckill-item-discount">
                                                    {{
                                                        $t("优惠{0}%", [getDiscount(Number(item.seckillPrice), Number(item.productPrice ?? item.marketPrice))])
                                                    }}
                                                </div>
                                            </div>
                                            <div class="item-info" :style="{ padding: module.goodsNamePadding == 1 ? '0 10px' : '' }">
                                                <template v-if="isShowName">
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
                                                <template v-if="isShowPrice">
                                                    <div class="price-container">
                                                        <div class="item-action seckill-item__price">
                                                            <div class="price-stack">
                                                                <div class="item-price">
                                                                    <FormatPrice v-model="item.seckillPrice" class="price price-miaosha" />
                                                                </div>
                                                                <template v-if="item?.marketPrice && Number(item.marketPrice) > 0">
                                                                    <FormatPrice :model-value="item.marketPrice" class="price price-origin" />
                                                                </template>
                                                            </div>
                                                        </div>
                                                        <div class="buy-container">
                                                            <div class="item-buy">
                                                                <div v-if="isReserveProduct(item) || hasMultiSeckillSku(item)">
                                                                    <NuxtLink
                                                                        :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                                                        target="_blank"
                                                                    >
                                                                        <template v-if="useRoundSvgBuyBtn">
                                                                            <div class="view_detail">
                                                                                <SvgIcon name="add-bold" width="32" height="32" />
                                                                            </div>
                                                                        </template>
                                                                        <template v-else>
                                                                            <ProductButton :buy-btn-style="resolvedBuyBtnStyle"></ProductButton>
                                                                        </template>
                                                                    </NuxtLink>
                                                                </div>
                                                                <template v-else>
                                                                    <ProductBuy
                                                                        :id="item.productId"
                                                                        :sku-id="item?.skuId ? String(item.skuId) : undefined"
                                                                        :is-quick="false"
                                                                        :disabled="
                                                                            (item?.isSeckill == 0 && item?.productStock == 0) ||
                                                                            (item?.isSeckill == 1 && item?.seckillStock == 0)
                                                                        "
                                                                        :number="1"
                                                                        @click.stop
                                                                        @callback="$emit('callback')"
                                                                    >
                                                                        <div
                                                                            class="view_detail"
                                                                            :class="{
                                                                                disabled:
                                                                                    (item?.isSeckill == 0 && item?.productStock == 0) ||
                                                                                    (item?.isSeckill == 1 && item?.seckillStock == 0)
                                                                            }"
                                                                        >
                                                                            <template v-if="useRoundSvgBuyBtn">
                                                                                <SvgIcon name="add-bold" width="32" height="32" />
                                                                            </template>
                                                                            <template v-else>
                                                                                <ProductButton :buy-btn-style="resolvedBuyBtnStyle"></ProductButton>
                                                                            </template>
                                                                        </div>
                                                                    </ProductBuy>
                                                                </template>
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
                            <div v-for="index in 6" :key="index" class="proditem" style="width: 200px">
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
// ===================== Vue 核心 =====================
import { computed } from "vue";

// ===================== 组件导入 =====================
import CountDown from "~/components/common/CountDown.vue";
import ProductBuy from "~/components/product/Buy.vue";
import HomeSkeleton from "~/components/home/Skeleton.vue";
import ProductButton from "~/components/shop/product/ProductButton.vue";

// ===================== API 导入 =====================
// NexTrade 无 ~/api 桶文件：秒杀取数直接用 home/home 的 getHomeSeckill（home/home/getSeckill）
import { getHomeSeckill } from "~/api/home/home";

// ===================== 类型导入 =====================
import type { SeckillFilterState } from "~/types/home/seckill";
import type { PcSeckillModuleDecor, PcSeckillTitleDecor } from "~/types/decoration/pcSeckillModule";

// NexTrade 未单独定义“前台秒杀商品行”类型，这里用宽松类型承接接口返回的扩展字段
type PcSeckillProductRow = SeckillFilterState & { [key: string]: any };

// ===================== 工具函数 =====================
import { imageFormat, productImage, urlFormat } from "~/utils/format";
import { resolveCountdownLabel, resolveMoreLinkTitle, resolveTitleMore } from "~/decorate/titleMore";
import { calcDiscountPercent } from "~/utils/product";

// ===================== Props & Emits =====================
const props = withDefaults(
    defineProps<{
        module?: PcSeckillModuleDecor;
    }>(),
    {
        module: () => ({})
    }
);

defineEmits(["callback"]);

const { frame } = props.module;

// 购买按钮样式：后台未配置时经常下发 0，且默认的 icon-tianjia 可能不存在于字体文件中。
// 秒杀模块这里兜底为 “3=加号”，确保前台默认有“加入购物车”按钮可用。
const resolvedBuyBtnStyle = computed(() => {
    const raw = props.module?.buyBtnStyle;
    const n = Number(raw);
    if (Number.isFinite(n) && n > 0) return n;
    return 3;
});
// 对齐“推荐商品卡片”的默认加购样式：圆形底 + SVG 加号
// - buyBtnStyle 1~4（图标类）：使用 SVG add（推荐商品同款）
// - buyBtnStyle 5~8（文字按钮类）：沿用后台配置的 ProductButton
const useRoundSvgBuyBtn = computed(() => Number(resolvedBuyBtnStyle.value) < 5);

// 装修/活动模块未配置 style 时，默认使用 6（swiper 横向滚动）
const styleNumber = computed(() => {
    const raw = props.module?.style as number | string | undefined | null;
    if (raw === undefined || raw === null || raw === "") return 6;
    const n = Number(raw);
    return Number.isFinite(n) ? n : 6;
});

// 装修/活动模块未配置 textAlign 时，默认居左（1=左，2=中）
const textAlignNumber = computed(() => {
    const raw = props.module?.textAlign as number | string | undefined | null;
    if (raw === undefined || raw === null || raw === "") return 1;
    const n = Number(raw);
    return Number.isFinite(n) ? n : 1;
});

// 标题配置兼容：优先使用 module.title（新装修结构），否则使用平铺字段（如 seckillTitle）
// 目标：标题默认显示
const titleCfg = computed(() => {
    const data: PcSeckillModuleDecor = props.module ?? {};
    const t: PcSeckillTitleDecor = data.title ?? {};
    return {
        showTitle: t?.showTitle ?? 1,
        titleStyle: t?.titleStyle ?? 1,
        // seckillTitle 作为主标题（兼容 pc-home-seckill 的 module 结构）
        titleText: t?.titleText ?? data?.seckillTitle ?? "",
        // 副标题仅来自 title.descText（未配置则不展示）
        descText: t?.descText ?? "",
        descColor: t?.descColor ?? "",
        titleColor: t?.titleColor ?? "",
        titleRadius: t?.titleRadius ?? 0,
        titlePaddingTop: t?.titlePaddingTop ?? 20,
        titlePaddingBottom: t?.titlePaddingBottom ?? 20,
        titleBackground: t?.titleBackground ?? "",
        titleBackground2: t?.titleBackground2 ?? "",
        titleBackgroundPic: t?.titleBackgroundPic ?? null,
        showMore: t?.showMore ?? 0,
        moreType: t?.moreType,
        countdownLabel: t?.countdownLabel ?? "",
        moreLink: t?.moreLink ?? null,
        moreColor: t?.moreColor ?? ""
    };
});

// 标题右侧区（更多链接 / 倒计时）
const titleMore = computed(() => resolveTitleMore(titleCfg.value, "countdown"));
const countdownLabel = computed(() => resolveCountdownLabel(titleCfg.value, "倒计时"));
const moreLinkText = computed(() => resolveMoreLinkTitle(titleCfg.value, "更多"));

// showName/showPrice 未配置时默认展示；只有明确为 0 时隐藏
const isShowName = computed(() => {
    const v = props.module?.showName as number | string | undefined | null;
    if (v === undefined || v === null || v === "") return true;
    return Number(v) === 1;
});
const isShowPrice = computed(() => {
    const v = props.module?.showPrice as number | string | undefined | null;
    if (v === undefined || v === null || v === "") return true;
    return Number(v) === 1;
});

const goodsGapNumber = computed(() => {
    const rawGap = props.module?.goodsGap as number | string | undefined | null;
    if (rawGap !== undefined && rawGap !== null && rawGap !== "") {
        const n = Number(rawGap);
        return Number.isFinite(n) ? n : 10;
    }

    const rawPadding = props.module?.goodsPadding as number | string | undefined | null;
    if (rawPadding === undefined || rawPadding === null || rawPadding === "") return 10;
    const n = Number(rawPadding);
    return Number.isFinite(n) ? n : 10;
});

// 格式化 frame 配置
const frameFormat = computed(() => {
    const frameData = frame ?? {};
    const marginTop = frameData.boxPaddingTop ?? props.module.boxPaddingTop ?? 0;
    const marginBottom = frameData.boxPaddingBottom ?? props.module.boxPaddingBottom ?? 0;
    const outerPadding = frameData.boxPadding ?? props.module.boxPadding ?? 0;
    // 左右内边距：仅作用于内容区（标题栏 + 商品列表），通过 CSS 变量下发，
    // 与后台预览（--dec-inner-padding）口径一致；左右边距（boxPadding）单独作用于容器左右 padding。
    const innerPadding = frameData.leftAndRightPadding ?? props.module.innerPadding ?? 0;
    return {
        boxPadding: `padding-left: ${outerPadding}px; padding-right: ${outerPadding}px;`,
        boxPaddingTop: `margin-top: ${marginTop}px;`,
        boxPaddingBottom: `margin-bottom: ${marginBottom}px;`,
        contentPaddingX: `--seckill-content-padding-x: ${innerPadding}px;`,
        backgroundColor: frameData.backgroundColor ? `background-color: ${frameData.backgroundColor};` : "",
        boxRadius: frameData.boxRadius ? `border-radius: ${frameData.boxRadius}px;` : ""
    };
});

// 格式化 all 配置
const allFormat = computed(() => {
    return {
        backgroundColor: props.module.backgroundColor ? `background-color: ${props.module.backgroundColor};` : "",
        boxRadius: props.module.boxRadius ? `border-radius: ${props.module.boxRadius}px;` : "",
        goodsPadding: props.module.goodsPadding ? `padding: ${props.module.goodsPadding}px;` : ""
    };
});

// 默认背景色：当“模块背景色”和“frame 背景色”都未配置时，给秒杀模块一个浅灰底
// 这样首页/装修未配置背景时默认就是 #fafafa，但不会覆盖后台配置的颜色
const defaultBgStyle = computed(() => {
    const moduleBg = props.module?.backgroundColor;
    const frameBg = frame?.backgroundColor;
    if (moduleBg || frameBg) return "";
    return "background-color: #fafafa;";
});

const getDiscount = calcDiscountPercent;

const isReserveProduct = (item: PcSeckillProductRow): boolean => Number(item?.productType ?? 1) !== 1;

const hasMultiSeckillSku = (item: PcSeckillProductRow): boolean => {
    const skuList = item?.seckkillData?.seckillItem;
    return Array.isArray(skuList) && skuList.length > 1;
};

// 判断是否使用 swiper 布局
const useSwiperLayout = computed(() => {
    // style 6 使用 swiper 横向滑动，style 1 和 2 使用 flex 布局
    return styleNumber.value === 6;
});

// 根据 style 获取 swiper 配置
const getSwiperConfig = computed(() => {
    const style = styleNumber.value;
    // style 6 使用横向滑动；即使只有 1 个商品，也保持一屏 6 个的卡片尺寸（不拉伸占满）
    const slidesPerView = 6;
    // 商品中间间隔为边距的两倍
    const spaceBetween = goodsGapNumber.value;

    // 数量不足时关闭 loop，避免渲染出重复商品
    const canLoop = style === 6 && seckillList.value.length >= slidesPerView * 2;

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
    // 商品中间间隔为边距的两倍，使用 gap 实现
    // gap 是商品之间的间隔，需要在宽度计算中减去相应的 gap
    const gap = goodsGapNumber.value;
    const style = styleNumber.value;

    switch (style) {
        case 1: {
            // 一行4个，每个25%；4个商品之间有3个间隔
            if (gap > 0) {
                const totalGap = (gap * 3) / 4;
                return `calc(25% - ${totalGap}px)`;
            }
            return "25%";
        }
        case 2: {
            // 一行5个，每个20%；5个商品之间有4个间隔
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

// ===================== SSR 数据获取 =====================
const { data: seckillData } = await useAsyncData("pc-seckill-list", () => getHomeSeckill(), { default: () => ({ records: [] as SeckillFilterState[] }) });
const seckillList = computed((): PcSeckillProductRow[] => (seckillData.value?.records ?? []) as PcSeckillProductRow[]);

const getTime = computed(() => {
    if (seckillList.value && seckillList.value.length > 0) {
        const firstItem = seckillList.value[0];
        if (firstItem?.seckkillData?.seckillEndTime) {
            return new Date(firstItem.seckkillData.seckillEndTime).getTime();
        }
        if (firstItem?.seckillEndTime) {
            return new Date(firstItem.seckillEndTime).getTime();
        }
    }
    return 0;
});

// 更多/倒计时点击固定跳转「秒杀专区」（后台不再提供链接地址配置，仅保留链接文字与颜色）
// 未配置链接地址时兜底跳「秒杀专区」
const seckillMoreLink = computed(() => urlFormat(props.module?.title?.moreLink) || "/seckill/list");
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
    padding-bottom: 20px;
    // 商品内容区左右留白（不影响 swiper 左右按钮定位）
    // 取值来源：容器内联 --seckill-content-padding-x（= 后台“左右内边距”leftAndRightPadding）；
    // 不在此处硬编码为 0，否则会覆盖继承值导致左右内边距不生效。

    // 商品列表容器
    // 顶部区域（标题和倒计时）
    .seckill-top {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0 var(--seckill-content-padding-x, 0);
        box-sizing: border-box;
        margin-bottom: 0;
        .seckill-title-wrapper {
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
            .seckill-title {
                font-size: 24px;
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
        .seckill-count-down {
            display: flex;
            align-items: center;
            font-size: 18px;
            // 继承 .seckill-top-right 的内联 moreColor（后台“文字颜色”），不要硬编码覆盖
            color: inherit;
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
        padding: 0;
    }

    .seckill_list {
        position: relative;
        z-index: 1;
        height: 100%;
        overflow: hidden;
        width: 100%;
        box-sizing: border-box;
        // 内容区左右留白，确保 swiper 计算宽度与可视区域一致
        padding: 0 var(--seckill-content-padding-x);

        :deep(.swiper) {
            width: 100%;
            padding: 0;
        }

        :deep(.swiper-wrapper) {
            box-sizing: border-box;
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
            object-fit: contain;
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

                // view_detail 的通用样式提升到下方（兼容 swiper/flex 两种布局）
            }
        }
    }
}

// ========= 加购按钮（对齐推荐商品 ProductCard.vue 的 view_detail） =========
// 说明：秒杀模块存在 swiper/flex 两种布局，flex 布局下没有 .buy 包裹，
// 因此这里用更通用的选择器确保两种布局都命中相同样式。
.module-seckill-goods_ad-wrapper {
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
        flex-shrink: 0;

        &:hover {
            color: var(--primary-dark-2);
        }

        &.disabled {
            opacity: 0.5;
            cursor: not-allowed;
        }

        // 确保 svg use/path 继承 currentColor（add.svg 使用 stroke="currentColor"）
        :deep(svg) {
            color: inherit;
        }
    }
}

// 倒计时盒子
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

// 秒杀价格：始终左对齐（不受 textAlign 配置影响）
.module-seckill-goods_ad-wrapper .price-container {
    align-items: flex-start !important;
}

.module-seckill-goods_ad-wrapper .price-stack {
    align-items: flex-start !important;
}

// 价格上下排列（秒杀价在上，原价在下）
.price-stack {
    display: flex;
    flex-direction: column;
    gap: 2px;
    align-items: flex-start;
}

.seckill-item__price .price-miaosha {
    align-items: baseline;
    justify-content: flex-start;
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
    justify-content: flex-start;
    height: 19px;
    color: #888b94 !important;
    text-decoration: line-through;

    :deep(.price-row) {
        line-height: 22px;
        font-weight: 500;
    }

    :deep(.num),
    :deep(.util) {
        color: #888b94 !important;
    }

    :deep(.num) {
        font-size: 16px;
    }

    :deep(.decimal) {
        font-size: 16px;
    }
}

.price-origin {
    box-sizing: border-box;
}

// 商品图缺失/加载失败灰色占位
.seckill-img-placeholder {
    width: 100%;
    height: 100%;
    background-color: #f0f0f0;
}

:deep(.swiper) {
    box-sizing: border-box;
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
    // flex 布局内容区左右留白（与 swiper 保持一致）
    box-sizing: border-box;
    padding: 0 var(--seckill-content-padding-x);
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
            background-color: #f0f0f0;

            .item-image,
            .item-image.pro_pic {
                height: 100%;
                width: 100%;
                object-fit: contain;
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

            .price-stack {
                align-items: center;
            }

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

            .price-stack {
                align-items: center;
            }

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
/*
 * 秒杀模块默认展示“+”加购按钮：
 * 后台很多场景会下发 buyBtnStyle=0（代表未配置/隐藏），但首页秒杀期望默认就能加购。
 * 仅对秒杀模块做兜底，不影响其它商品模块。
 */
.module-seckill-goods_ad.ad-buy_btn_style__0 .goods-ad-warp .goods-ad-item .item-info .item-buy {
    display: flex;
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
            background: #f0f0f0 !important;
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
