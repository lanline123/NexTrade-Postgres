<!--
  商品详情页核心组件
  - 支持普通商品、积分兑换商品、拼团商品等多种类型
  - 展示商品图片、视频、名称、价格、SKU选择、库存信息
  - 提供加入购物车、立即购买、收藏、分享功能
  - 集成店铺信息、商品评价、商品详情描述、相关推荐
  - 支持B2B批量采购、会员等级价格、各种营销活动展示
-->
<template>
    <!--
      加载骨架屏，按 uiVersion 切换三套布局
      - SSR 阶段商品详情走 $fetch（不阻塞渲染），loading 在服务端与客户端首次渲染均为 true，
        故骨架屏会被 SSR 输出并原样水合，不存在 DOM 不匹配
    -->
    <ProductSkeleton v-if="loading" :ui-version="uiVersion" />
    <div data-view-rev="djIyODJhd3E" class="product-page-video-scope" style="background: #fff" v-else>
        <!-- 新版头部 -->
        <!-- <CommonTopBar v-if="isNewUi" :colorType="colorType" :pageType="pageType" :wide="true"></CommonTopBar> -->
        <PcHeaderTopBarRuntime
            v-if="isNewUi"
            :styleType="headerStyleType"
            :isWhite="true"
            :config="decorateRuntimeStore.pcHeaderConfig || {}"
            :wide="true"
        ></PcHeaderTopBarRuntime>
        <!-- 老版 / 跨境版头部（跨境版复用标准头部，PageHeaderNew 亦为 CommonPageHeader 封装） -->
        <CommonPageHeader
            v-else
            ref="pageHeaderRef"
            :immediateLog="false"
            :showService="showService"
            :shopId="productShopInfo?.shopId ?? 0"
            :phone="productShopInfo?.kefuPhone || ''"
            :productId="productId ?? 0"
        >
        </CommonPageHeader>
        <CommonService
            v-if="isNewUi"
            :showService="showService"
            :shopId="productShopInfo?.shopId ?? 0"
            :phone="productShopInfo?.kefuPhone || ''"
            :productId="productId ?? 0"
        ></CommonService>
        <el-dialog v-model="visible" :title="$t('分享给小伙伴')" :footer="null" top="15vh" width="800px">
            <div>
                <ProductShareModal :picList="picList" :product="product" :productPrice="productPrice"> </ProductShareModal>
            </div>
        </el-dialog>

        <!-- ===================== 新版 UI 布局 ===================== -->
        <div class="container-new" v-if="isNewUi">
            <div>
                <ProductHead :shopInfo="storeInfo"></ProductHead>
                <div class="product_details flex justify-between">
                    <div class="left-wrap">
                        <div class="shop-info" v-if="(isMerchant() || isStore()) && productShopInfo && +productShopInfo?.shopId !== 0">
                            <ProductShopInfo uiVersion="new" :shopInfo="productShopInfo" @refresh="refreshShopInfo"></ProductShopInfo>
                        </div>
                        <div class="extra_details">
                            <div class="thumb">
                                <ProductThumbNew
                                    :product="product"
                                    :videoList="videoList"
                                    :picList="picList"
                                    :disableZoom="isEnquiryOpen"
                                    :productName="product.productName"
                                ></ProductThumbNew>
                            </div>
                        </div>
                        <div class="lyecs_main">
                            <ProductDetailNew
                                :product="product"
                                :attrList="attrList"
                                :rankDetail="rankDetail"
                                :consultationTotal="consultationTotal"
                                :shopInfo="productShopInfo"
                                :productId="productId"
                                :descArr="descArr"
                                :productStock="productStock"
                                :productNumber="productNumber"
                                :afterSalesFlag="afterSalesFlag"
                            ></ProductDetailNew>
                        </div>
                    </div>
                    <div class="right-wrap">
                        <el-scrollbar class="right-scroll" max-height="calc(100vh - 24px)">
                            <div class="details">
                                <div v-if="grouponDetail?.productId">
                                    <ProductJoining
                                        ref="productJoiningRef"
                                        uiVersion="new"
                                        :attrList="attrList"
                                        :product="product"
                                        :skuList="skuList"
                                        :storeInfo="storeInfo"
                                        :id="productId"
                                        :checkedValue="checkedValue"
                                        :productType="productType"
                                        :grouponDetail="grouponDetail"
                                        :activityPcImage="activityPcImage"
                                    ></ProductJoining>
                                </div>
                                <div v-else>
                                    <div class="product_name">
                                        <div class="name" v-if="!isPresale">
                                            {{ product.productName }}
                                        </div>
                                        <div class="productName" v-else>
                                            <span class="tagName">{{ $t("预售") }}</span
                                            >{{ product.productName }}
                                        </div>
                                        <p v-if="product.productBrief" class="desc">
                                            {{ product.productBrief }}
                                        </p>
                                    </div>
                                    <ProductInfo
                                        ref="productInfoRef"
                                        uiVersion="new"
                                        :product="product"
                                        :serviceList="serviceList"
                                        :productType="productType"
                                        :id="productId"
                                        :productPrice="productPrice"
                                        :attrList="attrList"
                                        :skuList="skuList"
                                        :b2bSkuList="b2bSkuList"
                                        :productStock="productStock"
                                        :productNumber="productNumber"
                                        :rankDetail="rankDetail"
                                        :descArr="descArr"
                                        :checkedValue="checkedValue"
                                        :storeInfo="storeInfo"
                                        :exchangeDetail="exchangeDetail"
                                        :isCycleInfo="isCycleInfo"
                                        :presaleDetail="presaleDetail"
                                        :activityPcImage="activityPcImage"
                                        :is-wholesale="isWholesaleMode"
                                        :has-product-brief="Boolean(product.productBrief)"
                                        :wholesale-price-list="wholesalePriceList"
                                    ></ProductInfo>
                                </div>
                            </div>
                        </el-scrollbar>
                    </div>
                </div>
            </div>
        </div>

        <!-- ===================== 跨境版 UI 布局 ===================== -->
        <div class="container-overseas" v-else-if="isOverseasUi">
            <div>
                <div class="flex_csb">
                    <div style="flex: 1; min-width: 0">
                        <OverseasCategoryNavigation v-model:id="product.categoryId" :productName="product.productName"></OverseasCategoryNavigation>
                    </div>
                    <div class="flex shareWidth">
                        <RawImageDownloadButton inline />
                        <template v-if="isOverseas()">
                            <div class="shareTitle">{{ $t("分享至") }}</div>
                            <a href="javascript:;" @click="shareTo('what')"
                                ><div><SvgIcon name="socialShare" width="18" height="18"></SvgIcon></div
                            ></a>
                            <a href="javascript:;" @click="shareTo('fackbook')"
                                ><div><SvgIcon name="faceBook" width="18" height="18"></SvgIcon></div
                            ></a>
                            <a href="javascript:;" @click="shareTo('emaile')"
                                ><div><SvgIcon name="emaile" width="18" height="18"></SvgIcon></div
                            ></a>
                            <a href="javascript:;" @click="shareTo('teitter')"
                                ><div><SvgIcon name="teitter" width="18" height="18"></SvgIcon></div
                            ></a>
                        </template>
                    </div>
                </div>
                <div class="shop-info" v-if="productShopInfo && +productShopInfo?.shopId !== 0">
                    <OverseasShopInfo :shopInfo="productShopInfo" @refresh="refreshShopInfo"></OverseasShopInfo>
                </div>
                <div class="product_details flex justify-between">
                    <div class="extra_details">
                        <div class="thumb">
                            <OverseasThumb
                                :showEnquiry="showEnquiry"
                                :productStock="productStock"
                                :videoList="videoList"
                                :picList="picList"
                                :productName="product.productName"
                                :isCollect="isCollect"
                                @addCollectEmpty="addCollect"
                            ></OverseasThumb>
                        </div>
                        <!-- 吸顶锚点 Tab（滚动联动） -->
                        <div class="tab-main">
                            <div
                                v-for="item in menuList"
                                :key="item.type"
                                :class="orderType == item.type ? 'active' : ''"
                                class="tab"
                                @click="changeType(item.type)"
                            >
                                <div :class="orderType == item.type ? 'fouerty' : ''" class="font-style">{{ $t(item.value) }}</div>
                            </div>
                        </div>
                        <div class="goods_desc paidcontent" id="xiangqing" :ref="(el) => (sectionRefs[0].value = el)">
                            <div class="specifications">{{ $t("商品详情") }}</div>
                            <template v-if="isPro() && product.productType === 4">
                                <div class="goods_description">
                                    <template v-if="product.isBuy && product.paidContent">
                                        <template v-for="item in product.paidContent" :key="item">
                                            <img
                                                class="el-img"
                                                v-if="item.type == 'pic'"
                                                fit="cover"
                                                :src="imageFormat(item.pic || '')"
                                                :alt="product.productName || ''"
                                            />
                                            <ClientOnly>
                                                <div class="desc-text-item" v-if="item.type == 'text'" v-html="formatRichText(item.html || '')"></div>
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
                                    <template v-else>
                                        <img fit="cover" class="img" src="~/assets/images/product/paidcontent-bg.png" />
                                    </template>
                                </div>
                            </template>
                            <div class="goods_description">
                                <template v-for="item in descArr">
                                    <img
                                        class="el-img"
                                        v-if="item.type == 'pic'"
                                        fit="cover"
                                        :src="imageFormat(item.pic || '')"
                                        :alt="product.productName || ''"
                                    />
                                    <ClientOnly>
                                        <div class="desc-text-item" v-if="item.type == 'text'" v-html="formatRichText(item.html || '')"></div>
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
                            <template v-if="attrList.normal && attrList.normal.length > 0">
                                <ul class="shuxin flex">
                                    <template v-for="(item, index) in attrList.normal" :key="index">
                                        <li>
                                            {{ item.attrName }}：
                                            <template v-for="(attr, arrtIndex) in item.attrList" :key="arrtIndex"> {{ attr.attrValue }} &nbsp; </template>
                                        </li>
                                    </template>
                                </ul>
                            </template>
                        </div>
                        <!-- D1146：评价功能关闭时整块隐藏，锚点菜单已同步摘掉 type 1 -->
                        <div class="goods_desc" id="pingjia" :ref="(el) => (sectionRefs[1].value = el)" v-if="commonStore.productCommentEnabled === 1">
                            <div class="tab_title">
                                <h2>{{ $t("用户评价") }}</h2>
                            </div>
                            <OverseasComment v-model="product.productId" :useToShow="false" @empty="empty"></OverseasComment>
                            <div v-if="moreComments" style="text-align: center; margin: 24px auto 0">
                                <el-button class="buy-btn buynow" @click="visibleComment = true">{{ $t("查看更多 >") }}</el-button>
                            </div>
                        </div>
                        <div class="goods_desc goods_ask" id="zixun" :ref="(el) => (sectionRefs[2].value = el)">
                            <div class="tab_title">
                                <h2>{{ $t("商品咨询") }}</h2>
                            </div>
                            <ProductConsultation :product="product" :productStock="productStock" :productNumber="productNumber"></ProductConsultation>
                        </div>
                        <div class="goods_desc shouhou advisory" id="shouhou" :ref="(el) => (sectionRefs[3].value = el)" v-if="afterSalesFlag">
                            <div class="tab_title">
                                <h2>{{ $t("售后服务") }}</h2>
                            </div>
                            <ClientOnly>
                                <ProductAfterSaleService></ProductAfterSaleService>
                            </ClientOnly>
                        </div>
                        <!-- 跨境独有：主详情区「相关文章」区块（o2o 无此区块，与老版布局对称） -->
                        <div class="goods_desc" id="article" :ref="(el) => (sectionRefs[4].value = el)">
                            <div class="tab_title">
                                <h2>{{ $t("相关文章") }}</h2>
                            </div>
                            <ProductRelevanceArticle :productId="productId"></ProductRelevanceArticle>
                        </div>
                    </div>
                    <div class="overseas-right-wrap">
                        <el-scrollbar class="overseas-right-scroll" max-height="calc(100vh - 24px)">
                            <div class="details">
                                <div v-if="grouponDetail?.productId">
                                    <ProductJoining
                                        ref="productJoiningRef"
                                        ui-version="overseas"
                                        :attrList="attrList"
                                        :product="product"
                                        :skuList="skuList"
                                        :storeInfo="storeInfo"
                                        :id="productId"
                                        :checkedValue="checkedValue"
                                        :productType="productType"
                                        :grouponDetail="grouponDetail"
                                        :serviceList="serviceList"
                                        :activityPcImage="activityPcImage"
                                    ></ProductJoining>
                                </div>
                                <OverseasProductInfo
                                    v-else
                                    ref="productInfoRef"
                                    v-model:productStock="productStock"
                                    v-model:showEnquiry="showEnquiry"
                                    :product="product"
                                    :serviceList="serviceList"
                                    :productType="productType"
                                    :id="productId"
                                    :productPrice="productPrice"
                                    :attrList="attrList"
                                    :skuList="skuList"
                                    :b2bSkuList="b2bSkuList"
                                    :productNumber="productNumber"
                                    :rankDetail="rankDetail"
                                    :descArr="descArr"
                                    :checkedValue="checkedValue"
                                    :exchangeDetail="exchangeDetail"
                                    :presaleDetail="presaleDetail"
                                    :is-wholesale="isWholesaleMode"
                                    :wholesale-price-list="wholesalePriceList"
                                    :activityPcImage="activityPcImage"
                                    :storeInfo="storeInfo"
                                    :buy-action-target="'#overseas-product-buy-actions'"
                                ></OverseasProductInfo>
                            </div>
                        </el-scrollbar>
                        <div id="overseas-product-buy-actions" class="overseas-right-actions"></div>
                    </div>
                </div>
            </div>
            <ProdSwiper titleName="精选好物"></ProdSwiper>
            <el-dialog v-model="visibleComment" :title="$t('更多评价')" :footer="null" top="15vh" width="65%">
                <OverseasComment v-model="product.productId" :useToShow="true"></OverseasComment>
            </el-dialog>
        </div>

        <!-- ===================== 老版 UI 布局 ===================== -->
        <div class="container" v-else>
            <div>
                <CommonCategoryNavigation v-model:id="product.categoryId" :productName="product.productName"> </CommonCategoryNavigation>
                <!-- 社交分享（跨境独有：WhatsApp/Facebook/Email/Twitter） -->
                <!-- <div class="flex shareWidth" v-if="isOverseas()">
                    <div class="shareTitle">{{ $t("分享至") }}</div>
                    <a href="javascript:;" @click="shareTo('what')"
                        ><div><SvgIcon name="socialShare" width="18" height="18"></SvgIcon></div
                    ></a>
                    <a href="javascript:;" @click="shareTo('fackbook')"
                        ><div><SvgIcon name="faceBook" width="18" height="18"></SvgIcon></div
                    ></a>
                    <a href="javascript:;" @click="shareTo('emaile')"
                        ><div><SvgIcon name="emaile" width="18" height="18"></SvgIcon></div
                    ></a>
                    <a href="javascript:;" @click="shareTo('teitter')"
                        ><div><SvgIcon name="teitter" width="18" height="18"></SvgIcon></div
                    ></a>
                </div> -->
                <div class="shop-info" v-if="(isMerchant() || isStore()) && productShopInfo && +productShopInfo?.shopId !== 0">
                    <ProductShopInfo :shopInfo="productShopInfo" @refresh="refreshShopInfo"></ProductShopInfo>
                </div>
                <div class="product_details flex justify-between">
                    <div class="extra_details">
                        <div class="thumb">
                            <ProductThumb :product="product" :videoList="videoList" :picList="picList" :disableZoom="isEnquiryOpen"></ProductThumb>
                        </div>
                        <div class="share_wish flex" v-if="productType == 'product' || productType == 'exchange'">
                            <span class="product-number" :title="product.productSn">{{ $t("商品编号") }}：{{ product.productSn }}</span>
                            <!--                            <a class="shareGold flex hand-pointer" @click="showModal"><i></i>分享</a>-->
                            <div class="product-tools">
                                <span
                                    class="iconfont-pc collect ico-color"
                                    :class="{ 'icon-shoucang': !isCollect, 'icon-yishoucang': isCollect }"
                                    @click="addCollect(isCollect)"
                                >
                                    <i>{{ isCollect ? $t("已收藏") : $t("收藏") }}</i>
                                </span>
                                <RawImageDownloadButton inline />
                            </div>
                        </div>
                    </div>
                    <div class="details" :class="{ 'details--wholesale-scroll': isWholesaleMode }">
                        <div v-if="grouponDetail?.productId">
                            <ProductJoining
                                ref="productJoiningRef"
                                :attrList="attrList"
                                :product="product"
                                :skuList="skuList"
                                :storeInfo="storeInfo"
                                :id="productId"
                                :checkedValue="checkedValue"
                                :productType="productType"
                                :grouponDetail="grouponDetail"
                                :activityPcImage="activityPcImage"
                            ></ProductJoining>
                        </div>
                        <div v-else>
                            <div class="product_name">
                                <div class="name" v-if="!isPresale">
                                    {{ product.productName }}
                                </div>
                                <div class="productName" v-else>
                                    <span class="tagName">{{ $t("预售") }}</span
                                    >{{ product.productName }}
                                </div>
                                <p v-if="product.productBrief" class="desc">
                                    {{ product.productBrief }}
                                </p>
                            </div>
                            <ProductInfo
                                ref="productInfoRef"
                                :product="product"
                                :serviceList="serviceList"
                                :productType="productType"
                                :id="productId"
                                :productPrice="productPrice"
                                :attrList="attrList"
                                :skuList="skuList"
                                :b2bSkuList="b2bSkuList"
                                :productStock="productStock"
                                :productNumber="productNumber"
                                :rankDetail="rankDetail"
                                :descArr="descArr"
                                :checkedValue="checkedValue"
                                :storeInfo="storeInfo"
                                :exchangeDetail="exchangeDetail"
                                :isCycleInfo="isCycleInfo"
                                :presaleDetail="presaleDetail"
                                :activityPcImage="activityPcImage"
                                :is-wholesale="isWholesaleMode"
                                :wholesale-price-list="wholesalePriceList"
                            ></ProductInfo>
                        </div>
                    </div>
                </div>
                <div class="detail_main flex">
                    <template v-if="productType == 'product' || productType == 'exchange'">
                        <div class="lyecs_side">
                            <!-- <template v-if="storeInfo && +storeInfo?.shopId !==0">
                                <div class="shop-box">
                                    <div class="tit">
                                        <NuxtLink :to="{ path: '/store/info', query: {shopId: storeInfo?.shopId}  }">{{
                                                storeInfo.shopTitle }}</NuxtLink>
                                    </div>
                                    <div class="btn-box flex justify-between">
                                        <div class="btn">
                                            <span class="iconfont-pc icon-guanli"></span>
                                            <NuxtLink :to="{ path: '/store/info', query: {shopId: storeInfo?.shopId}  }">{{
                                                $t("进店逛逛") }}</NuxtLink>
                                        </div>
                                        <div class="btn" @click="addShopCollect(storeInfo.shopId, storeInfo.collectShop)">
                                            <span class="iconfont-pc collect ico-color"
                                                :class="{ 'icon-shoucang': !storeInfo.collectShop, 'icon-yishoucang': storeInfo.collectShop }">
                                            </span>
                                            <span>{{ $t("收藏门店") }}</span>
                                        </div>
                                    </div>
                                </div>
                            </template> -->

                            <template v-if="!loading">
                                <ProductCompetitorsList :productId="productId"></ProductCompetitorsList>
                            </template>
                            <!-- 相关文章（跨境独有，接 o2o 现有 getProductRelateArticle） -->
                            <template v-if="!loading && isOverseas()">
                                <ProductRelevanceArticle :productId="productId"></ProductRelevanceArticle>
                            </template>
                        </div>
                    </template>

                    <ProductDetailTabs
                        :product="product"
                        :attrList="attrList"
                        :descArr="descArr"
                        :rankDetail="rankDetail"
                        :consultationTotal="consultationTotal"
                        :productStock="productStock"
                        :productNumber="productNumber"
                    ></ProductDetailTabs>
                </div>
            </div>
        </div>
        <CommonPageFooter></CommonPageFooter>
        <ClientOnly>
            <ElevatorProduct></ElevatorProduct>
        </ClientOnly>
    </div>
