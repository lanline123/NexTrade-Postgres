<template>
    <div class="shop-info-card">
        <NuxtLink :to="urlFormat({ path: 'shop', id: item.shopId })" target="_blank" class="shop-logo">
            <img :src="imageFormat(item.shopLogo)" class="logo-img" :alt="item.shopTitle || ''" loading="lazy" />
        </NuxtLink>
        <div class="shop-content">
            <div class="shop-title">
                <NuxtLink :to="urlFormat({ path: 'shop', id: item.shopId })" target="_blank" class="shop-name" :title="item.shopTitle">
                    <span class="ellipsis">{{ item.shopTitle }}</span>
                </NuxtLink>
            </div>
            <div class="shop-rating-row">
                <div v-if="item.shopStar" class="rating-box">
                    <span class="rating-score">{{ ratingFormat(item.shopStar) }}</span>
                    <div class="line"></div>
                    <div class="stars">
                        <CommonAppRate :model-value="item.shopStar" disabled :size="14" text-color="#ff7a00" />
                    </div>
                </div>
                <span class="fans-count">{{ formatFansCount(item.collectCount) }} {{ $t("粉丝") }}</span>
            </div>
            <div class="enter-shop-btn">
                <NuxtLink :to="urlFormat({ path: 'shop', id: item.shopId })" target="_blank" class="btn-link">
                    <img src="~/assets/images/shop/shop_icon.png" class="shop-icon" alt="" /> {{ $t("进入店铺") }}
                </NuxtLink>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
// ===================== 类型导入 =====================
import type { shopListItem } from "~/types/shop/shop.d";

// ===================== 工具函数 =====================
import { imageFormat, urlFormat, ratingFormat } from "~/utils/format";

// ===================== Props & Emits =====================
defineProps<{
    item: shopListItem;
}>();

// ===================== 工具方法 =====================
const formatFansCount = (count: number): string | number => {
    if (count >= 10000) {
        const wan = Math.floor(count / 100) / 100;
        return `${wan}万`;
    }
    return count || 0;
};
</script>

<style scoped lang="less">
.shop-info-card {
    max-width: 432px;
    min-width: 432px;
    width: 432px;
    min-width: 0;
    flex-shrink: 0;
    background: #fff;
    padding: 8px 0;
    box-sizing: border-box;
    display: flex;
    flex-direction: row;
    align-items: flex-start;
    gap: 16px;

    .shop-logo {
        max-height: 96px;
        max-width: 96px;
        border-radius: 12px;
        overflow: hidden;
        flex-shrink: 0;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        background-color: #f7f7f7;

        .logo-img {
            width: 100%;
            height: 100%;
            object-fit: contain;
        }
    }

    .shop-content {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        justify-content: flex-start;
        .shop-title {
            display: flex;
            align-items: center;
            gap: 8px;
        }

        .shop-name {
            color: #1f1f1f;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            //margin-bottom: 14px;
            text-decoration: none;
            font-family: PingFang SC;
            font-size: 20px;
            font-weight: 600;
            letter-spacing: 0;
            line-height: 28px;

            .ellipsis {
                display: inline-block;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
                max-width: 222px;
            }

            &:hover {
                color: #1f1f1f !important;
            }
        }

        .shop-rating-row {
            display: flex;
            align-items: center;
            margin-bottom: 12px;
            flex-wrap: wrap;

            .rating-box {
                align-items: center;
                background-color: #ffeee6;
                border-radius: 4px;
                display: flex;
                flex-direction: row;
                height: 20px;
                justify-content: center;
                margin-right: 8px;
                padding: 0 6px;

                .rating-score {
                    color: #ff5000;
                    font-size: 12px;
                    font-weight: 500;
                    letter-spacing: 0;
                    line-height: 12px;
                }
                .line {
                    background-color: rgba(255, 98, 0, 0.2);
                    height: 12px;
                    margin: 0 3px 0 5px;
                    width: 1px;
                }

                .stars {
                    display: flex;
                    align-items: center;
                    height: 14px;

                    :deep(.app-rate) {
                        height: 11px;
                    }
                }
            }

            .fans-count {
                color: #7a7a7a;
                font-size: 14px;
                font-weight: 400;
                letter-spacing: 0;
                line-height: 22px;
                margin-right: 8px;
            }
        }

        .enter-shop-btn {
            .btn-link {
                align-items: center;
                justify-content: center;
                gap: 8px;
                text-decoration: none;
                transition: all 0.2s ease;
                background: #fff;
                border: 1px solid #ebebeb;
                border-radius: 8px;
                cursor: pointer;
                display: inline-flex;
                flex-direction: row;
                height: 36px;
                padding: 10px 12px;
                width: fit-content;
                max-width: 100%;
                box-sizing: border-box;
                color: #1f1f1f;
                font-size: 15px;
                font-weight: 600;
                letter-spacing: 0;
                white-space: nowrap;

                .shop-icon {
                    width: 18px;
                    height: 18px;
                }

                &:hover {
                    background: #f5f5f5;
                    color: #1f1f1f !important;
                }
            }
            a {
                &:hover {
                    color: #1f1f1f;
                }
            }
        }
    }
}
</style>
