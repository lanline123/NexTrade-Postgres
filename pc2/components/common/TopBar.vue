<template>
    <div class="top-bar topbar-shell-d2QyMjgyY3Y" :class="[pageType != 1 ? color : '']">
        <div class="container" :class="{ wide: wide, 'container-new': isNewSearch }">
            <div class="wrapper">
                <div class="region-select-warp">
                    <CommonHeaderRegion :pageType="pageType" :colorType="colorType" v-if="!isOverseas()"></CommonHeaderRegion>
                </div>
                <div class="right">
                    <div class="auth-item">
                        <template v-if="isB2B() && commonStore.closeAuth === 1">
                            <NuxtLink to="/member/userCertification/info">
                                <AuthBadge class="auth-icon" :verified="Number(userStore.userInfo.isCompanyAuth) > 0" :height="17" />
                            </NuxtLink>
                        </template>
                    </div>
                    <client-only>
                        <div class="item" v-if="userStore.userInfoLoaded && !userStore.userInfo.userId">
                            <div class="l-name">
                                <NuxtLink class="top-bar-common-fontcolor" to="/member/login">{{ $t("你好,请登录") }}</NuxtLink>
                            </div>
                            <template v-if="commonStore.shopRegClosed === 0">
                                <div class="sline"></div>
                                <div class="r-name">
                                    <NuxtLink class="top-bar-common-fontcolor" to="/member/register">{{ $t("注册") }} </NuxtLink>
                                </div>
                            </template>
                        </div>
                        <div class="item item-menu" v-if="userStore.userInfoLoaded && userStore.userInfo.userId">
                            <div class="flex align-center top-bar-common-fontcolor">
                                <div class="name">
                                    <NuxtLink class="top-bar-common-fontcolor" to="/member/"
                                        >{{ userStore.userInfo.nickname ? userStore.userInfo.nickname : userStore.userInfo.dimUsername }}
                                    </NuxtLink>
                                </div>
                                <div class="iconfont-pc icon-xiajiantou" style="font-size: 10px; margin-left: 5px"></div>
                            </div>
                            <div class="dropdown mini">
                                <div class="areamini-content-wrap">
                                    <div class="topbar-usernav" style="width: 150px">
                                        <NuxtLink to="/member/account/detail"
                                            >{{ $t("我的余额") }}：
                                            <FormatPrice :showText="false" v-model="userStore.userInfo.balance"></FormatPrice>
                                        </NuxtLink>
                                        <NuxtLink to="/member/coupon/list">{{ $t("我的优惠券") }}</NuxtLink>
                                        <NuxtLink to="/member/profile/info">{{ $t("修改个人资料") }}</NuxtLink>
                                        <NuxtLink @click="onLogout" class="last">{{ $t("退出登录") }}</NuxtLink>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </client-only>
                    <div class="item" v-if="wide">
                        <SimShoppingCart :colorType="colorType" :linkColor="fontColor" compact></SimShoppingCart>
                    </div>
                    <div class="item">
                        <NuxtLink class="item-a top-bar-common-fontcolor" to="/member/userMessage/list/">
                            <span class="iconfont-pc icon-xiaoxi"></span>
                            <span class="name">{{ $t("站内消息") }}{{ stationMessage > 0 ? "(" + stationMessage + ")" : "" }}</span>
                        </NuxtLink>
                    </div>
                    <div class="item">
                        <NuxtLink class="item-a top-bar-common-fontcolor" to="/member/order/list/">
                            <span class="iconfont-pc icon-dingdan"></span>
                            <span class="name">{{ $t("我的订单") }}</span>
                        </NuxtLink>
                    </div>
                    <template v-for="(nav, index) in commonStore.topBarNav" :key="index">
                        <div v-if="nav.children && nav.children.length > 0" class="item item-menu">
                            <a class="flex align-center top-bar-common-fontcolor">
                                <div class="name">{{ $t(nav.title) }}</div>
                                <div class="iconfont-pc icon-xiajiantou" style="font-size: 10px; margin-left: 5px"></div>
                            </a>
                            <div class="dropdown mini">
                                <div class="areamini-content-wrap">
                                    <div class="topbar-usernav">
                                        <template v-for="child in nav.children">
                                            <CommonLink :item="child">{{ $t(child.title) }}</CommonLink>
                                        </template>
                                        <template v-if="isDemo()">
                                            <NuxtLink to="/join">{{ $t("招商加盟") }} </NuxtLink>
                                            <NuxtLink to="/join/intro">{{ $t("商家入驻") }} </NuxtLink>
                                        </template>
                                    </div>
                                </div>
                            </div>
                        </div>
                        <div v-else class="item">
                            <div class="name">
                                <NuxtLink class="top-bar-common-fontcolor" :to="nav.link ? urlFormat(nav.link) : ''">
                                    {{ nav.title }}
                                </NuxtLink>
                            </div>
                        </div>
                    </template>
                    <div class="item" v-if="userStore.userInfo.hasShop" @click="goTo">
                        <div class="item-a top-bar-common-fontcolor">
                            <!-- <span class="iconfont-pc icon-dingdan"></span> -->
                            <span class="name">{{ $t("商户后台") }}</span>
                        </div>
                    </div>
                    <SimShoppingCart :colorType="colorType" :linkColor="fontColor" v-if="pageType != 1 && !wide"></SimShoppingCart>
                    <LanguageChange v-if="isOverseas()" class="item" :style="{ color: fontColor }"></LanguageChange>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, onMounted } from "vue";
