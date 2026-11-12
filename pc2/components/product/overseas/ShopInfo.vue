<template>
    <div class="shop-info-wrap">
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
                        <SvgIcon name="loveSolid" width="14px" height="14px" class="icon"></SvgIcon>
                        {{ shopInfo?.collectShop ? $t("已关注") : followLabel }}
                    </span>
                </button>
                <TigButton @click="contactCustomerService(shopInfo?.kefuLink)" style="height: 32px" v-if="shopInfo?.kefuLink">
                    <SvgIcon name="customer-service" width="14px" height="14px" class="icon"></SvgIcon>
                    {{ $t("联系客服") }}
                </TigButton>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { imageFormat, urlFormat } from "@/utils/format";
import { isStore } from "@/utils/util";
import { getShopCollect } from "~/api/product/product";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const props = defineProps({
    shopInfo: {
        type: Object,
        default: () => {}
    }
});
const emit = defineEmits(["refresh"]);

// 与 components/product/ShopInfo.vue 一致：o2o 走门店，非 o2o 走店铺
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
