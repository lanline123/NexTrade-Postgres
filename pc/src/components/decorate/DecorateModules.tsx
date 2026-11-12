'use client';

import type { ReactNode } from 'react';
import Link from 'next/link';
import { Card, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { formatPrice, productImage } from '@/lib/format';

/**
 * 装修模块渲染器（React 版）。
 * Vue 基准：pages/activity/index.vue 的 module.type 分发 + 各 pcXxx/index.vue。
 * 设计：一个 type → 一个渲染函数，未知类型静默跳过（生产不出现红块）。
 * 秒杀/拼团/商品分组三个模块内部逻辑极重（Vue 侧 2000+ 行），这里降级为入口卡片，
 * 等业务确认后再逐模块补，不在渲染器里堆逻辑。
 */
export interface DecorateModule {
    moduleIndex?: number;
    type?: string;
    isShow?: boolean;
    module?: Record<string, unknown>;
}

type ModuleData = Record<string, unknown>;

const list = (value: unknown): ModuleData[] => (Array.isArray(value) ? (value as ModuleData[]) : []);
const text = (value: unknown): string => (typeof value === 'string' ? value : '');

function Images({ module }: { module: ModuleData }) {
    const images = list(module.imageList ?? module.picList ?? module.images);
    if (images.length === 0 && (module.picUrl || module.imageUrl)) {
        images.push({ picUrl: module.picUrl ?? module.imageUrl, link: module.link ?? module.url });
    }
    if (images.length === 0) return null;
    return (
        <div className="pc-dec-images">
            {images.map((image, index) => {
                const src = text(image.picUrl ?? image.imageUrl ?? image.url);
                if (!src) return null;
                const content = <img src={src} alt="" />;
                return image.link ? (
                    <a key={index} href={String(image.link)}>
                        {content}
                    </a>
                ) : (
                    <span key={index}>{content}</span>
                );
            })}
        </div>
    );
}

function NavIcons({ module }: { module: ModuleData }) {
    const items = list(module.navList ?? module.list ?? module.items);
    if (items.length === 0) return null;
    return (
        <div className="pc-dec-nav">
            {items.map((item, index) => {
                const icon = text(item.picUrl ?? item.icon ?? item.imageUrl);
                return (
                    <a key={index} href={String(item.link ?? '#')} className="pc-dec-nav-item">
                        {icon ? <img src={icon} alt="" /> : null}
                        <span>{text(item.name ?? item.title)}</span>
                    </a>
                );
            })}
        </div>
    );
}

function ProductBlock({ module }: { module: ModuleData }) {
    const products = list(module.productList ?? module.goodsList ?? module.list);
    if (products.length === 0) return null;
    return (
        <div className="pc-product-grid">
            {products.map((product, index) => (
                <a
                    key={index}
                    className="pc-product-card"
                    href={`/product/${product.productSn ?? product.productId ?? ''}`}
                >
                    <img
                        src={productImage(product as { picThumb?: string; productPicThumb?: string; productImage?: string })}
                        alt={text(product.productName)}
                    />
                    <div className="pc-product-name">{text(product.productName)}</div>
                    <div className="pc-product-price">{formatPrice(product.productPrice ?? product.price)}</div>
                </a>
            ))}
        </div>
    );
}

function CouponBlock({ module }: { module: ModuleData }) {
    const coupons = list(module.couponList ?? module.list);
    if (coupons.length === 0) return null;
    return (
        <div className="pc-card-grid">
            {coupons.map((coupon, index) => (
                <Card key={index} className="pc-coupon-card">
                    <div className="pc-coupon-amount">
                        {coupon.couponDiscount ? `${coupon.couponDiscount} 折` : formatPrice(coupon.couponMoney)}
                    </div>
                    <div className="pc-coupon-name">{text(coupon.couponName)}</div>
                    <div className="pc-muted">满 {formatPrice(coupon.minOrderAmount)} 可用</div>
                </Card>
            ))}
        </div>
    );
}

function NoticeBlock({ module }: { module: ModuleData }) {
    const notices = list(module.noticeList ?? module.list);
    if (notices.length === 0) return null;
    return (
        <Card className="pc-dec-notice">
            {notices.map((notice, index) => (
                <div key={index} className="pc-dec-notice-item">
                    <span>{text(notice.title ?? notice.content)}</span>
                    <span className="pc-muted">{text(notice.addTime ?? notice.time)}</span>
                </div>
            ))}
        </Card>
    );
}

function EntryCard({ module, label, href }: { module: ModuleData; label: string; href: string }) {
    return (
        <Card className="pc-dec-entry">
            <Typography.Title heading={4}>{text(module.title) || label}</Typography.Title>
            <Typography.Paragraph type="tertiary">
                {text(module.subtitle) || `进入${label}`}
            </Typography.Paragraph>
            <Link href={href}>{`去${label} →`}</Link>
        </Card>
    );
}

const RENDERERS: Record<string, (module: ModuleData) => ReactNode> = {
    pcImageAd: (module) => <Images module={module} />,
    pcImageSquareAd: (module) => <Images module={module} />,
    pcImageHotarea: (module) => <Images module={module} />,
    pcImageNav: (module) => <NavIcons module={module} />,
    pcProduct: (module) => <ProductBlock module={module} />,
    pcCoupon: (module) => <CouponBlock module={module} />,
    pcNotice: (module) => <NoticeBlock module={module} />,
    pcTitleAd: (module) => (
        <div className="pc-dec-title">
            <Typography.Title heading={4}>{text(module.title)}</Typography.Title>
            {text(module.subtitle) ? <span className="pc-muted">{text(module.subtitle)}</span> : null}
            {module.moreLink ? <a href={String(module.moreLink)}>更多 →</a> : null}
        </div>
    ),
    pcWhiteLine: () => <div className="pc-dec-line" />,
    pcWhiteBlank: (module) => <div style={{ height: Number(module.height ?? 24) }} />,
    pcSeckill: (module) => <EntryCard module={module} label="秒杀" href="/seckill/list" />,
    pcGroupon: (module) => <EntryCard module={module} label="拼团" href="/activity" />,
    pcProductGrouping: (module) => <ProductBlock module={module} />
};

export default function DecorateModules({ modules }: { modules: DecorateModule[] }) {
    const { t } = useTranslation();
    const visible = (modules ?? []).filter((item) => item.isShow !== false);
    if (visible.length === 0) return <div className="pc-muted">{t('暂无装修内容')}</div>;

    return (
        <div className="pc-dec-wrap">
            {visible.map((item, index) => {
                const renderer = item.type ? RENDERERS[item.type] : undefined;
                if (!renderer) return null;
                return (
                    <section className="pc-dec-section" key={`${item.type}-${item.moduleIndex ?? index}`}>
                        {renderer(item.module ?? {})}
                    </section>
                );
            })}
        </div>
    );
}
