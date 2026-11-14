/**
 * 我的服务 预览（类名对齐渲染端 dec-my-service__*；发票/分销/实名等条目在渲染端按版本与配置过滤，预览全量展示）
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { myServiceDefaults } from "./index";
import "./Preview.css";

export default function MyServicePreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, myServiceDefaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const layoutStyle = Number(module.layoutStyle ?? 1);
    const serviceList: any[] = module.serviceList || [];
    return (
        <div className="module-ad-con" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className="dec-my-service">
                    <div className="dec-my-service__container" style={{ padding: module.frame?.innerPadding ?? 15 }}>
                        <div className="dec-my-service__title">{module.moduleTitle || "我的服务"}</div>
                        <div className={`dec-my-service__list${layoutStyle === 2 ? " is-grid" : ""}${layoutStyle === 1 ? " is-column" : ""}`}>
                            {serviceList.map((item, index) => (
                                <div className="dec-my-service__item" key={index}>
                                    <div className="dec-my-service__icon" style={{ borderRadius: `${module.iconRadius ?? 0}%`, overflow: "hidden" }}>
                                        {item.picUrl ? <img src={imageFormat(item.picUrl)} alt="" /> : <span className="dec-my-service__icon-ph">{(item.picTitle || "服").slice(0, 1)}</span>}
                                    </div>
                                    <div className="dec-my-service__label">
                                        <span>{item.picTitle}</span>
                                        {layoutStyle === 1 ? <span className="dec-my-service__arrow">›</span> : null}
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