</template>
<script lang="ts" setup>
import { isProductOfflineError } from "@/utils/productSale";
import { ref, nextTick, computed, onMounted, onBeforeUnmount, provide } from "vue";
import { useProductRawImageDownload } from "~/composables/useProductRawImageDownload";
import RawImageDownloadButton from "~/components/product/RawImageDownloadButton.vue";
import { createRawImageContext, type RawImageContext } from "~/utils/productRawImageDownload";
import {
    getProductDetail,
    getCollectProduct,
    getShopDetail,
    getShopCollect,
    getExchangeDetail,
    getBatchProductAvailability,
    getAfterSaleService
} from "~/api/product/product";
import { getActiveStyle } from "~/api/product/productStyle";
import type { ProductItem, AttrList, PicList, RankDetail, SkuList, DescArr, ServiceList, ExchangeDetail, VideoList } from "~/types/product/product.d";
import { updateCollectProduct } from "~/api/user/collectProduct";
import { delCollectProduct } from "~/api/user/collectProduct";
import ElevatorProduct from "~/components/product/ElevatorProduct.vue";
import { useCommonStore } from "@/store/common";
import { useDecorateRuntimeStore } from "@/store/decorateRuntime";
import { useUserStore } from "@/store/user";
import { buildSeoKeywords, truncateSeoText } from "@/utils/seo";
import { buildProductPath } from "@/utils/seo-url";
import { formatRichText, isMerchant, isOverseas, isStore } from "@/utils/util";
import { imageFormat } from "@/utils/format";
import { createRequestGenerationGuard } from "@/utils/requestGeneration";
import { message } from "ant-design-vue";
import ProductJoining from "~/pages/joining/src/ProductJoining.vue";
import ProductHead from "./ProductHead.vue";
import ProductDetailTabs from "./ProductDetailTabs.vue";
import ProductSkeleton from "./ProductSkeleton.vue";
// 跨境版底部「精选好物」推荐，非 components/ 目录需显式引入
import ProdSwiper from "../../cart/src/ProdSwiper.vue";
// 跨境版组件副本，显式引入以覆盖 o2o 同名自动导入，保证与原版一模一样
import OverseasThumb from "~/components/product/overseas/Thumb.vue";
import OverseasProductInfo from "~/components/product/overseas/ProductInfo.vue";
import OverseasShopInfo from "~/components/product/overseas/ShopInfo.vue";
import OverseasComment from "~/components/product/overseas/Comment.vue";
import OverseasCategoryNavigation from "~/components/product/overseas/CategoryNavigation.vue";
import PcHeaderTopBarRuntime from "~/decorate/components/pc/pcHeaderRuntime/components/PcHeaderTopBarRuntime.vue";

