<template>
    <div :style="{ marginTop: mt30 ? '30px' : '0px' }" class="footer flex flex-column align-center footer-shell-MjI4MmhtbGI">
        <div v-if="false" class="foot-bannerw flex flex-column align-center">
            <div class="foot-banner flex justify-center">
                <div class="banner-item flex justify-center align-center">
                    <div class="icon">15</div>
                    <a href="#" target="_blank">{{ $t("15天免费换货1") }}</a>
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
        <!-- 语言/货币切换 toolbar 为海外版专属 -->
        <div class="toolbar-wrap" v-if="isOverseas()">
            <div class="content">
                <div class="content-inner">
                    <div class="right-wrap">
                        <LanguageChangeNew :show-lang="true" :show-currency="false" :top="false">
                            <template #trigger>
                                <div class="btn">
                                    <SvgIcon class="language" name="language" width="14" height="14"></SvgIcon>
                                    <div class="lang-text">{{ footerLanguage }}</div>
                                </div>
                            </template>
                        </LanguageChangeNew>

                        <LanguageChangeNew :show-lang="false" :show-currency="true" :top="false">
                            <template #trigger>
                                <div class="btn">
                                    <div class="lang-text" v-if="currentCurrencyData">
                                        {{ currentCurrencyData?.symbol + " - " + $t(currentCurrencyData?.name) }}
                                    </div>
                                </div>
                            </template>
                        </LanguageChangeNew>
                    </div>
                </div>
            </div>
        </div>
        <div class="foot-help" v-if="helpBottomList?.length > 0">
            <div class="content">
                <template v-for="item in helpBottomList">
                    <div class="f_help_item">
                        <NuxtLink :to="'/article/issue/info?categoryId=' + item.categorySn" class="f_help_tit" target="_blank"
                            >{{ item.articleCategoryName }}
                        </NuxtLink>
                        <NuxtLink v-for="items in item.articles" :to="'/article/issue/info?articleId=' + items.articleId" target="_blank"
                            >{{ items.articleTitle }}
                        </NuxtLink>
                    </div>
                </template>
            </div>
        </div>
    </div>

    <div :style="{ backgroundColor: bg }" class="f-bottom flex flex-column align-center">
        <div class="wrapper">
            <div class="wrapper-inner">
                <div class="foot_nav">
                    <template v-if="bottomNavList.length > 0">
                        <template v-for="(nav, index) in bottomNavList">
                            <template v-if="nav.link?.link">
                                <CommonLink class="hidden" :item="nav">{{ nav.title }}</CommonLink>
                            </template>
                            <template v-else>
                                <NuxtLink :to="'/article/issue/info?articleId=' + nav.link?.id" :class="{ first: index === 0 }" class="hidden">
                                    {{ nav.title }}
                                </NuxtLink>
                            </template>
                        </template>
                    </template>
                    <!-- Cookie 设置入口（跨境演示站） -->
                    <a v-if="isOverseas() && isDemo()" href="javascript:;" class="cookie-setting-link" @click="openConsentSettings">
                        {{ $t("Cookie 设置") }}
                    </a>
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
                        <template
                            v-if="
                                commonStore.businessLicenseEnabled === 1 &&
                                commonStore.businessLicenseType?.includes('icp') &&
                                commonStore.businessLicenseType?.includes('edi') &&
                                (commonStore.businessLicense || commonStore.ediLicense)
                            "
                        >
                            <text>{{ $t("增值电信业务经营许可证(ICP+EDI)") }}：{{ commonStore.businessLicense || commonStore.ediLicense }}｜</text>
                        </template>
                        <template v-else-if="commonStore.businessLicenseEnabled === 1 && commonStore.businessLicenseType?.includes('icp') && commonStore.businessLicense">
                            <text>{{ $t("增值电信业务经营许可证(ICP)") }}：{{ commonStore.businessLicense }}｜</text>
                        </template>
                        <template v-else-if="commonStore.businessLicenseEnabled === 1 && commonStore.businessLicenseType?.includes('edi') && commonStore.ediLicense">
                            <text>{{ $t("增值电信业务经营许可证(EDI)") }}：{{ commonStore.ediLicense }}｜</text>
                        </template>
                        <NuxtLink class="jj-link" rel="nofollow" target="_blank" :to="commonStore.shopIcpNoUrl">
                            {{ commonStore.shopIcpNo }}
                        </NuxtLink>
                        ｜
                        <NuxtLink class="jj-link" rel="nofollow" target="_blank" v-if="commonStore.shop110No" :to="commonStore.shop110Link"
                            ><img alt="police" :src="police" style="vertical-align: middle; width: 20px; height: 20px" />
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
import { isUrl, isOverseas, isDemo } from "~/utils/util";
import police from "~/assets/images/police.png";
import LanguageChangeNew from "~/components/locate/LanguageChangeNew.vue";
import { useI18nStore } from "~/store/i18n";

// Cookie 设置抽屉入口
const { openSettings: openConsentSettings } = useConsent();

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

const currentLanguage = useCookie("currentLanguage");
const currentCurrencyData = useCookie<{ symbol: string; name: string } | null>("currentCurrencyData");
const i18nStore = useI18nStore();
const commonStore = useCommonStore();

