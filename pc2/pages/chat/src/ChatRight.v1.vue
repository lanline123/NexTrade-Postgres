<!--
  o2o 原版组件存档（v1）
  切回方法：删除当前 ChatRight.vue，将本文件重命名为 ChatRight.vue
-->
<template>
    <div class="chat-right-container">
        <!-- 店铺信息 -->
        <div class="shop-card">
            <div class="shop-header">
                <img :src="imageFormat(shopAvatar)" class="shop-avatar" />
                <div class="shop-info">
                    <div class="flex" style="align-items: center">
                        <span class="shop-name">{{ shopName }}</span>
                        <span v-if="shopInfo?.shopId === 0" class="official-tag">{{ $t("官方") }}</span>
                    </div>

                    <div class="shop-actions">
                        <button v-if="shopInfo?.shopId" class="enter-shop-btn" @click="handleEnterShop">{{ $t("进入店铺") }}</button>
                    </div>
                </div>
            </div>
        </div>

        <!-- 商品信息 (如果有) -->
        <div v-if="productInfo" class="info-section">
            <div class="section-title">{{ $t("咨询商品") }}</div>
            <div class="product-card" @click="handleProductClick">
                <img :src="productImage(productInfo)" class="product-img" :alt="productInfo.productName || ''" />
                <div class="product-details">
                    <div class="product-name">{{ productInfo.productName }}</div>
                    <div class="product-price">¥{{ productInfo.productPrice }}</div>
                </div>
            </div>
        </div>

        <!-- 订单信息 (如果有) -->
        <div v-if="orderInfo" class="info-section">
            <div class="section-title">{{ $t("相关订单") }}</div>
            <div class="order-card" @click="handleOrderClick">
                <div class="order-header">
                    <span class="order-sn">{{ $t(orderInfo.orderSn) }}</span>
                    <span class="order-status">{{ orderInfo.orderStatusName }}</span>
                </div>
                <div class="order-body">
                    <img :src="productImage(orderInfo) || productImage(orderInfo.items?.[0])" class="product-img" :alt="orderInfo.productName || ''" />
                    <div class="order-details">
                        <div class="price-row">
                            <span class="label">{{ orderInfo.productName }}</span>
                            <span class="price">
                                <FormatPrice class="price" v-model="orderInfo.totalAmount" :showText="false"></FormatPrice>
                            </span>
                        </div>
                        <div class="sku-style" v-if="orderInfo.skuData && orderInfo.skuData.length">
                            <div v-for="sku in orderInfo.skuData">{{ sku.name }}：{{ sku.value }}</div>
                        </div>
                        <div class="quantity" v-if="orderInfo.quantity">
                            <span class="quality"><span style="font-size: 10px;">x</span>{{ orderInfo.quantity }}</span>
                        </div>
                        <div class="date-row">
                            <span class="date">{{ orderInfo.addTime || orderInfo.createTime }}</span>
<!--                            <el-button class="view-order-btn">{{ $t("查看订单") }}</el-button>-->
                             <button class="send-order-btn" @click.stop="handleSendOrder">{{ $t("发送") }}</button>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <!-- 快捷操作 -->
        <!-- <div class="quick-actions">
            <div class="section-title">快捷操作</div>
            <div class="action-list">
                <div class="action-item" @click="handleViewOrders">
                    <span class="icon">📦</span>
                    <span class="text">我的订单</span>
                </div>
                <div class="action-item" @click="handleViewAfterSales">
                    <span class="icon">🔧</span>
                    <span class="text">售后服务</span>
                </div>
                <div class="action-item" @click="handleViewHelp">
                    <span class="icon">❓</span>
                    <span class="text">帮助中心</span>
                </div>
            </div>
        </div> -->
    </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useCommonStore } from "~/store/common";
import { productImage } from "~/utils/format";
const { t } = useI18n();

const props = defineProps<{
    shopInfo?: any;
    productInfo?: any;
    orderInfo?: any;
}>();

const emit = defineEmits<{
    (e: "sendOrder", order: any): void;
}>();

const commonStore = useCommonStore();

// 店铺头像
const shopAvatar = computed(() => {
    if (props.shopInfo?.shopLogo) {
        return props.shopInfo.shopLogo;
    }
    if (commonStore.icoImg) {
        return commonStore.icoImg;
    }
    return new URL("/assets/images/im/self_shop.png", import.meta.url).href;
});

// 店铺名称
const shopName = computed(() => {
    return t(props.shopInfo?.shopTitle || commonStore.shopName || "平台自营");
});

// 进入店铺
const handleEnterShop = () => {
    if (props.shopInfo?.shopId) {
        window.open(`/shop/${props.shopInfo.shopId}`, "_blank");
    }
};

// 点击商品
const handleProductClick = () => {
    if (props.productInfo) {
        window.open(`/item/${props.productInfo.productSn || props.productInfo.productId}`, "_blank");
    }
};

// 点击订单
const handleOrderClick = () => {
    if (props.orderInfo) {
        window.open(`/member/order/info?id=${props.orderInfo.orderId}`, "_blank");
    }
};

// 发送订单
const handleSendOrder = () => {
    if (props.orderInfo) {
        emit("sendOrder", props.orderInfo);
    }
};

// 查看订单列表
const handleViewOrders = () => {
    window.open("/member/order/list", "_blank");
};

// 查看售后服务
const handleViewAfterSales = () => {
    window.open("/member/aftersales/list", "_blank");
};

// 查看帮助中心
const handleViewHelp = () => {
    window.open("/article/help", "_blank");
};
</script>