const { t } = useI18n();
const commonStore = useCommonStore();
const decorateRuntimeStore = useDecorateRuntimeStore();
// 新版头部顶部条风格：跟随后台 PC 头部装修配置（与首页一致），无配置时按 overseas 兜底
const headerStyleType = computed<1 | 2>(() => {
    const raw = decorateRuntimeStore.pcHeaderConfig?.styleType;
    if (raw === 1 || raw === 2) return raw;
    return isOverseas() ? 2 : 1;
});
// 新版头部装修配置
const pageType = computed(() => {
    return commonStore.decoratePageConfig?.headerStyle;
});
const colorType = computed(() => {
    return commonStore.decoratePageConfig?.colorStyle;
});
const title = ref(t("商品详情"));
const description = ref("");
const keywords = ref("");
const shopSeoTitle = ref("");
const shopSeoLogo = ref("");
const pageHeaderRef = ref();
const productJoiningRef = ref();
const productInfoRef = ref();
const isEnquiryOpen = computed(() => {
    return productJoiningRef.value?.showEnquiry || productInfoRef.value?.showEnquiry || false;
});

const props = defineProps({
    productType: {
        type: String,
        default: "product"
    },
    sn: {
        type: String,
        default: ""
    },
    // UI 版本：old 老版布局，new 新版布局
    uiVersion: {
        type: String,
        default: "old"
    }
});
const isNewUi = computed(() => props.uiVersion === "new");
// 跨境版 UI；预留切换：uiVersion === "overseas"
const isOverseasUi = computed(() => props.uiVersion === "overseas");
// 向子孙组件下发当前 UI 版本，供已合并的同名组件（如 Buy/ProductEnquiry/ProductAmount 等）按需切换跨境样式；
// old/new 分支注入非 overseas 值，组件走原逻辑，互不影响
provide(
    "detailUiVersion",
    computed(() => props.uiVersion)
);
// 规格区选中带图规格时，由主图区（三套 Thumb）消费该状态切换主图
useProvideSpecPic();
// 普通商品：规格与地址栏 skuId 同步（积分兑换不接入）
const skuUrl = props.productType === "product" ? useProvideSkuUrl() : null;
// 跨境版：询价弹层开关（与 ProductInfo/Thumb 联动）、评价「查看更多」弹窗
const showEnquiry = ref(false);
const moreComments = ref(false);
const visibleComment = ref(false);
const empty = (e: number) => {
    if (e > 2) moreComments.value = true;
};
// 售后服务展示开关（默认 true 兜底；由 getAfterSalesConfig 依后端 showTemplateContent 动态设置）
const afterSalesFlag = ref(true);
const route = useRoute();
const router = useRouter();
const effectiveProductSn = computed(() => {
    const propSn = props.sn.trim();
    if (propSn) return propSn;

    const routeSn = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id;
    return routeSn ? String(routeSn) : "";
});
const rawImageContext = ref<RawImageContext | null>(null);
useProductRawImageDownload(rawImageContext, () => getProduct());
const productId = ref<number>(0);
const consultationTotal = ref<number>(0);
const product = ref<ProductItem>({
    productId: 0,
    productStock: 0,
    productName: "",
    description: "",
    keywords: ""
});
const productStock = ref<number>(0);
const productNumber = ref(1);
const attrList = ref<AttrList>({
    normal: []
});
const skuList = ref<SkuList[]>([]);
const b2bSkuList = ref<SkuList[]>([]);
/**
 * 批发态判定
 * - 完全由后端 detail.wholesalePriceList 字段存在性驱动；后端在总开关关 / 非批发商时不返此字段
 * - 非批发态保持 false / null，供 ProductInfo 的 prop 绑定
 */
