<template>
    <!-- 新版 UI -->
    <div class="shop-info-wrap-new" v-if="isNewUi">
        <div class="shop-main">
            <div class="shop-logo">
                <NuxtLink v-if="shopHref" :to="shopHref" class="shop-link">
                    <img class="img" :src="imageFormat(shopInfo?.shopLogo)" alt="" />
                </NuxtLink>
                <img v-else class="img" :src="imageFormat(shopInfo?.shopLogo)" alt="" />
            </div>
            <div class="shop-detail">
                <NuxtLink v-if="shopHref" :to="shopHref" class="name-link">
                    <span class="name">{{ shopInfo?.shopTitle }}</span>
                    <el-icon class="name-arrow"><ArrowRight /></el-icon>
                </NuxtLink>
                <div v-else class="name-link">
                    <span class="name">{{ shopInfo?.shopTitle }}</span>
                </div>
                <div class="shop-meta">
                    <span v-if="shopBadge" class="shop-badge">{{ shopBadge }}</span>
                    <el-rate v-model="rating" disabled allow-half size="small" class="shop-rate" />
                    <span class="rate-score">{{ scoreText }}</span>
                    <span v-if="shopInfo?.shopDetailedAddress" class="meta-item">{{ shopInfo.shopDetailedAddress }}</span>
                </div>
            </div>
        </div>
        <div class="shop-actions">
            <CommonService :shopId="shopInfo?.shopId" :phone="shopInfo?.kefuPhone" :isIndex="true">
                <button type="button" class="action-btn">
                    <span class="iconfont-pc icon-kefu"></span>
                    {{ $t("客服") }}
                </button>
            </CommonService>
            <button v-if="shopHref" type="button" class="action-btn" @click="toShop">
                <span class="iconfont-pc icon-guanli"></span>
                {{ $t("进店") }}
            </button>
        </div>
    </div>

    <!-- 老版 UI -->
    <div class="shop-info-wrap" v-else>
        <div class="shop-logo">
            <NuxtLink v-if="shopHref" :to="shopHref" class="shop-link">
                <img class="img" :src="imageFormat(shopInfo?.shopLogo)" alt="" />
            </NuxtLink>
            <img v-else class="img" :src="imageFormat(shopInfo?.shopLogo)" alt="" />
        </div>
        <div class="shop-detail">
            <div class="name-warp">
                <NuxtLink v-if="shopHref" :to="shopHref" class="name-link">
                    <div class="name">{{ shopInfo?.shopTitle }}</div>
                </NuxtLink>
                <div v-else class="name">{{ shopInfo?.shopTitle }}</div>
            </div>
            <div class="button-wrap">
                <button v-if="shopHref" type="button" class="tig-button-no-ripple" @click="toShop" style="height: 32px">
                    <span class="button-content">
                        <div class="iconfont-pc icon-guanli"></div>
                        {{ enterLabel }}
                    </span>
                </button>
                <button type="button" class="tig-button-no-ripple" @click="addShopCollect(shopInfo?.shopId)" style="height: 32px">
                    <span class="button-content">
                        <div class="iconfont-pc icon-aixin"></div>
                        {{ shopInfo?.collectShop ? $t("已关注") : followLabel }}
                    </span>
                </button>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { ArrowRight } from "@element-plus/icons-vue";
import { imageFormat, urlFormat, ratingFormat } from "@/utils/format";
import { isStore } from "@/utils/util";
import { getShopCollect } from "~/api/product/product";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const props = defineProps({
    shopInfo: {
        type: Object,
        default: () => ({})
    },
    // UI 版本：old 老版（进店/关注按钮），new 新版（评分/客服）
    uiVersion: {
        type: String,
        default: "old"
    }
});
const emit = defineEmits(["refresh"]);

const isNewUi = computed(() => props.uiVersion === "new");

const shopHref = computed(() => {
    const shopId = (props as any)?.shopInfo?.shopId;
    // shopId=0（自营/官方）通常不进入门店/店铺页，这里做兜底避免跳到无效链接
    if (!shopId) return "";
    // 非 o2o：进入多商户店铺首页
    return urlFormat({ path: "shop", id: shopId, seoLink: (props as any)?.shopInfo?.seoLink });
});

// o2o 显示「门店」，非 o2o 显示「店铺」
const enterLabel = computed(() => {
    return t("进入店铺");
});
const followLabel = computed(() => {
    return t("关注店铺");
});

const rating = computed(() => Number(ratingFormat(props.shopInfo?.shopStar || props.shopInfo?.productScore || 5)));
const scoreText = computed(() => ratingFormat(props.shopInfo?.shopStar || props.shopInfo?.productScore || 5));
const shopBadge = computed(() => props.shopInfo?.shopRankName || props.shopInfo?.shopTypeName || "");

const router = useRouter();
const toShop = () => {
    if (!shopHref.value) return;
    router.push(shopHref.value);
};

