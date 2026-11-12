<template>
    <div class="nav_row member-nav-aGZ0MjI4Mm0">
        <template v-for="(group, gIdx) in menuList" :key="gIdx">
            <div v-if="isGroupVisible(group)" class="group">
                <div class="group__title" :class="{ 'group__title--collapsed': !expanded[gIdx] }" @click="toggleGroup(gIdx)">
                    <span>{{ $t(group.tit) }}</span>
                    <i class="group__arrow"></i>
                </div>
                <div class="group__list-wrapper" :class="{ 'group__list-wrapper--collapsed': !expanded[gIdx] }">
                    <div class="group__list">
                        <template v-for="(item, iIdx) in group.list" :key="iIdx">
                            <div v-if="item.show" class="col">
                                <div class="item">
                                    <NuxtLink :class="activePage(item.url) ? 'active' : ''" :to="item.url">
                                        <span class="item__name">{{ $t(item.name) }}</span>
                                        <span v-if="getBadge(item.url) > 0" class="item__badge">{{ getBadge(item.url) }}</span>
                                        <span v-else-if="getBadgeDot(item.url)" class="item__badge-dot"></span>
                                    </NuxtLink>
                                </div>
                            </div>
                        </template>
                    </div>
                </div>
            </div>
        </template>
    </div>
</template>
<script lang="ts" setup>
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { isOverseas, isB2B, isStore, isMerchant } from "~/utils/util";
import { useCommonStore } from "~/store/common";
import { useUserStore } from "~/store/user";
import { getOrderNum } from "~/api/user/order";
import { getMemberCenter } from "~/api/user/user";
import type { OrderNumState } from "~/types/user/order.d";

interface MenuItem {
    name: string;
    url: string;
    show: boolean;
}

interface MenuGroup {
    tit: string;
    list: MenuItem[];
}

const commonStore = useCommonStore();
const userStore = useUserStore();
const router = useRouter();

// 会员中心各页面会重新挂载导航，使用 Nuxt 状态保留用户手动展开/收起的选择
const expanded = useState<boolean[]>("member-nav-expanded", () => [true, true, true, true, true]);
const orderNum = ref<Partial<OrderNumState>>({});
const awaitCoupon = ref(0);

const isTermPay = computed(() => {
    return Object.keys(userStore.userInfo?.creditPaymentInfo || {}).length > 0;
});

const getBasePath = (path: string) => {
    if (path.match(/\//g)?.length === 1) {
        return path;
    }
    return path.slice(0, path.lastIndexOf("/"));
};

const activePage = (url: string) => {
    let currentPage = router.currentRoute.value.path;
    const pathNoQuery = currentPage.split("?")[0] ?? currentPage;
    currentPage = pathNoQuery.endsWith("/") ? pathNoQuery.slice(0, -1) : pathNoQuery;
    const normalizedUrl = url.endsWith("/") ? url.slice(0, -1) : url;
    return getBasePath(currentPage) === getBasePath(normalizedUrl);
};

const isGroupVisible = (group: MenuGroup) => {
    return group.list.some((item) => item.show);
};

const toggleGroup = (idx: number) => {
    expanded.value[idx] = !expanded.value[idx];
};

const getBadge = (url: string): number => {
    if (url === "/member/comment/list") {
        return Number(orderNum.value.awaitComment ?? 0);
    }
    if (url === "/member/coupon/list") {
        return Number(awaitCoupon.value ?? 0);
    }
    return 0;
};

const getBadgeDot = (url: string): boolean => {
    if (url === "/member/return/list") {
        return Number(orderNum.value.awaitAftersalesCollect ?? 0) > 0;
    }
    return false;
};

const fetchBadges = async () => {
    try {
        const [numRes, centerRes] = await Promise.all([getOrderNum(), getMemberCenter()]);
        if (numRes) {
            orderNum.value = numRes as unknown as Partial<OrderNumState>;
        }
        const center = centerRes as unknown as { awaitCoupon?: number; item?: { awaitCoupon?: number } };
        awaitCoupon.value = Number(center?.awaitCoupon ?? center?.item?.awaitCoupon ?? 0);
    } catch {
        // 角标加载失败不阻塞导航
    }
};

// 分组菜单：对齐 vortmall-pc，保留 o2o 路由与显隐守卫
const menuList = computed<MenuGroup[]>(() => [
    {
        tit: "订单中心",
        list: [
            {
                name: "我的订单",
                url: "/member/order/list",
                show: true
            },
            {
                name: "我的拼团",
                url: isOverseas() ? "/member/group/list" : "/member/joining/list",
                show: true
            },
            {
                name: "评价晒单",
                url: "/member/comment/list",
                // D1146：评价功能关闭时隐藏该入口
                show: commonStore.productCommentEnabled === 1
            },
            {
                name: "我的购物车",
                url: "/cart",
                show: true
            },
            // 发票：跨境/标准/o2o 都有，按后台开关显隐（对齐跨境 Nav）
            {
                name: "发票申请",
                url: "/member/orderInvoice/list",
                show: commonStore.canInvoice === 1
            },
            {
                name: "增票资质",
                url: "/member/userInvoice/info",
                show: commonStore.canInvoice === 1 && commonStore.invoiceAdded === 1
            }
        ]
    },
    {
        tit: "关注中心",
        list: [
            {
                name: "心愿单",
                url: "/member/collectProduct/list",
                show: true
            },
            // 收藏店铺：三端通用（标准/跨境叫「店铺」，o2o 叫「门店」），不可只绑 isStore()
            {
                name: isStore() ? "收藏的门店" : "收藏的店铺",
                url: "/member/collectShop/list",
                show: isMerchant() || isStore() || isOverseas()
            },
            {
                name: "浏览历史",
                url: "/member/browsingHistory/list",
                show: true
            }
        ]
    },
    {
        tit: "资产中心",
        list: [
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
            }
        ]
    },
    {
        tit: "客户服务",
        list: [
            {
                name: "售后管理",
                url: "/member/return/list",
                show: true
            },
            {
                name: "留言/咨询",
                url: "/member/feedback/list",
                show: true
            },
            {
                name: "我的询价单",
                url: "/member/inquiry/list",
                show: isB2B()
            }
        ]
    },
    {
        tit: "账户设置",
        list: [
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
                name: "地址",
                url: "/member/address/list",
                show: true
            },
        ]
    }
]);

