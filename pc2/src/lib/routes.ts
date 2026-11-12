/**
 * Vue 原版页面 → React/Next 路由的迁移清单。
 * vueFiles = pc2/pages/<dir> 下 .vue 文件数（2026-11-12 实测），用于排迁移优先级。
 * 每迁移完一个路由，把 status 改成 'done'，并在 src/app/<path>/page.tsx 落地实现。
 * 约定：Vue 源码只作 1:1 参照（运行时依赖已移除，不再参与构建）；
 *       多语言统一 react-i18next 的 useTranslation() / t()，不再用 Vue 的 $t。
 */
export type MigrationRoute = {
    path: string;
    label: string;
    source: string;
    vueFiles: number;
    status: 'done' | 'in_progress' | 'pending';
};

export const MIGRATION_ROUTES: MigrationRoute[] = [
    { path: '/', label: '首页', source: 'pages/index.vue', vueFiles: 1, status: 'done' },
    { path: '/member', label: '会员中心', source: 'pages/member/', vueFiles: 66, status: 'in_progress' },
    { path: '/order', label: '订单', source: 'pages/order/', vueFiles: 20, status: 'pending' },
    { path: '/shop', label: '店铺', source: 'pages/shop/', vueFiles: 18, status: 'pending' },
    { path: '/activity', label: '活动', source: 'pages/activity/', vueFiles: 16, status: 'pending' },
    { path: '/search', label: '搜索', source: 'pages/search/', vueFiles: 13, status: 'pending' },
    { path: '/chat', label: '客服/IM', source: 'pages/chat/', vueFiles: 12, status: 'pending' },
    { path: '/join', label: '入驻', source: 'pages/join/', vueFiles: 9, status: 'pending' },
    { path: '/cart', label: '购物车', source: 'pages/cart/', vueFiles: 7, status: 'pending' },
    { path: '/search.v1', label: '搜索（旧版）', source: 'pages/search.v1/', vueFiles: 7, status: 'pending' },
    { path: '/article', label: '文章/公告', source: 'pages/article/', vueFiles: 6, status: 'pending' },
    { path: '/install', label: '安装', source: 'pages/install/', vueFiles: 5, status: 'pending' },
    { path: '/item', label: '商品（旧路径）', source: 'pages/item/', vueFiles: 5, status: 'pending' },
    { path: '/giftCard', label: '礼品卡', source: 'pages/giftCard/', vueFiles: 4, status: 'pending' },
    { path: '/joining', label: '入驻流程', source: 'pages/joining/', vueFiles: 4, status: 'pending' },
    { path: '/shopCategory', label: '店铺分类', source: 'pages/shopCategory/', vueFiles: 3, status: 'pending' },
    { path: '/coupon', label: '优惠券', source: 'pages/coupon/', vueFiles: 2, status: 'pending' },
    { path: '/exchange', label: '换购', source: 'pages/exchange/', vueFiles: 2, status: 'pending' },
    { path: '/category', label: '分类', source: 'pages/category/', vueFiles: 1, status: 'pending' },
    { path: '/list', label: '商品列表', source: 'pages/list/', vueFiles: 1, status: 'pending' },
    { path: '/oauth', label: '第三方登录', source: 'pages/oauth/', vueFiles: 1, status: 'pending' },
    { path: '/pcMobile', label: '移动端壳', source: 'pages/pcMobile/', vueFiles: 1, status: 'pending' },
    { path: '/product', label: '商品详情', source: 'pages/product/', vueFiles: 1, status: 'pending' },
    { path: '/seckill', label: '秒杀', source: 'pages/seckill/', vueFiles: 1, status: 'pending' }
];
