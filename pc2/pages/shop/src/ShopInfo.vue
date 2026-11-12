<template>
    <div class="shop-info-card">
        <div class="shop-logo">
            <NuxtLink :to="urlFormat({ path: 'shop', seoLink: item.seoLink, id: item.shopId })" target="_blank">
                <el-image :src="imageFormat(item.shopLogo)" fit="contain" class="logo-img" :alt="item.shopTitle || ''">
                    <template #error>
                        <div class="image-slot">
                            <img src="" alt="" />
                        </div>
                    </template>
                </el-image>
            </NuxtLink>
        </div>
        <div class="shop-content">
            <div class="shop-name" :title="item.shopTitle">
                <NuxtLink :to="urlFormat({ path: 'shop', seoLink: item.seoLink, id: item.shopId })" target="_blank" class="name-link">
                    <span class="ellipsis">{{ item.shopTitle }}</span>
                </NuxtLink>
            </div>
            <div class="shop-rating-row">
                <div class="rating-box-container">
                    <div class="rating-box" v-if="item.shopStar">
                        <span class="rating-score">{{ ratingFormat(item.shopStar) }}</span>
                        <span class="separator">|</span>
                        <div class="stars">
                            <el-rate v-model="item.shopStar" disabled show-score text-color="#ff9900" score-template="" disabled-void-color="#999999" />
                        </div>
                    </div>
                    <div v-else>{{ $t("暂无评分") }}</div>
                </div>
                <span class="fans-count">{{ formatFansCount(item.collectCount) }} {{ $t("粉丝") }}</span>
            </div>
            <div class="enter-shop-btn">
                <NuxtLink :to="urlFormat({ path: 'shop', seoLink: item.seoLink, id: item.shopId })" target="_blank" class="btn-link">
                    <tig-icon name="shop" :size="14" class="shop-icon" /> {{ $t("进入店铺") }}
                </NuxtLink>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
/**
 * @description 店铺信息卡片组件（店铺街行卡片布局）
 * - 在店铺列表中展示单个店铺的信息
 * - 显示店铺 Logo、名称、评分、粉丝数与进入店铺入口
 */
import { imageFormat, urlFormat, ratingFormat } from "@/utils/format";
import type { shopListItem } from "~/types/shop/shop.d";

defineProps<{
    item: shopListItem;
}>();

// 格式化粉丝数，超过 999999 用万表示，保留两位小数不四舍五入
const formatFansCount = (count: number) => {
    if (count > 999999) {
        const wan = Math.floor(count / 100) / 100;
        return `${wan}w`;
    }
    return count;
};
</script>

<style scoped lang="less">
.shop-info-card {
    width: 320px;
    flex-shrink: 0;
    background: #fff;
    padding: 20px 0;
    box-sizing: border-box;
    display: flex;
    flex-direction: row;
    align-items: flex-start;
    border-right: none;

    .shop-logo {
        width: 70px;
        height: 70px;
        border: 1px solid #eee;
        border-radius: 4px;
        overflow: hidden;
        margin-right: 15px;
        flex-shrink: 0;
        cursor: pointer;

        .logo-img {
            width: 100%;
            height: 100%;
            object-fit: contain;
        }
    }

    .shop-content {
        flex: 1;
        display: flex;
        flex-direction: column;
        justify-content: space-between;
        min-height: 70px;

        .shop-name {
            font-size: 16px;
            line-height: 22px;
            font-weight: bold;
            color: #333;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            margin-bottom: 8px;

            .name-link {
                color: #333;
                .ellipsis {
                    display: inline-block;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    white-space: nowrap;
                    max-width: 200px;
                }
                &:hover {
                    color: var(--general);
                }
            }
        }

        .shop-rating-row {
            display: flex;
            align-items: center;
            font-size: 12px;
            color: #999;
            margin-bottom: 10px;

            .rating-box-container {
                display: flex;
                align-items: center;
                justify-content: center;
                flex-direction: row;
                padding: 2px 6px;
                background: var(--vice-bg);
                margin-right: 10px;
                border-radius: 3px;
            }

            .rating-box {
                padding: 2px 6px;
                display: inline-flex;
                width: fit-content;
                align-items: center;

                .rating-score {
                    color: var(--general);
                    font-weight: bold;
                    line-height: 1;
                }

                .separator {
                    color: var(--general);
                    opacity: 0.5;
                    margin: 0 4px;
                    font-size: 10px;
                    position: relative;
                    top: -1px;
                }

                .stars {
                    display: flex;
                    align-items: center;
                    height: 10px;

                    :deep(.el-rate) {
                        height: 10px;
                        --el-rate-icon-size: 10px;
                        --el-rate-fill-color: var(--general);
                        --el-rate-void-color: #c6d1de;
                        --el-rate-icon-margin: 1px;
                        .el-rate__item {
                            display: flex;
                            align-items: center;
                            margin-right: 0px;
                        }
                    }
                }
            }

            .fans-count {
                color: #999;
            }
        }

        .enter-shop-btn {
            .btn-link {
                display: inline-flex;
                align-items: center;
                justify-content: center;
                padding: 0 10px;
                height: 24px;
                border: 1px solid #e5e5e5;
                border-radius: 4px;
                color: #333;
                font-size: 12px;
                text-decoration: none;
                transition: all 0.3s;
                background: #fff;

                .shop-icon {
                    margin-right: 4px;
                    margin-bottom: -1px;
                }

                &:hover {
                    color: #333 !important;
                    border-color: var(--general);
                }
            }
        }
    }
}
</style>
