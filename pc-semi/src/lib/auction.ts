/**
 * 拍卖（我方新增功能，一期）前台共享类型与客户端接口。
 * 契约来源：后端 crates/core/src/models/marketing/auction.rs 的 VO/DTO（camelCase）。
 * 金额字段后端 Decimal 序列化为字符串（如 "31500.00"），统一以 string 承接、展示时再格式化。
 */
import { apiGet, apiPost } from '@/lib/client-api';

export type Money = string | number;

export interface AuctionSession {
  sessionId: number;
  sessionName: string;
  coverPic: string;
  startTime: number;
  endTime: number;
  startTimeText: string;
  endTimeText: string;
  sort: number;
  status: number;
  itemCount: number;
}

export interface AuctionItem {
  itemId: number;
  sessionId: number | null;
  productId: number;
  skuId: number;
  shopId: number;
  title: string;
  productName: string;
  productSn: string;
  picUrl: string;
  picThumb: string;
  startPrice: Money;
  /** 展示价：有出价为当前价，否则起拍价 */
  currentPrice: Money;
  hasBid: boolean;
  buyoutPrice: Money | null;
  hasReserve: boolean;
  depositAmount: Money;
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
  finalPrice: Money | null;
  winnerMaskedName: string | null;
  orderId: number | null;
  serverTime: number;
}

export interface AuctionBid {
  bidId: number;
  itemId: number;
  maskedName: string;
  bidPrice: Money;
  isBuyout: boolean;
  isLeader: boolean;
  isMine: boolean;
  addTime: number;
  addTimeText: string;
}

export interface AuctionItemDetail extends AuctionItem {
  productBrief: string;
  productDesc: string | null;
  productPrice: Money;
  marketPrice: Money;
  stepAmount: Money;
  minNextBid: Money;
  /** 0 未冻结，其余同保证金状态：1冻结中 2已解冻 3已抵扣 4已扣罚 5部分扣罚后退回 */
  myDepositStatus: number;
  myDepositAmount: Money;
  myHighestBid: Money;
  isLeader: boolean;
  isWinner: boolean;
  reminded: boolean;
  banReason: string | null;
  requireCertification: boolean;
  myCertified: boolean;
  bids: AuctionBid[];
}

export interface AuctionPoll {
  itemId: number;
  status: number;
  statusName: string;
  currentPrice: Money;
  hasBid: boolean;
  bidCount: number;
  realEndTime: number;
  remainSeconds: number;
  extendCount: number;
  isLeader: boolean;
  minNextBid: Money;
  stepAmount: Money;
  serverTime: number;
  latestBids: AuctionBid[];
}

export interface AuctionDeposit {
  depositId: number;
  itemId: number;
  itemTitle: string;
  userId: number;
  username: string;
  nickname: string;
  amount: Money;
  status: number;
  statusName: string;
  balanceLogId: number | null;
  orderId: number | null;
  penaltyAmount: Money;
  addTime: number;
  addTimeText: string;
  handleTime: number;
  handleTimeText: string;
  remark: string;
}

export interface AuctionMyJoin extends AuctionItem {
  myHighestBid: Money;
  /** leading 领先 / outbid 被超价 / won 已中标 / lost 未中标 / quit 已退出 */
  myStatus: string;
  myStatusName: string;
  myDepositStatus: number;
  payDeadline: number | null;
}

export interface BidResult {
  itemId: number;
  currentPrice: Money;
  isLeader: boolean;
  isBuyout: boolean;
  status: number;
  realEndTime: number;
  remainSeconds: number;
  orderId: number | null;
}

export interface PageResult<T> {
  records: T[];
  total: number;
  page?: number;
  pageSize?: number;
}

export const AUCTION_ITEM_STATUS = {
  DRAFT: 0,
  PREVIEW: 1,
  RUNNING: 2,
  SOLD: 3,
  UNSOLD: 4,
  CANCELED: 5,
  FORCE_ENDED: 6,
} as const;

export function moneyNum(v: Money | null | undefined): number {
  const n = typeof v === 'number' ? v : Number(v ?? 0);
  return Number.isFinite(n) ? n : 0;
}

/** 秒数 → HH:MM:SS（超过 24h 显示 X天X小时） */
export function formatCountdown(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const pad = (n: number) => String(n).padStart(2, '0');
  if (d > 0) return `${d}天 ${pad(h)}:${pad(m)}:${pad(sec)}`;
  return `${pad(h)}:${pad(m)}:${pad(sec)}`;
}

/** 以服务端 remainSeconds 为锚的剩余秒数（避免本地时钟偏差） */
export function anchoredRemain(remainSeconds: number, anchoredAtMs: number, nowMs: number): number {
  return Math.max(0, remainSeconds - Math.floor((nowMs - anchoredAtMs) / 1000));
}

// ———————————————————— 客户端接口（带登录态） ————————————————————

export function fetchAuctionDetail(id: number): Promise<AuctionItemDetail> {
  return apiGet<AuctionItemDetail>('promotion/auction/detail', { id });
}

export function fetchAuctionPoll(id: number): Promise<AuctionPoll> {
  return apiGet<AuctionPoll>('promotion/auction/poll', { id });
}

export function fetchAuctionBids(id: number, page: number, pageSize = 20): Promise<PageResult<AuctionBid>> {
  return apiGet<PageResult<AuctionBid>>('promotion/auction/bids', { id, page, pageSize });
}

export function freezeAuctionDeposit(itemId: number): Promise<AuctionDeposit> {
  return apiPost<AuctionDeposit>('promotion/auction/freeze', { itemId });
}

export function placeAuctionBid(itemId: number, bidPrice: number, opts?: { isBuyout?: boolean; confirmHigh?: boolean }): Promise<BidResult> {
  return apiPost<BidResult>('promotion/auction/bid', {
    itemId,
    bidPrice,
    isBuyout: opts?.isBuyout ? 1 : 0,
    confirmHigh: opts?.confirmHigh ? 1 : 0,
  });
}

export function quitAuction(itemId: number): Promise<unknown> {
  return apiPost('promotion/auction/quit', { itemId });
}

export function remindAuction(itemId: number, on: boolean): Promise<unknown> {
  return apiPost('promotion/auction/remind', { itemId, on });
}

export function fetchMyAuctionJoins(): Promise<AuctionMyJoin[]> {
  return apiGet<AuctionMyJoin[]>('promotion/auction/my/joins');
}

export function fetchMyAuctionDeposits(page: number, pageSize = 20): Promise<PageResult<AuctionDeposit>> {
  return apiGet<PageResult<AuctionDeposit>>('promotion/auction/my/deposits', { page, pageSize });
}
