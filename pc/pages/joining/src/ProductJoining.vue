<template>
    <div data-panel-rev="c2EyMjgydG4" class="product_joining" :class="{ 'new-ui': uiVersion === 'new', 'overseas-ui': uiVersion === 'overseas' }">
        <!-- ============ 跨境版 UI ============ -->
        <template v-if="uiVersion === 'overseas'">
            <div class="active-box flex" :class="{ 'has-activity-bg': !!activityPcImage }" :style="{ alignItems: 'center', ...overseasActiveBoxStyle }">
                <div class="active-ico flex align-center">
                    <span>{{ $t("拼团") }}</span>
                </div>
                <div class="active-time">
                    <OverseasCountdown :endTime="grouponDetail?.endTime"></OverseasCountdown>
                </div>
            </div>
            <div class="productName">
                <div class="name">{{ product.productName }}</div>
                <p v-if="product.productBrief" class="desc">{{ product.productBrief }}</p>
            </div>
            <div>
                <div class="item">
                    <div class="num">{{ $t("阶梯") }} {{ currentGroupNum }}{{ $t("人团") }}</div>
                    <div class="num">
                        {{ $t("已成团") }}
                        {{ grouponDetail?.groupSuccessCount === 0 || grouponDetail?.groupSuccessCount ? grouponDetail?.groupSuccessCount : "--" }}
                    </div>
                </div>
            </div>
            <div class="price1">
                <FormatPrice
                    v-model="price"
                    :currencyStyle="{ fontSize: '24px', fontWeight: '700' }"
                    :font-style="{ fontSize: '36px', fontWeight: '700' }"
                ></FormatPrice>
            </div>
            <!-- 门店切换（跨境风：与 overseas 店铺信息风格一致） -->
        </template>
        <!-- ============ 老版 / 新版 UI ============ -->
        <template v-else>
            <div class="productName">
                <span class="tagName">{{ $t("拼团") }}</span>
                <span class="name">{{ product.productName }}</span>
            </div>
            <div class="active-box flex justify-between" :style="activeBoxStyle">
                <div class="active-ico flex align-center">
                    <div class="text">{{ $t("拼团优惠") }}</div>
                    <div v-if="grouponDetail?.minGroupNum" class="group-num">{{ currentGroupNum }}{{ $t("人团") }}</div>
                </div>
                <div class="active-time">
                    <CountdownNew countdown-type="specifyTime" :specify-time="new Date(grouponDetail?.endTime)?.getTime()"></CountdownNew>
                </div>
            </div>
            <div class="joiningPrice">
                <div class="joiningPriceLine">
                    <span>{{ canJoinCurrentSku ? $t("拼团价") : $t("原价") }}：</span
                ><FormatPrice
                    :decimalsStyle="{ fontSize: '14px', alignSelf: 'self-end' }"
                    class="priceBox"
                    v-model="price"
                ></FormatPrice>
                </div>
                <div>
                    {{ $t("已成团") }}：{{
                        grouponDetail?.groupSuccessCount === 0 || grouponDetail?.groupSuccessCount ? grouponDetail?.groupSuccessCount : "--"
                    }}
                </div>
            </div>
            <!-- 门店切换 -->
        </template>
        <ToJoiningList
            class="toJoiningListScroll"
            :uiVersion="uiVersion"
            :span="uiVersion === 'overseas' ? 24 : 12"
            :extraAttrIds="attrIds"
            :productId="product.productId"
            :productNumber="productNumber"
            :skuId="route.query.skuId ? Number(route.query.skuId) : undefined"
            :grouponDetail="{ ...grouponDetail, groupNumKey, price }"
        >
            <template #header>
                <div class="toJoiningListHeader">
                    <div class="total">{{ total ? total : 0 }}{{ $t("人在拼单，可直接参与") }}</div>
                    <div class="more" @click="viewMore">{{ $t("查看更多") }} <i class="iconfont-pc icon-a-right-arrow"></i></div>
                </div>
            </template>
        </ToJoiningList>

        <template v-if="skuList.length > 0">
            <ProductSku
                style="padding: 0"
                v-model="attrList"
                :skuList="skuList"
                :checkedValue="checkedValue"
                :productType="productType"
                :uiVersion="uiVersion"
                @change="onProductSkuChange"
            >
            </ProductSku>
        </template>
        <!--groupType 1=普通团， 2=阶梯团-->
        <template v-if="grouponDetail?.groupNums && grouponDetail?.groupNums?.length > 0 && grouponDetail?.groupType === 2 && isJoin === 1">
            <ProductJt
                v-model:groupNums="grouponDetail.groupNums"
                v-model:groupNumKey="groupNumKey"
                :currentGroupNum="currentGroupNum"
                :type="type"
                :uiVersion="uiVersion"
            >
            </ProductJt>
        </template>

        <div class="btns">
            <div class="number-box" :class="{ 'new-ui': uiVersion === 'new' }">
                <div class="title">{{ uiVersion === "new" ? $t("数量") : $t("数量") + "：" }}</div>
                <div class="con">
                    <InputNumber v-if="limitNum" v-model="productNumber" :max="limitNum" :rounded="uiVersion === 'new'"></InputNumber>
                    <InputNumber v-else v-model="productNumber" :rounded="uiVersion === 'new'"></InputNumber>
                    <div class="flex" style="margin-left: 10px">
                        {{ $t("件") }}
                    </div>
                </div>
            </div>
            <!-- 二期会员标签：不在人群范围，数量选择下方红字提示 -->
            <div class="member-restrict-tip" v-if="grouponDetail?.canJoin === false">
                <i class="iconfont-pc icon-tishi"></i>
                <span>{{ $t("该商品仅限指定会员购买") }}</span>
            </div>
            <!-- 跨境版按钮区（单独购买 / 我要开团 / 询价） -->
            <div class="add_cart" v-if="uiVersion === 'overseas'">
                <ProductBuy
                    :shop-id="product.shopId"
                    :extraAttrIds="attrIds"
                    :id="product.productId"
                    :skuId="Number(route.query.skuId)"
                    :number="productNumber"
                    :isQuick="true"
                >
                    <el-button class="btn" type="primary" style="background: #000; border: none">{{ $t("单独购买") }}</el-button>
                </ProductBuy>
                <ProductBuy
                    :shop-id="product.shopId"
                    :extraAttrIds="attrIds"
                    :id="product.productId"
                    :skuId="route.query.skuId ? Number(route.query.skuId) : undefined"
                    :number="productNumber"
                    :isQuick="true"
                    :grouponDetail="{
                        ...grouponDetail,
                        groupNumKey,
                        price,
                        grouponPromotionRecordId: route.query.grouponPromotionRecordId ? Number(route.query.grouponPromotionRecordId) : 0
                    }"
                >
                    <el-button class="startAGroup btn" type="primary" :disabled="!canJoinCurrentSku || grouponDetail?.canJoin === false">{{
                        type === "invited" ? $t("参与拼团") : $t("我要开团")
                    }}</el-button>
                </ProductBuy>
                <template v-if="commonStore.isEnquiry === 1 && (isB2B() || isOverseas() || isS2b2c())">
                    <el-button class="btn soloBuy" style="width: 50%; flex: auto" @click="showEnquiry = !showEnquiry">{{ $t("立即询价") }}</el-button>
                </template>
            </div>
            <!-- 新版 UI 下购买操作统一由悬浮购买栏(ProductFloatBuyBar)承载，隐藏此处冗余按钮 -->
            <template v-if="uiVersion !== 'new' && uiVersion !== 'overseas'">
                <ProductBuy
                    :shop-id="product.shopId"
                    :extraAttrIds="attrIds"
                    :id="product.productId"
                    :skuId="Number(route.query.skuId)"
                    :number="productNumber"
                    :isQuick="true"
                >
                    <el-button class="btn" type="primary" style="background: #000; border: none">{{ $t("单独购买") }}</el-button>
                </ProductBuy>
                <ProductBuy
                    :shop-id="product.shopId"
                    :extraAttrIds="attrIds"
                    :id="product.productId"
                    :skuId="route.query.skuId ? Number(route.query.skuId) : undefined"
                    :number="productNumber"
                    :isQuick="true"
                    :grouponDetail="{
                        ...grouponDetail,
                        groupNumKey,
                        price,
                        grouponPromotionRecordId: route.query.grouponPromotionRecordId ? Number(route.query.grouponPromotionRecordId) : 0
                    }"
                >
                    <el-button class="startAGroup btn" type="primary" :disabled="!canJoinCurrentSku || grouponDetail?.canJoin === false">{{
                        type === "invited" ? $t("参与拼团") : $t("我要开团")
                    }}</el-button>
                </ProductBuy>
                <template v-if="commonStore.isEnquiry === 1 && (isB2B() || isOverseas() || isS2b2c())">
                    <el-button class="soloBuy btn" @click="showEnquiry = !showEnquiry">{{ $t("立即询价") }}</el-button>
                </template>
            </template>
        </div>
        <!-- 更多拼团弹框：append-to-body 避免新版右侧滚动容器/悬浮栏层级遮挡 -->
        <el-dialog :title="$t('更多拼团')" v-model="formModalVisible" width="20%" class="joining-more-dialog" append-to-body :z-index="joiningDialogZIndex">
            <ToJoiningList
                :uiVersion="uiVersion"
                :extraAttrIds="attrIds"
                :productId="product.productId"
                :productNumber="productNumber"
                :skuId="route.query.skuId ? Number(route.query.skuId) : undefined"
                :grouponDetail="{ ...grouponDetail, groupNumKey, price }"
                :isMore="true"
                :product="product"
            >
            </ToJoiningList>
        </el-dialog>

        <ProductEnquiry v-model="showEnquiry" :productId="product.productId"></ProductEnquiry>

        <!-- 跨境版售后服务折叠 -->
        <div class="serviceAction" v-if="uiVersion === 'overseas' && serviceList && serviceList.length > 0">
            <el-collapse v-model="activeName">
                <template v-for="(item, index) in serviceList" :key="index">
                    <el-collapse-item :title="item.productServiceName">
                        <template #icon="{ isActive }">
                            <span class="collapse-icon" :class="{ active: isActive }">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="none" class="arrow-icon">
                                    <path
                                        stroke="#888B94"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-width="1.5"
                                        d="M11.5 6.25 8 9.75l-3.5-3.5"
                                    ></path>
                                </svg>
                            </span>
                        </template>
                        <div class="basic">
                            <div class="item align-start">
                                <div class="tips_i_box">
                                    <span>{{ $t((item as any).productServiceDesc) }} </span>
                                </div>
                            </div>
                        </div>
                    </el-collapse-item>
                </template>
            </el-collapse>
        </div>

        <!-- 悬浮购买栏（拼团）：仅新版 UI -->
        <ProductFloatBuyBar
            v-if="uiVersion === 'new'"
            :product="product"
            :productType="productType"
            :shopId="product.shopId"
            :skuId="route.query.skuId ? Number(route.query.skuId) : undefined"
            :number="productNumber"
            :extraAttrIds="attrIds"
            :isGroupon="true"
            :grouponType="type"
            :isJoin="canJoinCurrentSku"
            :memberRestricted="grouponDetail?.canJoin === false"
            :restrictTip="grouponDetail?.restrictTip || $t('该商品仅限指定会员购买')"
            :grouponBuyDetail="{
                ...grouponDetail,
                groupNumKey,
                price,
                grouponPromotionRecordId: route.query.grouponPromotionRecordId ? Number(route.query.grouponPromotionRecordId) : 0
            }"
        ></ProductFloatBuyBar>
    </div>
