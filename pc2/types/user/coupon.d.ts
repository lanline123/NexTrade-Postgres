export interface CouponFilterParams {
    page: number;
    size?: number;
    isShow: number;
    shopId: number;
}

// 获取列表返回参数类型
export interface CouponFilterState {
    couponId: number; // 优惠券id
    isReceive: number; //
    couponName: string; // 优惠券名称
    couponMoney: number; // 优惠券金额
    couponDiscount: number; // 优惠券折扣
    couponDesc: string; // 优惠券描述
    couponType: number; // 优惠券类型，1：满减券，2：折扣券
    sendRange: number//使用范围:0,全部商品;1,制定分类;2,指定品牌;3,指定商品;4,不包含指定商品
    sendRangeData: string; // [json]使用范围相关信息
    minOrderAmount: number; // 最少订单金额
    sendStartDate: number; // 优惠券可领取的开始时间
    sendEndDate: number; // 优惠券可领取的结束时间
    sendType: number; // 优惠券使用时间方式，0：按领取后多少天内可用，1：按固定时间范围可用
    delayDay: number; // 延迟天数
    useDay: number; // 优惠券多少天过期
    useStartDate: string; // 优惠券可以使用的开始时间
    useEndDate: string; // 优惠券可以使用的结束时间
    isShow: number; // 是否展示
    isGlobal: number//全场通用券跨店铺:0,否;1,是
    isNewUser: number; // 是否为新人专享优惠券
    enabledClickGet: number//商城点击领取:0,否;1,是
    limitUserRank: string; // [JSON]仅限会员等级
    shopId: number; // 店铺id
    receiveNum: number; //
    limitNum: number; //
}
export interface MyCouponFilterState {
    userId: number; // 用户id
    id: number; //
    couponSn: string; // 优惠券SN
    couponId: number; // 优惠券id
    status: number; // 优惠券id
    couponName: string; // 优惠券名称
    statusName: string; // 状态名称
    userTime: number; // 优惠券使用时间
    useDay: number; // 优惠券多少天过期
    delayDay: number; // 延期天数
    orderId: number; // 优惠券使用时间
    startDate: string; // 优惠券可领取的开始时间
    endDate: string; // 优惠券可领取的结束时间
    couponMoney: number; // 优惠券金额
    couponDiscount: number; // 优惠券折扣
    couponDesc: string; // 优惠券描述
    couponType: number; // 优惠券类型，1：满减券，2：折扣券
}

export interface CouponFilterResult {
    records: MyCouponFilterState[];
    total: number;
}
