<template>
    <div v-if="type == 1" :style="{ marginTop: mt30 ? '30px' : '0px' }" class="footer flex flex-column align-center">
        <div v-if="false" class="foot-bannerw flex flex-column align-center">
            <div class="foot-banner flex justify-center">
                <div class="banner-item flex justify-center align-center">
                    <div class="icon">15</div>
                    <a href="#" target="_blank">{{ $t("15天免费换货") }}</a>
                </div>
                <div class="banner-item flex justify-center align-center">
                    <div class="icon">7</div>
                    <a href="#" target="_blank">{{ $t("7天无理由退货") }}</a>
                </div>
                <div class="banner-item flex justify-center align-center">
                    <div class="icon">{{ $t("包") }}</div>
                    <a href="#" target="_blank">{{ $t("满99元包邮") }}</a>
                </div>
                <div class="banner-item flex justify-center align-center">
                    <div class="icon">{{ $t("顺") }}</div>
                    <a href="#" target="_blank">{{ $t("顺丰物流") }}</a>
                </div>
            </div>
        </div>
        <div class="foot-help flex justify-around" v-if="helpBottomList?.length > 0">
            <template v-for="item in helpBottomList">
                <div class="f_help_item">
                    <NuxtLink :to="'/article/issue/info?categoryId=' + item.categorySn" class="f_help_tit" target="_blank"
                        >{{ $t(item.articleCategoryName) }}
                    </NuxtLink>
                    <NuxtLink v-for="items in item.articles" :to="'/article/issue/info?articleId=' + items.articleId" target="_blank"
                        >{{ $t(items.articleTitle) }}
                    </NuxtLink>
                </div>
            </template>
        </div>
    </div>

    <div :style="{ backgroundColor: bg }" class="f-bottom flex flex-column align-center">
        <div class="wrapper">
            <div class="foot_nav">
                <template v-if="commonStore.bottomNav.length > 0">
                    <template v-for="(nav, index) in commonStore.bottomNav">
                        <template v-if="nav.link?.link">
                            <!-- <NuxtLink :to="nav.link?.link" :class="{ first: index === 0 }" class="hidden">
                                {{ $t(nav.title) }}
                            </NuxtLink> -->
                            <CommonLink class="hidden" :item="nav">{{ $t(nav.title) }}</CommonLink>
                        </template>
                        <template v-else>
                            <NuxtLink :to="'/article/issue/info?articleId=' + nav.link.id" :class="{ first: index === 0 }" class="hidden">
                                {{ $t(nav.title) }}
                            </NuxtLink>
                        </template>
                    </template>
                </template>
            </div>
            <div class="foot_nav friendly_link" v-if="friendLinksList.length > 0 && showFriendly">
                {{ $t("友情链接") }}：
                <template v-for="(nav, index) in friendLinksList">
                    <NuxtLink :class="{ first: index === 0 }" :to="nav.linkUrl" class="hidden" target="_blank"
                        ><span style="color: #999999">{{ $t(nav.linkTitle) }}</span></NuxtLink
                    >
                </template>
            </div>
            <div class="copy">
                <p class="copyright">
                    <text v-if="commonStore.shopCompanyTxt">{{ commonStore.shopCompanyTxt }}｜</text>
                    <template v-if="commonStore.businessLicense">
                        <text>增值电信业务经营许可证：{{ commonStore.businessLicense }}｜</text>
                    </template>
                    <NuxtLink v-if="commonStore.shopIcpNo" class="jj-link" rel="nofollow" target="_blank" :to="commonStore.shopIcpNoUrl">
                        {{ commonStore.shopIcpNo }}｜
                    </NuxtLink>
                    <NuxtLink class="jj-link" rel="nofollow" target="_blank" v-if="commonStore.shop110No" :to="commonStore.shop110Link">
                        <img alt="police" :src="police" style="vertical-align: middle; width: 20px; height: 20px" />
                        {{ commonStore.shop110No }}
                    </NuxtLink>
                </p>
                <p class="copyright">
                    <text v-if="commonStore.kefuPhone">{{ $t("客服热线") }}：{{ commonStore.kefuPhone }}</text>
                    <text v-if="commonStore.kefuTime" class="ml10">{{ $t("客服时间") }}：{{ commonStore.kefuTime }}</text>
                    <text v-if="commonStore.kefuAddress" class="ml10">{{ $t("联系地址") }}：{{ commonStore.kefuAddress }}</text>
                </p>
            </div>
        </div>

        <template v-if="imgUrl">
            <div class="copyright-img">
                <div class="copyright-bg" :style="{ backgroundImage: `url(${imgUrl})` }"></div>
            </div>
        </template>

        <div class="bot-link"></div>
    </div>