</template>

<script setup lang="ts">
import { InputNumber, CountdownNew } from "~/components/product";
import ToJoiningList from "~/pages/joining/src/ToJoiningList.vue";
import { computed, ref, shallowRef, watch } from "vue";
import { imageFormat } from "~/utils/format";
import OverseasCountdown from "~/components/product/overseas/Countdown.vue";
import { getJoiningDetail } from "~/api/joining/joining";
import { getProductSkuDetail } from "~/api/product/product";
import { useCommonStore } from "~/store/common";
import { isO2o } from "~/utils/util";

const productNumber = ref(1);
const attrList = defineModel<any>("attrList");

const formModalVisible = ref<boolean>(false);
const showEnquiry = ref(false);
const commonStore = useCommonStore();

const route = useRoute();
const { grouponPromotionItemId, productSn, type } = route.query as any;
const router = useRouter();

const props = defineProps({
    product: {
        type: Object,
        default: () => {
            return {};
        }
    },
    skuList: {
        type: Array,
        default: () => {
            return [];
        }
    },
    checkedValue: {
        type: Array,
        default: () => {
            return [];
        }
    },
    productType: {
        type: String,
        default: "product"
    },
    // UI 版本：old 老版，new 新版（控制悬浮购买栏等新版专属功能）
    uiVersion: {
        type: String,
        default: "old"
    },
    grouponDetail: {
        type: Object,
        default: () => {
            return {};
        }
    },
    type: {
        type: String,
        default: ""
    },
    storeInfo: {
        type: Object,
        default: () => {
            return {};
        }
    },
    id: {
        type: Number,
        default: 0
    },
    activityPcImage: {
        type: String as PropType<string | null>,
        default: null
    },
    // 跨境版售后服务列表（overseas UI 折叠展示）
    serviceList: {
        type: Array as PropType<any[]>,
        default: () => []
    }
});

