/**
 * 视频号 预览（移植自 view/admin editor/modules/mobilePromotion/videoNumber/Preview.vue）
 *
 * 双模式：video（1:1 封面卡 + 底部渐变 mask：视频号图标+昵称、♥ 点赞 158）/
 * live（92×92 封面 + 标题（颜色按 titleColorType：default #2A2A2A / theme #155BD4 / custom titleColor）+
 * 商品图（productImages 优先，pickPic 取图 ≤3；为空回退账号 productImage）+ 作者行 + 「立即预约」CTA）。
 * 账号数据取既有接口 /content/wechatVideoAccount/list（后台视频号管理同源，size 100，不改路径），
 * currentVideoNumber = videoNumberId 命中项或首项。原版默认封面为打包资源图、本机无此资源：
 * 无封面时以浅灰占位块承接；作者小图标以同尺寸内联 SVG 承接（原版为 assets/live/videoNumber.svg）。
 * 原版 Preview 打开时会把首个 videoNumberId/liveTitle 静默写回模块——本实现不写回（见摘要）。
 */
import { useEffect, useMemo, useState } from "react";
import { apiGet } from "../../../../../api";
import { cssText, pickPic } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { videoNumberDefaults } from "./index";
import "./Preview.css";

const DEFAULT_TITLE_COLOR = "#2A2A2A";
const THEME_TITLE_COLOR = "#155BD4";
const DEFAULT_LIKES = 158;

function VideoAccountIcon({ size = 16 }: { size?: number }) {
    return (
        <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true" style={{ flexShrink: 0 }}>
            <rect x="1.5" y="4" width="21" height="16" rx="4" fill="#2BA640" />
            <path d="M10 9.2v5.6c0 .8.9 1.3 1.6.9l4.4-2.8c.6-.4.6-1.3 0-1.7L11.6 8.4c-.7-.5-1.6 0-1.6.8z" fill="#fff" />
        </svg>
    );
}

function CalendarIcon() {
    return (
        <svg viewBox="0 0 24 24" width="11" height="11" aria-hidden="true" style={{ flexShrink: 0 }}>
            <path
                d="M7 2v2H5a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-2V2h-2v2H9V2H7zm12 17H5V9h14v10z"
                fill="currentColor"
            />
        </svg>
    );
}

export default function VideoNumberPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, videoNumberDefaults);
        return merged;
    }, [data]);

    const [options, setOptions] = useState<any[]>([]);
    useEffect(() => {
        let alive = true;
        apiGet<any>("/content/wechatVideoAccount/list", { page: 1, size: 100, keyword: "" })
            .then((res: any) => {
                if (!alive) return;
                const rows = res?.records || res?.list || [];
                setOptions(
                    rows.map((item: any) => ({
                        videoNumberId: item.finderId,
                        nickname: item.nickName || "视频号名称",
                        cover: item.cover,
                        productImage: item.productImage || "",
                        likes: DEFAULT_LIKES,
                        liveTitle: ""
                    }))
                );
            })
            .catch(() => undefined);
        return () => {
            alive = false;
        };
    }, []);

    const current = options.find((o) => o.videoNumberId === module.videoNumberId) || options[0];
    const frame = module.frame ?? {};
    const frameFormat = formatFrame(frame);
    const radius = Number(frame.boxRadius || 0);
    const liveTitleColor = module.titleColorType === "theme" ? THEME_TITLE_COLOR : module.titleColorType === "custom" ? module.titleColor || DEFAULT_TITLE_COLOR : DEFAULT_TITLE_COLOR;
    const productImages = (module.productImages || []).map((item: any) => pickPic(item)).filter(Boolean).slice(0, 3);
    const displayProductImages: string[] = productImages.length ? productImages : current?.productImage ? [current.productImage] : [];

    return (
        <div className="module-ad-con module-video-number module-promotion-video-number" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className="video-number-container" style={cssText(frameFormat.innerPadding, frameFormat.boxRadius)}>
                    {module.contentType === "video" ? (
                        <div className="video-card video-card--video" style={{ borderRadius: radius }}>
                            {current?.cover ? (
                                <img className="video-card__cover" style={{ borderRadius: radius }} src={current.cover} alt="视频号封面" />
                            ) : (
                                <div className="video-card__cover video-card__cover--empty" style={{ borderRadius: radius }} />
                            )}
                            <div className="video-card__mask">
                                <div className="video-card__author">
                                    <VideoAccountIcon size={16} />
                                    <span>{current?.nickname || "视频号名称"}</span>
                                </div>
                                <div className="video-card__likes">♥ {current?.likes || DEFAULT_LIKES}</div>
                            </div>
                        </div>
                    ) : (
                        <div className="video-card video-card--live" style={{ borderRadius: radius, backgroundColor: frame.backgroundColor || "#ffffff" }}>
                            <div className="coverBox" style={{ borderRadius: radius }}>
                                {current?.cover ? (
                                    <img className="video-card__cover" style={{ borderRadius: radius }} src={current.cover} alt="视频号封面" />
                                ) : (
                                    <div className="video-card__cover video-card__cover--empty" style={{ borderRadius: radius }} />
                                )}
                            </div>
                            <div className="video-card__live-content">
                                <div className="video-card__live-title" style={{ color: liveTitleColor }}>
                                    {module.liveTitle || current?.liveTitle || ""}
                                </div>
                                {displayProductImages.length ? (
                                    <div className="video-card__live-main">
                                        {displayProductImages.map((img, index) => (
                                            <img key={`${img}-${index}`} className="video-card__product" src={img} alt="商品图片" />
                                        ))}
                                    </div>
                                ) : null}
                                <div className="video-card__live-footer">
                                    <div className="video-card__author video-card__author--light">
                                        <VideoAccountIcon size={12} />
                                        <span>{current?.nickname || "视频号名称"}</span>
                                        <div className="video-card__cta">
                                            <CalendarIcon /> 立即预约
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
