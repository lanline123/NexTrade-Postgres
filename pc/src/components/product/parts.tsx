'use client';

import { useState } from 'react';
import { Button, Input, Tag, Toast } from '@douyinfe/semi-ui';
import { useRouter } from 'next/navigation';
import { useTranslation } from 'react-i18next';
import { apiPost } from '@/lib/client-api';
import { formatPrice, productImage, skuText } from '@/lib/format';
import { CountdownBoxes, useCountdown } from '@/components/promotion/cards';
import type { ProductDetailData, ProductSku } from './useProductDetail';

/**
 * 三套详情布局共用的积木：图集、购买面板、图文详情、规格参数表。
 * Vue 基准：ProductThumb / ProductInfo / ProductDetailTabs 的对应区块；
 * 每套布局的差别在「怎么摆这些积木 + 独有区块」，积木本身是同一份接口数据。
 */

export function ProductGallery({ detail, skus }: { detail: ProductDetailData; skus: ProductSku[] }) {
    const { t } = useTranslation();
    const pics = (detail.picList ?? []).filter((pic) => pic.picUrl || pic.picThumb);
    const list = pics.length ? pics : [{ picUrl: detail.item?.picThumb || detail.item?.picUrl }];
    const [index, setIndex] = useState(0);
    const [showVideo, setShowVideo] = useState(false);
    const video = detail.videoList?.[0];
    const cover = list[index]?.picUrl ?? list[index]?.picThumb ?? detail.item?.picThumb;

    return (
        <div className="pc-pd-gallery-block">
            <div className="pc-pd-stage">
                {showVideo && video?.videoUrl ? (
                    <video src={video.videoUrl} poster={video.videoCover} controls preload="metadata" />
                ) : (
                    <img src={cover || '/assets/avatar/default.gif'} alt={detail.item?.productName ?? ''} />
                )}
                {video?.videoUrl ? (
                    <button type="button" className="pc-pd-video-toggle" onClick={() => setShowVideo(!showVideo)}>
                        {showVideo ? t('看图片') : t('看视频')}
                    </button>
                ) : null}
            </div>
            {list.length > 1 ? (
                <div className="pc-pd-thumbs">
                    {list.map((pic, i) => (
                        <button
                            key={i}
                            type="button"
                            className={`pc-pd-thumb${i === index && !showVideo ? ' is-active' : ''}`}
                            onClick={() => {
                                setIndex(i);
                                setShowVideo(false);
                            }}
                        >
                            <img src={productImage(pic)} alt="" loading="lazy" />
                        </button>
                    ))}
                </div>
            ) : null}
            {skus.length > 0 && !showVideo ? (
                <div className="pc-pd-thumbs-hint pc-muted">
                    {t('共')} {list.length} {t('张图')}
                </div>
            ) : null}
        </div>
    );
}

export interface BuyPanelProps {
    detail: ProductDetailData;
    skus: ProductSku[];
    skuId: number;
    setSkuId: (id: number) => void;
    price?: number | string;
    stock?: number;
    invited?: boolean;
}