const isWholesaleMode = ref(false);
const wholesalePriceList = ref<any[] | null>(null);
const userStore = useUserStore();
const picList = ref<PicList[]>([]);
const serviceList = ref<ServiceList[]>([]);
const rankDetail = ref<RankDetail>({});
const productPrice = ref<string>("");
const loading = ref(true);
const descArr = ref<DescArr[]>([]);
const checkedValue = ref<string[]>([]);
const exchangeDetail = ref<ExchangeDetail>({});
const videoList = ref<VideoList[]>([]);
const grouponDetail = ref<any>({});
const presaleDetail = ref<any>(null);
const activityPcImage = ref<string | null>(null);
const productVideoScopeSelector = ".product-page-video-scope";
const pauseOtherProductVideos = (event: Event) => {
    const currentVideo = event.target as HTMLVideoElement | null;
    if (!currentVideo || currentVideo.tagName !== "VIDEO") return;

    const scope = currentVideo.closest(productVideoScopeSelector);
    if (!scope) return;

    scope.querySelectorAll<HTMLVideoElement>("video").forEach((video) => {
        if (video !== currentVideo && !video.paused) {
            video.pause();
        }
    });
};

// ===== 跨境版：详情 / 评价 / 咨询 / 售后 / 相关文章 锚点滚动联动 =====
const orderType = ref<number>(0);
const baseMenuList = [
    { type: 0, value: "详情", anchor: "xiangqing" },
    { type: 1, value: "评价", anchor: "pingjia" },
    { type: 2, value: "咨询", anchor: "zixun" },
    { type: 3, value: "售后", anchor: "shouhou" },
    { type: 4, value: "相关文章", anchor: "article" }
];
// 售后关闭时不展示「售后」Tab（与 afterSalesFlag 联动）；D1146：评价关闭时不展示「评价」Tab
// type 值同时是 sectionRefs 的下标，故只过滤菜单项、不重排 type
const menuList = computed(() =>
    baseMenuList.filter((item) => {
        if (item.type === 3) return afterSalesFlag.value;
        if (item.type === 1) return commonStore.productCommentEnabled === 1;
        return true;
    })
);
// 模板 ref 回调返回 Element | ComponentPublicInstance，用 any 承接后按 HTMLElement 使用
const sectionRefs = [ref<any>(null), ref<any>(null), ref<any>(null), ref<any>(null), ref<any>(null)];
// 点击 Tab：滚动到对应区块（顶部预留 80px 头部高度）
const changeType = (type: number) => {
    orderType.value = type;
    nextTick(() => {
        const el = sectionRefs[type]?.value;
        if (el) {
            const top = el.getBoundingClientRect().top + window.scrollY - 80;
            window.scrollTo({ top, behavior: "auto" });
        }
    });
};
// 滚动时高亮当前区块对应的 Tab（仅跨境版生效）
const onOverseasScroll = () => {
    if (!isOverseasUi.value) return;
    const scrollY = window.scrollY || window.pageYOffset;
    let currentIndex = 0;
    sectionRefs.forEach((refItem, index) => {
        if (refItem.value) {
            const offsetTop = refItem.value.offsetTop - 100;
            if (scrollY >= offsetTop) {
                currentIndex = index;
            }
        }
    });
    orderType.value = currentIndex;
};

