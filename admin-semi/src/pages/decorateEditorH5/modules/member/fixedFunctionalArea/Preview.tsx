/**
 * 固定功能区 预览（类名对齐渲染端 dec-fixed-area__*；内容为账期卡占位，真实卡片由渲染端按授信状态决定是否显示）
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { fixedFunctionalAreaDefaults } from "./index";
import "./Preview.css";

export default function FixedFunctionalAreaPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, fixedFunctionalAreaDefaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    return (
        <div className="module-ad-con" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className="dec-fixed-area">
                    <div className="dec-fixed-area__container">
                        <div className="dec-fixed-area__demo-card">
                            <span className="dec-fixed-area__demo-title">账期支付</span>
                            <span className="dec-fixed-area__demo-desc">授信额度卡片（登录且有授信时由渲染端显示）</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
