<template>
    <div class="cart" @mouseenter="onShowCartList" @mouseleave="showMinCart = false">
        <div class="cart_info">
            <div class="iconfont-pc icon-shiwu-gouwuche"></div>
            <a href="/cart/">
                {{ $t("我的购物车") }}
            </a>
            <div class="num">{{ total.totalCount }}</div>
        </div>
        <div v-if="showMinCart" class="list_wrapper">
            <template v-if="filterState.length > 0">
                <template v-for="(item, index) in filterState">
                    <div v-for="product in item.carts.slice(0, 5)" class="product-list">
                        <div class="lin">
                            <a :href="urlFormat({ path: 'product', seoLink: product.seoLink, sn: product.productSn })" class="photo" target="_blank">
                                <img :src="productImage(product)" :alt="product.productName || ''" width="50px" />
                            </a>
                            <div>
                                <a :href="urlFormat({ path: 'product', seoLink: product.seoLink, sn: product.productSn })" class="name" target="_blank">
                                    {{ product.productName }}
                                </a>
                                <div>
                                    <div>
                                        <FormatPrice
                                            :showText="false"
                                            v-model="product.price"
                                            :fontStyle="{ fontSize: '12px', color: '#ff4040' }"
                                        ></FormatPrice>
                                        × {{ product.quantity }}
                                    </div>
                                    <span @click="removeCartItem(product.cartId)">{{ $t("删除") }}</span>
                                </div>
                            </div>
                        </div>
                    </div>
                </template>
                <div class="tips">
                    <a href="/cart/" target="_blank">{{ $t("更多商品，去购物车查看") }}></a>
                </div>
                <div class="cart_border">&nbsp;</div>
                <div class="info">
                    <div>
                        {{ $t("购物车共") }} <span class="red">{{ total.totalCount }}</span> {{ $t("件商品") }}
                    </div>
                    <div>
                        {{ $t("总价") }}：
                        <FormatPrice
                            :showText="false"
                            v-model="total.productAmount"
                            :fontStyle="{ fontSize: '12px', color: '#ff4040', fontWeight: 'bold' }"
                        ></FormatPrice>
                    </div>
                </div>
                <div class="btu">
                    <NuxtLink to="/cart/">{{ $t("去购物车结算") }}</NuxtLink>
                </div>
            </template>
            <template v-else>
                <div class="cart_empty">
                    <div class="cartempty"></div>
                    <div>
                        {{ $t("购物车中还没有商品") }}
                        <br />{{ $t("赶紧选购吧") }}！
                    </div>
                </div>
            </template>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { getCart, removeCartItemData } from "@/api/cart";
import { urlFormat, productImage } from "~/utils/format";

const showMinCart = ref(false);
const filterState: any = ref([]);
const total = ref<object>({});
const onShowCartList = () => {
    showMinCart.value = true;
};
const loadFilter = async () => {
    try {
        const result = await getCart();
        filterState.value = result.cartList;
        Object.assign(total.value, result.total);
    } catch (error: any) {
        message.error(error.message);
    }
};
loadFilter();

const removeCartItem = async (cartId: number) => {
    try {
        const result = await removeCartItemData([cartId]);
        await loadFilter();
    } catch (error) {
    } finally {
    }
};
</script>
<style lang="less" scoped>
.cart {
    background: #fff;
    border: 1px solid #ddd;
    height: 38px;
    padding: 0;
    text-align: center;
    width: 140px;
    position: relative;
    z-index: 10;

    .cart_info {
        border-radius: 2px;
        cursor: pointer;
        display: flex;
        flex-direction: row;
        align-items: center;
        justify-content: center;
        line-height: 38px;
        gap: 4px;
        position: relative;

        & > a {
            font-size: 14px;
        }

        .num {
            position: absolute; /* 设置为绝对定位 */
            top: -5px; /* 顶部对齐 */
            right: -5px; /* 右侧对齐 */
            height: 20px;
            line-height: 20px;
            background-color: #ffcc00; /* 背景颜色，仅用于示例 */
            padding: 0 5px;
            color: #fff;
        }
    }

    .list_wrapper {
        width: 270px;
        background-color: #fff;
        position: absolute;
        top: 100%; /* 位于 .container 底部正下方 */
        right: -1px; /* 与 .container 右侧对齐 */
        border: 1px solid #ddd;
        border-radius: 2px;

        .product-list {
            display: flex;
            flex-direction: column;
            margin: 10px 0;

            .lin {
                display: flex;
                gap: 8px;
                margin: 10px 10px;

                & > img {
                    width: 50px;
                    height: 50px;
                }

                & > div {
                    display: flex;
                    flex-direction: column;
                    gap: 10px;
                    text-align: left;

                    & > div {
                        display: flex;
                        justify-content: space-between;

                        & > div {
                            color: #ff4040;
                        }

                        & > span {
                            color: #999;
                            cursor: pointer;
                        }
                    }
                }
            }
        }

        .tips {
            & > a {
                color: #999;
            }

            width: 100%;
            text-align: right;

            padding-right: 10px;
            box-sizing: border-box;
        }

        .cart_border {
            background: url("/assets/images/common/bg62.png") no-repeat scroll center 0 transparent;
            height: 8px;
            overflow: hidden;
            width: 100%;
        }

        .info {
            display: flex;
            justify-content: space-between;
            height: 40px;
            align-items: center;

            & > div {
                flex: 1;
                font-weight: bold;
                text-align: center;
            }
        }

        .btu {
            height: 40px;
            display: flex;
            justify-content: flex-end;
            padding-right: 10px;
            box-sizing: border-box;
        }

        .cart_empty {
            height: 42px;
            margin: 14px 0 25px 60px;
            padding-left: 50px;
            color: #999;
            display: flex;
            justify-content: center;
            align-items: center;

            .cartempty {
                background: url("/assets/images/common/spirit.png") no-repeat scroll -50px -76px;
                height: 49px;
                left: 35px;
                position: absolute;
                top: 12px;
                width: 56px;
            }
        }
    }
}
</style>