/** 新版 UI 悬浮购买栏 z-index 为 1000，弹窗需挂到 body 并抬高层级 */
const joiningDialogZIndex = computed(() => (props.uiVersion === "new" ? 2001 : undefined));

const activeName = ref<any>();

const activeBoxStyle = computed(() => {
    if (props.activityPcImage) {
        return {
            background: `url(${imageFormat(props.activityPcImage)}) no-repeat center center`,
            backgroundSize: "cover"
        };
    }
    return {};
});

// 跨境版活动条背景（带活动图时铺满并预留底部圆角）
const overseasActiveBoxStyle = computed(() => {
    if (props.activityPcImage) {
        return {
            background: `url(${imageFormat(props.activityPcImage)}) no-repeat center center`,
            backgroundSize: "cover",
            width: "calc(100% + 20px)",
            marginLeft: "-10px",
            paddingBottom: "23px",
            borderRadius: "10px 10px 0 0"
        };
    }
    return {};
});

const groupNumKey = ref<number | null>(null);
const currentGroupNum = ref(0);
const total = ref(0);
const joinList = ref<any[]>([]);
const isJoin = ref(1);
const currentShopName = ref(props.storeInfo.shopTitle);

// 阶梯团默认档位策略：
// 1) 若存在 checked 项（用户/后端已指定），优先返回该项；
// 2) 否则返回 groupNum 最大的档位（押团人数最高的那档）。
const getDefaultGroupNumItem = (groupNums: Array<{ groupNum?: number; groupNumKey?: number; checked?: boolean }> = []) => {
    const checkedItem = groupNums.find((item) => item.checked);
    if (checkedItem) {
        return checkedItem;
    }

    return groupNums.reduce(
        (maxItem, item) => {
            if (!maxItem || (item.groupNum || 0) > (maxItem.groupNum || 0)) {
                return item;
            }
            return maxItem;
        },
        null as { groupNum?: number; groupNumKey?: number; checked?: boolean } | null
    );
};

