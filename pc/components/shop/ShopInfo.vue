<template>
    <div class="shop-info">
        <div class="shop-info-bar">
            <div class="shop-info-title">
                <div class="title">
                    {{ shopInfo.shopTitle }}
                </div>
                <el-popover
                    v-if="hasLicense"
                    placement="bottom-end"
                    :show-arrow="true"
                    :teleported="false"
                    :width="246"
                    :offset="6"
                    transition="shop-qualification-popover"
                    trigger="hover"
                    persistent
                    popper-class="shop-qualification-popper"
                >
                    <Mark :shopInfo="shopInfo"></Mark>
                    <template #reference>
                        <span class="meta-link">{{ $t("查看资质") }}<el-icon><ArrowDown /></el-icon></span>
                    </template>
                </el-popover>
                <div class="shop-rating">
                    <div class="shop-gray"></div>
                    <div class="shop-red" :style="{ width: shopInfo.shopStar ? Math.round((shopInfo.shopStar / 5) * 100) + '%' : '50px' }"></div>
                </div>
                <!-- 8.00分 -->
                <span class="shop-ratingHigh"></span>
            </div>
            <el-divider direction="vertical" />
            <div class="right-cao">
                <div v-if="shopInfo.kefuLink" class="collect-style-btn" @click="contactCustomerService()">
                    <span class="kefuicon"></span>
                    <span class="text">{{ $t("联系客服") }}</span>
                </div>
                <div class="collect-style-btn" @click="addShopCollect()">
                    <span
                        class="iconfont-pc collect font-color"
                        :class="{ 'icon-shoucang': !shopInfo.collectShop, 'icon-yishoucang': shopInfo.collectShop }"
                    ></span>
                    <span class="text">{{ !shopInfo.collectShop ? $t("收藏店铺") : $t("取消收藏") }}</span>
                </div>
            </div>
        </div>
        <div v-if="module.image" class="image-bar">
            <el-image :class="['box-size']" :src="imageFormat(module.image?.picUrl)"></el-image>
        </div>
        <div class="controls-bar" :style="{ backgroundColor: module.backgroundColor }">
            <div class="info-bar">
                <div class="left-bar">
                    <NuxtLink :to="{ path: '/shop/info', query: { shopId: shopInfo?.shopId } }" :style="{ color: module.textColor }" class="bar-item"
                        >{{ $t("首页") }}
                    </NuxtLink>
                    <el-popover placement="bottom-start" :show-arrow="false" :teleported="false" :width="1190" trigger="hover">
                        <ProductSort :shopId="shopInfo.shopId" :showSublevel="module.showSublevel" :treeList="treeList"></ProductSort>
                        <template #reference>
                            <NuxtLink :to="'/ShopCategory/' + shopId" :style="{ color: module.textColor }" class="bar-item">
                                {{ $t("全部分类") }}
                                <div class="arrow"></div>
                            </NuxtLink>
                        </template>
                    </el-popover>
                </div>
                <div class="right-bar">
                    <el-input size="small" v-model="input3" style="max-width: 600px" :placeholder="$t('请输入关键字')">
                        <template #append>
                            <NuxtLink :to="'/shop/search/' + shopId + '?keyword=' + input3">
                                <el-button class="query">{{ $t("搜索") }}</el-button>
                            </NuxtLink>
                        </template>
                    </el-input>
                </div>
            </div>
        </div>
    </div>
</template>
<script setup lang="ts">
import { computed, ref } from "vue";
import { ArrowDown } from "@element-plus/icons-vue";
import { getShopCollect, getShopDetail, getShopHead, getShopTree } from "~/api/product/product";
import { useUserStore } from "~/store/user";
import Mark from "~/components/shop/src/Mark.vue";
import ProductSort from "~/components/shop/src/ProductSort.vue";

const percentage = ref(80);
const duration = ref(10);
const props = defineProps({
    shopId: { type: Number, default: -1 },
    pageModule: { type: Object, default: {} }
});
const input3 = ref("");

