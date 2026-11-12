'use client';

import { useCallback, useEffect, useState } from 'react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { Badge, Input } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import LocaleSwitch from '@/components/LocaleSwitch';
import { apiGet } from '@/lib/client-api';

/**
 * 全局站点头。Vue 基准：components/common/Header.vue + PageHeaderNew.vue。
 * 数据：common/pc/getNav（topBarNav）、common/pc/getCatFloor（分类楼层）、
 * cart/cart/getCount（角标，需登录）、user/user/memberCenter（昵称）。
 * 后台 mainNav 为空时用 FALLBACK_NAV 兜底，保证导航永远可用。
 */
interface NavItem {
    id?: number;
    title?: string;
    isShow?: number;
    children?: NavItem[];
}

interface CatFloor {
    catFloorId?: number;
    catFloorName?: string;
    catList?: Array<{ categoryId?: number; categoryName?: string; name?: string }>;
}

const FALLBACK_NAV = [
    { title: '首页', href: '/' },
    { title: '全部商品', href: '/list' },
    { title: '活动专题', href: '/activity' },
    { title: '店铺', href: '/shop/list' },
    { title: '商家入驻', href: '/join' }
];

export default function SiteHeader() {
    const { t } = useTranslation();
    const router = useRouter();
    const [keyword, setKeyword] = useState('');
    const [floors, setFloors] = useState<CatFloor[]>([]);
    const [topLinks, setTopLinks] = useState<NavItem[]>([]);
    const [nickname, setNickname] = useState('');
    const [cartCount, setCartCount] = useState(0);
    const [catOpen, setCatOpen] = useState(false);
    const [loggedIn, setLoggedIn] = useState(false);

    const loadAuth = useCallback(() => {
        const token = localStorage.getItem('accessToken') || localStorage.getItem('token');
        if (!token) {
            setLoggedIn(false);
            setNickname('');
            setCartCount(0);
            return;
        }
        setLoggedIn(true);
        apiGet<{ nickname?: string }>('user/user/memberCenter')
            .then((info) => setNickname(info?.nickname ?? ''))
            .catch(() => setNickname(''));
        apiGet<number>('cart/cart/getCount')
            .then((count) => setCartCount(Number(count) || 0))
            .catch(() => setCartCount(0));
    }, []);

    useEffect(() => {
        apiGet<{ topBarNav?: NavItem[] }>('common/pc/getNav')
            .then((nav) => setTopLinks((nav?.topBarNav ?? []).filter((item) => item.isShow !== 0)))
            .catch(() => setTopLinks([]));
        apiGet<{ catFloor?: CatFloor[] }>('common/pc/getCatFloor')
            .then((result) => setFloors(result?.catFloor ?? []))
            .catch(() => setFloors([]));
        loadAuth();
        // 登录/退出后由 storage 事件或页面跳转触发重新读取
        const onStorage = () => loadAuth();
        window.addEventListener('storage', onStorage);
        return () => window.removeEventListener('storage', onStorage);
    }, [loadAuth]);

    const search = () => {
        const value = keyword.trim();
        router.push(value ? `/search?keyword=${encodeURIComponent(value)}` : '/search');
    };

    const logout = () => {
        localStorage.removeItem('accessToken');
        localStorage.removeItem('token');
        setLoggedIn(false);
        setNickname('');
        setCartCount(0);
        router.push('/');
    };

    return (
        <header className="pc-site-header">
            <div className="pc-header-top">
                <div className="pc-header-top-inner">
                    <div className="pc-header-top-links">
                        {topLinks.slice(0, 3).map((item) => (
                            <Link key={item.id ?? item.title} href={item.title === '帮助中心' ? '/article/issue/index' : '/article/news/list'}>
                                {item.title}
                            </Link>
                        ))}
                        <Link href="/article/news/list">{t('公告')}</Link>
                    </div>
                    <div className="pc-header-top-right">
                        {loggedIn ? (
                            <>
                                <Link href="/member">{nickname || t('我的账户')}</Link>
                                <Link href="/member/order/list">{t('我的订单')}</Link>
                                <button type="button" className="pc-link-btn" onClick={logout}>
                                    {t('退出')}
                                </button>
                            </>
                        ) : (
                            <>
                                <Link href="/member/login">{t('登录')}</Link>
                                <Link href="/member/register">{t('注册')}</Link>
                            </>
                        )}
                        <LocaleSwitch />
                    </div>
                </div>
            </div>

            <div className="pc-header-main">
                <Link href="/" className="pc-header-logo">
                    NexTrade
                </Link>

                <div className="pc-header-search">
                    <Input
                        value={keyword}
                        onChange={(value) => setKeyword(String(value))}
                        onEnterPress={search}
                        placeholder={t('搜索商品 / 店铺')}
                    />
                    <button type="button" className="pc-header-search-btn" onClick={search}>
                        {t('搜索')}
                    </button>
                </div>

                <Link href="/cart" className="pc-header-cart">
                    <Badge count={cartCount} overflowCount={99}>
                        <span className="pc-header-cart-icon">🛒</span>
                    </Badge>
                    <span>{t('购物车')}</span>
                </Link>
            </div>

            <nav className="pc-header-nav">
                <div className="pc-header-nav-inner">
                    <button type="button" className="pc-header-cat-btn" onClick={() => setCatOpen(!catOpen)}>
                        {t('全部商品分类')}
                    </button>
                    {FALLBACK_NAV.map((item) => (
                        <Link key={item.href} href={item.href} className="pc-header-nav-link">
                            {t(item.title)}
                        </Link>
                    ))}
                </div>

                {catOpen && floors.length > 0 ? (
                    <div className="pc-cat-dropdown">
                        {floors.map((floor) => (
                            <div key={floor.catFloorId ?? floor.catFloorName} className="pc-cat-floor">
                                <div className="pc-cat-floor-name">{floor.catFloorName}</div>
                                <div className="pc-cat-floor-items">
                                    {(floor.catList ?? []).map((cat) => (
                                        <Link
                                            key={cat.categoryId ?? cat.categoryName}
                                            href={`/list?categoryId=${cat.categoryId ?? ''}`}
                                            onClick={() => setCatOpen(false)}
                                        >
                                            {cat.categoryName ?? cat.name}
                                        </Link>
                                    ))}
                                </div>
                            </div>
                        ))}
                    </div>
                ) : null}
            </nav>
        </header>
    );
}