watch(
    () => props.grouponDetail,
    (value) => {
        if (value) {
            const defaultGroupNumItem = getDefaultGroupNumItem(value.groupNums);
            groupNumKey.value = defaultGroupNumItem?.groupNumKey || null;
            currentGroupNum.value = defaultGroupNumItem?.groupNum || 0;
            total.value = value.joinNumCount;
        }
    },
    { deep: true, immediate: true }
);
// 选择门店
const handleSelectStore = (item: any) => {
    currentShopName.value = item.shopTitle;

    // 更新路由参数
    const query = { ...route.query };
    query.shopId = item.shopId;

    router.push({
        path: route.path,
        query: query
    });
};

// 规格id，多个用逗号隔开
const attrIds = ref<any>(null);
// 未参与拼团的 SKU 价格需要通过商品可用性接口实时获取，不能沿用拼团活动价。
const nonJoinSkuPrice = ref<string | number | null>(null);
const nonJoinSkuId = ref<number | null>(null);
const nonJoinSkuPriceLoading = ref(false);
// 切换到未参团 SKU 时暂存切换前已展示的价格，接口返回后再一次性替换。
const previousDisplayedPrice = ref<string | number | null>(null);
let latestNonJoinPriceRequestId = 0;

// 获取当前规格对应的拼团明细及价格
const grouponPromotionDetails = computed(() =>
    Array.isArray(props.grouponDetail?.grouponPromotionDetails) ? props.grouponDetail.grouponPromotionDetails : []
);