const addShopCollect = async (shopId: number) => {
    try {
        await getShopCollect(shopId);
        message.success(props.shopInfo?.collectShop ? t("取消关注成功") : t("关注成功"));
        emit("refresh", shopId);
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};

const contactCustomerService = (kefuLink: any) => {
    window.open(kefuLink, "newWindow", "width=800,height=600,scrollbars=yes,resizable=yes");
};
</script>

<style lang="less" scoped>
/* ===================== 新版 UI 样式 ===================== */
.shop-info-wrap-new {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 24px;

    .shop-main {
        display: flex;
        align-items: center;
        min-width: 0;
    }

    .shop-logo {
        flex-shrink: 0;
        width: 52px;
        height: 52px;
        overflow: hidden;
        cursor: pointer;
        border: 2px solid #ebebeb;
        border-radius: 12px;
        .shop-link {
            display: block;
            width: 100%;
            height: 100%;
        }
        .img {
            width: 100%;
            height: 100%;
            background: #fff;
            object-fit: contain;
        }
    }

    .shop-detail {
        display: flex;
        flex-direction: column;
        justify-content: center;
        gap: 8px;
        margin-left: 12px;
        min-width: 0;

        .name-link {
            display: inline-flex;
            align-items: center;
            text-decoration: none;
            cursor: pointer;
            .name {
                font-size: 16px;
                line-height: 18px;
                color: #1a1a1a;
                font-weight: 600;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
            }
            .name-arrow {
                margin-left: 2px;
                font-size: 14px;
                color: #1a1a1a;
            }
            &:hover .name {
                color: var(--general);
            }
        }

        .shop-meta {
            display: flex;
            align-items: center;
            flex-wrap: wrap;
            gap: 8px;
            font-size: 12px;
            line-height: 18px;
            color: #9ba0a8;

            .shop-badge {
                display: inline-flex;
                align-items: center;
                height: 16px;
                padding: 0 4px;
                border-radius: 2px;
                background: #ff4d4f;
                color: #fff;
                font-size: 11px;
                line-height: 16px;
            }

            .shop-rate {
                height: 16px;
                line-height: 16px;
                :deep(.el-rate__icon) {
                    margin-right: 0;
                    font-size: 14px;
                }
                --el-rate-void-color: #9ba0a8;
            }

            .rate-score {
                color: var(--main-bg);
                font-weight: 600;
            }

            .meta-item {
                color: #9ba0a8;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
                max-width: 320px;
            }
        }
    }

    .shop-actions {
        display: flex;
        align-items: center;
        gap: 8px;
        flex-shrink: 0;

        .action-btn {
            box-sizing: border-box;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            gap: 6px;
            height: 40px;
            padding: 0 18px;
            border: 1px solid rgba(0, 0, 0, 0.08);
            border-radius: 20px;
            background: #fff;
            color: rgba(9, 9, 10, 0.92);
            font-size: 14px;
            font-weight: 400;
            letter-spacing: 0;
            cursor: pointer;
            user-select: none;
            transition:
                color 0.3s ease,
                border-color 0.3s ease;

            .iconfont-pc {
                font-size: 16px;
            }

            &:hover {
                color: var(--main-bg);
                border-color: var(--main-bg);
            }
        }
    }
}

/* ===================== 老版 UI 样式 ===================== */
.shop-info-wrap {
    display: flex;
    margin-bottom: 24px;
    .shop-logo {
        width: 58px;
        height: 58px;
        overflow: hidden;
        // border: .5px solid rgba(0, 0, 0, .06);
        border-radius: 4px;
        cursor: pointer;
        .shop-link {
            display: block;
            width: 100%;
            height: 100%;
        }
        .img {
            width: 100%;
            height: 100%;
            background: #fff;
            object-fit: contain;
        }
    }
    .shop-detail {
        display: flex;
        flex-direction: column;
        height: 58px;
        margin-left: 8px;
        .name-warp {
            display: flex;
            margin-bottom: auto;
            .name-link {
                display: inline-flex;
                text-decoration: none;
            }
            .name {
                font-size: 16px;
                line-height: 16px;
                color: #1a1a1a;
                font-weight: 600;
                cursor: pointer;
                &:hover {
                    color: var(--general);
                }
            }
        }
        .button-wrap {
            display: flex;
            column-gap: 8px;
            .icon {
                margin-right: 4px;
            }

            // 替换 TigButton：保持同款样式，但不带水波纹
            .tig-button-no-ripple {
                box-sizing: border-box;
                border: 1px solid #c2c4cc;
                outline: 0;
                margin: 0;
                background: none;
                min-width: 64px;
                color: #1a1a1a;
                cursor: pointer;
                user-select: none;
                display: inline-flex;
                align-items: center;
                justify-content: center;
                position: relative;
                padding: 5px 15px;
                font-size: 14px;
                height: 30px;
                border-radius: 4px;
                overflow: hidden;
                transition:
                    color 0.3s ease,
                    border-color 0.3s ease,
                    background-color 0.3s ease;

                &:hover {
                    color: var(--main-bg);
                    border-color: var(--main-bg);
                }

                &:disabled {
                    cursor: not-allowed;
                    opacity: 0.5;
                }

                .button-content {
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    gap: 8px;
                }
            }
        }
    }
}
</style>
