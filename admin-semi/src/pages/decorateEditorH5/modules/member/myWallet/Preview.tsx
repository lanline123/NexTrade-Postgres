/**
 * 我的钱包 预览（移植自 view/admin editor/modules/mobileMember/myWallet/Preview.vue）
 *
 * 样式一：标题「我的钱包」15px/600 + 右「进入钱包」13px #999（箭头内联 SVG）；三列演示数据
 * 999.99/99/9999999（值 18px/500）。样式二：钱包图标 24×22 + 标签，分割线图 10×54，
 * 三列演示数据 640826.82/328/24086062（值 16px/600）。frame 驱动 wrap 背景/圆角与 container padding 16。
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { myWalletDefaults } from "./index";
import "./Preview.css";

const DEFAULT_WALLET_ICON = "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779873859zO64yVrNvp2y0lUsOz.png";

function ArrowIcon() {
    return (
        <svg viewBox="0 0 24 24" width="10" height="10" aria-hidden="true" style={{ flexShrink: 0 }}>
            <polyline points="9 4.5 16.5 12 9 19.5" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

const STYLE1_STATS = [
    { value: "999.99", label: "余额" },
    { value: "99", label: "优惠券" },
    { value: "9999999", label: "积分" }
];
const STYLE2_STATS = [
    { value: "640826.82", label: "余额" },
    { value: "328", label: "优惠券" },
    { value: "24086062", label: "积分" }
];

export default function MyWalletPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, myWalletDefaults);
        return merged;
    }, [data]);

    const frame = module.frame ?? {};
    const frameFormat = formatFrame(frame);
    const layoutStyle = module.layoutStyle ?? 1;
    const wrapStyle = { backgroundColor: frame.backgroundColor || "#ffffff", borderRadius: `${frame.boxRadius ?? 0}px` };
    const containerStyle = { padding: `${frame.innerPadding ?? 16}px` };
    const walletIconUrl = module.walletIcon?.picUrl || DEFAULT_WALLET_ICON;

    return (
        <div className="module-ad-con module-member-my-wallet" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content">
                {layoutStyle === 1 ? (
                    <div className="wallet-style1-wrap" style={wrapStyle}>
                        <div className="wallet-style1-container" style={containerStyle}>
                            <div className="wallet-style1-header">
                                <div className="wallet-style1-title">我的钱包</div>
                                <div className="wallet-style1-link">
                                    <span>进入钱包</span>
                                    <ArrowIcon />
                                </div>
                            </div>
                            <div className="wallet-style1-stats">
                                {STYLE1_STATS.map((s) => (
                                    <div className="wallet-style1-stat" key={s.label}>
                                        <div className="wallet-style1-value">{s.value}</div>
                                        <div className="wallet-style1-label">{s.label}</div>
                                    </div>
                                ))}
                            </div>
                        </div>
                    </div>
                ) : (
                    <div className="my-wallet-wrap" style={wrapStyle}>
                        <div className="my-wallet-container" style={containerStyle}>
                            <div className="my-wallet-icon-section">
                                <img className="my-wallet-icon" src={walletIconUrl} alt="钱包" />
                                <div className="my-wallet-icon-label">我的钱包</div>
                            </div>
                            <div className="my-wallet-divider" />
                            <div className="my-wallet-stats">
                                {STYLE2_STATS.map((s) => (
                                    <div className="my-wallet-stat" key={s.label}>
                                        <div className="my-wallet-value">{s.value}</div>
                                        <div className="my-wallet-label">{s.label}</div>
                                    </div>
                                ))}
                            </div>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}
