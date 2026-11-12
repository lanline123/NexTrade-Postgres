<template>
    <!-- <CommonPageHeaderNew></CommonPageHeaderNew> -->
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader :title="pageTitle"></CommonHeader>
    <TopJumpBar :tltle="pageTitle"></TopJumpBar>

    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row collect-product">
            <div class="title-or-tabs">
                <span class="card-title">{{ cardTitle }}</span>
            </div>
            <div class="page-info-body">
                <div v-loading="loading" class="collect-list-content">
                    <template v-if="total > 0">
                        <div v-for="item in filterState" class="card">
                            <div class="left-card">
                                <div class="inner-div" @click="addShopCollect(item.shop.shopId)">{{ $t("取消收藏") }}</div>
                                <div class="logo">
                                    <el-image class="image" :src="imageFormat(item.shop.shopLogo)" :alt="item.shop.shopTitle || ''"></el-image>
                                </div>
                                <div class="title">{{ item.shop.shopTitle }}</div>
                                <div class="operation-card">
                                    <NuxtLink target="_blank" :to="getShopHref(item.shop)" class="butt butt1">
                                        <i class="iconfont-pc icon-guanli font-color"></i>{{ enterLabel }}
                                    </NuxtLink>
                                    <div v-if="item.shop.kefuLink" class="butt" @click="customerService(item.shop.kefuLink)">
                                        <i class="iconfont-pc icon-kefu font-color"></i>{{ $t("联系客服") }}
                                    </div>
                                </div>
                            </div>
                            <div class="right-card">
                                <div class="top-bar">
                                    <div class="left-bar">
                                        <div
                                            v-for="(itemBar, index) in item.items"
                                            :key="index"
                                            class="inner-div"
                                            :class="[itemBar.id == item.id ? 'inner-div-hover' : '']"
                                            @mouseover="handleMouseOver(item, item.items, index)"
                                        >
                                            {{ $t(itemBar.text) }}
                                        </div>
                                    </div>
                                    <div v-if="item.shop.hotProduct.length > 5" class="right-bar">
                                        <div class="inner-div" @click="checkPage(item, -1)">
                                            <span class="iconfont-pc icon-jiantou_zuo"></span>
                                        </div>
                                        <div class="inner-div" @click="checkPage(item, +1)">
                                            <span class="iconfont-pc icon-jiantou_you"></span>
                                        </div>
                                    </div>
                                </div>
                                <div class="product-list">
                                    <template v-if="item.id == 1">
                                        <template v-if="item.shop.hotProduct.length > 0">
                                            <NuxtLink
                                                :to="urlFormat({ path: 'product', seoLink: product.seoLink, sn: product.productSn })"
                                                target="_blank"
                                                class="product-card"
                                                v-for="product in item.shop.hotProduct.slice((item.page - 1) * 5, item.page * 5)"
                                            >
                                                <div class="product-img-wrap">
                                                    <el-image class="image" :src="productImage(product)" :alt="product.productName || ''"></el-image>
                                                    <img
                                                        v-if="product.productId && styleMap[product.productId]?.borderImage"
                                                        class="product-style-border"
                                                        :src="styleMap[product.productId].borderImage || undefined"
                                                        alt=""
                                                    />
                                                </div>
                                                <div class="product_price">
                                                    <FormatPrice v-model="product.productPrice" :fontStyle="{ fontSize: '14px', color: '#333' }"></FormatPrice>
                                                </div>
                                            </NuxtLink>
                                        </template>
                                        <div v-else class="no-data">
                                            <span v-if="!loading">{{ $t("暂无商品") }}</span>
                                        </div>
                                    </template>
                                    <template v-if="item.id == 2">
                                        <template v-if="item.shop.newProduct.length > 0">
                                            <NuxtLink
                                                :to="urlFormat({ path: 'product', seoLink: product.seoLink, sn: product.productSn })"
                                                target="_blank"
                                                class="product-card"
                                                v-for="product in item.shop.newProduct.slice((item.page - 1) * 5, item.page * 5)"
                                            >
                                                <div class="product-img-wrap">
                                                    <el-image class="image" :src="productImage(product)" :alt="product.productName || ''"></el-image>
                                                    <img
                                                        v-if="product.productId && styleMap[product.productId]?.borderImage"
                                                        class="product-style-border"
                                                        :src="styleMap[product.productId].borderImage || undefined"
                                                        alt=""
                                                    />
                                                </div>
                                                <div class="product_price">
                                                    <FormatPrice v-model="product.productPrice" :fontStyle="{ fontSize: '14px', color: '#333' }"></FormatPrice>
                                                </div>
                                            </NuxtLink>
                                        </template>
                                        <div v-else class="no-data">
                                            <span v-if="!loading">{{ $t("暂无商品") }}</span>
                                        </div>
                                    </template>
                                    <template v-if="item.id == 3">
                                        <template v-if="item.shop.bestProduct.length > 0">
                                            <NuxtLink
                                                :to="urlFormat({ path: 'product', seoLink: product.seoLink, sn: product.productSn })"
                                                target="_blank"
                                                class="product-card"
                                                v-for="product in item.shop.bestProduct.slice((item.page - 1) * 5, item.page * 5)"
                                            >
                                                <div class="product-img-wrap">
                                                    <el-image class="image" :src="productImage(product)" :alt="product.productName || ''"></el-image>
                                                    <img
                                                        v-if="product.productId && styleMap[product.productId]?.borderImage"
                                                        class="product-style-border"
                                                        :src="styleMap[product.productId].borderImage || undefined"
                                                        alt=""
                                                    />
                                                </div>
                                                <div class="product_price">
                                                    <FormatPrice v-model="product.productPrice" :fontStyle="{ fontSize: '14px', color: '#333' }"></FormatPrice>
                                                </div>
                                            </NuxtLink>
                                        </template>
                                        <div v-else class="no-data">
                                            <span v-if="!loading">{{ $t("暂无商品") }}</span>
                                        </div>
                                    </template>
                                </div>
                            </div>
                        </div>
                        <div class="el-page">
                            <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
                        </div>
                    </template>
                    <template v-else>
                        <div class="no-result">
                            <div v-if="!loading" style="display: flex; flex-direction: column; align-items: center">
                                <img class="empty-img" src="/assets/images/empty-box.png" alt="" />
                                <div class="empty-text">{{ emptyText }}</div>
                            </div>
                        </div>
                    </template>
                </div>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 店铺/门店收藏列表页
 * - 展示用户收藏的店铺或门店（文案与跳转随 VITE_IS_STORE 编译环境切换）
 * - 支持取消收藏、进入店铺/门店首页
 * - 支持分页浏览
 */