import { useCommonStore } from "@/store/common";
import { useUserStore } from "@/store/user";
import { urlFormat } from "@/utils/format";
import SimShoppingCart from "~/components/shoppingCart/SimShoppingCart.vue";
import { getMessageList } from "~/api/user/userMessage";
import LanguageChange from "~/components/locate/LanguageChange.vue";
import { isOverseas, isDemo, isB2B } from "@/utils/util";
import AuthBadge from "~/components/common/AuthBadge.vue";

const commonStore = useCommonStore();
const userStore = useUserStore();
await commonStore.loadNav();
userStore.getUserInfo();
const stationMessage = ref(0);

onMounted(() => {
    if (userStore.userInfoLoaded && userStore.userInfo.userId > 0) {
        loadFilter();
    }
});

const loadFilter = async () => {
    try {
        const result = await getMessageList({ page: 1, unread: 1 });
        stationMessage.value = result.total;
    } catch (error: any) {
        // 处理错误信息
        message.error(error.message);
    } finally {
    }
};

const props = defineProps({
    pageType: {
        type: Number,
        default: 1
    },
    colorType: {
        type: Number,
        default: 1
    },
    wide: {
        type: Boolean,
        default: false
    },
    isNewSearch: {
        type: Boolean,
        default: false
    }
});

const fontColor = computed(() => {
    let temp = "";
    if (props.pageType == 1) {
        temp = "black";
    } else {
        if (props.colorType == 1 || props.colorType == 2) {
            temp = "#b0b0b0";
        } else if (props.colorType == 3) {
            temp = "black";
        }
    }
    return temp;
});

const color = computed(() => {
    return "top-bar-bac-" + props.colorType;
});

const onLogout = () => {
    userStore.loginOut();
};

const goTo = () => {
    if (commonStore.adminDomain) {
        window.open(`${commonStore.adminDomain}/admin`);
    } else {
        window.open(`${location.origin}/admin`);
    }
};
const toAuth = () => {
    navigateTo(`/member/userCertification/info`);
};
</script>
<style lang="less" scoped>
.container {
    min-width: var(--pc-home-min-width, auto);

    &.container-new {
        max-width: var(--pc-home-max-width, 1600px) !important;
        width: 100% !important;
        margin: 0 auto !important;
        padding: 0 var(--pc-home-padding-x, 38px) !important;
        box-sizing: border-box !important;
        min-width: var(--pc-home-min-width, auto) !important;
    }
}
.top-bar-common-fontcolor {
    color: v-bind(fontColor) !important;
}
.top-bar-bac-1 {
    background: rgb(0, 0, 0) !important;
}

.top-bar-bac-2 {
    background: #333 !important;
}

.top-bar-bac-3 {
    background: #e0f4e4 !important;
}

.top-bar {
    background: #f5f5f5;
    height: 30px;
    line-height: 30px;
    border-bottom: 1px solid #e5e5e5;
    position: relative;
    z-index: 1001;
    min-width: var(--pc-home-min-width, auto);

    .container {
        width: 100%;
        max-width: var(--pc-home-max-width, 1190px);
        margin: 0px auto;
        padding: 0 var(--pc-home-padding-x, 0px);
        box-sizing: border-box;
        min-width: var(--pc-home-min-width, auto);
        clear: both;

        &.wide {
            max-width: 1700px;
            width: 100%;
            padding-left: 50px;
            padding-right: 50px;
            box-sizing: border-box;
        }
    }

    .wrapper {
        display: flex;
        align-items: center;
        justify-content: space-between;

        .iconfont-pc {
            font-size: 16px;
        }
    }

    .right {
        display: flex;
        align-items: center;
        justify-content: flex-end;
        font-size: 12px;
        color: #666;
        .auth-item {
            display: flex;
            align-items: center;
            height: 31px;
            position: relative;
            z-index: 12;
            cursor: pointer;
            padding-right: 5px;
            .auth-icon {
                height: 17px;
                margin-top: 10px;
            }
        }

        .item {
            display: flex;
            align-items: center;
            height: 31px;
            // margin-left: 20px;
            padding: 0 10px;
            border-left: 1px solid transparent;
            border-right: 1px solid transparent;
            position: relative;
            z-index: 12;
            cursor: pointer;

            .item-a {
                display: flex;
                align-items: center;

                .iconfont-pc {
                    margin-right: 2px;
                }
            }

            .name {
                margin-left: 0;
                display: flex;
                align-items: center;
            }

            .l-name {
                color: var(--general);
                margin-right: 5px;

                a {
                    color: var(--general);
                }
            }

            .r-name {
                margin-left: 5px;

                a {
                    color: #aaa;
                }
            }

            .sline {
                width: 1px;
                height: 12px;
                background-color: #cccccc;
            }
        }

        .item-menu {
            &:hover {
                background: #fff;
                border-left: 1px solid #dbdbdb !important;
                border-right: 1px solid #dbdbdb !important;
            }

            .icon-xiajiantou {
                transition: transform ease 0.5s;
            }

            &:hover .icon-xiajiantou {
                transform: rotate(180deg);
            }

            &:hover .dropdown {
                display: block;
            }

            .dropdown {
                position: absolute;
                border: 1px solid #dbdbdb;
                background-color: #fff;
                box-shadow: 1px 2px 1px rgba(0, 0, 0, 0.1);
                width: 300px;
                padding: 10px;
                line-height: 24px;
                top: 31px;
                z-index: 11;
                display: none;
                border-top: 0;
            }

            .dropdown.mini {
                width: auto;
                padding: 0;
                left: -1px;
            }
        }
    }
}

.topbar-usernav {
    width: 83px;
}

.topbar-usernav.userinfo-mini {
    width: 155px;
}

.topbar-usernav a {
    background-color: #ffffff;
    display: block;
    padding: 6px 10px;
    line-height: 18px;
}

.topbar-usernav a:hover {
    background-color: #f8f8f8;
    color: #333;
}
</style>
