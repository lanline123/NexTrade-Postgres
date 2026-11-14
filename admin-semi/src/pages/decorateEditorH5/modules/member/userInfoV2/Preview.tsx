/**
 * 会员信息 V2 预览（类名对齐渲染端 dec-user-info-v2__*）
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { userInfoV2Defaults } from "./index";
import "./Preview.css";

export default function UserInfoV2Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, userInfoV2Defaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    return (
        <div className="module-ad-con" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className="dec-user-info-v2">
                    <div className="dec-user-info-v2__container" style={{ padding: module.frame?.innerPadding ?? 16 }}>
                        <div className="dec-user-info-v2__profile">
                            <div className="dec-user-info-v2__avatar">
                                <div className="dec-user-info-v2__avatar-img" />
                            </div>
                            <div className="dec-user-info-v2__info">
                                <div className="dec-user-info-v2__name">{module.nickname}</div>
                                <div className="dec-user-info-v2__desc">{module.description}</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
