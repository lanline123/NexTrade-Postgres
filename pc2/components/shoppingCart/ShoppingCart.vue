<template>
    <ShoppingCartList>
        <template #default="{ count }">
            <div data-control-rev="aTIyODJyZWo" class="cart_info">
                <NuxtLink to="/cart/" class="acea-row row-center-wrapper">
                    <div class="iconfont-pc icon-shiwu-gouwuche"></div>
                    <div class="my-cart">{{ $t("我的购物车") }}</div>
                    <em>{{ count }}</em>
                </NuxtLink>
            </div>
        </template>
    </ShoppingCartList>
</template>
<script setup lang="ts">
import { onMounted, ref } from "vue";
import { removeCartItemData } from "~/api/cart";
import { useCartStore } from "~/store/cart";
import { useUserStore } from "~/store/user";
const cartStore: any = useCartStore();
const userStore = useUserStore();

const props = defineProps({
    pageType: {
        type: Number,
        default: 1
    }
});


onMounted(() => {
    if (userStore.userInfo.userId > 0) {
        cartStore.changeCartList();
    }
});
</script>
<style scoped lang="less">
/* CSS部分 */
.cart {
    background: #fff;
    border: 1px solid #ddd;
    height: 38px;
    padding: 0;
    text-align: center;
    width: 140px;
    flex-shrink: 0;
    position: relative;
    z-index: 10;
    margin-left: 80px;

    @media screen and (max-width: 1200px) {
        margin-left: 20px;
    }

    .cart_info {
        position: relative;
        z-index: 9;
    }

    .cart_info a {
        line-height: 38px;
        font-size: 12px;
        color: #666;

        div:first-child {
            margin-right: 5px;
        }

        .my-cart {
            font-size: 14px;
        }

        em {
            padding: 0 5px;
            color: var(--main-text);
            background-color: var(--main-bg);
            position: absolute;
            right: -5px;
            top: -5px;
            display: block;
            line-height: 20px;
            height: 20px;
            border-radius: 2px;
        }
    }

    .list_wrapper {
        position: absolute;
        right: 0;
        top: 30px;
        background-color: #f5f5f5;
        border: 1px solid #ddd;

        .list {
            padding-top: 10px;
            background-color: #ffffff;
            border: 1px solid #ddd;
            position: absolute;
            right: -2px;
            text-align: left;
            top: 5px;
            width: 270px;

            .cart_empty {
                height: 42px;
                margin: 14px 0 25px 60px;
                padding-left: 50px;
                color: #999;

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
}

/*购物车窄树形类*/
.goods-list-min .item {
    padding: 10px 5px;
    background: #fff;
    margin-bottom: 6px;
    display: flex;
}

.goods-list-con {
    max-height: 350px;
    overflow-y: auto;
    overflow-x: hidden;
}

.goods-list-min .info {
    display: inline-block;
    font-size: 12px;
    width: 100%;
}

.goods-list-min .photo {
    display: inline-block;
    width: 50px;
    height: 50px;
    margin-right: 5px;
    border: 1px solid #ddd;
}

.goods-list-min .photo img {
    width: 50px;
    height: 50px;
}

.goods-list-min .name {
    line-height: 16px;
    color: #333;
    height: 32px;
    overflow: hidden;
    display: inline-block;
    margin-bottom: 4px;
}

.goods-list-min .name:hover {
    text-decoration: none;
}

.goods-list-min .name em {
    margin-left: 3px;
}

.goods-list-min span.price {
    overflow: hidden;
    display: flex;
    width: 100%;
    justify-content: space-between;
}

.goods-list-min span.price b {
    font-weight: normal;
    font-size: 12px;
    flex: 1;
}

.goods-list-min .del_btn {
    color: #999999;
    height: 15px;
    line-height: 16px;
    right: 10px;
    text-align: center;
    width: 35px;
    display: inline-block;
    overflow: hidden;
    cursor: pointer;
}
.del_btn:hover {
    color: var(--general);
}

.goods-list-min .item {
    margin-bottom: 0;
    padding: 10px;
}

.goods-list-min .cart_more {
    padding: 5px 10px;
    text-align: right;
}

.goods-list-min .cart_more a {
    color: #999999;
}

.cart:hover .cart_info {
    background-color: #fff;
    background-position: 4px -270px;
}

.cart .list .cart_border {
    margin-top: 10px;
    background: url("/assets/images/common/bg62.png") no-repeat scroll center 0 transparent;
    height: 8px;
    overflow: hidden;
    width: 100%;
}

.cart .list .cart_box {
    padding-bottom: 15px;
    text-align: right;
}

.cart .list .cart_num {
    font-size: 12px;
    padding: 10px 18px 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.cart .list .cart_num1 {
    text-align: left;
    flex: 1;
}

.cart .list .cart_num .red {
    color: var(--general);
    font-weight: bold;
}

.cart .list .cart_num .red b {
    color: var(--general);
    font-weight: bold;
}

.cart .list .cart_num2 {
    text-align: right;
}

.cart .list .cart_box .btn-css3 {
    margin-top: 10px;
    margin-right: 15px;
}

.cart .list .cart_more {
    padding: 5px 10px;
    text-align: right;
}

.cart .list .cart_more a {
    color: #999999;
}
.acea-row {
    display: flex;
    flex-wrap: wrap;
}

.acea-row.row-middle {
    align-items: center;
}

.acea-row.row-right {
    justify-content: flex-end;
}

.acea-row.row-center-wrapper {
    align-items: center;
    justify-content: center;
}

.acea-row.row-between-wrapper {
    align-items: center;
    justify-content: space-between;
}
</style>