<style lang="less" scoped>
.chat-right-container {
    width: 300px;
    background-color: #fff;
    height: 100%;
    border-left: 1px solid #f0f0f0;
    padding: 12px;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    position: relative;

    &::-webkit-scrollbar {
        width: 4px;
    }
    &::-webkit-scrollbar-thumb {
        background: #e0e0e0;
        border-radius: 2px;
    }

    .shop-card {
        margin-bottom: 25px;
        padding-bottom: 20px;
        border-bottom: 1px solid #f0f0f0;

        .shop-header {
            display: flex;
            align-items: center;
            margin-bottom: 15px;

            .shop-avatar {
                width: 65px;
                height: 65px;
                border-radius: 4px;
                margin-right: 12px;
                object-fit: cover;
            }

            .shop-info {
                display: flex;
                flex-direction: column;
                justify-content: space-between;
                height: 65px;
                gap: 4px;

                .shop-name {
                    font-size: 15px;
                    font-weight: 500;
                    color: #333;
                }

                .official-tag {
                    display: inline-block;
                    font-size: 10px;
                    color: #ff4d4f;
                    border: 1px solid #ff4d4f;
                    padding: 1px 4px;
                    box-sizing: border-box;
                    border-radius: 2px;
                    width: fit-content;
                    margin-left: 8px;
                }
            }
        }

        .shop-actions {
            .enter-shop-btn {
                border: 1px solid #1890ff;
                color: #1890ff;
                background: white;
                padding: 2px 10px;
                border-radius: 2px;
                font-size: 12px;
                cursor: pointer;
                transition: all 0.2s;

                &:hover {
                    background: #e6f7ff;
                }
            }
        }
    }

    .info-section {
        margin-bottom: 20px;

        .section-title {
            font-size: 13px;
            color: #999;
            margin-bottom: 12px;
        }

        .product-card {
            background-color: #f9f9f9;
            padding: 12px;
            border-radius: 6px;
            display: flex;
            cursor: pointer;
            transition: all 0.2s;

            &:hover {
                background-color: #f0f0f0;
            }

            .product-img {
                width: 60px;
                height: 60px;
                object-fit: cover;
                border-radius: 4px;
                margin-right: 10px;
            }

            .product-details {
                flex: 1;
                display: flex;
                flex-direction: column;
                justify-content: space-between;

                .product-name {
                    font-size: 13px;
                    color: #333;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    display: -webkit-box;
                    -webkit-line-clamp: 2;
                    -webkit-box-orient: vertical;
                    line-height: 1.4;
                }

                .product-price {
                    font-size: 14px;
                    color: #ff4d4f;
                    font-weight: 500;
                }
            }
        }

        .order-card {
            //background-color: #f9f9f9;
            padding-bottom: 12px;
            border-radius: 6px;
            cursor: pointer;
            transition: all 0.2s;
            border-bottom: .5px solid #f0f0f0;

            .order-header {
                display: flex;
                justify-content: space-between;
                margin-bottom: 8px;
                font-size: 12px;

                .order-sn {
                    color: #888b94;
                }
                .order-status {
                    color: #1890ff;
                    white-space: nowrap;
                }
            }

            .order-body {
                display: flex;

                .product-img {
                    width: 60px;
                    height: 60px;
                    border-radius: 4px;
                    margin-right: 10px;
                    object-fit: cover;
                }

                .order-details {
                    flex: 1;
                    min-width: 0;
                    overflow: hidden;
                    display: flex;
                    flex-direction: column;
                    justify-content: space-between;

                    .price-row {
                        display: flex;
                        align-items: center;
                        margin-bottom: 4px;
                        font-size: 12px;

                        .label {
                            flex: 1;
                            min-width: 0;
                            color: #000;
                            font-size: 13px;
                            overflow: hidden;
                            text-overflow: ellipsis;
                            white-space: nowrap;
                            margin-right: 8px;
                        }
                        .price {
                            flex-shrink: 0;
                            color: #ff4d4f;
                            font-size: 14px;
                            font-weight: 500;
                            white-space: nowrap;
                        }
                    }
                    .sku-style {
                        display: flex;
                        gap: 5px;
                        overflow: hidden;
                        white-space: nowrap;
                        text-overflow: ellipsis;
                    }

                    .date-row {
                        font-size: 11px;
                        color: #999;
                        display: flex;
                        justify-content: space-between;
                        align-items: center;

                        .send-order-btn {
                            border: 1px solid var(--general);
                            color: var(--general);
                            background: white;
                            padding: 2px 10px;
                            border-radius: 2px;
                            font-size: 12px;
                            cursor: pointer;
                            transition: all 0.2s;
                            transform: translateY(-2px);

                            &:hover {
                                background: var(--primary-light-9);
                            }
                        }
                    }
                }
            }
        }
    }

    .quick-actions {
        position: absolute;
        bottom: 100px;
        left: 0;
        padding-top: 20px;
        border-top: 1px solid #f0f0f0;
        width: 320px;

        .section-title {
            font-size: 13px;
            color: #999;
            margin-bottom: 12px;
            padding: 0 10px;
        }

        .action-list {
            padding: 0 10px;
            display: flex;
            flex-direction: column;
            gap: 8px;

            .action-item {
                display: flex;
                align-items: center;
                padding: 10px 12px;
                background-color: #f9f9f9;
                border-radius: 6px;
                cursor: pointer;
                transition: all 0.2s;

                &:hover {
                    background-color: #e6f7ff;
                }

                .icon {
                    margin-right: 10px;
                    font-size: 16px;
                }

                .text {
                    font-size: 13px;
                    color: #333;
                }
            }
        }
    }
}
</style>