export function ProductBuyPanel({ detail, skus, skuId, setSkuId, price, stock, invited }: BuyPanelProps) {
    const { t } = useTranslation();
    const router = useRouter();
    const [quantity, setQuantity] = useState(1);
    const [pending, setPending] = useState<'' | 'cart' | 'buy'>('');
    const item = detail.item;
    const groupon = detail.grouponDetail ?? null;
    const grouponCd = useCountdown(groupon?.endTime ?? 0);
    const activities = (item?.activityInfo ?? []).filter((row) => row.typeText || row.promotionName);
    const soldOut = Number(stock ?? 0) <= 0;

    const addToCart = async (thenCheckout = false) => {
        const productId = item?.productId;
        if (!productId) return Toast.warning(t('商品信息不完整'));
        setPending(thenCheckout ? 'buy' : 'cart');
        try {
            // 参数名必须是 id / number（Vue 的 addToCart 同样如此）
            await apiPost('cart/cart/addToCart', { id: productId, number: quantity, skuId: skuId || undefined });
            if (thenCheckout) {
                router.push('/order/check');
                return;
            }
            Toast.success(t('已加入购物车'));
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setPending('');
        }
    };

    return (
        <div className="pc-pd-buy">
            {groupon ? (
                <div className="pc-pd-groupon">
                    <div>
                        <span className="pc-promo-price">{formatPrice(groupon.minGrouponPrice)}</span>
                        <span className="pc-promo-origin">{formatPrice(price)}</span>
                        {groupon.minGroupNum ? (
                            <Tag size="small" color="red">
                                {groupon.minGroupNum}
                                {t('人团')}
                            </Tag>
                        ) : null}
                    </div>
                    <div className="pc-muted">
                        {t('已参团')} {groupon.joinNumCount ?? 0}
                        {groupon.endTime ? (
                            <>
                                {' · '}
                                {t('距结束')} <CountdownBoxes cd={grouponCd} endedText={t('已结束')} />
                            </>
                        ) : null}
                    </div>
                    {groupon.canJoin === false ? (
                        <div className="pc-muted">{groupon.restrictTip || t('仅限指定会员参与')}</div>
                    ) : null}
                </div>
            ) : (
                <div className="pc-pd-price-row">
                    <div className="pc-pd-price">{formatPrice(price)}</div>
                    {item?.marketPrice && Number(item.marketPrice) > Number(price ?? 0) ? (
                        <span className="pc-promo-origin">{formatPrice(item.marketPrice)}</span>
                    ) : null}
                    {detail.seckillDetail?.seckillPrice ? (
                        <Tag size="small" color="red">
                            {t('秒杀价')}
                            {formatPrice(detail.seckillDetail.seckillPrice)}
                        </Tag>
                    ) : null}
                </div>
            )}

            <TypographyTitle name={item?.productName} brief={item?.productBrief} />

            <div className="pc-muted pc-pd-sn">
                {t('货号')} {item?.productSn ?? '-'} · {t('库存')} {stock ?? '-'}
                {item?.limitNumber && item.limitNumber > 0 ? ` · ${t('限购')}${item.limitNumber}` : ''}
            </div>

            {activities.length > 0 ? (
                <div className="pc-pd-activities">
                    {activities.map((row, index) => (
                        <Tag key={row.promotionId ?? index} size="small" color="blue">
                            {row.typeText ? `${row.typeText}：` : ''}
                            {row.promotionName ?? ''}
                        </Tag>
                    ))}
                </div>
            ) : null}

            {skus.length > 0 ? (
                <div className="pc-pd-skus">
                    {skus.map((sku) => {
                        const key = sku.skuId ?? sku.id ?? 0;
                        return (
                            <button
                                key={key}
                                type="button"
                                className={`pc-pay-channel${key === skuId ? ' is-active' : ''}`}
                                onClick={() => setSkuId(key)}
                            >
                                {sku.skuName ?? skuText(sku.skuData) ?? key}
                            </button>
                        );
                    })}
                </div>
            ) : null}

            <div className="pc-cart-qty">
                <button type="button" onClick={() => setQuantity(Math.max(1, quantity - 1))}>
                    -
                </button>
                <span>{quantity}</span>
                <button type="button" onClick={() => setQuantity(quantity + 1)}>
                    +
                </button>
            </div>

            <div className="pc-order-actions">
                <Button
                    theme="solid"
                    type="primary"
                    loading={pending === 'cart'}
                    disabled={soldOut}
                    onClick={() => void addToCart(false)}
                >
                    {soldOut ? t('已售罄') : t('加入购物车')}
                </Button>
                <Button theme="light" loading={pending === 'buy'} disabled={soldOut} onClick={() => void addToCart(true)}>
                    {groupon ? (invited ? t('参与拼团') : t('我要开团')) : t('立即购买')}
                </Button>
                <a href="/cart">
                    <Button theme="borderless">{t('去购物车')}</Button>
                </a>
            </div>

            {(detail.serviceList ?? []).length > 0 ? (
                <div className="pc-pd-services">
                    {(detail.serviceList ?? []).map((service, index) => (
                        <span key={index} title={service.productServiceDesc}>
                            {service.icoImg ? <img src={service.icoImg} alt="" /> : null}
                            {service.productServiceName}
                        </span>
                    ))}
                </div>
            ) : null}
        </div>
    );
}

function TypographyTitle({ name, brief }: { name?: string; brief?: string }) {
    const { t } = useTranslation();
    if (!name) return <span className="pc-muted">{t('商品信息不完整')}</span>;
    return (
        <div className="pc-pd-name">
            <h1>{name}</h1>
            {brief ? <p className="pc-muted">{brief}</p> : null}
        </div>
    );
}

/** 图文详情：descArr 的 pic/text/video 三种条目；富文本只渲染图片，避免注入。 */
export function ProductDescSection({ detail }: { detail: ProductDetailData }) {
    const { t } = useTranslation();
    const items = detail.descArr ?? [];
    if (items.length === 0) return <div className="pc-muted">{t('暂无图文详情')}</div>;
    return (
        <div className="pc-pd-desc">
            {items.map((row, index) => {
                if (row.type === 'pic' && row.pic) return <img key={index} src={row.pic} alt="" loading="lazy" />;
                if (row.type === 'video' && row.videoUrl)
                    return <video key={index} src={row.videoUrl} poster={row.cover} controls preload="metadata" />;
                if (row.type === 'text' && row.html) {
                    // 后台富文本里主要是 <img>，只提取图片，文本走纯文本
                    const text = row.html.replace(/<[^>]*>/g, '').trim();
                    const pics = Array.from(row.html.matchAll(/<img[^>]+src=["']([^"']+)["']/g)).map((m) => m[1]);
                    return (
                        <div key={index}>
                            {pics.map((src, i) => (
                                <img key={i} src={src} alt="" loading="lazy" />
                            ))}
                            {text ? <p>{text}</p> : null}
                        </div>
                    );
                }
                return null;
            })}
        </div>
    );
}

/** 规格参数表：attrList.spe 分组。 */
export function ProductSpecTable({ detail }: { detail: ProductDetailData }) {
    const { t } = useTranslation();
    const groups = detail.attrList?.spe ?? [];
    if (groups.length === 0) return <div className="pc-muted">{t('暂无规格参数')}</div>;
    return (
        <div className="pc-pd-spec">
            {groups.map((group, groupIndex) => (
                <div key={group.attrName ?? groupIndex} className="pc-pd-spec-group">
                    {group.attrName ? <div className="pc-pd-spec-title">{group.attrName}</div> : null}
                    <table>
                        <tbody>
                            {(group.attrList ?? []).map((row, index) => (
                                <tr key={row.attributesId ?? index}>
                                    <td>{row.attrName}</td>
                                    <td>{row.attrValue}</td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            ))}
        </div>
    );
}
