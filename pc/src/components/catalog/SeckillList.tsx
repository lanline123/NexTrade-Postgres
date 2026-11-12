'use client';

import { useEffect, useState } from 'react';
import { Empty, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import { formatPrice, productImage } from '@/lib/format';

/** Vue 基准：pages/seckill/List.vue；接口 home/home/getSeckill。 */
interface Row {
    productId?: number;
    productName?: string;
    productSn?: string;
    picThumb?: string;
    seckillPrice?: number | string;
    productPrice?: number | string;
    stock?: number;
}

export default function SeckillList() {
    const { t } = useTranslation();
    const [rows, setRows] = useState<Row[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        apiGet<Record<string, unknown>>('home/home/getSeckill')
            .then((result) => {
                const list = (result.records ?? result.list ?? result.data ?? []) as Row[];
                setRows(Array.isArray(list) ? list : []);
            })
            .catch(() => setRows([]))
            .finally(() => setLoading(false));
    }, []);

    return (
        <div className="pc-catalog">
            <Typography.Title heading={3}>{t('秒杀')}</Typography.Title>
            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : rows.length === 0 ? (
                <Empty title={t('暂无秒杀商品')} />
            ) : (
                <div className="pc-product-grid">
                    {rows.map((row, index) => (
                        <a className="pc-product-card" key={row.productId ?? index} href={`/product/${row.productSn ?? row.productId ?? ''}`}>
                            <img src={productImage(row)} alt={row.productName ?? ''} />
                            <div className="pc-product-name">{row.productName}</div>
                            <div className="pc-product-price">{formatPrice(row.seckillPrice ?? row.productPrice)}</div>
                            {row.productPrice ? (
                                <div className="pc-muted pc-line-through">{formatPrice(row.productPrice)}</div>
                            ) : null}
                        </a>
                    ))}
                </div>
            )}
        </div>
    );
}
