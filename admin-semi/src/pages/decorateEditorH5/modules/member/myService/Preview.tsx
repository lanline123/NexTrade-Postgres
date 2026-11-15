/**
 * 我的服务 预览（移植自 view/admin editor/modules/mobileMember/myService/Preview.vue）
 *
 * 标题 moduleTitle（默认「服务管理」）15px/600 #333；serviceList 空时空态「点击添加服务项」；
 * 样式一 列表版（项 padding 12px 0、图标 20×20 圆角 iconRadius%、文字 14px、右箭头）；
 * 样式二 九宫格 4 列（图标 40×40、文字 12px）。
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { myServiceDefaults } from "./index";
import "./Preview.css";

function ArrowIcon() {
    return (
        <svg viewBox="0 0 24 24" width="10" height="10" aria-hidden="true" style={{ flexShrink: 0, color: "#888b94" }}>
            <polyline points="9 4.5 16.5 12 9 19.5" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

export default function MyServicePreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, myServiceDefaults);
        return merged;
    }, [data]);

    const frame = module.frame ?? {};
    const frameFormat = formatFrame(frame);
    const list: any[] = module.serviceList ?? [];
    const iconRadius = module.iconRadius ?? 50;

    return (
        <div className="module-ad-con module-member-my-service" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content">
                <div
                    className="my-service-wrap"
                    style={{ backgroundColor: frame.backgroundColor || "#ffffff", borderRadius: `${frame.boxRadius ?? 0}px` }}
                >
                    <div className="my-service-container" style={{ padding: `${frame.innerPadding ?? 15}px` }}>
                        <div className="my-service-header">
                            <div className="my-service-title">{module.moduleTitle || "服务管理"}</div>
                        </div>
                        {!list.length ? (
                            <div className="my-service-empty">
                                <div className="my-service-empty-text">点击添加服务项</div>
                            </div>
                        ) : (module.layoutStyle ?? 1) === 1 ? (
                            <div className="my-service-list">
                                {list.map((item, index) => (
                                    <div className="my-service-list-item" key={index}>
                                        <div className="my-service-list-left">
                                            <div className="my-service-list-icon" style={{ borderRadius: `${iconRadius}%` }}>
                                                {item.picUrl ? <img src={item.picUrl} alt={item.picTitle} /> : null}
                                            </div>
                                            <div className="my-service-list-text">{item.picTitle}</div>
                                        </div>
                                        <ArrowIcon />
                                    </div>
                                ))}
                            </div>
                        ) : (
                            <div className="my-service-grid">
                                {list.map((item, index) => (
                                    <div className="my-service-grid-item" key={index}>
                                        <div className="my-service-grid-icon" style={{ borderRadius: `${iconRadius}%` }}>
                                            {item.picUrl ? <img src={item.picUrl} alt={item.picTitle} /> : null}
                                        </div>
                                        <div className="my-service-grid-text">{item.picTitle}</div>
                                    </div>
                                ))}
                            </div>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}