const shopInfo: any = ref({});
const module: any = ref({});
const hasLicense = computed(() => {
    const pic = shopInfo.value?.businessLicensePic;
    return typeof pic === "string" && pic.trim().length > 0;
});
const getShop = async () => {
    try {
        const result = await getShopDetail(props.shopId);
        Object.assign(shopInfo.value, result);
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
const treeList = ref([]);
const emit = defineEmits(["treeList"]);
const getSort = async () => {
    try {
        const result = await getShopTree(props.shopId);
        treeList.value = result;

        emit("treeList", treeList.value);
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
const getHead = async () => {
    try {
        const result = await getShopHead(props.shopId);
        Object.assign(module.value, result?.data);
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
onMounted(() => {
    getHead();
    getSort();
});
const contactCustomerService = () => {
    window.open(shopInfo.value.kefuLink, "newWindow", "width=800,height=600,scrollbars=yes,resizable=yes");
};
const { t } = useI18n();
const userStore = useUserStore();
const addShopCollect = async () => {
    try {
        await getShopCollect(props.shopId);
        // message.success(t('操作成功'));
        await getShop();
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
const hoverColor = computed(() => {
    return module.value.hoverColor || "#333";
});
getShop();
</script>
<style scoped lang="less">
.shop-info {
    display: flex;
    flex-direction: column;
    align-items: center;
    background: #fafafa none repeat scroll 0 0;

    .shop-info-bar {
        width: 1190px;
        height: 30px;
        display: flex;
        align-items: center;
        justify-content: flex-end;

            .shop-info-title {
            display: flex;
            gap: 12px;
            align-items: center;

            .title {
                font-size: 12px;
                color: #333333;
            }

            .meta-link {
                display: inline-flex;
                align-items: center;
                gap: 2px;
                font-size: 12px;
                line-height: 12px;
                color: #999;
                cursor: pointer;
                white-space: nowrap;

                .el-icon {
                    font-size: 9px;
                    width: 9px;
                    height: 9px;
                }
            }

            .shop-rating {
                position: relative;
                height: 16px;
                .shop-gray,
                .shop-red {
                    background: url("@/assets/images/shop/store_header.png") no-repeat scroll 0 -18px;
                    display: inline-block;
                    height: 16px;
                    width: 70px;
                }
                .shop-red {
                    background-position: right 0;
                    height: 16px;
                    left: 0;
                    position: absolute;
                    top: 0;
                }
            }
            .shop-ratingHigh {
                background: url("@/assets/images/shop/store_header.png") 0 -60px no-repeat;
                height: 9px;
                display: inline-block;
                zoom: 1;
                vertical-align: middle;
                width: 9px;
            }
        }

        .right-cao {
            display: flex;
            gap: 8px;
            .kefuicon {
                width: 18px;
                height: 18px;
                background: url("@/assets/images/shop/store_kefu.png") no-repeat 100% 100%;
            }

            .text:hover {
                color: var(--general);
            }
        }

        .collect-style-btn {
            cursor: pointer;
            display: flex;
            gap: 4px;
            align-items: center;
        }
    }

    .image-bar {
        display: flex;

        .box-size {
            flex: 1;
        }
    }

    .controls-bar {
        display: flex;
        width: 100%;
        background-color: #158fce;
        height: 40px;
        align-items: center;
        justify-content: center;

        .info-bar {
            width: 1190px;
            display: flex;
            justify-content: space-between;
            align-items: center;

            .left-bar {
                display: flex;
                gap: 10px;

                .bar-item {
                    cursor: pointer;
                    height: 26px;
                    color: #ffffff;
                    font-weight: bold;
                    line-height: 26px;
                    font-size: 14px;
                    display: flex;
                    align-items: center;
                    gap: 4px;
                    padding: 0 20px;
                    margin: 8px 0;
                    border-radius: 26px;
                    .arrow {
                        display: inline-block;
                        vertical-align: middle;
                        margin-left: 4px;
                        font-size: 0;
                        line-height: 0;
                        height: 0;
                        width: 0;
                        border-bottom: 0;
                        border-left: 5px dashed transparent;
                        border-right: 5px dashed transparent;
                        border-top: 5px solid #fff;
                        position: static;
                        transition: all 500ms;
                    }
                }

                .bar-item:hover {
                    background-color: v-bind("hoverColor") !important;
                    color: #fff !important;
                    .arrow {
                        transform: rotateZ(-180deg);
                    }
                }
            }

            .right-bar {
                display: flex;

                .query {
                    background-color: #646464;
                    color: #fff;
                    height: 22px;
                    display: flex;
                    align-items: center;
                    border-radius: 0;
                    padding: 0 10px;
                    border: 1px solid #646464;
                    &:hover {
                        opacity: 0.8;
                    }
                }
            }
        }
    }
}

:deep(.el-input__wrapper) {
    height: 20px;
    border-radius: 0;
}
</style>
