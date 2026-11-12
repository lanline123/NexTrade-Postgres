<template>
    <TotalContainer
        v-if="productList?.length > 0"
        :module="module"
        :class="
            'module-goods_ad ad-style__' +
            module.style +
            ' ad-buy_btn_style__' +
            module.buyBtnStyle +
            ' ad-goods_style__' +
            module.goodsStyle +
            ' ad-goods_radio_style__' +
            module.goodsRadioStyle +
            ' ad-text_align__' +
            module.textAlign +
            ' ad-text_weight__' +
            module.textWeight +
            ' ad-goods_name_row__' +
            module.goodsNameRow +
            ' ad-goods_name_padding__' +
            module.goodsNamePadding +
            ' ad-goods-title_style__' +
            module.title?.titleStyle
        "
    >
        <Title :module="module.title"></Title>
        <template v-if="productList.length > 0">
            <div class="goods-ad-warp">
                <div class="goods-ad-con">
                    <ProductCard v-for="item in productList" :key="item.productId" :module="module" :product="item" :size="getSize()" :product-style="styleMap[item.productId]"></ProductCard>
                </div>
            </div>
        </template>
    </TotalContainer>
</template>

<script lang="ts" setup>
/**
 * PC端装修商品列表组件
 * - 支持三种商品选择方式：手动选择(1)、按分类(2)、按标签(3)
 * - 支持多种布局样式（style 1-7）和购买按钮样式
 * - 支持店铺筛选、商品名称行数、圆角样式等配置
 */
import { ref } from "vue";
import { getProductsList } from "@/api/common";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";
import Title from "~/pages/activity/components/Title.vue";
import ProductCard from "~/components/shop/product/ProductCard.vue";
import ProductCardLand from "~/components/shop/product/ProductCardLand.vue";
import OneRow3Carousel from "~/components/shop/product/OneRow3Carousel.vue";
import { useProductStyle } from "~/hooks";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    },
    moduleIndex: {
        type: Number,
        default: 0
    },
    shopId: {
        type: Number,
        default: -1
    }
});

const paramsValueMap: { [key: number]: string[] } = {
    1: ["productIds"],
    2: ["productCategoryId", "productNumber"],
    3: ["productTag", "productNumber"]
};
const paramsKeyMap: { [key: string]: string } = {
    productIds: "ids",
    productCategoryId: "categoryId",
    productNumber: "size",
    productTag: "introType"
};
const productList = ref<any[]>([]);
const { styleMap } = useProductStyle(productList);

const _getproductList = async () => {
    let param: any = {};
    if (typeof props.module.products === "object") {
        if (props.module.products.productSelectType) {
            const type = props.module.products.productSelectType;
            const keys = paramsValueMap[type];
            if (type == 1) {
                param[paramsKeyMap[keys[0]]] = props.module.products.productIds.join(",");
                param.size = props.module.products.productIds.length;
            } else {
                param.page = 1;
                keys.forEach((key) => {
                    param[paramsKeyMap[key]] = props.module.products[key];
                });
            }
        }
    }
    if (props.shopId && props.shopId != -1) {
        param.shopId = props.shopId;
    }
    try {
        const result = await getProductsList(param);
        productList.value = result.records;
    } catch (error) {
        console.log(error);
    }
};
_getproductList();

const getSize = (): number => {
    // 根据style返回每行显示的商品数量
    const style = typeof props.module.style === "string" ? Number(props.module.style) : props.module.style;

    switch (style) {
        case 1:
            return 4;
        case 2:
            return 5;
        default:
            return 4;
    }
};
</script>

<style lang="less" scoped>
/*商品类型*/
.module-goods_ad {
    :deep(.base-class) {
        width: 100%;
        min-height: 1px;
        box-sizing: border-box;
    }

    .goods-ad-warp {
        width: 100%;
        box-sizing: border-box;

        .goods-ad-con {
            display: flex;
            flex-wrap: wrap;
            width: 100%;
        }
    }
}

