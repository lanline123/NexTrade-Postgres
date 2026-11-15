/**
 * 固定功能区 预览（移植自 view/admin editor/modules/mobileMember/fixedFunctionalArea/Preview.vue）
 *
 * 仅 B2B/门店版渲染账期卡（原版 isB2b()||isStore() 门控，我方 version.ts 同名函数）：
 * 卡 margin 10px 0、圆角 9、背景图 termPay_card.png cover/center；标题栏高 44、
 * 标题 13px/600 #ae5d34 + 眼睛图标 + 「查看详情」胶囊；内容三列 信用额度/已用额度/剩余额度（值 ******）。
 * frame 仅控外层 boxPadding（原版 wrap/container 不消费 frame 其它字段）。
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { isB2b, isStore } from "../../../contract/version";
import { fixedFunctionalAreaDefaults } from "./index";
import "./Preview.css";

const AMOUNTS = [
    { label: "信用额度", value: "******" },
    { label: "已用额度", value: "******" },
    { label: "剩余额度", value: "******" }
];

function EyeIcon() {
    return (
        <svg viewBox="0 0 24 24" width="15" height="15" aria-hidden="true" style={{ flexShrink: 0, color: "#ae5d34" }}>
            <path
                d="M12 5c5 0 9.3 3 11 7-1.7 4-6 7-11 7S2.7 16 1 12c1.7-4 6-7 11-7zm0 2C8.2 7 4.8 9 3.2 12 4.8 15 8.2 17 12 17s7.2-2 8.8-5C19.2 9 15.8 7 12 7zm0 1.5a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7zm0 2a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3z"
                fill="currentColor"
            />
        </svg>
    );
}

export default function FixedFunctionalAreaPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, fixedFunctionalAreaDefaults);
        return merged;
    }, [data]);

    const frame = module.frame ?? {};
    const frameFormat = formatFrame(frame);

    if (!isB2b() && !isStore()) return null;

    return (
        <div
            className="module-ad-con module-member-fixed-functional-area"
            style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}
        >
            <div className="module-ad-content">
                <div className="ffa-card">
                    <div className="ffa-card-header">
                        <div className="ffa-card-title">
                            <span className="ffa-card-title-text">账期支付</span>
                            <EyeIcon />
                        </div>
                        <div className="ffa-card-detail">
                            <span>查看详情</span>
                        </div>
                    </div>
                    <div className="ffa-card-content">
                        {AMOUNTS.map((item) => (
                            <div className="ffa-amount-item" key={item.label}>
                                <div className="ffa-amount-value">{item.value}</div>
                                <div className="ffa-amount-label">{item.label}</div>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
}
