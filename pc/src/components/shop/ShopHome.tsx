'use client';

import { useEffect, useState } from 'react';
import { Empty, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import ProductResults from '@/components/catalog/ProductResults';

/**
 * 店铺主页。
 * Vue 基准：pages/shop/[id].vue + src/{ShopRefHeader,ShopDefaultPage,ShopInfo,ShopList}.vue。
 * 已实现：店铺信息（shop/shop/decorate）+ 店内商品（复用商品网格）。
 * 未迁：店铺装修模块（轮播/推荐/优惠券/分类导航）——装修模块体系单独一轮。
 */
interface ShopInfo {
    shopId?: number;
    shopName?: string;
    shopTitle?: string;
    shopLogo?: string;
    logo?: string;
    shopDesc?: string;
}

export default function ShopHome({ shopId }: { shopId: number }) {
    const { t } = useTranslation();
    const [info, setInfo] = useState<ShopInfo | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        if (!shopId) {
            setLoading(false);
            return;
        }
        apiGet<Record<string, unknown>>('shop/shop/decorate', { id: shopId, shopId })
            .then((result) => {
                const data = (result.shopInfo ?? result.shop ?? result) as ShopInfo;
                setInfo(data ?? null);
            })
            .catch(() => setInfo(null))
            .finally(() => setLoading(false));
    }, [shopId]);

    if (!shopId) return <Empty title={t('缺少店铺参数')} />;
    if (loading) return <div className="pc-loading">{t('加载中')}</div>;

    return (
        <div className="pc-catalog">
            <div className="pc-shop-head">
                <img src={info?.shopLogo ?? info?.logo ?? '/assets/avatar/default.gif'} alt={info?.shopName ?? ''} />
                <div>
                    <Typography.Title heading={3}>{info?.shopName ?? info?.shopTitle ?? t('店铺')}</Typography.Title>
                    {info?.shopDesc ? <div className="pc-muted">{info.shopDesc}</div> : null}
                </div>
            </div>

            <Typography.Title heading={4}>{t('店内商品')}</Typography.Title>
            <ProductResults shopId={shopId} emptyText={t('该店铺暂无商品')} />
        </div>
    );
}
