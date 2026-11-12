/** 装修商品列表 / 商品卡片（commonApi.getProductsList 等与前台展示共用形态） */

export interface PcDecorateProductSkuNode {
    skuId?: string | number;
    id?: string | number;
    skuData?: unknown[];
}

/** 装修场景商品行（字段随接口扩展，一律可选） */
export interface PcDecorateProductRow {
    productId?: string | number;
    productSn?: string;
    seoLink?: string;
    productType?: number;
    productSku?: PcDecorateProductSkuNode[];
    productStock?: number;
    isSeckill?: number;
    seckillStock?: number;
    picUrl?: string;
    /** 列表缩略图（装修/列表接口可能返回） */
    picThumb?: string;
    productName?: string;
    productBrief?: string;
    brief?: string;
    productPrice?: string | number;
    /** 当前售价（装修组件展示） */
    price?: string | number;
    marketPrice?: string | number;
    id?: string | number;
    /** 虚拟销量 */
    virtualSales?: number;
    /** 店铺信息（非自营商品） */
    shop?: { shopTitle?: string };
}
