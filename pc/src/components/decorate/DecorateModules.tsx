'use client';

import type { ReactNode } from 'react';
import Link from 'next/link';
import { Card, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { formatPrice, productImage } from '@/lib/format';
import { GrouponSection, SeckillSection } from '@/components/promotion/sections';

/**
 * 装修模块渲染器（React 版）。
 * Vue 基准：pages/index.vue（首页 20 个模块）+ pages/activity/index.vue（12 个模块）+ decorate/components/pc/*。
 *
 * 关键：真实数据里 type 是 kebab-case（pc-image-ad），旧装修数据是 camelCase（pcImageAd）。
 * 统一 normalize 成 kebab 再查表，两种数据都能渲染。
 * 未知 type 静默跳过（生产不出现红块）。
 * 秒杀/拼团楼层：装修数据里只有标题与倒计时文案，商品另取（getSeckill / groupon/itemList）；
 * 优惠券仍是入口卡（券数据需登录态，放首页会白跑一次请求）。
 */
export interface DecorateModule {
    moduleIndex?: number;
    type?: string;
    isShow?: boolean;
    module?: Record<string, unknown>;
}

type Data = Record<string, unknown>;

const asList = (value: unknown): Data[] => (Array.isArray(value) ? (value as Data[]) : []);
const str = (value: unknown): string => (typeof value === 'string' ? value : '');
const normalizeType = (type: string) => type.replace(/([A-Z])/g, '-$1').toLowerCase();

/** 图片项统一取图与跳转：picUrl / picThumb 是图，picLink 是跳转。 */
function imageOf(item: Data): string {
    return str(item.picUrl) || str(item.picThumb) || str(item.imageUrl) || str(item.img);
}
function linkOf(item: Data): string {
    return str(item.picLink) || str(item.link) || str(item.url) || '#';
}

/** 横滑图片区（banner / 广告 / 宫格通用）。 */
function ImageRow({ items, ratio }: { items: Data[]; ratio?: string }) {
    if (items.length === 0) return null;
    return (
        <div className="pc-dec-scroller">
            {items.map((item, index) => {
                const src = imageOf(item);
                if (!src) return null;
                return (
                    <a
                        key={index}
                        href={linkOf(item)}
                        className="pc-dec-slide"
                        style={ratio ? { flex: `0 0 ${ratio}` } : undefined}
                        title={str(item.picTitle) || str(item.picName)}
                    >
                        <img src={src} alt={str(item.picTitle) || str(item.picName)} />
                    </a>
                );
            })}
        </div>
    );
}

/** 图标导航（quick-nav / image-nav / category-nav3 共用）。 */
function NavIcons({ items }: { items: Data[] }) {
    if (items.length === 0) return null;
    return (
        <div className="pc-dec-nav">
            {items.map((item, index) => {
                const icon = imageOf(item);
                return (
                    <a key={index} href={linkOf(item)} className="pc-dec-nav-item">
                        {icon ? <img src={icon} alt="" /> : null}
                        <span>{str(item.picTitle) || str(item.picName)}</span>
                    </a>
                );
            })}
        </div>
    );
}

/** 商品网格。products 项字段在不同模块里可能是 productName / goodsName。 */
function ProductGrid({ products }: { products: Data[] }) {
    if (products.length === 0) return null;
    return (
        <div className="pc-product-grid">
            {products.map((product, index) => {
                const name = str(product.productName) || str(product.goodsName);
                const href = str(product.productLink) || `/product/${product.productSn ?? product.productId ?? ''}`;
                return (
                    <a key={index} className="pc-product-card" href={href}>
                        <img
                            src={productImage(
                                product as { picThumb?: string; productPicThumb?: string; productImage?: string }
                            )}
                            alt={name}
                        />
                        <div className="pc-product-name">{name}</div>
                        <div className="pc-product-price">
                            {formatPrice(product.productPrice ?? product.price ?? product.goodsPrice)}
                        </div>
                    </a>
                );
            })}
        </div>
    );
}

function Title({ module }: { module: Data }) {
    const title = str(module.title) || str(module.titleName) || str(module.titleText);
    const more = str(module.moreLink);
    if (!title) return null;
    return (
        <div className="pc-dec-title">
            <Typography.Title heading={4}>{title}</Typography.Title>
            {module.subtitle ? <span className="pc-muted">{str(module.subtitle)}</span> : null}
            {more ? <Link href={more}>{'更多 →'}</Link> : null}
        </div>
    );
}

/** 楼标题：真实数据在 module.title.titleText（对象），旧数据可能是字符串。 */
function sectionTitle(module: Data, fallback = ''): string {
    const nested = (module.title ?? {}) as Data;
    return str(nested.titleText) || str(module.title) || str(module.seckillTitle) || str(module.titleName) || fallback;
}
function sectionCountdownLabel(module: Data): string {
    const nested = (module.title ?? {}) as Data;
    return str(nested.countdownLabel);
}

function EntryCard({ module, label, href }: { module: Data; label: string; href: string }) {
    const title = str(module.title) || str(module.seckillTitle) || label;
    return (
        <Card className="pc-dec-entry">
            <Typography.Title heading={4}>{title}</Typography.Title>
            <Typography.Paragraph type="tertiary">
                {str(module.desc) || `进入${label}`}
            </Typography.Paragraph>
            <Link href={href}>{`去${label} →`}</Link>
        </Card>
    );
}

function HeroBlock({ module }: { module: Data }) {
    const parts = [module.left, module.center, module.right].filter(Boolean) as Data[];
    const items = parts.map((part) => (part.image ? (part.image as Data) : part)).filter((part) => imageOf(part));
    if (items.length === 0) return null;
    return <ImageRow items={items} ratio="1" />;
}

const RENDERERS: Record<string, (module: Data) => ReactNode> = {
    'pc-image-ad': (m) => <ImageRow items={asList(m.picList)} />,
    'pc-image-square-ad': (m) => <ImageRow items={asList(m.picList)} ratio="calc(50% - 6px)" />,
    'pc-image-hotarea': (m) => <ImageRow items={asList(m.picList)} />,
    'pc-banner2': (m) => (
        <ImageRow items={[...asList(m.picList), ...asList(m.picList2), ...asList(m.picList3)]} />
    ),
    'pc-group-banner': (m) => (
        <ImageRow items={[...asList(m.picList), ...asList(m.picList2), ...asList(m.picList3)]} />
    ),
    'pc-home-hero': (m) => <HeroBlock module={m} />,
    'pc-image-nav': (m) => <NavIcons items={asList(m.picList)} />,
    'pc-home-quick-nav': (m) => <NavIcons items={asList(m.items ?? m.picList)} />,
    'pc-category-nav3': (m) => <NavIcons items={asList(m.picList)} />,
    'pc-category-nav': (m) => (
        <div className="pc-dec-title">
            <Typography.Title heading={4}>{str(m.titleName) || '商品分类'}</Typography.Title>
            <Link href="/list">全部 →</Link>
        </div>
    ),
    'pc-product': (m) => <ProductGrid products={asList(m.products ?? m.goodsList)} />,
    'pc-recommend-product': (m) => (
        <>
            {str(m.titleName) ? (
                <div className="pc-dec-title">
                    <Typography.Title heading={4}>{str(m.titleName)}</Typography.Title>
                </div>
            ) : null}
            <ProductGrid products={asList(m.products ?? m.goodsList)} />
        </>
    ),
    'pc-product-group': (m) => (
        <>
            {asList(m.productGroups).map((group, index) => (
                <div key={index} className="pc-dec-group">
                    {str(group.title) ? (
                        <div className="pc-dec-title">
                            <Typography.Title heading={4}>{str(group.title)}</Typography.Title>
                        </div>
                    ) : null}
                    <ProductGrid products={asList(group.productList)} />
                </div>
            ))}
        </>
    ),
    'pc-coupon': (m) => <EntryCard module={m} label="领券中心" href="/coupon/list" />,
    'pc-home-coupon': (m) => <EntryCard module={m} label="领券中心" href="/coupon/list" />,
    'pc-seckill': (m) => (
        <SeckillSection title={sectionTitle(m, '限时秒杀')} countdownLabel={sectionCountdownLabel(m)} />
    ),
    'pc-home-seckill': (m) => (
        <SeckillSection title={sectionTitle(m, '限时秒杀')} countdownLabel={sectionCountdownLabel(m)} />
    ),
    'pc-groupon': (m) => <GrouponSection title={sectionTitle(m, '拼团')} countdownLabel={sectionCountdownLabel(m)} />,
    'pc-home-groupon': (m) => (
        <GrouponSection title={sectionTitle(m, '拼团')} countdownLabel={sectionCountdownLabel(m)} />
    ),
    'pc-notice': (m) => {
        const texts = asList(m.noticeList).map((item) => str(item.title ?? item.content));
        const single = str(m.text);
        const lines = texts.length > 0 ? texts : single ? [single] : [];
        if (lines.length === 0) return null;
        return (
            <Card className="pc-dec-notice">
                {lines.map((line, index) => (
                    <div key={index} className="pc-dec-notice-item">
                        <span>{line}</span>
                    </div>
                ))}
            </Card>
        );
    },
    'pc-title-ad': (m) => <Title module={m} />,
    'pc-white-line': () => <div className="pc-dec-line" />,
    'pc-divider': () => <div className="pc-dec-line" />,
    'pc-white-blank': (m) => <div style={{ height: Number(m.height ?? 24) }} />,
    'pc-spacer': (m) => <div style={{ height: Number(m.blankHeight ?? 24) }} />,
    'pc-custom-ad1': (m) => <ImageRow items={asList(m.picList)} />,
    'pc-custom-ad2': (m) => <ImageRow items={asList(m.picList)} ratio="calc(50% - 6px)" />
};

export default function DecorateModules({ modules }: { modules: DecorateModule[] }) {
    const { t } = useTranslation();
    const visible = (modules ?? []).filter((item) => {
        const flag = item.isShow as unknown;
        return flag !== false && flag !== 0;
    });
    if (visible.length === 0) return <div className="pc-muted">{t('暂无装修内容')}</div>;

    return (
        <div className="pc-dec-wrap">
            {visible.map((item, index) => {
                const renderer = item.type ? RENDERERS[normalizeType(item.type)] : undefined;
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