</template>
<script lang="ts" setup>
import { ref } from "vue";
import { useCommonStore } from "~/store/common";
import { getHomeFriendLinks } from "~/api/home/home";
import type { FriendLinksFilterState } from "~/types/home/friendLinks";
import type { CategoryFilterState } from "~/types/article/category";
import { getBzzxCategoryList } from "~/api/article/category";
import { isUrl } from "~/utils/util";
import police from "~/assets/images/police.png";

defineProps({
    //是否显示尾部上半部分 1显示 2不显示
    type: {
        type: Number,
        default: 1
    },
    bg: {
        type: String,
        default: ""
    },
    mt30: {
        type: Boolean,
        default: true
    },
    showFriendly: {
        type: Boolean,
        default: false
    }
});

const commonStore = useCommonStore();

commonStore.loadNav();

const urlFormat = (url: string) => {
    if (!url) {
        return "";
    }
    if (isUrl(url)) {
        return url;
    }
    return `${import.meta.env.VITE_API_URL ? import.meta.env.VITE_API_URL : location.origin}${url}`;
};

const imgUrl = computed(() => {
    let url = urlFormat(commonStore.defaultTechSupport);
    if (commonStore.poweredBy == 1) {
        url = commonStore.poweredByLogo;
    }

    return url;
});

const friendLinksList = ref<FriendLinksFilterState[]>([]);

const helpBottomList = ref<CategoryFilterState[]>([]);

const loadFriendLinks = async () => {
    try {
        const result = await getHomeFriendLinks();
        friendLinksList.value = result;
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};

loadFriendLinks();

const loadBzzxList = async () => {
    try {
        const result = await getBzzxCategoryList({ categorySize: 5, articleSize: 4 });
        helpBottomList.value = result;
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};

loadBzzxList();
</script>
<style lang="less" scoped>
.footer {
    width: 100%;

    border-top: 1px solid #eee;

    .foot-bannerw {
        width: 100%;
        border-bottom: 1px solid #e8e8e8;

        .foot-banner {
            width: 1190px;
            padding: 16px 0;

            .banner-item {
                width: 298px;
                border-right: 1px solid #e8e8e8;
                height: 60px;

                &:last-child {
                    border: none;
                }

                .icon {
                    text-align: center;
                    border: 2px solid #ff4040;
                    border-radius: 20px;
                    height: 36px;
                    line-height: 36px;
                    margin: 0 10px 0 0;
                    width: 36px;
                    font-size: 18px;
                    color: #ff4040;
                }

                a {
                    color: #302e33;
                    font-size: 18px;
                }
            }
        }
    }

    .foot-help {
        width: 1190px;
        padding: 16px 0;

        .f_help_item {
            cursor: pointer;
            text-align: center;

            .f_help_tit {
                color: #666666;
                font-weight: bold;
                font-size: 14px;
            }

            a {
                display: block;
                color: #888;
                font-size: 12px;
                margin-top: 10px;
            }

            a:hover {
                color: var(--general);
            }
        }
    }
}

.f-bottom {
    box-sizing: border-box;
    border-top: 1px solid #eee;
    height: 170px;

    .wrapper {
        width: 1190px;

        .foot_nav {
            height: 50px;
            line-height: 50px;
            text-align: center;
            text-shadow: 0 1px 0 #fff;

            a {
                color: #333;
                padding: 0 19px 0 20px;
                background: url("/assets/images/common/border-right.png") no-repeat left center;
                display: inline-block;
                cursor: pointer;
            }

            .first {
                padding-left: 5px;
                background-image: none;
            }
        }

        .friendly_link {
            color: #999999;
            line-height: 30px;
            height: 30px;
        }

        .copy {
            color: #999;
            line-height: 25px;
            text-shadow: 0 1px 0 #fff;
            text-align: center;

            .jj-link {
                color: #999;
            }
        }
    }

    .bot-link {
        height: 40px;
        text-align: center;
        padding-bottom: 30px;
        width: 1190px;
    }
}

.copyright-img {
    padding: 10px 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    row-gap: 10px;

    .copyright-bg {
        max-width: 100%;
        overflow: hidden;
        background-position: 50%;
        width: 160px;
        height: 16px;
        background-repeat: no-repeat;
        background-size: contain;
    }
}
</style>
