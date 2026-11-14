/**
 * 直播 预览（DOM 类名逐字对齐 uniapp-react 渲染端 live/index.tsx + index.scss）
 *
 * 直播间数据存模块 selectedLiveList（渲染端 normalize 后展示，截断 10 条）；
 * 未配置时预览用演示直播间兜底，字段与渲染端 LiveRoomState 一致：roomId、name、
 * anchorName、feedsImg、liveStatus（101 直播中 / 102 未开始）、productData。
 */
import { useMemo } from "react";
import { cssText, imageFormat, pickPic } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { liveDefaults } from "./index";
import "./Preview.css";

const DEMO_PIC = "https://oss.tigshop.com/img/gallery/202411/17319190474y0XWFxO8GJZF0dg64.jpg";

/** 演示直播间（仅预览兜底，不写入模块数据） */
const DEMO_LIVE_LIST = [
    {
        wechatLiveId: 1,
        roomId: 1,
        name: "演示直播间 · 好物推荐",
        anchorName: "官方主播",
        feedsImg: DEMO_PIC,
        coverImg: DEMO_PIC,
        liveStatus: 101,
        productData: [{ coverImg: DEMO_PIC }, { coverImg: DEMO_PIC }]
    },
    {
        wechatLiveId: 2,
        roomId: 2,
        name: "演示直播预告 · 新品首发",
        anchorName: "官方主播",
        feedsImg: DEMO_PIC,
        coverImg: DEMO_PIC,
        liveStatus: 102,
        productData: [{ coverImg: DEMO_PIC }]
    }
];

const LIVE_STATUS_TEXT: Record<number, string> = {
    101: "直播中",
    102: "未开始",
    103: "已结束",
    104: "禁播",
    105: "暂停",
    106: "异常",
    107: "已过期"
};

function statusClass(item: any): string {
    const status = Number(item.liveStatus);
    if (status === 101) return "live-card__status-tag--live";
    if (status === 102) return "live-card__status-tag--pending";
    return "live-card__status-tag--ended";
}

export default function LivePreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, liveDefaults);
        return merged;
    }, [data]);

    const frameFormat = formatFrame(module.frame ?? {});
    const cardStyle: React.CSSProperties = {
        borderRadius: module.frame?.boxRadius ?? 8,
        backgroundColor: module.frame?.backgroundColor || "#ffffff"
    };
    const isBigImageMode = (module.displayMode || "list") === "bigImage";
    const selected: any[] = Array.isArray(module.selectedLiveList) ? module.selectedLiveList : [];
    const liveList = (selected.length > 0 ? selected : DEMO_LIVE_LIST).slice(0, 10).map((item: any) => ({
        ...item,
        name: item.name || item.wechatLiveTitle || "",
        anchorName: item.anchorName || item.anchorOpenid || item.anchorWechat || "",
        liveStatusText: item.liveStatusText || LIVE_STATUS_TEXT[Number(item.liveStatus)] || ""
    }));
    const getCover = (item: any) => imageFormat(item.feedsImg || "");
    const getGoodsImages = (item: any) =>
        (item.productData || [])
            .map((goods: any) => imageFormat(goods.coverImg || pickPic(goods) || goods.productThumb || goods.productImage || goods.image || ""))
            .filter(Boolean)
            .slice(0, 3);

    return (
        <div className="module-ad-con module-live" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className="live-container" style={cssText(frameFormat.innerPadding)}>
                    {liveList.length > 0 ? (
                        isBigImageMode ? (
                            <div className="live-list live-list--bigImage">
                                {liveList.map((item: any) => (
                                    <div key={item.wechatLiveId || item.roomId} className="live-card live-card--bigImage" style={cardStyle}>
                                        <div className="live-card__cover live-card__cover--full" style={{ borderRadius: module.frame?.boxRadius ?? 8 }}>
                                            {getCover(item) ? (
                                                <img className="live-card__cover-image" src={getCover(item)} alt="" />
                                            ) : (
                                                <div className="live-card__cover-placeholder">
                                                    <span className="live-card__cover-placeholder-text">直播预告</span>
                                                </div>
                                            )}
                                            <div className={"live-card__status-tag " + statusClass(item)}>{item.liveStatusText || "直播中"}</div>
                                            <div className="live-card__mask">
                                                <div className="live-card__mask-title">{item.name}</div>
                                                <div className="live-card__mask-meta">
                                                    <span>主播：{item.anchorName}</span>
                                                    <div className="live-card__action">进入直播间</div>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        ) : (
                            <div className="live-list live-list--list">
                                {liveList.map((item: any) => (
                                    <div key={item.wechatLiveId || item.roomId} className="live-card live-card--list" style={cardStyle}>
                                        <div className="live-card__cover live-card__cover--list" style={{ borderRadius: module.frame?.boxRadius ?? 8 }}>
                                            {getCover(item) ? (
                                                <img className="live-card__cover-image" src={getCover(item)} alt="" />
                                            ) : (
                                                <div className="live-card__cover-placeholder">
                                                    <span className="live-card__cover-placeholder-text">直播预告</span>
                                                </div>
                                            )}
                                            <div className="live-card__badge">小程序直播</div>
                                            <div className="live-card__status">{item.liveStatusText || "直播中"}</div>
                                        </div>
                                        <div className="live-card__content">
                                            <div className="live-card__title">{item.name}</div>
                                            <div className="live-card__goods">
                                                {getGoodsImages(item).map((goodsImg: string, goodsIndex: number) => (
                                                    <img key={`${item.roomId}-${goodsIndex}`} className="live-card__goods-item" src={goodsImg} alt="" />
                                                ))}
                                            </div>
                                            <div className="live-card__footer">
                                                <div className="live-card__meta">主播：{item.anchorName || "官方直播间"}</div>
                                                <div className="live-card__action">进入直播间</div>
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        )
                    ) : (
                        <div className="live-empty" style={{ borderRadius: module.frame?.boxRadius ?? 8 }}>
                            <div className="live-empty__title">暂无可展示直播间</div>
                            <div className="live-empty__desc">请在后台先配置小程序直播内容</div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