/* Style 1: 单列 */
.ad-style__1 .goods-ad-warp .goods-ad-con {
    display: flex;
    // flex-direction: column;
}

/* Style 2: 两列 */
.ad-style__2 .goods-ad-warp .goods-ad-con {
    display: flex;
    flex-wrap: wrap;
}

/* Style 3: 三列 */
.ad-style__3 .goods-ad-warp .goods-ad-con {
    display: flex;
    flex-wrap: wrap;
}

/* Style 4: 特殊布局 */
.ad-style__4 .goods-ad-warp .goods-ad-con {
    display: flex;
    flex-wrap: wrap;
}

/* Style 6: 横向滑动 */
.ad-style__6 .goods-ad-warp {
    overflow: hidden;

    .goods-ad-con {
        display: flex;
        flex-wrap: nowrap;
        overflow-x: auto;
        margin-bottom: -20px;
        padding-bottom: 20px;
        -webkit-overflow-scrolling: touch;
        scrollbar-width: none; /* Firefox */
        -ms-overflow-style: none; /* IE and Edge */

        &::-webkit-scrollbar {
            display: none; /* Chrome, Safari, Opera */
        }
    }
}

/* 购买按钮样式 */
.ad-buy_btn_style__0 .goods-ad-warp .goods-ad-item .item-info .item-buy {
    display: none;
}

/* 商品样式 */
.ad-goods_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item .item-con {
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.06);
}

.ad-goods_style__3 .goods-ad-warp .goods-ad-con .goods-ad-item .item-con {
    border: 1px solid rgba(50, 50, 51, 0.1);
}

.ad-goods_style__4 .goods-ad-warp .goods-ad-item .item-info {
    background: transparent;
}

/* 圆角样式 */
.ad-goods_radio_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item .item-con {
    border-radius: 6px;
}

.ad-goods_radio_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item .item-con .item-photo {
    border-radius: 6px 6px 0 0;
    overflow: hidden;
}

.ad-goods_radio_style__2 .goods-ad-warp .goods-ad-con .goods-ad-item .item-con .item-info {
    border-radius: 0 0 6px 6px;
}

/* 文本对齐 */
.ad-text_align__2 .goods-ad-warp .goods-ad-con .goods-ad-item {
    text-align: center;
}

/* 文本粗细 */
.ad-text_weight__2 .goods-ad-warp .goods-ad-item .item-info .item-name .item-name-a {
    font-weight: bold;
}

/* 商品名称行数 */
.ad-goods_name_row__1 .goods-ad-warp :deep(.goods-ad-item .item-info .item-name .item-name-a),
.ad-goods_name_row__1 .goods-ad-warp :deep(.goods-ad-item .item-info .item-name .item-product-name),
.ad-goods_name_row__1 .goods-ad-warp :deep(.product-card .item-info .item-name .item-product-name) {
    line-height: 20px;
    height: 20px;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 1;
    line-clamp: 1;
    -webkit-box-orient: vertical;
    overflow: hidden;
    white-space: nowrap;
}
.ad-goods_name_row__2 .goods-ad-warp :deep(.goods-ad-item .item-info .item-name .item-name-a),
.ad-goods_name_row__2 .goods-ad-warp :deep(.goods-ad-item .item-info .item-name .item-product-name),
.ad-goods_name_row__2 .goods-ad-warp :deep(.product-card .item-info .item-name .item-product-name) {
    line-height: 20px;
    height: 40px;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    white-space: normal;
    word-break: break-word;
}

/* 商品名称内边距 */
.ad-goods_name_padding__0 .goods-ad-warp .goods-ad-item .item-info .item-name {
    margin-left: 0;
    margin-right: 0;
}

.ad-goods_name_padding__0 .goods-ad-warp .goods-ad-item .item-info .item-action {
    padding-left: 0;
    padding-right: 0;
}

/* 标题样式 */
.ad-goods-title_style__3 .goods-ad-warp {
    border-top-left-radius: 15px;
    margin-top: -25px !important;
    overflow: hidden;
    background: #fff;
    padding-top: 10px;
    position: relative;
}
</style>
