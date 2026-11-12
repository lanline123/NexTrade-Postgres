<template>
    <div class="shop-ref-header">
        <div class="container">
            <div class="top-row">
                <NuxtLink to="/" class="site-logo">
                    <img :src="imageFormat(commonStore.shopLogo)" alt="logo" />
                </NuxtLink>
                <div class="search-wrap">
                    <input v-model.trim="keyword" type="text" :placeholder="$t('搜索宝贝')" @keydown.enter="onSiteSearch" />
                    <!-- 有搜本店时相机只搜本店，否则与搜索按钮一致搜全站 -->
                    <ImageSearchEntry :shop-id="imageSearchShopId" />
                    <button class="btn btn-site" :class="{ 'btn-site--only': !showShopSearch }" @click="onSiteSearch">{{ $t("搜索") }}</button>
                    <button v-if="showShopSearch" class="btn btn-shop" @click="onShopSearch">{{ $t("搜本店") }}</button>
                </div>
            </div>

            <div class="info-row">
                <div class="info-card">
                    <div class="shop-main">
                    <div class="shop-logo">
                        <el-image :src="imageFormat(shopInfo.shopLogo)" fit="contain" :alt="shopInfo.shopTitle || ''">
                            <!-- <template #error>
                                <div class="image-slot">
                                    <img src="@/assets/images/shop/shop_icon.png" alt="" />
                                </div>
                            </template> -->
                        </el-image>
                    </div>
                    <div class="shop-meta">
                        <div class="shop-title-row">
                            <span class="shop-title">{{ shopInfo.shopTitle || $t("店铺名称") }}</span>
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
                        <div class="shop-sub">
                            <el-rate v-model="rating" disabled text-color="#ff4d4f" disabled-void-color="#d9d9d9" />
                            <span class="fans">{{ shopInfo.collectCount || 0 }}{{ $t("粉丝") }}</span>
                        </div>
                        <div class="shop-score shop-meta-bHJpczIyODI">
                            <span class="score-item" v-for="item in shopInfo.tipNames" :key="item">{{ item }}</span>
                        </div>
<!--                        <div class="shop-score">-->
<!--                            <span class="score-item">-->
<!--                                {{ $t("综合体验") }}<i class="score-val">{{ ratingFormat(shopInfo.productScore) }}<el-icon><Top /></el-icon></i>-->
<!--                            </span>-->
<!--                            <span class="score-item">-->
<!--                                {{ $t("宝贝质量") }}<i class="score-val">{{ ratingFormat(shopInfo.productScore) }}<el-icon><Top /></el-icon></i>-->
<!--                            </span>-->
<!--                            <span class="score-item">-->
<!--                                {{ $t("物流速度") }}<i class="score-val">{{ ratingFormat(shopInfo.shopStar) }}</i>-->
<!--                            </span>-->
<!--                            <span class="score-item">-->
<!--                                {{ $t("服务保障") }}<i class="score-val">{{ ratingFormat(shopInfo.shopStar) }}<el-icon><Top /></el-icon></i>-->
<!--                            </span>-->
<!--                        </div>-->
                    </div>
                    </div>
                    <div class="shop-actions">
                    <el-button v-if="isShopSearchPage" class="action-btn" @click="goShopHomepage">
                        <el-icon><HomeFilled /></el-icon>{{ $t("店铺首页") }}
                    </el-button>
                    <el-button class="action-btn" @click="emit('kefu')">
                        <span class="iconfont-pc icon-kefu"></span>{{ $t("客服") }}
                    </el-button>
                    <el-button
                        class="follow-btn"
                        :type="shopInfo.collectShop ? 'default' : 'primary'"
                        :class="{ followed: shopInfo.collectShop }"
                        @click="emit('collect')"
                    >
                        <el-icon v-if="!shopInfo.collectShop"><Plus /></el-icon>{{ shopInfo.collectShop ? $t("已关注") : $t("关注") }}
                    </el-button>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { HomeFilled, Plus, ArrowDown, Top } from "@element-plus/icons-vue";
import { imageFormat, ratingFormat } from "@/utils/format";
import { useCommonStore } from "@/store/common";
import ShopQualification from "~/components/shop/src/ShopQualification.vue";
import ImageSearchEntry from "~/components/imageSearch/ImageSearchEntry.vue";

const props = defineProps<{
    shopInfo: any;
}>();
const emit = defineEmits(["collect", "kefu"]);

const route = useRoute();
const router = useRouter();
const commonStore = useCommonStore();
const keyword = ref("");
const showShopSearch = computed(() => commonStore.searchSelfStore === 1);

watch(
    () => route.query.keyword,
    (val) => {
        keyword.value = val ? String(val) : "";
    },
    { immediate: true }
);

