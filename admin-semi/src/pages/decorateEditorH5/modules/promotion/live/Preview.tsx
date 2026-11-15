/**
 * 小程序直播 预览（移植自 view/admin editor/modules/mobilePromotion/live/Preview.vue）
 *
 * 双模式：list（92×92 封面 + 徽章「小程序直播」+ 状态 + 标题 + 商品图 44×44×3 + 主播/进入直播间）
 * 与 bigImage（5:4 大封面 + 状态标签 + 底部白色 mask 标题/主播）。空列表显示「暂无可展示直播间」。
 * 数据：auto 时取既有接口 /promotion/wechatLive/list（后台微信直播管理同源，keyword 空、size 10，
 * 过滤 liveStatus!==4——同原版 getWechatLiveRoomList 语义映射，不改接口路径）；manual 时用 selectedLiveList。
 * 列表按 autoLiveNum（钳制 1..10）/ manual 上限 10 截断；每房间商品图取 /content/wechatLiveGoods/list 的 coverImg（≤3）。
 * 封面字段 feedsImg 优先、回退 coverImg（我方列表接口字段）。原版引用的 defaultGoodsImages 在快照中未定义，
 * 本实现以空数组承接（无商品图时不渲染）。卡片样式：padding=frame.innerPadding，大图模式另叠 frame 圆角/背景。
 */
import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { apiGet } from "../../../../../api";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { liveDefaults } from "./index";
import "./Preview.css";

const STATUS_TEXT: Record<number, string> = { 101: "直播中", 102: "未开始", 103: "已结束", 104: "禁播", 105: "暂停", 106: "异常", 107: "已过期" };

const statusText = (item: any) => item.liveStatusText || STATUS_TEXT[Number(item.liveStatus)] || "未知";
const statusClass = (item: any) => {
    const s = Number(item.liveStatus);
    if (s === 101) return "live-card__status-tag--live";
    if (s === 102) return "live-card__status-tag--pending";
    return "live-card__status-tag--ended";
};
const coverOf = (item: any) => item.feedsImg || item.coverImg || "";

export default function LivePreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, liveDefaults);
        return merged;
    }, [data]);

    const [autoList, setAutoList] = useState<any[]>([]);
    const [goodsMap, setGoodsMap] = useState<Record<number, string[]>>({});

    const autoLiveNum = Math.min(Math.max(Number(module.autoLiveNum || 10), 1), 10);
    const liveList: any[] = useMemo(() => {
        const source = module.sourceType === "manual" ? module.selectedLiveList || [] : autoList;
        return source.slice(0, module.sourceType === "manual" ? 10 : autoLiveNum);
    }, [module.sourceType, module.selectedLiveList, autoList, autoLiveNum]);

    useEffect(() => {
        if (module.sourceType !== "auto") return;
        let alive = true;
        apiGet<any>("/promotion/wechatLive/list", { keyword: "", page: 1, size: 10 })
            .then((res: any) => {
                if (!alive) return;
                const rows = res?.records || res?.list || [];
                setAutoList(rows.filter((item: any) => Number(item.liveStatus) !== 4).slice(0, 10));
            })
            .catch(() => {
                if (alive) setAutoList([]);
            });
        return () => {
            alive = false;
        };
    }, [module.sourceType]);

    useEffect(() => {
        let alive = true;
        const roomIds = liveList.map((item) => Number(item.roomId)).filter((id) => id > 0);
        if (!roomIds.length) {
            setGoodsMap({});
            return;
        }
        Promise.all(
            roomIds.map(async (roomId) => {
                try {
                    const res: any = await apiGet("/content/wechatLiveGoods/list", { roomId, page: 1, size: 3 });
                    const rows = res?.records || res?.list || (Array.isArray(res) ? res : []);
                    const imgs = rows.map((g: any) => g.coverImg || "").filter(Boolean).slice(0, 3);
                    return [roomId, imgs] as const;
                } catch {
                    return [roomId, [] as string[]] as const;
                }
            })
        ).then((entries) => {
            if (alive) setGoodsMap(Object.fromEntries(entries));
        });
        return () => {
            alive = false;
        };
    }, [liveList]);

    const frame = module.frame ?? {};
    const frameFormat = formatFrame(frame);
    const big = module.displayMode === "bigImage";
    const itemStyle: CSSProperties = {
        padding: frame.innerPadding ?? 12,
        ...(big ? { borderRadius: frame.boxRadius ?? 8 } : {}),
        backgroundColor: frame.backgroundColor || undefined
    };

    return (
        <div className="module-ad-con module-live module-promotion-live" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={{ borderRadius: frame.boxRadius ?? 8, overflow: "hidden" }}>
                <div className="live-container">
                    {liveList.length ? (
                        big ? (
                            <div className="live-list live-list--bigImage">
                                {liveList.map((item, idx) => (
                                    <div className="live-card live-card--bigImage" style={itemStyle} key={item.wechatLiveId || item.roomId || idx}>
                                        <div className="live-card__cover live-card__cover--full" style={coverOf(item) ? { background: "none" } : undefined}>
                                            {coverOf(item) ? <img src={coverOf(item)} alt="直播封面" /> : null}
                                            <div className={`live-card__status-tag ${statusClass(item)}`}>{statusText(item)}</div>
                                            <div className="live-card__mask">
                                                <div className="live-card__mask-title">{item.wechatLiveTitle || "-"}</div>
                                                <div className="live-card__mask-meta">
                                                    <span>主播：{item.anchorName || "-"}</span>
                                                    <span className="live-card__action">进入直播间</span>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        ) : (
                            <div className="live-list live-list--list">
                                {liveList.map((item, idx) => (
                                    <div className="live-card live-card--list" style={itemStyle} key={item.wechatLiveId || item.roomId || idx}>
                                        <div className="live-card__cover live-card__cover--large" style={coverOf(item) ? { background: "none" } : undefined}>
                                            {coverOf(item) ? <img src={coverOf(item)} alt="直播封面" style={{ height: "100%" }} /> : null}
                                            <div className="live-card__badge">小程序直播</div>
                                            <div className="live-card__status">{item.liveStatusText || "-"}</div>
                                        </div>
                                        <div className="live-card__content">
                                            <div className="live-card__title">{item.wechatLiveTitle || "-"}</div>
                                            <div className="live-card__goods">
                                                {(goodsMap[Number(item.roomId)] || []).map((img, gi) => (
                                                    <img key={gi} className="live-card__goods-item" src={img} alt="商品图片" />
                                                ))}
                                            </div>
                                            <div className="live-card__footer">
                                                <div className="live-card__meta">主播：{item.anchorName || "-"}</div>
                                                <div className="live-card__action">进入直播间</div>
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        )
                    ) : (
                        <div className="live-empty">暂无可展示直播间</div>
                    )}
                </div>
            </div>
        </div>
    );
}
