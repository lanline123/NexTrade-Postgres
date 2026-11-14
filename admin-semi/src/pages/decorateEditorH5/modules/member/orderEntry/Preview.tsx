/**
 * 订单入口 预览（类名对齐渲染端 dec-order-entry__*；角标为 store 实时数，预览不展示）
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { orderEntryDefaults } from "./index";
import "./Preview.css";

export default function OrderEntryPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, orderEntryDefaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const layoutStyle = Number(module.layoutStyle ?? 1);
    const showHeader = layoutStyle === 2 ? false : Number(module.showTitle ?? 1) !== 0;
    const entryList: any[] = module.entryList || [];
    return (
        <div className="module-ad-con" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className="dec-order-entry">
                    <div className="dec-order-entry__container" style={{ padding: module.frame?.innerPadding ?? 16 }}>
                        {showHeader ? (
                            <div className="dec-order-entry__header">
                                <div className="dec-order-entry__title">{module.title || "我的订单"}</div>
                                {Number(module.showAllOrders ?? 1) !== 0 ? (
                                    <div className="dec-order-entry__more">
                                        <span>全部订单</span>
                                        <span className="dec-order-entry__arrow">›</span>
                                    </div>
                                ) : null}
                            </div>
                        ) : null}
                        <div className={`dec-order-entry__list${layoutStyle === 1 ? " dec-order-entry__list--style1" : ""}`}>
                            {entryList.map((item, index) => (
                                <div className="dec-order-entry__item" key={index}>
                                    <div className="dec-order-entry__icon-wrapper">
                                        {item.picUrl ? <img className="dec-order-entry__icon-img" src={imageFormat(item.picUrl)} alt="" /> : <div className="dec-order-entry__icon-default">{(item.picTitle || "订").slice(0, 1)}</div>}
                                    </div>
                                    <div className="dec-order-entry__label">{item.picTitle}</div>
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
