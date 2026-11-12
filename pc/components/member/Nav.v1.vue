<!-- o2o 原版会员侧边导航（分组紧凑样式）归档。现网已切换为跨境扁平卡片样式 Nav.vue。客户如需切回，参考本文件。 -->
<template>
    <div class="nav_row">
        <div class="title">
            <NuxtLink class="white" to="/member">
                <div>{{ $t("会员中心") }}</div>
                <div class="iconfont-pc icon-sanjiaoright"></div>
            </NuxtLink>
        </div>
        <div v-for="item in menuList" class="col">
            <h3><span class="s1"></span>{{ $t(item.tit) }}</h3>
            <div class="item">
                <template v-for="a in item.list">
                    <NuxtLink v-if="a.show" :class="activePage(a.url) ? 'active' : ''" :to="a.url">{{ $t(a.name) }}</NuxtLink>
                </template>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { computed } from "vue";
import { useRouter } from "vue-router";
import { isOverseas, isB2B, isStore } from "~/utils/util";
import { useCommonStore } from "~/store/common";
import { useUserStore } from "~/store/user";

const commonStore = useCommonStore();
const userStore = useUserStore();

const router = useRouter();
const activePage = (url: string) => {
    let currentPage = router.currentRoute.value.path;
    currentPage = currentPage.split("?")[0].endsWith("/") ? currentPage.slice(0, -1) : currentPage;
    const normalizedUrl = url.endsWith("/") ? url.slice(0, -1) : url;
    return getBasePath(currentPage) === getBasePath(normalizedUrl);
};

const getBasePath = (path: string) => {
    // 如果只有一个 '/'，返回整个路径
    if (path.match(/\//g)?.length === 1) {
        return path;
    }
    // 否则，去除最后一个 '/' 及之后的内容
    return path.slice(0, path.lastIndexOf("/"));
};

const isTermPay = computed(() => {
    return Object.keys(userStore.userInfo?.creditPaymentInfo || {}).length > 0;
});

const menuList = computed(() => [
    {
        tit: "订单中心",
        list: [
            {
                name: "我的订单",
                url: "/member/order/list",
                show: true
            },
            {
                name: "收藏的商品",
                url: "/member/collectProduct/list",
                show: true
            },
            {
                name: "收藏的门店",
                url: "/member/collectShop/list",
                show: true
            },
            {
                name: "浏览历史",
                url: "/member/browsingHistory/list",
                show: isOverseas()
            },
            {
                name: "评价晒单",
                url: "/member/comment/list",
                show: true
            },
            {
                name: "我的购物车",
                url: "/cart",
                show: true
            },
            {
                name: "我的拼团",
                url: "/member/joining/list",
                show: true
            }
        ]
    },
    {
        tit: "账户管理",
        list: [
            {
                name: "个人资料",
                url: "/member/profile/info",
                show: true
            },
            {
                name: "账户安全",
                url: "/member/security/info",
                show: true
            },
            {
                name: "会员实名认证",
                url: "/member/userCertification/info",
                show: isB2B() && commonStore.closeAuth === 1
            },
            {
                name: "我的优惠券",
                url: "/member/coupon/list",
                show: true
            },
            {
                name: "账期支付",
                url: "/member/termPay/list",
                show: isB2B() && isTermPay.value
            },
            {
                name: "账户信息",
                url: "/member/account/detail",
                show: true
            },
            {
                name: `${commonStore.integralName}明细`,
                url: "/member/point/list",
                show: true
            },
            {
                name: "礼品卡兑换",
                url: "/member/giftCard/exchange",
                show: true
            },
            {
                name: "收货地址簿",
                url: "/member/address/list",
                show: true
            },
        ]
    },
    {
        tit: "客户服务",
        list: [
            {
                name: "我的询价单",
                url: "/member/inquiry/list",
                show: isB2B()
            },
            {
                name: "留言/咨询",
                url: "/member/feedback/list",
                show: true
            },
            {
                name: "售后管理",
                url: "/member/return/list",
                show: true
            },
        ]
    }
]);
</script>
<style lang="less" scoped>
.nav_row {
    width: 164px;
    margin-right: 40px;

    .title {
        .white {
            height: 72px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 18px;
            flex-direction: row;
            flex-wrap: nowrap;
            gap: 4px;
        }
    }

    .col {
        h3 {
            border-top: 1px solid #e7e7e7;
            padding: 14px 0 14px 14px;
            font-size: 14px;

            span {
                display: inline-block;
                height: 20px;
                margin: 0 8px 0 0;
                vertical-align: middle;
                width: 20px;
                background: url("/assets/images/user/user_ico.png") no-repeat center center;
            }
        }

        .item {
            a {
                display: block;
                line-height: 36px;
                padding-left: 42px;
                font-size: 12px;
            }

            .active {
                color: var(--general);
                font-weight: bold;
            }
        }
    }
}
</style>
