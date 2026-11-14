/**
 * 会员广告图 预览（类名对齐渲染端 dec-ad-image__*）
 */
import { useMemo } from "react";
import { cssText, productImage } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { adImageDefaults } from "./index";

export default function AdImagePreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, adImageDefaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const picUrl = productImage(module.pic);
    return (
        <div className="module-ad-con" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className="dec-ad-image">
                    <div className="dec-ad-image__wrapper">
                        <div className="dec-ad-image__img">
                            {picUrl ? <img src={picUrl} alt="" style={{ width: "100%", display: "block", borderRadius: module.frame?.boxRadius ?? 12 }} /> : <div className="module-ad-empty empty-image_ad"><div className="desc">点击添加广告图</div></div>}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
