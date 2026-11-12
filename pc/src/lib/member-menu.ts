/**
 * 会员中心侧栏 / 路由映射。
 * 来源：Vue 版 pages/member/ 下 66 个 .vue（2026-11-12 盘点）。
 * path 一律小写（Next App Router 大小写敏感，与 pc-semi 口径一致），
 * source 保留原 Vue 文件名，供逐页 1:1 重写时对照。
 */
export type MemberMenuItem = { label: string; path: string; source: string };
export type MemberMenuGroup = { group: string; items: MemberMenuItem[] };

export const MEMBER_MENU: MemberMenuGroup[] = [
    {
        group: '交易',
        items: [
            { label: '我的订单', path: '/member/order/list', source: 'pages/member/order/List.vue' },
            { label: '售后退款', path: '/member/return/list', source: 'pages/member/return/List.vue' },
            { label: '我的评价', path: '/member/comment/list', source: 'pages/member/comment/List.vue' },
            { label: '订单发票', path: '/member/orderInvoice/list', source: 'pages/member/orderInvoice/List.vue' },
            { label: '我的拼团', path: '/member/joining/list', source: 'pages/member/joining/List.vue' },
            { label: '我的团购', path: '/member/pin/list', source: 'pages/member/pin/List.vue' }
        ]
    },
    {
        group: '资产',
        items: [
            { label: '账户余额', path: '/member/account/detail', source: 'pages/member/account/Detail.vue' },
            { label: '资金明细', path: '/member/account/log', source: 'pages/member/account/Log.vue' },
            { label: '我的优惠券', path: '/member/coupon/list', source: 'pages/member/coupon/List.vue' },
            { label: '积分', path: '/member/point/list', source: 'pages/member/point/List.vue' },
            { label: '账期明细', path: '/member/termPay/list', source: 'pages/member/termPay/list.vue' },
            { label: '礼品卡', path: '/member/giftCard/exchange', source: 'pages/member/giftCard/exchange.vue' }
        ]
    },
    {
        group: '收藏与足迹',
        items: [
            { label: '收藏商品', path: '/member/collectProduct/list', source: 'pages/member/collectProduct/List.vue' },
            { label: '收藏店铺', path: '/member/collectShop/list', source: 'pages/member/collectShop/List.vue' },
            { label: '浏览记录', path: '/member/browsingHistory/list', source: 'pages/member/browsingHistory/List.vue' }
        ]
    },
    {
        group: '账户与资料',
        items: [
            { label: '个人资料', path: '/member/profile/info', source: 'pages/member/profile/Info.vue' },
            { label: '收货地址', path: '/member/address/list', source: 'pages/member/address/List.vue' },
            { label: '账号安全', path: '/member/security/info', source: 'pages/member/security/Info.vue' },
            { label: '实名认证', path: '/member/userCertification/info', source: 'pages/member/userCertification/Info.vue' },
            { label: '发票管理', path: '/member/userInvoice/list', source: 'pages/member/userInvoice/Info.vue' }
        ]
    },
    {
        group: '服务',
        items: [
            { label: '我的消息', path: '/member/userMessage/list', source: 'pages/member/userMessage/List.vue' },
            { label: '反馈与咨询', path: '/member/feedback/list', source: 'pages/member/feedback/List.vue' },
            { label: '询价单', path: '/member/inquiry/list', source: 'pages/member/inquiry/List.vue' }
        ]
    }
];

export function findMemberSource(slug: string): string | null {
    for (const group of MEMBER_MENU) {
        for (const item of group.items) {
            if (item.path === `/member/${slug}`) return item.source;
        }
    }
    return null;
}
