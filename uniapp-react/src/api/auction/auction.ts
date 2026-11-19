import request from "@/utils/request";
import type {
    AuctionBidItem,
    AuctionBidResult,
    AuctionDepositItem,
    AuctionItem,
    AuctionItemDetail,
    AuctionMyJoinItem,
    AuctionPageResult,
    AuctionPollResult,
    AuctionSessionItem
} from "@/types/auction/auction";

/** 拍卖场次列表（公开） */
export const getAuctionSessions = () => {
    return request<AuctionSessionItem[]>({
        url: "promotion/auction/sessions",
        method: "get"
    });
};

/** 拍卖拍品列表（公开）：tab=all/preview/running/finished */
export const getAuctionItems = (params: {
    tab?: string;
    sessionId?: number;
    sort?: string;
    page?: number;
    pageSize?: number;
}) => {
    return request<AuctionPageResult<AuctionItem>>({
        url: "promotion/auction/items",
        method: "get",
        params
    });
};

/** 拍品详情（带登录态时含我的状态） */
export const getAuctionDetail = (id: number) => {
    return request<AuctionItemDetail>({
        url: "promotion/auction/detail",
        method: "get",
        params: { id }
    });
};

/** 拍品轮询（竞价中 5 秒节拍） */
export const getAuctionPoll = (id: number) => {
    return request<AuctionPollResult>({
        url: "promotion/auction/poll",
        method: "get",
        params: { id }
    });
};

/** 出价记录分页 */
export const getAuctionBids = (id: number, page: number, pageSize = 20) => {
    return request<AuctionPageResult<AuctionBidItem>>({
        url: "promotion/auction/bids",
        method: "get",
        params: { id, page, pageSize }
    });
};

/** 冻结参拍保证金（需登录） */
export const freezeAuctionDeposit = (itemId: number) => {
    return request<AuctionDepositItem>({
        url: "promotion/auction/freeze",
        method: "post",
        data: { itemId }
    });
};

/** 出价（需登录且已冻结保证金） */
export const placeAuctionBid = (itemId: number, bidPrice: number, opts?: { isBuyout?: boolean; confirmHigh?: boolean }) => {
    return request<AuctionBidResult>({
        url: "promotion/auction/bid",
        method: "post",
        data: {
            itemId,
            bidPrice,
            isBuyout: opts?.isBuyout ? 1 : 0,
            confirmHigh: opts?.confirmHigh ? 1 : 0
        }
    });
};

/** 退出参拍（需登录，未成为最高出价者时） */
export const quitAuction = (itemId: number) => {
    return request<unknown>({
        url: "promotion/auction/quit",
        method: "post",
        data: { itemId }
    });
};

/** 开拍提醒开关（需登录，预告拍品） */
export const remindAuction = (itemId: number, on: boolean) => {
    return request<unknown>({
        url: "promotion/auction/remind",
        method: "post",
        data: { itemId, on }
    });
};

/** 我的参拍（需登录） */
export const getMyAuctionJoins = () => {
    return request<AuctionMyJoinItem[]>({
        url: "promotion/auction/my/joins",
        method: "get"
    });
};

/** 我的保证金流水（需登录） */
export const getMyAuctionDeposits = (page: number, pageSize = 20) => {
    return request<AuctionPageResult<AuctionDepositItem>>({
        url: "promotion/auction/my/deposits",
        method: "get",
        params: { page, pageSize }
    });
};
