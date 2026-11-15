/**
 * 我的订单 预览（移植自 view/admin editor/modules/mobileMember/orderEntry/Preview.vue）
 *
 * 样式一带标题头（title 16px/600 + 全部订单 11px #999）；入口 flex space-between，
 * 图标盒 28×28（样式一图 24×24、样式二 28×28），角标 absolute translate(90%,-50%)、
 * min-width 16/高 16/字 10px，演示值 99+/88/32/18（末项空不渲染，同原版 :empty 隐藏）；
 * 样式二在末项前插分割线图（scaleX(-1)）。角标色缺省按样式回退（1→#4d9be4，2→#ff4d4f）。
 */
import { Fragment, useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { orderEntryDefaults, orderEntryBadgeBg } from "./index";
import "./Preview.css";

const DEMO_BADGES = ["99+", "88", "32", "18", ""];

function ArrowIcon() {
    return (
        <svg viewBox="0 0 24 24" width="10" height="10" aria-hidden="true" style={{ flexShrink: 0 }}>
            <polyline points="9 4.5 16.5 12 9 19.5" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

export default function OrderEntryPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, orderEntryDefaults);
        return merged;
    }, [data]);

    const frame = module.frame ?? {};
    const frameFormat = formatFrame(frame);
    const layoutStyle = module.layoutStyle ?? 1;
    const list: any[] = module.entryList ?? [];
    const badgeStyle = {
        backgroundColor: module.badgeBgColor || orderEntryBadgeBg(layoutStyle),
        color: module.badgeTextColor || "#ffffff"
    };

    return (
        <div className="module-ad-con module-member-order-entry" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content">
                <div
                    className="order-entry-wrap"
                    style={{ backgroundColor: frame.backgroundColor || "#ffffff", borderRadius: `${frame.boxRadius ?? 0}px` }}
                >
                    <div className="order-entry-container" style={{ padding: `${frame.innerPadding ?? 16}px` }}>
                        {layoutStyle === 1 ? (
                            <div className="order-entry-header">
                                <div className="order-entry-title">{module.title || "我的订单"}</div>
                                <div className="order-entry-more">
                                    <span>全部订单</span>
                                    <ArrowIcon />
                                </div>
                            </div>
                        ) : null}
                        <div className={`order-entry-list${layoutStyle === 1 ? " layout-style1" : ""}`}>
                            {list.map((item, index) => (
                                <Fragment key={index}>
                                    {layoutStyle === 2 && index === list.length - 1 ? <div className="order-entry-divider" /> : null}
                                    <div className="order-entry-item">
                                        <div className="order-entry-icon-wrapper">
                                            {item.picUrl ? (
                                                <img className="order-entry-icon-img" src={item.picUrl} alt={item.picTitle} />
                                            ) : (
                                                <div className="order-entry-icon-empty" />
                                            )}
                                            {DEMO_BADGES[index] ? (
                                                <span className="order-entry-badge" style={badgeStyle}>
                                                    {DEMO_BADGES[index]}
                                                </span>
                                            ) : null}
                                        </div>
                                        <div className="order-entry-label">{item.picTitle}</div>
                                    </div>
                                </Fragment>
                            ))}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
