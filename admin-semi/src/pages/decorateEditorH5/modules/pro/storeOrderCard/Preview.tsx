/**
 * 店铺订单卡 预览（渲染端暂无入口，此预览按订单状态卡通用结构还原，供存量数据在编辑器内查看）
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { storeOrderCardDefaults } from "./index";
import "./Preview.css";

export default function StoreOrderCardPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, storeOrderCardDefaults);
        return merged;
    }, [data]);
    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {});
    const items: any[] = (module.items || []).slice(0, Number(module.showCount) || 4);
    return (
        <div className="module-ad-con" style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content" style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}>
                <div className="store-order-card" style={{ backgroundColor: module.backgroundColor, borderRadius: `${module.cardRadius ?? 8}px` }}>
                    <div className="store-order-card-head">
                        <span className="store-order-card-title" style={{ color: module.titleColor }}>{module.title}</span>
                        <span className="store-order-card-more">全部订单 &gt;</span>
                    </div>
                    <div className="store-order-card-list">
                        {items.map((item, i) => (
                            <div className="store-order-card-item" key={i}>
                                <span className="store-order-card-icon">{item.icon || "📦"}</span>
                                {Number(item.count) > 0 ? <span className="store-order-card-badge" style={{ backgroundColor: module.badgeColor }}>{item.count}</span> : null}
                                <span className="store-order-card-text" style={{ color: module.itemColor }}>{item.title}</span>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
}