import { reactive, ref } from "vue";
import type { CollectShopFilterParams, CollectShopFilterState } from "~/types/user/collectShop";
import { delCollectShop, getCollectShopList } from "~/api/user/collectShop";
import { urlFormat } from "~/utils/format";
import { Pagination } from "~/components/list";
import Buy from "~/components/product/Buy.vue";
import { useRouter } from "vue-router";
import { useUserStore } from "~/store/user";
import { getShopCollect } from "~/api/product/product";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
import { useProductStyle } from "~/hooks";
import { isStore } from "@/utils/util";

const { t } = useI18n();
definePageMeta({
    middleware: "auth"
});

// o2o 门店版显示「门店」，非门店版显示「店铺」（TopJumpBar/CommonHeader 内部会再 $t）
const pageTitle = isStore() ? "收藏的门店" : "收藏的店铺";
const cardTitle = isStore() ? t("收藏门店") : t("收藏店铺");
const enterLabel = isStore() ? t("进入门店") : t("进入店铺");
const emptyText = isStore() ? t("您还没有收藏的门店") : t("您还没有收藏的店铺");

/** 门店 → /store/info；店铺 → 店铺首页 */
const getShopHref = (shop: { shopId?: number; seoLink?: string }) => {
    const shopId = shop?.shopId;
    if (!shopId) return "";
    return urlFormat({ path: "shop", id: shopId, seoLink: shop?.seoLink });
};

const filterState = ref(<CollectShopFilterState[]>[]);
const allProducts = computed(() =>
    filterState.value.flatMap((item) => [...(item.shop.hotProduct || []), ...(item.shop.newProduct || []), ...(item.shop.bestProduct || [])])
);
const { styleMap } = useProductStyle(allProducts);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<CollectShopFilterParams>({
    //初使化用于查询的参数
    page: 1,
    keyword: ""
});
// 数据定义
const items = ref([
    { text: "热销", id: 1 },
    { text: "新品", id: 2 },
    { text: "精品", id: 3 }
]);

