/**
 * 视频号 预览（DOM 类名逐字对齐 uniapp-react 渲染端 videoNumber/index.tsx + index.scss）
 *
 * 渲染端用微信专属 channel-video / channel-live 组件，编辑器无法内嵌，
 * 按渲染端的占位/卡片结构还原：video 模式显示 video-placeholder（未配置时）或
 * 已配置提示；live 模式显示 live-cover + live-content（标题色规则与渲染端一致：
 * theme=var(--general)、custom=titleColor、default=#2A2A2A）。
 */
import { useMemo } from "react";
import { cssText, imageFormat, pickPic } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { videoNumberDefaults } from "./index";
import "./Preview.css";

export default function VideoNumberPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, videoNumberDefaults);
        return merged;
    }, [data]);

    const frameFormat = formatFrame(module.frame ?? {});
    const cardRadiusStyle = `border-radius:${module.frame?.boxRadius ?? 0}px;`;
    const isVideoType = module.contentType !== "live";
    const finderUserName = String(module.videoNumberId || "").trim();
    const videoFeedId = String(module.videoId || "").trim();
    const hasVideoInfo = Boolean(finderUserName && videoFeedId);
    const liveTitle = String(module.liveTitle || "").trim() || "这场直播特别有料";
    const liveTitleColor = module.titleColorType === "theme" ? "var(--general)" : module.titleColorType === "custom" && module.titleColor ? module.titleColor : "#2A2A2A";
    const productImageList = (module.productImages || [])
        .map((item: any) => pickPic(item))
        .filter(Boolean)
        .slice(0, 3)
        .map((item: string) => imageFormat(item));

    return (
        <div className="module-ad-con module-video-number" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className="video-number-container" style={cssText(frameFormat.innerPadding)}>
                    {isVideoType ? (
                        <div className="video-card video-card--video" style={cssText(cardRadiusStyle)}>
                            <div className="video-placeholder" style={cssText(cardRadiusStyle)}>
                                <div className="video-placeholder__tag">视频号</div>
                                {hasVideoInfo ? (
                                    <>
                                        <div className="video-placeholder__title">视频号视频（小程序端播放）</div>
                                        <div className="video-placeholder__desc">
                                            {finderUserName} · {videoFeedId}
                                        </div>
                                    </>
                                ) : (
                                    <>
                                        <div className="video-placeholder__title">请在后台配置视频号视频</div>
                                        <div className="video-placeholder__desc">需填写视频号ID与视频ID</div>
                                    </>
                                )}
                            </div>
                        </div>
                    ) : (
                        <div className="video-card video-card--live" style={cssText(cardRadiusStyle)}>
                            <div className="live-cover" style={cssText(cardRadiusStyle)}>
                                <span className="live-cover__badge">视频号直播</span>
                                <span className="live-cover__name">{finderUserName || "视频号"}</span>
                            </div>
                            <div className="live-content">
                                <div className="live-title line2" style={{ color: liveTitleColor }}>
                                    {liveTitle}
                                </div>
                                <div className="product-list">
                                    {productImageList.length > 0
                                        ? productImageList.map((item: string, productIndex: number) => (
                                              <img key={`${item}-${productIndex}`} className="product-item" src={item} alt="" />
                                          ))
                                        : [1, 2, 3].map((item) => <div key={item} className="product-item product-item--placeholder" />)}
                                </div>
                                <div className="live-footer">
                                    <div className="live-author line1">{finderUserName}</div>
                                    <div className="live-button">去观看</div>
                                </div>
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
