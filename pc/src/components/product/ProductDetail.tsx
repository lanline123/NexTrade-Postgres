'use client';

import { useEffect, useState } from 'react';
import { Button, Card, Empty, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';
import { formatPrice, skuText } from '@/lib/format';

/**
 * 商品详情。
 * Vue 基准：pages/product/[...slug].vue + components/product/**（详情组件体量较大）。
 * 已实现：主图、名称、价格、库存、SKU 选择、数量、加入购物车。
 * 未迁：图文详情、评价、咨询、优惠券领取、秒杀/拼团/预售定价。
 */
interface SkuRow {
    skuId?: number;
    id?: number;
    skuName?: string;
    skuData?: unknown;
    productPrice?: number | string;
    price?: number | string;
    stock?: number;
}

interface ProductItem {
    productId?: number;
    productName?: string;
    productSn?: string;
    productPrice?: number | string;
    picThumb?: string;
    stock?: number;
    productStock?: number;
    unit?: string;
}

interface DetailResult {
    item?: ProductItem;
    skuList?: SkuRow[];
    picList?: Array<{ picUrl?: string; picThumb?: string }>;
    serviceList?: Array<{ name?: string; content?: string }>;
}

export default function ProductDetail({ idOrSn }: { idOrSn: string }) {
    const { t } = useTranslation();
    const [detail, setDetail] = useState<DetailResult | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');
    const [skuId, setSkuId] = useState<number>(0);
    const [quantity, setQuantity] = useState(1);
    const [adding, setAdding] = useState(false);

    useEffect(() => {
        if (!idOrSn) {
            setError(t('缺少商品参数'));
            setLoading(false);
            return;
        }
        const numeric = Number(idOrSn);
        const params = Number.isFinite(numeric) && numeric > 0 ? { id: numeric } : { sn: idOrSn };
        apiGet<DetailResult>('product/product/detail', params)
            .then((result) => {
                setDetail(result);
                const firstSku = result.skuList?.[0];
                setSkuId(firstSku?.skuId ?? firstSku?.id ?? 0);
            })
            .catch((e: Error) => setError(e.message))
            .finally(() => setLoading(false));
    }, [idOrSn, t]);

    const addToCart = async () => {
        const productId = detail?.item?.productId;
        if (!productId) return Toast.warning(t('商品信息不完整'));
        setAdding(true);
        try {
            await apiPost('cart/cart/addToCart', { productId, skuId, quantity });
            Toast.success(t('已加入购物车'));
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setAdding(false);
        }
    };

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('商品加载失败')} description={error} />;
    if (!detail?.item) return <Empty title={t('商品不存在')} />;

    const item = detail.item;
    const skus = detail.skuList ?? [];
    const activeSku = skus.find((sku) => (sku.skuId ?? sku.id) === skuId);
    const price = activeSku?.productPrice ?? activeSku?.price ?? item.productPrice;
    const stock = activeSku?.stock ?? item.stock ?? item.productStock;
    const cover = detail.picList?.[0]?.picUrl ?? detail.picList?.[0]?.picThumb ?? item.picThumb;

    return (
        <div className="pc-product-detail">
            <Card>
                <div className="pc-pd-top">
                    <div className="pc-pd-gallery">
                        <img src={cover || '/assets/avatar/default.gif'} alt={item.productName ?? ''} />
                    </div>
                    <div className="pc-pd-info">
                        <Typography.Title heading={3}>{item.productName}</Typography.Title>
                        <div className="pc-pd-price">{formatPrice(price)}</div>
                        <div className="pc-muted">
                            {t('货号')} {item.productSn ?? '-'} · {t('库存')} {stock ?? '-'}
                        </div>

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
                            <button type="button" onClick={() => setQuantity(Math.max(1, quantity - 1))}>-</button>
                            <span>{quantity}</span>
                            <button type="button" onClick={() => setQuantity(quantity + 1)}>+</button>
                        </div>

                        <div className="pc-order-actions">
                            <Button theme="solid" type="primary" loading={adding} onClick={() => void addToCart()}>
                                {t('加入购物车')}
                            </Button>
                            <a href="/cart">
                                <Button theme="light">{t('去购物车')}</Button>
                            </a>
                        </div>

                        {(detail.serviceList ?? []).length > 0 ? (
                            <div className="pc-muted">
                                {(detail.serviceList ?? []).map((service, index) => (
                                    <span key={index}>{service.name} </span>
                                ))}
                            </div>
                        ) : null}
                    </div>
                </div>
            </Card>

            <div className="pc-note">
                {t('图文详情、评价、咨询、领券、秒杀/拼团价格尚未迁移；当前为主图 + SKU + 加购的核心链路。')}
            </div>
        </div>
    );
}