const shopId = computed(() => Number(props.shopInfo?.shopId || route.params.id || route.query.shopId || 0));
const imageSearchShopId = computed(() => (showShopSearch.value && shopId.value > 0 ? shopId.value : undefined));
const rating = computed(() => Number(ratingFormat(props.shopInfo?.shopStar || props.shopInfo?.productScore || 5)));
const isShopSearchPage = computed(() => /\/shop\/search\//.test(route.path));
const hasLicense = computed(() => {
    const pic = props.shopInfo?.businessLicensePic;
    return typeof pic === "string" && pic.trim().length > 0;
});

const onSiteSearch = () => {
    location.href = `/search/?keyword=${encodeURIComponent(keyword.value)}`;
};

const onShopSearch = () => {
    if (!shopId.value) return;
    location.href = `/shop/search/${shopId.value}?keyword=${encodeURIComponent(keyword.value)}`;
};

const goShopHomepage = () => {
    if (!shopId.value) return;
    router.push(`/shop/${shopId.value}`);
};
</script>

<style scoped lang="less">
.shop-ref-header {
    margin-bottom: 16px;
}

.container {
    max-width: 1700px;
    width: 100%;
    margin: 0 auto;
    padding-left: 50px;
    padding-right: 50px;
    box-sizing: border-box;
}

.top-row {
    height: 86px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    border-bottom: 1px solid #f5f5f5;
    padding: 16px 17px 0 17px;
}

.site-logo {
    display: flex;
    align-items: center;
    img {
        width: 135px;
        height: auto;
        object-fit: contain;
    }
}

.search-wrap {
    width: 368px;
    height: 40px;
    background: #fff;
    border: 2px solid #e5e8ec;
    border-radius: 8px;
    display: flex;
    overflow: hidden;
    flex-shrink: 0;

    input {
        flex: 1;
        // 加了相机后由输入框让出宽度，按钮不被挤压
        min-width: 0;
        border: none;
        outline: none;
        background: transparent;
        padding: 0 14px;
        font-size: 13px;
        color: #333;
        &::placeholder {
            color: #bbb;
        }
    }

    .btn {
        flex-shrink: 0;
        white-space: nowrap;
        border: none;
        cursor: pointer;
        padding: 0 22px;
        font-size: 14px;
        font-weight: 600;
    }

    .btn-site {
        margin: 3px 3px;
        border-radius: 6px;
        margin-right: 0;
        background: var(--primary-light-9, #fff0f3);
        color: var(--general);

        &--only {
            margin-right: 3px;
        }
    }

    .btn-shop {
        margin: 3px 3px;
        border-radius: 6px;
        background: var(--general);
        color: #fff;
    }
}

.info-row {
    margin-top: 16px;
}

.info-card {
    min-height: 140px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 16px 20px;
    background: #fff;
    border-radius: 16px;
    box-sizing: border-box;
}

.shop-main {
    display: flex;
    align-items: stretch;
    flex: 1;
    min-width: 0;
}

.shop-meta {
    display: flex;
    flex-direction: column;
    gap: 8px;
    //justify-content: space-between;
    min-width: 0;
}

.shop-logo {
    width: 108px;
    height: 108px;
    border: 1px solid #eee;
    border-radius: 4px;
    overflow: hidden;
    margin-right: 14px;
    flex-shrink: 0;
    :deep(.el-image) {
        width: 100%;
        height: 100%;
    }
}

.shop-title-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 16px;
    margin-bottom: 8px;
}

.shop-title {
    color: #11192d;
    font-family: PingFang SC, sans-serif;
    font-size: 24px;
    font-weight: 500;
    height: 26px;
    letter-spacing: 0;
    line-height: 24px;
    max-width: 484px;
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
    .el-icon {
        font-size: 9px;
        width: 9px;
        height: 9px;
    }
}

.shop-sub {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
    :deep(.el-rate) {
        --el-rate-icon-size: 14px;
        height: 14px;
    }
    .fans {
        font-size: 12px;
        color: #888;
    }
}

.shop-score {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;

    .score-item {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        min-height: 20px;
        font-size: 12px;
        font-weight: 400;
        color: #50607a;
        background-color: #f3f6f8;
        border-radius: 4px;
        box-sizing: border-box;
        padding: 4px 8px;
        // margin-right: 8px;
        font-family: PingFang SC;
    }

    .score-val {
        display: inline-flex;
        align-items: center;
        gap: 1px;
        font-style: normal;
        font-size: 12px;
        // font-weight: 600;
        // color: #ff5a36;
        .el-icon {
            font-size: 12px;
        }
    }
}

.shop-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
}

.action-btn {
    height: 40px;
    border: 1px solid #e2e4e6;
    border-radius: 8px;
    box-sizing: border-box;
    font-size: 16px;
    font-weight: 500;
    color: #333;
    padding: 8px 12px;
    :deep(.el-icon),
    .iconfont-pc {
        margin-right: 4px;
        font-size: 16px;
    }
}

.follow-btn {
    height: 40px;
    border: 1px solid #e2e4e6;
    border-radius: 8px;
    box-sizing: border-box;
    font-size: 16px;
    font-weight: 500;
    &.el-button--primary {
        background: var(--general);
        border-color: var(--general);
        color: #fff;
    }
    &.followed {
        background: #f5f5f5;
        color: #999;
        border-color: #ddd;
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

@media (max-width: 1104px) {
    .top-row {
        height: auto;
        padding-top: 10px;
        padding-bottom: 10px;
        // flex-direction: column;
        // align-items: flex-start;
    }

    .search-wrap {
        // width: 100%;
    }

    .info-card {
        min-height: auto;
        align-items: flex-start;
        flex-direction: column;
        gap: 12px;
    }

    .shop-title {
        font-size: 20px;
        line-height: 22px;
        height: auto;
        max-width: 100%;
    }

    .shop-title-row {
        flex-wrap: wrap;
    }

    .shop-actions {
        width: 100%;
        justify-content: flex-end;
    }
}
</style>
