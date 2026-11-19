/**
 * 拍卖（我方新增功能，一期）类型：与后端 VO 对齐（camelCase）。
 * 金额为 Decimal 序列化字符串，展示时 Number() 转换。
 */
export type AuctionMoney = string | number;

export interface AuctionSessionItem {
    sessionId: number;
    sessionName: string;
    coverPic: string;
    startTime: number;
    endTime: number;
    startTimeText: string;
    endTimeText: string;
    status: number;
    itemCount: number;
}

export interface AuctionItem {
    itemId: number;
    sessionId: number | null;
    productId: number;
    shopId: number;
    title: string;
    productName: string;
    picUrl: string;
    picThumb: string;
    startPrice: AuctionMoney;
    currentPrice: AuctionMoney;
    hasBid: boolean;
    buyoutPrice: AuctionMoney | null;
    depositAmount: AuctionMoney;
    startTime: number;
    endTime: number;
    realEndTime: number;
    startTimeText: string;
    endTimeText: string;
    remainSeconds: number;
    extendCount: number;
    /** 0草稿 1预告中 2竞价中 3已成交 4流拍 5已撤拍 6强制结束 */
    status: number;
    statusName: string;
    bidCount: number;
    finalPrice: AuctionMoney | null;
    winnerMaskedName: string | null;
    orderId: number | null;
    serverTime: number;
}

export interface AuctionBidItem {
    bidId: number;
    itemId: number;
    maskedName: string;
    bidPrice: AuctionMoney;
    isBuyout: boolean;
    isLeader: boolean;
    isMine: boolean;
    addTime: number;
    addTimeText: string;
}

export interface AuctionItemDetail extends AuctionItem {
    productBrief: string;
    productDesc: string | null;
    productPrice: AuctionMoney;
    marketPrice: AuctionMoney;
    stepAmount: AuctionMoney;
    minNextBid: AuctionMoney;
    myDepositStatus: number;
    myDepositAmount: AuctionMoney;
    myHighestBid: AuctionMoney;
    isLeader: boolean;
    isWinner: boolean;
    reminded: boolean;
    banReason: string | null;
    requireCertification: boolean;
    myCertified: boolean;
    bids: AuctionBidItem[];
}

export interface AuctionPollResult {
    itemId: number;
    status: number;
    statusName: string;
    currentPrice: AuctionMoney;
    hasBid: boolean;
    bidCount: number;
    realEndTime: number;
    remainSeconds: number;
    extendCount: number;
    isLeader: boolean;
    minNextBid: AuctionMoney;
    stepAmount: AuctionMoney;
    serverTime: number;
    latestBids: AuctionBidItem[];
}

export interface AuctionDepositItem {
    depositId: number;
    itemId: number;
    itemTitle: string;
    amount: AuctionMoney;
    status: number;
    statusName: string;
    orderId: number | null;
    penaltyAmount: AuctionMoney;
    addTime: number;
    addTimeText: string;
    handleTime: number;
    handleTimeText: string;
    remark: string;
}

export interface AuctionMyJoinItem extends AuctionItem {
    myHighestBid: AuctionMoney;
    myStatus: string;
    myStatusName: string;
    myDepositStatus: number;
    payDeadline: number | null;
}

export interface AuctionPageResult<T> {
    records: T[];
    total: number;
    page?: number;
    pageSize?: number;
}

export interface AuctionBidResult {
    itemId: number;
    currentPrice: AuctionMoney;
    isLeader: boolean;
    isBuyout: boolean;
    status: number;
    realEndTime: number;
    remainSeconds: number;
    orderId: number | null;
}
