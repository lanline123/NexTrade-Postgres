/**
 * 广告图 预览（移植自 view/admin editor/modules/mobileMember/adImage/Preview.vue）
 *
 * 单张 img width 100% block；无图时空态 .module-ad-empty（「点击添加图片」）；
 * wrap 背景/圆角取 frame（bg 空时 transparent），container padding 取 frame.innerPadding（默认 0）。
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { adImageDefaults } from "./index";
import "./Preview.css";

export default function AdImagePreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, adImageDefaults);
        return merged;
    }, [data]);

    const frame = module.frame ?? {};
    const frameFormat = formatFrame(frame);

    return (
        <div className="module-ad-con module-member-ad-image" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content">
                <div
                    className="ad-image-wrap"
                    style={{ backgroundColor: frame.backgroundColor || "transparent", borderRadius: `${frame.boxRadius ?? 0}px` }}
                >
                    <div className="ad-image-container" style={{ padding: `${frame.innerPadding ?? 0}px` }}>
                        {!module.pic?.picUrl ? (
                            <div className="module-ad-empty">
                                <div className="image-empty-bg" />
                                <div className="desc">点击添加图片</div>
                            </div>
                        ) : (
                            <img className="ad-image-pic" src={module.pic.picUrl} alt="广告图" />
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}