const selectedSku = computed(() => {
    const skuId = Number(attrIds.value);
    if (skuId > 0) {
        const matchedById = props.skuList.find((item: any) => Number(item.skuId) === skuId);
        if (matchedById) return matchedById;
    }

    const selectedValue = Array.isArray(props.checkedValue) ? props.checkedValue : [];
    if (selectedValue.length > 0) {
        const matchedByValue = props.skuList.find((item: any) => String(item.skuValue || "") === selectedValue.join("|"));
        if (matchedByValue) return matchedByValue;
    }

    return props.skuList[0];
});

const selectedGrouponDetail = computed(() => {
    const skuId = Number(selectedSku.value?.skuId);
    // 无规格商品的拼团明细使用 skuId=0，不能因为 skuId 不大于 0 就跳过匹配。
    // 有规格商品仍严格按当前 SKU 匹配，避免误用其他规格的拼团价。
    const matchedDetail = grouponPromotionDetails.value.find((item: any) => Number(item.skuId) === skuId);
    if (matchedDetail) return matchedDetail;

    // 无规格商品可能没有 skuList，或 skuList 中的默认 SKU 不是 0；
    // 此时接口仍会返回 skuId=0（或首条）的拼团明细。
    const hasExplicitSkuSelection =
        (attrIds.value !== null && attrIds.value !== "") || (Array.isArray(props.checkedValue) && props.checkedValue.length > 0);
    if (!hasExplicitSkuSelection) {
        return grouponPromotionDetails.value.find((item: any) => Number(item.skuId) === 0) ?? grouponPromotionDetails.value[0];
    }

    return undefined;
});

const canJoinCurrentSku = computed(() => {
    if (!grouponPromotionDetails.value.length) return Number(isJoin.value) === 1;
    return Number(selectedGrouponDetail.value?.isJoin) === 1;
});

// 首次进入详情时也要根据当前 SKU 的拼团明细同步状态，不能一直沿用默认值 1。
watch(
    () => canJoinCurrentSku.value,
    (value) => {
        isJoin.value = value ? 1 : 0;
    },
    { immediate: true }
);

const parseGroupPrice = (detail: any) => {
    if (!detail?.groupPrice) return [];
    try {
        const parsed = typeof detail.groupPrice === "string" ? JSON.parse(detail.groupPrice) : detail.groupPrice;
        return Array.isArray(parsed) ? parsed : [];
    } catch {
        return [];
    }
};

/**
 * 获取未参与拼团 SKU 的真实商品价格。
 * 详情接口返回的 grouponDetail.groupPrice 仅代表拼团价，未参与拼团的 SKU
 * 必须以 getProductAvailability 返回的 price 为准。
 */
const loadNonJoinSkuPrice = async (skuId: number | null) => {
    const requestId = ++latestNonJoinPriceRequestId;
    nonJoinSkuId.value = skuId;
    nonJoinSkuPrice.value = null;
    nonJoinSkuPriceLoading.value = false;

    if (skuId === null || skuId === undefined || canJoinCurrentSku.value) return;

    nonJoinSkuPriceLoading.value = true;
    try {
        const params: Record<string, unknown> = { skuId };
        const result: any = await getProductSkuDetail(props.id, params);
        if (requestId !== latestNonJoinPriceRequestId) return;

        const price = result?.price ?? result?.sku?.skuPrice ?? result?.sku?.price;
        if (price !== undefined && price !== null && price !== "") {
            nonJoinSkuPrice.value = price;
        }
    } catch (error) {
        // 接口失败时保留 skuList 中的原价作为兜底，避免价格区域空白。
        console.error("获取未参与拼团 SKU 价格失败", error);
    } finally {
        if (requestId === latestNonJoinPriceRequestId) {
            nonJoinSkuPriceLoading.value = false;
        }
    }
};

watch(
    () => selectedSku.value?.skuId ?? selectedGrouponDetail.value?.skuId,
    (skuId) => {
        void loadNonJoinSkuPrice(skuId === undefined || skuId === null ? null : Number(skuId));
    },
    { immediate: true }
);

