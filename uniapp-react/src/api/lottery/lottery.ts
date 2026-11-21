import request from "@/utils/request";
import type { ApiEnvelope } from "@/types/api";
import type {
    LotteryInfoResponse,
    DrawResultResponse,
    PrizeRecordPageResponse,
    GetRecordsParams,
    ConfirmReceiveParams,
    LotteryWinnersResponse,
    ShareRewardParams,
    ShareRewardResult
} from "@/types/lottery/lottery";

/**
 * 获取抽奖活动信息
 * @returns 抽奖活动信息
 */
export const getLotteryInfo = () => {
    return request<LotteryInfoResponse>({
        url: "promotion/lottery/info",
        method: "get"
    });
};

/**
 * 执行抽奖
 * @returns 抽奖结果
 */
export const drawLottery = () => {
    return request<DrawResultResponse>({
        url: "promotion/lottery/draw",
        method: "post"
    });
};

/**
 * 获取中奖记录
 * @param params 分页参数
 * @returns 中奖记录列表
 */
export const getMyRecords = (params: GetRecordsParams) => {
    return request<PrizeRecordPageResponse>({
        url: "promotion/lottery/myRecords",
        method: "get",
        params
    });
};

/**
 * 确认领取商品奖品
 * @param data 领取参数
 * @returns 领取结果
 */
export const confirmReceive = (data: ConfirmReceiveParams) => {
    return request({
        url: "promotion/lottery/confirmReceive",
        method: "post",
        data
    });
};

/**
 * 分享得次数上报（我方新增）：分享动作完成点调用，服务端按防刷口径入账
 * @param data 分享事件标识 + 渠道
 * @returns 入账结果与最新剩余次数
 */
export const shareRewardLottery = (data: ShareRewardParams) => {
    return request<ApiEnvelope<ShareRewardResult>>({
        url: "promotion/lottery/shareReward",
        method: "post",
        data
    });
};

/**
 * 获取中奖名单
 * @param activityId 活动ID（可选，不传则查询当前进行中的活动）
 * @returns 中奖名单列表
 */
export const getWinners = (activityId?: number) => {
    return request<LotteryWinnersResponse>({
        url: "promotion/lottery/winners",
        method: "get",
        params: activityId ? { activityId } : {}
    });
};
