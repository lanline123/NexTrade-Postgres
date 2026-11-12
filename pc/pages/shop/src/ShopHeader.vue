<template>
    <div class="shop-header-card">
        <div class="container">
            <div class="shop-info-wrapper">
                <div class="shop-logo">
                    <el-image :src="imageFormat(shopInfo.shopLogo)" fit="contain" :alt="shopInfo.shopTitle || ''">
                        <template #error>
                            <div class="image-slot">
                                <img src="@/assets/images/shop/shop_icon.png" alt="" />
                            </div>
                        </template>
                    </el-image>
                </div>
                <div class="shop-meta">
                    <div class="shop-title-row">
                        <div class="shop-title">{{ shopInfo.shopTitle }}</div>
                        <el-popover
                            v-if="hasLicense"
                            placement="bottom-start"
                            :show-arrow="true"
                            trigger="hover"
                            :width="246"
                            :offset="6"
                            transition="shop-qualification-popover"
                            :teleported="true"
                            persistent
                            popper-class="shop-qualification-popper"
                        >
                            <ShopQualification :shop-info="shopInfo" />
                            <template #reference>
                                <span class="meta-link">{{ $t("查看资质") }}<el-icon><ArrowDown /></el-icon></span>
                            </template>
                        </el-popover>
                    </div>
                    <div class="shop-ratings">
                        <el-rate v-model="rating" disabled text-color="#ff9900" disabled-void-color="#999999" />
                        <span class="fans-count">{{ shopInfo.collectCount || 0 }}&nbsp;{{ $t("粉丝") }}</span>
                    </div>
                    <div class="shop-scores">
                        <div class="score-item">
                            {{ $t("商品评分") }} <span class="score-val">{{ ratingFormat(shopInfo.productScore) }}</span>
                        </div>
                        <div class="score-item">
                            {{ $t("服务评分") }} <span class="score-val">{{ ratingFormat(shopInfo.shopStar) }}</span>
                        </div>
                    </div>
                </div>
                <div class="shop-actions">
                    <el-button class="action-btn kefu" @click="contactCustomerService">
                        <span class="iconfont-pc icon-kefu"></span>
                        {{ $t("客服") }}
                    </el-button>
                    <el-button
                        class="action-btn follow"
                        :type="shopInfo.collectShop ? 'default' : 'primary'"
                        :class="{ followed: shopInfo.collectShop }"
                        @click="toggleCollect"
                    >
                        <el-icon v-if="!shopInfo.collectShop" style="margin-right: 4px"><Plus /></el-icon>
                        <!-- <span v-else class="iconfont-pc icon-yishoucang" style="margin-right: 4px"></span> -->
                        {{ shopInfo.collectShop ? $t("已关注") : $t("关注") }}
                    </el-button>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
/**
 * @description 店铺头部信息组件
 * - 展示店铺Logo、名称、评分、粉丝数
 * - 提供联系客服和关注/取消关注操作
 * - 店名旁「查看资质」hover 展示评分和企业资质
 */
import { computed, ref } from "vue";
import { imageFormat, ratingFormat } from "@/utils/format";
import { ArrowDown, Plus } from "@element-plus/icons-vue";
import ShopQualification from "~/components/shop/src/ShopQualification.vue";

const props = defineProps<{
    shopInfo: any;
}>();

const emit = defineEmits(["collect", "kefu"]);

const rating = ref(5.0);
const hasLicense = computed(() => {
    const pic = props.shopInfo?.businessLicensePic;
    return typeof pic === "string" && pic.trim().length > 0;
});

const contactCustomerService = () => {
    emit("kefu");
};

const toggleCollect = () => {
    emit("collect");
};
</script>

<style scoped lang="less">
.shop-header-card {
    background: #fff;
    width: 1190px;
    margin: 0 auto;
    padding: 20px 0px;
    margin-bottom: 20px;
    margin-top: 20px;
    box-sizing: border-box;
    border-radius: 8px;

    .container {
        width: 1190px;
        box-sizing: border-box;
    }

    .shop-info-wrapper {
        display: flex;
        align-items: center;
        padding: 0 10px;

        .shop-logo {
            width: 80px;
            height: 80px;
            border-radius: 4px;
            border: 1px solid #eee;
            overflow: hidden;
            margin-right: 20px;

            :deep(.el-image) {
                width: 100%;
                height: 100%;
            }
        }

        .shop-meta {
            flex: 1;
            min-width: 0;
            display: flex;
            flex-direction: column;
            justify-content: space-between;
            height: 80px;
            padding: 2px 0;
            margin-right: 20px;

            .shop-title-row {
                display: flex;
                align-items: center;
                gap: 12px;
                min-width: 0;
            }

            .shop-title {
                font-size: 20px;
                font-weight: bold;
                color: #333;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
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
                flex-shrink: 0;

                .el-icon {
                    font-size: 9px;
                    width: 9px;
                    height: 9px;
                }
            }

            .shop-ratings {
                display: flex;
                align-items: center;
                font-size: 12px;
                color: #666;

                :deep(.el-rate) {
                    height: 16px;
                    --el-rate-icon-size: 14px;
                    margin-right: 10px;
                }

                .fans-count {
                    color: #999;
                    margin-left: 10px;
                }
            }

            .shop-scores {
                display: flex;
                gap: 12px;
                font-size: 12px;
                color: #666;

                .score-item {
                    background-color: #f7f8fa;
                    padding: 2px 8px;
                    border-radius: 2px;

                    .score-val {
                        color: #e23e3e;
                        font-weight: bold;
                        margin-left: 4px;
                    }
                }
            }
        }

        .shop-actions {
            display: flex;
            gap: 10px;
            flex-shrink: 0;
            box-sizing: border-box;
            justify-content: flex-end;

            .action-btn {
                height: 32px;
                padding: 0 16px;
                border-radius: 5px;
                font-size: 14px;

                .iconfont-pc {
                    margin-right: 4px;
                    font-size: 16px;
                }

                &.kefu {
                    border: 1px solid #ddd;
                    color: #666;
                    background: #fff;

                    &:hover {
                        border-color: var(--general);
                        color: var(--general);
                    }
                }

                &.follow {
                    &.el-button--primary {
                        background: #ff4040;
                        border-color: #ff4040;
                        color: #fff;

                        &:hover {
                            background: #e23e3e;
                            border-color: #e23e3e;
                        }
                    }

                    &.followed {
                        background: #f5f5f5;
                        color: #999;
                        border-color: #ddd;

                        &:hover {
                            background: #e6e6e6;
                            border-color: #dcdcdc;
                            color: #999;
                        }
                    }
                }
            }
        }
    }
}

.image-slot {
    width: 100%;
    height: 100%;
    img {
        width: 100%;
        height: 100%;
        object-fit: contain;
    }
}
</style>