onMounted(() => {
    document.addEventListener("play", pauseOtherProductVideos, true);
    if (isOverseasUi.value) {
        window.addEventListener("scroll", onOverseasScroll);
    }
});

const productRequestGuard = createRequestGenerationGuard();
onBeforeUnmount(() => {
    productRequestGuard.invalidate();
    document.removeEventListener("play", pauseOtherProductVideos, true);
    window.removeEventListener("scroll", onOverseasScroll);
});
const seoTitle = computed(() => {
    const productName = title.value || product.value?.productName || t("商品详情");
    return shopSeoTitle.value ? `${productName} - ${shopSeoTitle.value}` : productName;
});
const seoDescription = computed(() => {
    return description.value || truncateSeoText(product.value?.productBrief || product.value?.productDesc || "", 150);
});
const seoKeywords = computed(() => {
    return keywords.value || buildSeoKeywords([product.value?.productName, shopSeoTitle.value]);
});
const canonical = computed(() => {
    return (
        buildProductPath({
            seoLink: product.value?.seoLink,
            sn: effectiveProductSn.value
        }) || "/"
    );
});

useSeo({
    title: seoTitle,
    description: seoDescription,
    keywords: seoKeywords,
    image: computed(() => picList.value?.[0]?.picUrl || product.value?.picUrl || shopSeoLogo.value || ""),
    ogType: "product",
    canonical
});