const footerLanguage = computed(() => {
    const localeCode = String(i18nStore.XLocaleCode || currentLanguage.value || "");
    return localeCode.split(/[-_]/)[0]?.toUpperCase() || "";
});

// o2o store 未给 bottomNav 标类型（推断 never[]），此处本组件内收敛为可用类型，避免逐项访问报错
type FootNavItem = { title: string; link?: { link?: string; id?: string | number } };
const bottomNavList = computed(() => (commonStore.bottomNav ?? []) as FootNavItem[]);

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
    min-width: var(--pc-home-min-width, 1264px);
    background-color: #f5f6fa;
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

    .toolbar-wrap {
        width: 100%;
        .content {
            margin: 0 auto;
            width: 100%;
            max-width: var(--pc-home-max-width, 1264px);
            padding: 0 var(--pc-home-padding-x, 0px);
            box-sizing: border-box;
            .content-inner {
                display: flex;
                justify-content: space-between;
                align-items: center;
                height: 73px;
                border-bottom: 1px solid #0000000f;
                .left-wrap {
                    display: flex;
                    align-items: center;
                    .logo {
                        display: block;
                        height: 27px;
                    }
                }
                .right-wrap {
                    display: flex;
                    gap: 30px;
                    margin-left: auto;
                    .btn {
                        box-sizing: border-box;
                        display: flex;
                        align-items: center;
                        height: 40px;
                        padding: 10px 14px;
                        border: 1px solid #c2c4cc;
                        border-radius: 4px;
                        cursor: pointer;
                        font-size: 14px;
                        color: #1a1a1a;
                        .lang-text {
                            margin-left: 8px;
                        }
                    }
                }
            }
        }
    }

    .foot-help {
        width: 100%;
        padding: 32px 0;

        .content {
            display: flex;
            justify-content: space-between;
            margin: 0 auto;
            width: 100%;
            max-width: var(--pc-home-max-width, 1264px);
            padding: 0 var(--pc-home-padding-x, 0px);
            box-sizing: border-box;

            .f_help_item {
                cursor: pointer;
                text-align: left;

                .f_help_tit {
                    font-size: 16px;
                    font-weight: 700;
                    line-height: 20px;
                    margin-bottom: 14px;
                    color: #1a1a1a;
                }

                a {
                    display: block;
                    color: #000;
                    font-size: 14px;
                    margin-bottom: 16px;
                    &:last-child {
                        margin-bottom: 0;
                    }
                }

                a:hover {
                    color: var(--general);
                }
            }
        }
    }
}

.f-bottom {
    box-sizing: border-box;
    height: auto;
    overflow: visible;
    background-color: #f5f6fa !important;
    width: 100%;
    min-width: var(--pc-home-min-width, 1264px);

    .wrapper {
        width: 100%;
        max-width: var(--pc-home-max-width, 1264px);
        padding: 0 var(--pc-home-padding-x, 0px);
        box-sizing: border-box;
        margin: 0 auto;

        .wrapper-inner {
            border-top: 1px solid rgba(0, 0, 0, 0.059);
            padding-top: 32px;

            // 友情链接不展示时，导航行直接紧贴版权行，间距过小，这里补足；
            // 友情链接展示时由 .friendly_link 行撑开间距，这里重置为 0 避免叠加。
            .foot_nav + .copy {
                margin-top: 6px;
            }

            .friendly_link + .copy {
                margin-top: 0;
            }

            .foot_nav {
                text-align: center;
                text-shadow: 0 1px 0 #fff;

                a {
                    padding: 0 19px 0 20px;
                    background: url("/assets/images/common/border-right.png") no-repeat left center;
                    display: inline-block;
                    cursor: pointer;
                    font-weight: 400;
                    font-size: 12px;
                    line-height: 16px;
                    color: #888b94;
                    // text-decoration: underline;
                    &:hover {
                        color: #888b94 !important;
                    }
                }

                .first {
                    padding-left: 5px;
                    background-image: none;
                }
            }

            .friendly_link {
                font-weight: 400;
                font-size: 12px;
                color: #888b94;
                line-height: 30px;
                height: auto;
                min-height: 30px;
                white-space: normal;
                overflow-wrap: break-word;
                word-break: break-word;
                a {
                    // text-decoration: underline;
                    &:hover {
                        color: #888b94 !important;
                    }
                }
            }

            .copy {
                font-weight: 400;
                font-size: 12px;
                color: #888b94;
                line-height: 25px;
                text-align: center;
                white-space: normal;
                overflow-wrap: break-word;
                word-break: break-word;
                a {
                    font-weight: 400;
                    font-size: 12px;
                    line-height: 16px;
                    color: #888b94;
                    // text-decoration: underline;
                    &:hover {
                        color: #888b94 !important;
                    }
                }
                .copyright {
                    padding: 3px 0;
                    font-weight: 400;
                    font-size: 12px;
                    line-height: 16px;
                    color: #888b94;
                }
            }
        }
    }

    .bot-link {
        // height: 40px;
        text-align: center;
        width: 100%;
        max-width: var(--pc-home-max-width, 1264px);
        padding: 0 var(--pc-home-padding-x, 0px);
        box-sizing: border-box;
        margin: 0 auto;
    }
}

.copyright-img {
    padding: 10px 0;
    padding-bottom: 32px;
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
