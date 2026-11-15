/**
 * 账号 预览（移植自 view/admin editor/modules/mobileMember/account/Preview.vue）
 *
 * 标题 moduleTitle（默认「账号」）16px/600；accountList 纵向列表（isShow===false 过滤），
 * 项 padding 16px 0、底边 #f5f5f5（末项无），左图标 20×20 contain + 标题 15px，
 * 右 picDesc 14px + 箭头（原 iconfont-h5 icon-a-gengduo20 10px，以内联 SVG 承接）。
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { accountDefaults } from "./index";
import "./Preview.css";

function ArrowIcon() {
    return (
        <svg viewBox="0 0 24 24" width="10" height="10" aria-hidden="true" style={{ flexShrink: 0 }}>
            <polyline points="9 4.5 16.5 12 9 19.5" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

export default function AccountPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, accountDefaults);
        return merged;
    }, [data]);

    const frame = module.frame ?? {};
    const frameFormat = formatFrame(frame);
    const list = (module.accountList ?? []).filter((item: any) => item.isShow !== false);

    return (
        <div className="module-ad-con module-member-account" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content">
                <div
                    className="account-wrap"
                    style={{ backgroundColor: frame.backgroundColor || "#ffffff", borderRadius: `${frame.boxRadius ?? 0}px` }}
                >
                    <div className="account-container" style={{ padding: `${frame.innerPadding ?? 15}px` }}>
                        <div className="account-header">
                            <div className="account-title">{module.moduleTitle || "账号"}</div>
                        </div>
                        <div className="account-list">
                            {list.map((item: any, idx: number) => (
                                <div className="account-item" key={idx}>
                                    <div className="account-item-left">
                                        <div className="account-item-icon">{item.picUrl ? <img src={item.picUrl} alt={item.picTitle} /> : null}</div>
                                        <div className="account-item-title">{item.picTitle}</div>
                                    </div>
                                    <div className="account-item-right">
                                        {item.picDesc ? <span className="account-item-subtitle">{item.picDesc}</span> : null}
                                        <ArrowIcon />
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