const jsonLd = computed(() =>
    JSON.stringify({
        "@context": "https://schema.org",
        "@type": "Product",
        name: product.value?.productName || "",
        description: description.value || "",
        image: picList.value?.[0]?.picUrl || "",
        offers: {
            "@type": "Offer",
            price: productPrice.value || "",
            priceCurrency: commonStore.dollarSign || "CNY",
            availability: productStock.value > 0 ? "https://schema.org/InStock" : "https://schema.org/OutOfStock"
        }
    })
        // 转义 < 与 /，防止 productName 等字段含脚本闭合标签突破 JSON-LD 脚本块导致 XSS
        .replace(/</g, "\\u003c")
        .replace(/\//g, "\\u002f")
);

useHead({
    script: [
        {
            type: "application/ld+json",
            innerHTML: jsonLd
        }
    ]
});
const isPresale = computed(() => {
    return !!presaleDetail.value && typeof presaleDetail.value === "object" && Object.keys(presaleDetail.value).length > 0;
});

// 周期购信息
const cycleConfig = ref<any>({});
// 是否是周期购
const isCycleInfo = ref(false);
const getRouteShopId = () => {
    const rawShopId = Array.isArray(route.query.shopId) ? route.query.shopId[0] : route.query.shopId;
    if (rawShopId === undefined || rawShopId === null || rawShopId === "") return null;
    const parsedShopId = Number(rawShopId);
    return Number.isInteger(parsedShopId) && parsedShopId >= 0 ? parsedShopId : null;
};

const getProduct = async () => {
    const requestGeneration = productRequestGuard.begin();
    rawImageContext.value = null;
    try {
        let result: any = {};
        let rawImageRequestParams = {};
        const routeShopId = getRouteShopId();
        if (props.productType == "exchange") {
            result = await getExchangeDetail(route.params.id);
        } else {
            const params: any = {
                sn: effectiveProductSn.value,
                // shopId : route.query.shopId || 0
                // 被邀请参团进详情时透传拼团记录ID，保证详情数据与所参团一致（未参团时为 null）
                grouponPromotionRecordId: route.query.grouponPromotionRecordId ? Number(route.query.grouponPromotionRecordId) : null
            };
            if (routeShopId !== null) {
                params.shopId = routeShopId;
            } else {
            }
            rawImageRequestParams = { ...params };
            result = await getProductDetail(params);
        }
        if (!productRequestGuard.isCurrent(requestGeneration)) return;
        if (props.productType == "exchange") {
            exchangeDetail.value = result.exchangeInfo;
            productStock.value = result.exchangeInfo.productStock;
        } else {
            productStock.value = result.item.productStock;
        }
        const detailShopId = result.item?.shopId ?? result.shopId;
        const resolvedShopId = typeof detailShopId === "number" && Number.isFinite(detailShopId) ? detailShopId : undefined;
        // 周期购
        cycleConfig.value = result.cycleConfig;
        isCycleInfo.value = result.deliveryMode === 2;
        // 如果是周期购，跳转到 pcMobile 页面
        if (isCycleInfo.value && import.meta.client) {
            let url = `/pcMobile?path=/pages/product/index&id=${result.item.productId}`;
            if (resolvedShopId !== undefined) {
                url += `&shopId=${resolvedShopId}`;
            }
            navigateTo(url);
            return;
        }
        title.value = result.item.productName;
        keywords.value = result.keywords || buildSeoKeywords([result.item.productName]);
        description.value = result.description || truncateSeoText(result.item.productBrief || result.item.productDesc || "", 150);
        productId.value = result.item.productId;
        rawImageContext.value = createRawImageContext(result.item.productId, rawImageRequestParams);
        let effectiveShopId = routeShopId ?? Number(result.item?.shopId ?? 0);
        product.value = {
            ...result.item,
            shopId: resolvedShopId
        };
        presaleDetail.value = result.item?.presaleInfo ?? null;
        attrList.value = result.attrList;
        picList.value = result.picList;
        rankDetail.value = result.rankDetail ?? {};
        skuList.value = result.skuList;
        checkedValue.value = result.checkedValue;
        // 拼团商品（详情区渲染 ProductJoining）不接入：其购买按钮直接读取 route.query.skuId
        skuUrl?.applyUrlSkuId(result.skuList, !result.grouponDetail?.productId);
        descArr.value = result.descArr;
        serviceList.value = result.serviceList;
        consultationTotal.value = result.consultationTotal;
        grouponDetail.value = result.grouponDetail; // 拼团详情
        videoList.value = result.videoList;

        // 活动背景与店铺/门店信息互不依赖，先发出去与后续请求并行，末尾统一等齐
        const firstScreenTasks: Promise<unknown>[] = [
            (async () => {
                try {
                    const styleResult: any = await getActiveStyle(productId.value);
                    if (!productRequestGuard.isCurrent(requestGeneration)) return;
                    activityPcImage.value = styleResult?.activityBackgroundPcImage || null;
                } catch {}
            })()
        ];


        // 店铺/门店信息必须在首屏露出前就绪，否则 shop-info 后插入会把主图区整体顶下去造成跳动
        if (resolvedShopId !== undefined) {
            firstScreenTasks.push(getShopInfo(effectiveShopId, requestGeneration));
        }
        await Promise.all(firstScreenTasks);
        if (!productRequestGuard.isCurrent(requestGeneration)) return;
        // 门店详情与店铺详情都会写 SEO 字段，并发后先后顺序不再固定；按原串行顺序重放一次店铺详情，
        // 保证标题取值不随两个接口的快慢变化
        if (shopInfo.value?.shopTitle) shopSeoTitle.value = shopInfo.value.shopTitle;
        if (shopInfo.value?.shopLogo) shopSeoLogo.value = shopInfo.value.shopLogo;

        loading.value = false;

        if (isB2B() && result.skuList.length > 1 && commonStore.bulkPurchase === 1) {
            getBatchProductAvailabilityList();
        }
        if (props.productType == "product") {
            if (!userStore.isAuthenticated) {
                return;
            }

            getCollect();
        }

        if (import.meta.client) {
            nextTick(() => {
                pageHeaderRef.value?.handleLog();
            });
        }
    } catch (error: any) {
        if (!productRequestGuard.isCurrent(requestGeneration)) return;
        console.error(error);
        if (String(error?.message || "").includes("销售门店不一致")) {
            wholesalePriceList.value = null;
            isWholesaleMode.value = false;
            message.error(t("销售门店信息已变化，请重新选择门店"));
            loading.value = false;
            return;
        }
        if (isProductOfflineError(error)) {
            navigateTo({ path: "/404", query: { reason: "offline" } });
            return;
        }
        navigateTo("/404");
    } finally {
    }
};


getProduct();

/**
 * @description 获取售后服务配置，判断是否展示售后服务 Tab
 * 通用能力补齐：afterSalesFlag 默认 true，仅当后端明确返回 showTemplateContent=false 时才隐藏，避免回退 o2o 原有"始终展示"行为
 */
const getAfterSalesConfig = async () => {
    try {
        const result: any = await getAfterSaleService(productId.value);
        afterSalesFlag.value = result?.showTemplateContent ?? true;
    } catch (error: any) {
        console.error(error);
    }
};
getAfterSalesConfig();

/**
 * @description 获取B2B批量采购场景下的SKU库存和价格信息
 * @returns 无返回值，设置b2bSkuList供批量采购组件使用
 */
const getBatchProductAvailabilityList = async () => {
    try {
        const skuIds = skuList.value.map((item: SkuList) => item.skuId).join(",");
        const result = await getBatchProductAvailability({ skuIds });
        b2bSkuList.value = skuList.value.map((sku: SkuList) => {
            const newSku = {
                ...sku,
                num: 0,
                title: sku.skuValue.replace(/\|/g, ",")
            };

            const skuId = sku.skuId.toString();
            if (result[skuId]) {
                Object.assign(newSku, {
                    price: result[skuId].price,
                    stock: result[skuId].stock
                });
            }

            return newSku;
        });
    } catch (error) {
        console.error(error);
    }
};

const isCollect = ref(false);
/**
 * @description 检查当前商品是否已被用户收藏
 * @returns 无返回值，设置isCollect状态
 */
const getCollect = async () => {
    try {
        const result = await getCollectProduct({ id: productId });
        isCollect.value = result;
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
const storeInfo = ref<any>({});
const shopInfo = ref<any>({});
/**
 * @description 获取店铺详情信息（客服配置等）
 * @param id 店铺ID
 */
const getShopInfo = async (id: number, requestGeneration = productRequestGuard.snapshot()) => {
    try {
        const result = await getShopDetail(id);
        if (!productRequestGuard.isCurrent(requestGeneration)) return;
        shopInfo.value = result;
        shopSeoTitle.value = result.shopTitle || shopSeoTitle.value;
        shopSeoLogo.value = result.shopLogo || shopSeoLogo.value;
    } catch (error: any) {
        if (!productRequestGuard.isCurrent(requestGeneration)) return;
        console.error(error);
    }
};

// 店铺展示数据统一出口：o2o 优先门店详情 storeInfo，无数据时降级 shopInfo；非 o2o 直接用 shopInfo
const productShopInfo = computed(() => {
    return shopInfo.value || {};
});

// 关注/取消关注后刷新店铺数据：o2o 刷门店详情，非 o2o 刷店铺详情
const refreshShopInfo = (id: number) => {
    getShopInfo(id);
};

const showService = computed(() => {
    const info = shopInfo.value || {};
    if (typeof info === "object" && Object.keys(info).length > 0) {
        if (Number(info.shopId) === 0) return true;
        const kefuInlet = Array.isArray(info.kefuInlet) ? info.kefuInlet : (String(info.kefuInlet ?? "").match(/\d+/g) ?? []);
        return kefuInlet.some((item: unknown) => Number(item) === 1);
    } else {
        return null;
    }
});

const visible = ref<boolean>(false);

/**
 * @description 社交平台分享（跨境独有），支持 WhatsApp / Facebook / Email / Twitter
 * @param type 分享平台类型: 'what' | 'fackbook' | 'emaile' | 其它(Twitter)
 */
const shareTo = (type: string) => {
    if (type == "what") {
        const shareURL = "https://api.whatsapp.com/send?text=" + product.value.productName + "|" + productPrice.value + window.location.href;
        window.open(shareURL, "_blank");
    } else if (type == "fackbook") {
        const shareURL = "https://www.facebook.com/sharer/sharer.php?u=" + window.location.href;
        window.open(shareURL, "_blank");
    } else if (type == "emaile") {
        const shareURL = "mailto:?subject=NexTrade;body=" + product.value.productName + "|" + productPrice.value + window.location.href;
        window.open(shareURL, "_blank");
    } else {
        const shareURL = "https://x.com/intent/post?button_hashtag=NexTrade&url=" + window.location.href + "&text=" + product.value.productName;
        window.open(shareURL, "_blank");
    }
};

/**
 * @description 收藏/取消收藏商品
 * @param isCollect 当前是否已收藏，true则取消，false则收藏
 */
const addCollect = async (isCollect: boolean) => {
    try {
        if (!isCollect) {
            await updateCollectProduct({
                productId: productId.value
            });
        } else {
            await delCollectProduct({
                id: productId.value
            });
        }

        await getCollect();
        message.success(isCollect ? t("取消收藏成功") : t("收藏成功"));
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};


watch(
    () => product.value,
    (newVal) => {
        title.value = newVal.productName;
    },
    { deep: true }
);

// 监听详情关键参数变化，确保切换门店/规格都会刷新商品详情
watch(
    () => `${effectiveProductSn.value}|${route.query.skuId ?? ""}|${route.query.shopId ?? ""}`,
    (newKey, oldKey) => {
        if (newKey !== oldKey) {
            loading.value = true;
            getProduct();
        }
    }
);
</script>

<style lang="less" scoped>
/* ===================== 老版 UI 样式 ===================== */
.container {
    width: 1190px !important;
    margin: 0 auto;
    padding: 0;
    min-width: 1190px;
    .product_details {
        margin-top: 15px;

        .extra_details {
            width: 360px;

            .gallery {
                width: 358px;
                height: 358px;
                margin: 0 auto 0;
                margin-bottom: 10px;
                border: 1px solid #eee;

                img {
                    width: 358px;
                    height: 358px;
                    font-size: 5px;
                }
            }

            .share_wish {
                width: 100%;
                margin-top: 20px;
                padding-bottom: 30px;
                display: flex;
                align-items: center;
                justify-content: space-between;
                column-gap: 10px;

                .product-number {
                    color: #999;
                    flex: 1;
                    min-width: 0;
                    font-size: 12px;
                    white-space: nowrap;
                    overflow: hidden;
                    text-overflow: ellipsis;
                }

                .product-tools {
                    display: flex;
                    flex-shrink: 0;
                    align-items: center;
                    gap: 14px;
                    color: #666;
                    font-size: 12px;
                }

                a {
                    font-size: 12px;
                    color: #5e5e5e;
                    margin-left: 22px;
                    overflow: hidden;
                    text-decoration: none;

                    i {
                        background: url("/assets/images/common/sprite-product.png") no-repeat scroll 0 0;
                    }
                }

                .shareGold i {
                    background-position: 0 0;
                    height: 13px;
                    margin-bottom: 2px;
                    margin-right: 3px;
                    margin-top: 2px;
                    width: 13px;
                }

                .showWish i {
                    background-position: -14px 0;
                    height: 11px;
                    margin: 3px 4px 3px 0;
                    width: 12px;
                }

                .collect {
                    display: inline-flex;
                    align-items: center;
                    gap: 4px;
                    cursor: pointer;
                    color: #666;
                    font-size: 16px;
                    line-height: 1;
                    white-space: nowrap;

                    &:hover,
                    &.icon-yishoucang {
                        color: var(--general);
                    }
                }

                .collect i {
                    font-size: 12px;
                    font-style: normal;
                }
            }
        }

        .details {
            width: 807px;

            .productName {
                font-size: 18px;
                color: #666;
                font-weight: bold;
                padding-bottom: 12px;
                line-height: 1.5;

                .tagName {
                    display: inline-block;
                    background-color: var(--general);
                    border-radius: 2px;
                    font-weight: 500;
                    font-size: 12px;
                    color: #ffffff;
                    text-align: center;
                    padding: 2px 4px;
                    margin-right: 6px;
                    vertical-align: middle;
                }
            }

            .product_name {
                .name {
                    font-size: 18px;
                    color: #666;
                    font-weight: bold;
                    padding-bottom: 12px;
                }

                .desc {
                    padding: 6px 0 12px;
                    color: #888888;
                    font-size: 14px;
                    font-weight: normal;
                }
            }

            /* 批发态：商品信息整块（标题 + 阶梯价 + 服务 + 规格选购）过长时随区域内部一起滚动，底部购买栏吸底固定 */
            &.details--wholesale-scroll {
                max-height: 70vh;
                overflow-y: auto;
                box-sizing: border-box;
                padding-right: 16px;

                &::-webkit-scrollbar {
                    width: 6px;
                }
                &::-webkit-scrollbar-thumb {
                    background: #e0e0e0;
                    border-radius: 3px;
                }
                &::-webkit-scrollbar-track {
                    background: transparent;
                }

                /* .actions 默认 overflow:hidden 会令吸底 sticky 失效，批发态放开 */
                :deep(.actions) {
                    overflow: visible;
                }

                :deep(.wholesale-buy-bar) {
                    position: sticky;
                    bottom: 0;
                    background: #fff;
                    z-index: 5;
                }
            }
        }
    }

    .detail_main {
        margin-bottom: 20px;

        .lyecs_side {
            width: 216px;

            .shop-box {
                width: 100%;
                background-color: #fff;
                border: 1px solid #eeeeee;
                margin-bottom: 10px;

                .tit {
                    background: #f6f6f6;
                    padding: 8px 12px;
                    line-height: 25px;
                    min-height: 25px;
                    font-size: 16px;
                    white-space: nowrap;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    display: flex;
                    align-items: center;
                    justify-content: space-between;
                    gap: 5px;
                    .shopTitle {
                        display: block;
                        flex: 1;
                        text-overflow: ellipsis;
                        overflow: hidden;
                        white-space: nowrap;
                    }
                    .switchStores {
                        font-size: 12px;
                        display: flex;
                        align-items: center;
                        transform: translateY(1px);
                        flex-shrink: 0;
                        cursor: pointer;
                    }
                    i {
                        //cursor: pointer;
                        display: block;
                        font-size: 10px;
                    }
                }

                .btn-box {
                    padding: 10px;

                    .btn {
                        width: 93px;
                        height: 35px;
                        background: #f6f6f6;
                        text-align: center;
                        border: 1px solid #e8e8e8;
                        cursor: pointer;
                        display: flex;
                        align-items: center;
                        justify-content: center;

                        .iconfont-pc {
                            font-size: 18px;
                            color: var(--general);
                            margin-right: 5px;
                        }
                    }
                }
            }
        }
    }

    .ico-color {
        color: var(--general);
    }
    .shop-info {
        padding-top: 30px;
    }
}

/* ===================== 新版 UI 样式 ===================== */
.container-new {
    max-width: 1760px;
    min-width: 1056px;
    width: 100%;
    margin: 0 auto;
    padding-left: 80px;
    padding-right: 80px;
    box-sizing: border-box;
    font-family: Inter, "SF Pro Text", Roboto, "Helvetica Neue", Helvetica, Tahoma, Arial, "PingFang SC", "Microsoft YaHei";

    .left-wrap {
        min-width: 480px;
        width: calc(60% - 48px);
        margin-bottom: 20px;
    }
    .right-wrap {
        flex-shrink: 0;
        margin-left: 48px;
        min-width: 400px;
        width: 40%;
        position: sticky;
        top: 12px;
        align-self: flex-start;

        .right-scroll {
            :deep(.el-scrollbar__view) {
                padding-right: 2px;
            }
        }
    }

    .product_details {
        margin-top: 15px;

        .extra_details {
            width: 100%;

            .share_wish {
                width: 100%;
                margin-top: 20px;
                padding-bottom: 30px;
                display: flex;
                align-items: center;
                column-gap: 10px;

                .product-number {
                    color: #999;
                    width: fit-content;
                    max-width: calc(100% - 60px);
                    float: left;
                    font-size: 12px;
                    white-space: nowrap;
                    overflow: hidden;
                    text-overflow: ellipsis;
                }

                .collect {
                    cursor: pointer;
                    width: 77px;
                }

                .collect i {
                    background-size: 100%;
                    margin-bottom: 2px;
                    margin-right: 4px;
                    width: 16px;
                    height: 16px;
                    font-size: 12px;
                }
            }
        }

        .details {
            width: 100%;

            .product_name {
                .name {
                    color: rgba(9, 9, 10, 0.92);
                    font-size: 20px;
                    font-weight: 600;
                }

                .desc {
                    padding: 12px 0 12px;
                    color: #888888;
                    font-size: 14px;
                    font-weight: normal;
                }

                .productName {
                    color: rgba(9, 9, 10, 0.92);
                    font-size: 20px;
                    font-weight: 600;
                    margin-bottom: 10px;
                    line-height: 1.5;

                    .tagName {
                        display: inline-block;
                        background-color: var(--general);
                        border-radius: 2px;
                        font-weight: 500;
                        font-size: 12px;
                        color: #ffffff;
                        text-align: center;
                        padding: 2px 4px;
                        margin-right: 6px;
                        vertical-align: middle;
                    }
                }
            }
        }
    }

    .lyecs_main {
        width: 100%;
        margin-top: 56px;
        position: relative;
    }

    .shop-info {
        padding-bottom: 4px;
    }

    .ico-color {
        color: var(--general);
    }
}

/* ===================== 跨境版 UI 样式 ===================== */
.container-overseas {
    width: 1264px;
    margin: 0 auto;
    padding: 0;

    .flex_csb {
        display: flex;
        align-items: center;
        justify-content: space-between;
        column-gap: 60px;
    }

    .shareTitle {
        font-size: inherit;
        color: inherit;
        line-height: 1;
        white-space: nowrap;
    }

    .raw-image-download + .shareTitle {
        padding-left: 16px;
        border-left: 1px solid #e5e7eb;
    }

    .shareWidth {
        display: flex;
        flex-shrink: 0;
        gap: 16px;
        align-items: center;
        font-size: 13px;
        color: #888b94;
    }

    .shop-info {
        padding-top: 20px;
    }

    .paidcontent {
        .img {
            width: 100%;
        }
    }

    .product_details {
        .extra_details {
            width: 752px;

            .thumb {
                margin-bottom: 16px;
            }
        }

        .overseas-right-wrap {
            width: 496px;
            margin-left: 16px;
            position: sticky;
            top: 12px;
            align-self: flex-start;
            max-height: calc(100vh - 24px);
            display: flex;
            flex-direction: column;

            .overseas-right-scroll {
                flex: 1 1 auto;
                min-height: 0;
                overflow: auto;

                /* 组件默认 height:100% 会按整列高度撑开，底部购买按钮因此盖住最后一行 */
                :deep(.el-scrollbar),
                :deep(.el-scrollbar__wrap) {
                    height: auto;
                    max-height: none;
                    overflow: visible;
                }

                :deep(.el-scrollbar__view) {
                    padding-right: 2px;
                    padding-bottom: 8px;
                }
            }

            .overseas-right-actions {
                flex: 0 0 auto;
                position: relative;
                z-index: 2;
                margin-top: 12px;
                padding: 0 16px 8px;
                box-sizing: border-box;
                background: #fff;

                &:empty {
                    display: none;
                    margin-top: 0;
                    padding-bottom: 0;
                }
            }
        }

        .details {
            width: 100%;
            padding: 0 16px;
            box-sizing: border-box;
            height: fit-content;

            .product_name {
                .name {
                    font-size: 18px;
                    color: #000;
                    font-weight: 700;
                    font-family: "微软雅黑";
                    padding-bottom: 16px;
                }

                .desc {
                    padding: 6px 0 12px;
                    color: #ff4040;
                    font-size: 12px;
                    font-family: "微软雅黑";
                    font-weight: normal;
                }
            }
        }
    }

    /* 吸顶锚点 Tab */
    .tab-main {
        display: flex;
        column-gap: 35px;
        position: sticky;
        top: 0px;
        background: #fff;
        color: #000;

        .tab.active {
            color: var(--general);
        }

        .fouerty::before {
            content: "";
            position: absolute;
            width: 32px;
            height: 3px;
            bottom: 3px;
            left: 50%;
            background: var(--general);
            color: var(--general);
            border-radius: 10px;
            transform: translate(-50%);
        }

        .font-style {
            position: relative;
            padding: 16px 0 16px 0;
            font-size: 16px;
            font-weight: 600;
            cursor: pointer;
        }
    }

    .goods_desc {
        width: 738px;
        padding: 16px 10px 10px 0;
        margin-bottom: 46px;

        .shuxin {
            padding: 10px;
            height: 100%;
            margin-bottom: 10px;
            flex-wrap: wrap;

            li {
                line-height: 26px;
                overflow: hidden;
                width: 350px;
                font-weight: normal;
                padding: 0 10px;
                box-sizing: border-box;
            }
        }

        .goods_description {
            margin-bottom: 24px;

            .el-img {
                width: 100%;
                display: block;
            }
        }

        .tab_title {
            height: 30px;

            h2 {
                display: inline-block;
                font-family: "微软雅黑", "SimHei";
                margin-bottom: 24px;
                font-size: 20px;
                font-weight: 700;
                line-height: 24px;
                color: rgb(26, 26, 26);
            }
        }
    }

    .specifications {
        font-size: 20px;
        font-weight: 700;
        margin-bottom: 24px;
        line-height: 24px;
        color: rgb(26, 26, 26);
    }

    .ico-color {
        color: var(--general);
    }
}
</style>