const price = computed(() => {
    const grouponDetail = selectedGrouponDetail.value;
    const currentSkuId = selectedSku.value?.skuId ?? grouponDetail?.skuId;
    // 未参与拼团的规格走商品可用性接口返回的真实 SKU 价格。
    if (!grouponDetail || Number(grouponDetail.isJoin) !== 1) {
        if (nonJoinSkuPriceLoading.value && previousDisplayedPrice.value !== null) {
            return previousDisplayedPrice.value;
        }
        if (nonJoinSkuId.value === Number(currentSkuId) && nonJoinSkuPrice.value !== null) {
            return nonJoinSkuPrice.value;
        }
        return selectedSku.value?.skuPrice ?? selectedSku.value?.price ?? props.product?.productPrice ?? "";
    }

    const groupPrice = parseGroupPrice(grouponDetail);
    if (props.grouponDetail?.groupNums?.length) {
        const groupNumInfo = groupPrice.find((item: any) => Number(item.groupNumKey) === Number(groupNumKey.value));
        return groupNumInfo?.price ?? props.grouponDetail?.minGrouponPrice ?? selectedSku.value?.skuPrice ?? selectedSku.value?.price ?? "";
    }

    return groupPrice[0]?.price ?? props.grouponDetail?.minGrouponPrice ?? selectedSku.value?.skuPrice ?? selectedSku.value?.price ?? "";
});

// 获取限购数量
const limitNum = computed(() => {
    // 每单限购数量
    const orderLimitNum = props.grouponDetail?.orderLimitNum;
    // 如果是阶梯团（groupType === 2）
    if (props.grouponDetail?.groupType === 2) {
        const grouponPromotionDetail = props.grouponDetail?.grouponPromotionDetails?.find((item: any) => item.skuId === +attrIds.value);
        // 阶梯库存数量
        const skuLimitNum = grouponPromotionDetail?.limitNum || props.grouponDetail?.grouponPromotionDetails?.[0]?.limitNum;
        // 如果设置了每单限购数量，则取最小值
        return orderLimitNum && orderLimitNum > 0 ? Math.min(skuLimitNum || 0, orderLimitNum) : skuLimitNum;
    } else {
        // 普通团直接返回每单限购数量
        return orderLimitNum || 0;
    }
});

const onProductSkuChange = (data: any) => {
    const priceBeforeSkuChange = price.value;
    const detail = grouponPromotionDetails.value.find((item: any) => Number(item.skuId) === Number(data?.skuId));
    isJoin.value = detail ? (Number(detail.isJoin) === 1 ? 1 : 0) : 0;
    previousDisplayedPrice.value = isJoin.value === 1 ? null : priceBeforeSkuChange;
    attrIds.value = data.skuId + "";
    attrList.value?.spe?.forEach((attrListItem: any) => {
        attrListItem.attrList.forEach((item: any) => {
            if (item.checked && data.skuStock === item.skuStock) {
                item.skuId = data.skuId;
            }
        });
    });
};

const viewMore = () => {
    formModalVisible.value = true;
};

// 被邀请参团进详情时，按拼团记录回显对应阶梯档位
const fetchJoiningDetail = async () => {
    if (!route.query.grouponPromotionRecordId) return;
    const result = await getJoiningDetail({ grouponPromotionRecordId: route.query.grouponPromotionRecordId });
    currentGroupNum.value = result?.minGroupNum;
    const matchedKey = props.grouponDetail?.groupNums?.find((item: any) => item.groupNum === result?.minGroupNum)?.groupNumKey;
    if (matchedKey !== undefined && matchedKey !== null) {
        groupNumKey.value = matchedKey;
    }
};
fetchJoiningDetail();

defineExpose({ showEnquiry });
</script>

