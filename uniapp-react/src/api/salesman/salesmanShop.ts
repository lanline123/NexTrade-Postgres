import request from "@/utils/request";

/**
 * 分销员外壳店「店中店」（我方新增，论坛帖 257876041323848173）。
 * 只是呈现 + 归因层：商品/下单/售后仍走主商城既有链，订单归因与佣金
 * 走既有分销绑定/生成链，本组接口只读写外壳呈现配置。
 */

export interface ShellShopDetail {
    salesmanId: number;
    /** 展示店名（自定义优先，未自定义回退分销员昵称） */
    shopName: string;
    banner: string;
    intro: string;
    avatar: string;
    nickname: string;
    /** 平台展示开关：false 时前台只出关闭提示且不写归因 */
    enabled: boolean;
    customized: boolean;
}

export interface ShellShopMine {
    salesmanId: number;
    /** 原始自定义值（未自定义为空串） */
    shopName: string;
    banner: string;
    intro: string;
    enabled: boolean;
    customized: boolean;
}

/** 响应包（与全仓 request<T extends ResponseData> 惯例一致：T 为整包，返回 T["data"]） */
interface ShellShopDetailResponse {
    data: ShellShopDetail;
    code: number;
    message: string;
}

interface ShellShopMineResponse {
    data: ShellShopMine;
    code: number;
    message: string;
}

/**
 * @description 买家侧外壳店详情（公开，含游客）
 * @param salesmanId 分销员 ID（既有公开标识，与分享链/名片同款）
 */
export const getShellShopDetail = (salesmanId: number) => {
    return request<ShellShopDetailResponse>({
        url: "salesman/shop/detail",
        method: "get",
        params: { salesmanId }
    });
};

/**
 * @description 分销员自助视图（需登录；非分销员返回业务错误）
 */
export const getMyShellShop = () => {
    return request<ShellShopMineResponse>({
        url: "salesman/shop/myShop",
        method: "get"
    });
};

/**
 * @description 分销员保存店名/店招/简介（需登录）
 */
export const saveMyShellShop = (data: { shopName: string; banner: string; intro: string }) => {
    return request<ShellShopMineResponse>({
        url: "salesman/shop/save",
        method: "post",
        data
    });
};
