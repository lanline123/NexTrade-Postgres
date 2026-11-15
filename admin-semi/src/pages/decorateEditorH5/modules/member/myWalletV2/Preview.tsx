/**
 * 我的钱包2 预览（移植自 view/admin editor/modules/mobileMember/myWalletV2/Preview.vue）
 *
 * 2 列网格，cell padding 8；1/2 项 border-bottom #f2f3f5、奇数项 border-right #eeeeee；
 * 图标 20×20 contain、标签 12px #666、演示值右对齐 600（640826.82/328/1224/5699）、箭头 10px #999。
 * items 为空时回退内置默认四项（同原版 displayItems）。
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { myWalletV2Defaults, WALLET_V2_ICONS } from "./index";
import "./Preview.css";

const DEMO_VALUES: Record<string, string> = { balance: "640826.82", coupon: "328", collect: "1224", history: "5699" };

function ArrowIcon() {
    return (
        <svg viewBox="0 0 24 24" width="10" height="10" aria-hidden="true" style={{ flexShrink: 0, color: "#999", marginLeft: 3 }}>
            <polyline points="9 4.5 16.5 12 9 19.5" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

export default function MyWalletV2Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, myWalletV2Defaults);
        return merged;
    }, [data]);

    const frame = module.frame ?? {};
    const frameFormat = formatFrame(frame);
    const items: any[] = Array.isArray(module.items) && module.items.length ? module.items : myWalletV2Defaults.items;

    return (
        <div className="module-ad-con module-member-my-wallet-v2" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content">
                <div
                    className="my-wallet-v2-wrap"
                    style={{ backgroundColor: frame.backgroundColor || "#ffffff", borderRadius: `${frame.boxRadius ?? 0}px` }}
                >
                    <div className="my-wallet-v2-grid" style={{ padding: `${frame.innerPadding ?? 12}px` }}>
                        {items.map((item) => (
                            <div className="my-wallet-v2-cell" key={item.key}>
                                <img className="my-wallet-v2-icon" src={item.icon?.picUrl || WALLET_V2_ICONS[item.key] || ""} alt="" />
                                <span className="my-wallet-v2-label">{item.label}</span>
                                <span className="my-wallet-v2-value">{DEMO_VALUES[item.key] ?? ""}</span>
                                <ArrowIcon />
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
}
