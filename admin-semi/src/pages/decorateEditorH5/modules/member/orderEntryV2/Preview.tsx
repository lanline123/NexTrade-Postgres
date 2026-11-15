/**
 * 我的订单2 预览（移植自 view/admin editor/modules/mobileMember/orderEntryV2/Preview.vue）
 *
 * 头部 title 16px/600（默认「订单」）+ 查看更多 13px #999；卡片 flex gap 10，padding 25px 12px、
 * 圆角 8、背景 cardBackgroundColor，图标 32×32，角标 top -10/right -12、演示值 183/5；
 * 列表行 padding 14px 0、border-top #f0f0f0（首行无），左图标 20×20 + 文字 14px，右箭头。
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { orderEntryV2Defaults } from "./index";
import "./Preview.css";

const DEMO_CARD_BADGES = ["183", "5", ""];

function ArrowIcon() {
    return (
        <svg viewBox="0 0 24 24" width="10" height="10" aria-hidden="true" style={{ flexShrink: 0, color: "#888b94" }}>
            <polyline points="9 4.5 16.5 12 9 19.5" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

export default function OrderEntryV2Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, orderEntryV2Defaults);
        return merged;
    }, [data]);

    const frame = module.frame ?? {};
    const frameFormat = formatFrame(frame);
    const cards: any[] = module.cardList ?? [];
    const listItems: any[] = module.listItems ?? [];
    const badgeStyle = { backgroundColor: module.badgeBgColor || "#ff4d4f", color: module.badgeTextColor || "#ffffff" };

    return (
        <div className="module-ad-con module-member-order-entry-v2" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content">
                <div
                    className="order-entry-v2-wrap"
                    style={{ backgroundColor: frame.backgroundColor || "#ffffff", borderRadius: `${frame.boxRadius ?? 0}px` }}
                >
                    <div className="order-entry-v2-container" style={{ padding: `${frame.innerPadding ?? 16}px` }}>
                        <div className="order-entry-v2-header">
                            <div className="order-entry-v2-title">{module.title || "订单"}</div>
                            <div className="order-entry-v2-more">
                                <span>查看更多</span>
                                <ArrowIcon />
                            </div>
                        </div>
                        <div className="order-entry-v2-cards">
                            {cards.map((item, index) => (
                                <div className="order-card" key={index} style={{ backgroundColor: module.cardBackgroundColor || "#f6f7fb" }}>
                                    <div className="order-card-icon">
                                        {item.picUrl ? <img src={item.picUrl} alt={item.picTitle} /> : null}
                                        {DEMO_CARD_BADGES[index] ? (
                                            <span className="order-card-badge" style={badgeStyle}>
                                                {DEMO_CARD_BADGES[index]}
                                            </span>
                                        ) : null}
                                    </div>
                                    <div className="order-card-label">{item.picTitle}</div>
                                </div>
                            ))}
                        </div>
                        <div className="order-entry-v2-list">
                            {listItems.map((item, index) => (
                                <div className="order-entry-v2-list-item" key={index}>
                                    <div className="order-entry-v2-list-left">
                                        {item.picUrl ? <img className="order-entry-v2-list-icon-img" src={item.picUrl} alt={item.picTitle} /> : null}
                                        <span className="order-entry-v2-list-label">{item.picTitle}</span>
                                    </div>
                                    <ArrowIcon />
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
