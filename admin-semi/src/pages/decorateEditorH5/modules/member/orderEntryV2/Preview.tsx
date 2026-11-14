/**
 * 订单入口 V2 预览（类名对齐渲染端 dec-order-entry-v2__*）
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { orderEntryV2Defaults } from "./index";
import "./Preview.css";

export default function OrderEntryV2Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, orderEntryV2Defaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const cardList: any[] = module.cardList || [];
    const listItems: any[] = module.listItems || [];
    return (
        <div className="module-ad-con" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className="dec-order-entry-v2">
                    <div className="dec-order-entry-v2__container" style={{ padding: module.frame?.innerPadding ?? 16 }}>
                        {Number(module.showTitle ?? 1) !== 0 ? (
                            <div className="dec-order-entry-v2__header">
                                <div className="dec-order-entry-v2__title">{module.title || "订单"}</div>
                                {Number(module.showMore ?? 1) !== 0 ? (
                                    <div className="dec-order-entry-v2__more">
                                        <span>查看更多</span>
                                        <span className="dec-order-entry-v2__arrow">›</span>
                                    </div>
                                ) : null}
                            </div>
                        ) : null}
                        {cardList.length ? (
                            <div className="dec-order-entry-v2__cards" style={{ background: module.cardBackgroundColor || "#f6f7fb" }}>
                                {cardList.map((item, index) => (
                                    <div className="dec-order-entry-v2__card" key={index}>
                                        <div className="dec-order-entry-v2__card-icon">
                                            {item.picUrl ? <img className="dec-order-entry-v2__icon-img" src={imageFormat(item.picUrl)} alt="" /> : <div className="dec-order-entry-v2__icon-default">{(item.picTitle || "订").slice(0, 1)}</div>}
                                        </div>
                                        <div className="dec-order-entry-v2__card-label">{item.picTitle}</div>
                                    </div>
                                ))}
                            </div>
                        ) : null}
                        {listItems.length ? (
                            <div className="dec-order-entry-v2__list">
                                {listItems.map((item, index) => (
                                    <div className="dec-order-entry-v2__list-item" key={index}>
                                        <div className="dec-order-entry-v2__list-left">
                                            <div className="dec-order-entry-v2__list-icon">
                                                {item.picUrl ? <img className="dec-order-entry-v2__list-icon-img" src={imageFormat(item.picUrl)} alt="" /> : <div className="dec-order-entry-v2__icon-default">{(item.picTitle || "订").slice(0, 1)}</div>}
                                            </div>
                                            <span className="dec-order-entry-v2__list-label">{item.picTitle}</span>
                                        </div>
                                        <span className="dec-order-entry-v2__list-arrow">›</span>
                                    </div>
                                ))}
                            </div>
                        ) : null}
                    </div>
                </div>
            </div>
        </div>
    );
}
