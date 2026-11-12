'use client';

import Link from 'next/link';
import { useTranslation } from 'react-i18next';

/**
 * 全局站点脚。Vue 基准：components/common/PageFooter.vue。
 * 链接全部指向已迁移的路由，不留死链。
 */
export default function SiteFooter() {
    const { t } = useTranslation();
    const groups = [
        {
            title: t('购物指南'),
            links: [
                { label: t('全部商品'), href: '/list' },
                { label: t('活动专题'), href: '/activity' },
                { label: t('领券中心'), href: '/coupon/list' },
                { label: t('秒杀'), href: '/seckill/list' }
            ]
        },
        {
            title: t('我的账户'),
            links: [
                { label: t('我的订单'), href: '/member/order/list' },
                { label: t('售后与退款'), href: '/member/return/list' },
                { label: t('收货地址'), href: '/member/address/list' },
                { label: t('账号安全'), href: '/member/security/info' }
            ]
        },
        {
            title: t('商家服务'),
            links: [
                { label: t('商家入驻'), href: '/join' },
                { label: t('入驻申请'), href: '/member/joining/list' },
                { label: t('全部店铺'), href: '/shop/list' },
                { label: t('在线客服'), href: '/chat' }
            ]
        },
        {
            title: t('关于我们'),
            links: [
                { label: t('帮助中心'), href: '/article/issue/index' },
                { label: t('公告与资讯'), href: '/article/news/list' },
                { label: t('换购专区'), href: '/exchange/list' },
                { label: t('礼品卡兑换'), href: '/member/giftCard/exchange' }
            ]
        }
    ];

    return (
        <footer className="pc-site-footer">
            <div className="pc-footer-cols">
                {groups.map((group) => (
                    <div key={group.title} className="pc-footer-col">
                        <h4>{group.title}</h4>
                        {group.links.map((link) => (
                            <Link key={link.href} href={link.href}>
                                {link.label}
                            </Link>
                        ))}
                    </div>
                ))}
            </div>
            <div className="pc-footer-copy">
                © {new Date().getFullYear()} NexTrade · {t('本站为 PC 端重构版')}
            </div>
        </footer>
    );
}