onMounted(() => {
    fetchBadges();
});
</script>
<style lang="less" scoped>
.nav_row {
    width: 216px;
    background-color: #fff;
    border-radius: 8px;
    padding: 12px 0;
    font-size: 16px;
    margin-right: 16px;
    margin-bottom: 50px;

    .group {
        & + .group {
            margin-top: 12px;
        }

        &__title {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 8px 24px;
            font-size: 16px;
            color: #333;
            font-weight: 600;
            cursor: pointer;
            user-select: none;
            transition: color 0.2s;

            &:hover {
                color: var(--general);
            }
        }

        &__arrow {
            display: inline-block;
            width: 6px;
            height: 6px;
            border-right: 1.5px solid #ccc;
            border-bottom: 1.5px solid #ccc;
            transform: rotate(45deg);
            transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
        }

        &__title--collapsed .group__arrow {
            transform: rotate(-45deg);
        }

        &__list-wrapper {
            display: grid;
            grid-template-rows: 1fr;
            transition: grid-template-rows 0.3s cubic-bezier(0.4, 0, 0.2, 1);

            &--collapsed {
                grid-template-rows: 0fr;
            }
        }

        &__list {
            overflow: hidden;
            padding-top: 4px;
        }
    }

    .col {
        .item {
            a {
                height: 40px;
                padding: 0 24px 0 36px;
                margin: 4px 0;
                display: flex;
                align-items: center;
                justify-content: space-between;
                cursor: pointer;
                position: relative;
                color: #666;
                font-size: 15px;
                transition: all 0.2s;

                &:hover:not(.active) {
                    background-color: #fafafa;
                    color: var(--general);
                }
            }

            .item__name {
                flex: 1;
            }

            .item__badge {
                min-width: 20px;
                height: 20px;
                line-height: 20px;
                padding: 0 6px;
                border-radius: 10px;
                background: #e4393c;
                color: #fff;
                font-size: 12px;
                text-align: center;
                box-sizing: border-box;
                font-family: Arial, Helvetica, sans-serif;
            }

            .item__badge-dot {
                width: 6px;
                height: 6px;
                border-radius: 50%;
                background: #e4393c;
                margin-right: 6px;
            }

            .active {
                color: var(--general) !important;
                font-weight: 500;
                /* 使用 color-mix 生成基于主题色的极淡背景，兼容大多数现代浏览器 */
                background-color: color-mix(in srgb, var(--general) 8%, transparent);
            }
        }
    }
}
</style>