const loadFilter = async () => {
    loading.value = true;
    try {
        const result = await getCollectShopList({ ...filterParams });
        filterState.value = result.records;
        filterState.value.forEach((item, index) => {
            item.items = items.value;
            item.id = 1;
            item.current = "热销";
            item.page = 1;
        });
        console.log(filterState.value);
        total.value = result.total;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
loadFilter();

const deleteASiteCollection = async (value: number) => {
    try {
        loading.value = true;
        const result = await delCollectShop({ id: value });
        message.success(t("操作成功"));
        await loadFilter();
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const router = useRouter();

const menuList = reactive<any>([
    // { value: "收藏的商品", url: "/member/collectProduct/list", size: 0 },
    { value: pageTitle, url: "/member/collectShop/list", size: 0 }
]);

// 鼠标移入事件处理
function handleMouseOver(product: any, one: any, index: any) {
    product.id = one[index].id;
}

const checkPage = (product: any, size: any) => {
    product.page = product.page == 1 ? 2 : 1;
};

const addShopCollect = async (shopId: number) => {
    try {
        await getShopCollect(shopId);
        message.success(t("操作成功"));
        await loadFilter();
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};

const customerService = async (url: string) => {
    window.open(url, "newWindow", "width=800,height=600,scrollbars=yes,resizable=yes");
};
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.collect-product {
    .title-or-tabs {
        .tag-and-input {
            display: flex;
            height: 32px;
            flex-direction: row;
            justify-content: space-between;
            width: 100%;

            .tig-tabs {
                & > div {
                    color: #333333;
                    font-size: 16px;
                    font-weight: 400;
                    letter-spacing: 0;
                    display: inline-block;
                }
            }

            .tig-font-style {
                font-size: 14px;
            }

            .image {
                margin-bottom: 20px;
            }

            .input {
                display: flex;
                align-items: center;
            }
        }
    }

    .collect-list-content {
        background-color: #ffffff;
        box-sizing: border-box;
        display: flex;
        flex-direction: row;
        flex-wrap: wrap;
        gap: 20px;

        .card {
            border: 1px solid #eee;
            width: 100%;
            display: flex;
            height: 260px;

            .left-card {
                width: 260px;
                display: flex;
                box-sizing: border-box;
                flex-direction: column;
                border-right: 1px solid #eee;
                position: relative; /* 容器需要相对定位 */
                padding-top: 24px; /* 为内div预留空间，保持布局不变 */

                .inner-div {
                    position: absolute; /* 绝对定位，相对于最近的已定位祖先（这里是.outer-div） */
                    top: 0; /* 紧贴在.outer-div的顶部 */
                    left: 0;
                    width: 100%; /* 可以根据需要调整宽度 */
                    height: 24px;
                    line-height: 24px;
                    background-color: rgba(0, 0, 0, 0.5); /* 示例背景色 */
                    display: none; /* 默认隐藏 */
                    transition: opacity 0.3s ease; /* 添加过渡效果 */
                    cursor: pointer;
                    color: white;
                    text-align: center;
                }

                .logo {
                    height: 60px;
                    display: flex;
                    align-items: center;
                    justify-content: center;

                    .image {
                        width: 60px;
                        height: 60px;
                    }
                }

                .title {
                    text-align: center;
                    font-size: 14px;
                    color: #333333;
                    height: 40px;
                    line-height: 40px;
                    font-weight: bold;
                    white-space: nowrap; /* 不换行 */
                    overflow: hidden; /* 隐藏超出容器的内容 */
                    text-overflow: ellipsis; /* 超出部分用省略号表示 */
                    flex: 1;
                }

                .operation-card {
                    height: 34px;
                    cursor: pointer;
                    background-color: #f9f9f9;
                    border-top: 1px solid #eee;
                    font-size: 12px;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    color: #666666;
                    line-height: 34px;

                    .butt1 {
                        border-right: 1px solid #eee;
                    }

                    .butt {
                        flex: 1;
                        text-align: center;
                        align-items: center;
                        display: flex;
                        justify-content: center;
                        gap: 8px;
                    }
                }
            }

            .right-card {
                flex: 1;
                padding: 15px;
                box-sizing: border-box;
                display: flex;
                flex-direction: column;

                .top-bar {
                    display: flex;
                    flex-direction: row;
                    justify-content: space-between;
                    align-items: center;
                    margin-bottom: 15px;

                    .left-bar {
                        display: flex;

                        .inner-div {
                            flex: 1; /* 让每个子div占据相等空间 */
                            border: 1px solid #dddddd;
                            padding: 0 14px;
                            height: 25px;
                            line-height: 25px;
                            text-align: center;
                            transition: border-color 0.3s; /* 平滑过渡效果 */
                            cursor: pointer;
                            margin-right: -1px;
                        }

                        .inner-div-hover {
                            border: 1px solid var(--general);
                            z-index: 10;
                        }
                    }

                    .right-bar {
                        display: flex;

                        .inner-div {
                            flex: 1; /* 让每个子div占据相等空间 */
                            border: 1px solid #dddddd;
                            padding: 0 14px;
                            height: 25px;
                            line-height: 25px;
                            text-align: center;
                            transition: border-color 0.3s; /* 平滑过渡效果 */
                            cursor: pointer;
                            margin-right: -1px;
                        }
                    }
                }

                .product-list {
                    display: flex;
                    flex-direction: row;
                    margin-top: 20px;

                    .product-card {
                        width: 20%;
                        height: 100%;
                        display: flex;
                        flex-direction: column;
                        align-items: center;
                        gap: 20px;

                        .product-img-wrap {
                            position: relative;
                            width: 100px;
                            height: 100px;
                        }

                        .image {
                            width: 100%;
                            height: 100%;
                        }

                        .product-style-border {
                            position: absolute;
                            left: 0;
                            top: 0;
                            width: 100%;
                            height: 100%;
                            pointer-events: none;
                            object-fit: fill;
                        }

                        .product_price {
                            text-align: center;
                        }
                    }
                }
            }
        }

        .card:hover .inner-div {
            display: block; /* 鼠标悬停时显示 */
            opacity: 1; /* 可选：渐显效果 */
        }
    }
}

.no-data {
    display: flex;
    flex: 1;
    font-size: 16px;
    justify-content: center;
    align-items: center;
    margin-top: 50px;
}
</style>
