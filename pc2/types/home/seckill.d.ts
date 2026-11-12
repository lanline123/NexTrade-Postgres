// 获取列表返回参数类型
export interface SeckillFilterState {
    productId: number;
    seckillLimitNum: number;
    seckillStock: number;
    seckillSales: number;
    productName: string;
    picThumb: string;
    productSn: string;
    seoLink?: string;
    skuSn: string;
    seckillPrice: string;
    shopId: number;
    picLarge: string;
    picUrl: string;
    // 二期会员标签（D1096）：false 时前台展示秒杀价但置灰「抢购」
    canJoin?: boolean;
    restrictTip?: string;
}

export interface SeckillFilterResult {
    records: SeckillFilterState[];
    total: number;
}