<style scoped lang="less">
.product_joining {
    .productName {
        display: flex;
        align-items: flex-start;
        margin-bottom: 10px;

        .tagName {
            flex-shrink: 0;
            background-color: var(--general);
            border-radius: 2px;
            font-weight: 500;
            font-size: 12px;
            color: #ffffff;
            text-align: center;
            padding: 2px 4px;
            margin-top: 4px;
            line-height: 1.2;
        }

        .name {
            font-size: 18px;
            color: #666;
            font-weight: bold;
            // margin-left: 10px;
            line-height: 1.4;
        }
    }

    .active-box {
        background: url("/assets/images/product/pro-time-bg.png") no-repeat;
        background-size: 100% 100%;
        padding: 13px;
        border-radius: 6px 6px 0 0;
        font-size: 16px;

        .active-ico {
            color: #fff;
            font-size: 16px;

            .text {
                position: relative;
                margin-right: 26px;
            }

            .group-num {
                position: relative;

                &::before {
                    content: "";
                    position: absolute;
                    display: block;
                    width: 2px;
                    height: 100%;
                    background-color: #fff;
                    left: -13px;
                    top: 0;
                }
            }
        }

        .active-time {
            //margin-right: 10px;
            color: #fff;
        }
    }

    .joiningPrice {
        background: url("/assets/images/product/price-bg.png") repeat-x scroll 0 bottom #f3f3f3;
        padding: 10px;
        box-sizing: border-box;
        font-weight: 500;
        font-size: 16px;
        color: #9a9a9a;
        display: flex;
        flex-direction: column;
        gap: 15px;
        margin-bottom: 25px;
        .joiningPriceLine {
            .priceBox {
                height: 24px;
            }
        }
    }

    .price {
        align-items: baseline;
        vertical-align: baseline;
        font-size: 20px;
        color: var(--price);
    }

    .toJoiningListHeader {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-top: 10px;
        margin-bottom: 15px;

        .total {
            font-weight: bold;
            font-size: 16px;
            color: #1a1a1a;
        }

        .more {
            font-weight: 500;
            font-size: 16px;
            color: #3a3a3a;
            cursor: pointer;
        }
    }

    .btns {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 15px;
        margin-bottom: 24px;

        // 二期会员标签：不可购买红字提示，置于数量选择下方
        .member-restrict-tip {
            width: 100%;
            display: flex;
            align-items: center;
            margin: -3px 0;
            color: #f54e4e;
            font-size: 13px;

            .iconfont-pc {
                margin-right: 4px;
                font-size: 14px;
            }
        }

        .btn {
            padding: 20px 45px;
            font-weight: 500;
            font-size: 16px;

            &:hover {
                opacity: 0.9;
            }

            .startAGroup {
                color: #ffffff;
                //background-color: var(--general);
            }
        }

        :deep(.soloBuy) {
            background: #ffffff;
            border-radius: 2px;
            border: 1px solid #000;
            font-weight: 500;
            color: #000;
        }
    }

    .toJoiningListScroll {
        :deep(.scrollList) {
            height: 120px;
            overflow: hidden;
            margin-bottom: 20px;
        }
    }

    &.new-ui {
        overflow-x: hidden;

        .productName {
            display: flex;
            flex-wrap: wrap;
            align-items: flex-start;
            gap: 8px;
            // max-width: 490px;

            .name {
                color: rgba(9, 9, 10, 0.92);
                font-size: 20px;
                font-weight: 600;
                margin-left: 0;
                flex: 1;
                min-width: 0;
                word-break: break-word;
            }
        }

        .active-box {
            box-sizing: border-box;
            margin-top: 12px !important;
            // max-width: 490px;
            border-radius: 12px 12px 0 0 !important;
            flex-wrap: wrap;
            gap: 8px;
        }

        .joiningPrice {
            // max-width: 490px;
            margin-bottom: 24px;
            border-radius: 0 0 12px 12px !important;
            :deep(.price) {
                line-height: 18px;
                font-size: 28px;
                font-weight: 600;
                letter-spacing: -0.02em;
            }
        }

        .toJoiningListScroll {
            // max-width: 490px;
            overflow-x: hidden;
        }
    }

    /* ============ 跨境版 UI ============ */
    &.overseas-ui {
        .active-box {
            background: none;
            background-size: 100% 100%;
            padding: 13px 0;
            border-radius: 6px 6px 0 0;
            font-size: 16px;
            position: relative;

            &.has-activity-bg {
                padding: 13px 10px;
                display: flex;
                .countdown {
                    color: #fff !important;
                }
                .active-time {
                    margin-left: auto;
                }
            }
            &.has-activity-bg::after {
                content: "";
                position: absolute;
                left: -1px;
                right: -1px;
                bottom: -4px;
                height: 16px;
                background: #fff;
                border-radius: 12px 12px 0 0;
            }

            .active-ico {
                font-weight: bold;
                font-size: 16px;
                color: #fff;
                line-height: 24px;
                border-radius: 4px;
                background-color: var(--general);
                padding: 0 6px;
                margin-right: 8px;
                i {
                    margin-right: 5px;
                    font-size: 18px;
                }
            }
            .active-time {
                margin-right: 10px;
                color: #fff;
            }
        }

        .productName {
            display: block;
            margin-bottom: 10px;
            .name {
                font-size: 18px;
                color: #000;
                font-weight: 700;
                padding-bottom: 16px;
            }
            .desc {
                padding: 6px 0 12px;
                color: #ff4040;
                font-size: 12px;
                font-weight: normal;
            }
        }

        .item {
            display: flex;
            flex-direction: column;
            gap: 16px;
            margin-bottom: 16px;
            .num {
                font-weight: normal;
                font-size: 14px;
                color: rgb(136, 139, 148);
                line-height: 14px;
            }
        }

        .price1 {
            margin-bottom: 16px;
        }

        .btns {
            display: flex;
            flex-direction: column;
            align-items: stretch;
            gap: 25px;
            margin-bottom: 0;

            .add_cart {
                display: flex;
                justify-content: space-between;
                padding-bottom: 25px;
                gap: 20px;
            }
            .btn {
                flex: 1;
                padding: 20px 0;
                font-weight: 500;
                font-size: 16px;
                &:hover {
                    opacity: 0.9;
                }
            }
            :deep(.soloBuy) {
                background: #fff;
                border-radius: 2px;
                border: 1px solid #000;
                font-weight: 500;
                color: #000;
            }
        }

        .number-box {
            flex-direction: row;
            align-items: center;
            .title {
                color: #999;
                height: 33px;
                line-height: 32px;
                padding-right: 10px;
                flex-shrink: 0;
            }
            .con {
                :deep(.el-input-number) {
                    width: 98px;
                    border: none;
                }
                :deep(.el-input-number__decrease) {
                    border-right: none !important;
                }
                :deep(.el-input-number__increase) {
                    border-left: none !important;
                }
                :deep(.el-input__wrapper) {
                    box-shadow: none !important;
                    background: #f7f8fc;
                }
            }
        }

        .toJoiningListScroll {
            :deep(.scrollList) {
                height: 120px;
                overflow: hidden;
                margin-bottom: 20px;
            }
            :deep(.el-row) {
                gap: 20px;
            }
        }

        .serviceAction {
            :deep(.el-collapse-item__title) {
                font-weight: 700;
                font-size: 16px;
            }
            :deep(.el-collapse-item__content) {
                padding: 0;
            }
            .collapse-icon {
                svg {
                    transition: transform 0.3s ease;
                }
                &.active svg {
                    transform: rotate(180deg);
                }
            }
            .basic .item.align-start {
                display: block;
                .tips_i_box {
                    display: flex;
                    flex-wrap: wrap;
                    span {
                        font-size: 14px;
                        color: #888b94;
                    }
                }
            }
        }
    }
}

.number-box {
    display: flex;
    align-items: center;

    .title {
        color: #999;
        height: 33px;
        line-height: 32px;
        padding-right: 10px;
    }

    .con {
        display: flex;
        align-items: center;
    }

    // 新版 UI：数量标题置于上方，圆角步进器（与商品详情新版样式保持一致）
    &.new-ui {
        flex-direction: column;
        align-items: flex-start;
        gap: 12px;

        .title {
            height: auto;
            line-height: 22px;
            padding-right: 0;
            font-size: 14px;
            font-weight: 500;
        }

        .con {
            gap: 12px;
        }
    }
}

:deep(.ant-modal-body) {
    height: 600px;
    overflow-y: auto;
}

:deep(.el-dialog) {
    min-width: 500px;
}

:deep(.countdown-text) {
    color: #fff !important;
}
</style>

<style lang="less">
.joining-more-dialog {
    min-width: 500px;
}
</style>
