'use client';

import { useEffect, useState } from 'react';
import { apiGet } from '@/lib/client-api';
import { useTranslation } from 'react-i18next';

/**
 * 商品详情公共数据层：三套 UI（old 经典版 / new 新版 / overseas 跨境版）共用一份取数。
 * Vue 基准：pages/item/src/Product.vue 的 script 部分（三条入口路由只换模板）。
 *
 * 接口：GET product/product/detail?sn=&grouponPromotionRecordId=
 * 返回（实测 2026-11-13）：
 * - item：商品主体（含 activityInfo、presaleInfo、paidContent、limitNumber…）
 * - skuList：[{ skuId, skuPrice, skuStock, skuData, skuSn, marketPrice… }]
 * - picList：图集；videoList：[{ videoUrl, videoCover }]
 * - attrList：{ normal: SKU 选择组, spe: 规格参数表 }（组内 [{ attrName, attrValue }]）
 * - descArr：图文详情 [{ type:'pic'|'text'|'video', pic, html, videoUrl }]
 * - serviceList：服务承诺 [{ productServiceName, productServiceDesc, icoImg }]
 * - grouponDetail / seckillDetail：活动信息（当前库为 null，按「有则展示」处理）
 */

export interface ProductSku {
    skuId?: number;
    id?: number;
    skuName?: string;
    skuSn?: string;
    skuData?: unknown;
    productPrice?: number | string;
    skuPrice?: number | string;
    price?: number | string;
    stock?: number;
    skuStock?: number;
    marketPrice?: number | string;
}

export interface ProductActivity {
    promotionId?: number;
    promotionName?: string;
    type?: number;
    typeText?: string;
}

export interface ProductItem {
    productId?: number;
    categoryId?: number;
    productName?: string;
    productSn?: string;
    productBrief?: string;
    productPrice?: number | string;
    marketPrice?: number | string;
    picThumb?: string;
    picUrl?: string;
    stock?: number;
    productStock?: number;
    unit?: string;
    activityInfo?: ProductActivity[];
    presaleInfo?: { presalePrice?: string | number; presaleStartTime?: number } | null;
    limitNumber?: number;
}

export interface ProductAttrGroup {
    attrName?: string;
    attrList?: Array<{ attrName?: string; attrValue?: string; attrId?: number; attributesId?: number }>;
}

export interface ProductDetailData {
    item?: ProductItem;
    skuList?: ProductSku[];
    picList?: Array<{ picUrl?: string; picThumb?: string; picOriginal?: string }>;
    videoList?: Array<{ videoUrl?: string; videoCover?: string }>;
    attrList?: { normal?: ProductAttrGroup[]; spe?: ProductAttrGroup[] };
    descArr?: Array<{ type?: string; pic?: string; html?: string; videoUrl?: string; cover?: string }>;
    serviceList?: Array<{ productServiceName?: string; productServiceDesc?: string; icoImg?: string }>;
    grouponDetail?: {
        productId?: number;
        minGrouponPrice?: string | number;
        minGroupNum?: number;
        joinNumCount?: number;
        endTime?: string | number;
        grouponPromotionItemId?: number;
        canJoin?: boolean;
        restrictTip?: string;
    } | null;
    seckillDetail?: { seckillPrice?: string | number; seckillStock?: number } | null;
}

export type ProductUiVersion = 'old' | 'new' | 'overseas';

/** 后台「商品页风格」：1=经典版(old) 2=新版(new) 3=跨境版(overseas)；拿不到按 3（与当前部署一致）。 */
export async function fetchProductDetailStyle(): Promise<ProductUiVersion> {
    const style = await apiGet<number>('common/config/productDetailStyle');
    if (Number(style) === 1) return 'old';
    if (Number(style) === 2) return 'new';
    return 'overseas';
}

export function useProductDetail(idOrSn: string, grouponRecordId?: number) {
    const { t } = useTranslation();
    const [detail, setDetail] = useState<ProductDetailData | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');
    const [skuId, setSkuId] = useState(0);

    useEffect(() => {
        if (!idOrSn) {
            setError(t('缺少商品参数'));
            setLoading(false);
            return;
        }
        const numeric = Number(idOrSn);
        const params =
            Number.isFinite(numeric) && numeric > 0
                ? { id: numeric, grouponPromotionRecordId: grouponRecordId || undefined }
                : { sn: idOrSn, grouponPromotionRecordId: grouponRecordId || undefined };
        apiGet<ProductDetailData>('product/product/detail', params)
            .then((result) => {
                setDetail(result);
                const firstSku = result.skuList?.[0];
                setSkuId(firstSku?.skuId ?? firstSku?.id ?? 0);
            })
            .catch((e: Error) => setError(e.message))
            .finally(() => setLoading(false));
    }, [idOrSn, t, grouponRecordId]);

    const skus = detail?.skuList ?? [];
    const activeSku = skus.find((sku) => (sku.skuId ?? sku.id) === skuId);
    const price =
        activeSku?.productPrice ?? activeSku?.skuPrice ?? activeSku?.price ?? detail?.item?.productPrice;
    const stock = activeSku?.stock ?? activeSku?.skuStock ?? detail?.item?.stock ?? detail?.item?.productStock;

    return { detail, loading, error, skuId, setSkuId, skus, activeSku, price, stock };
}
