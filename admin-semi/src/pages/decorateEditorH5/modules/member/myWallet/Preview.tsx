/**
 * 我的钱包 预览（类名对齐渲染端 dec-my-wallet--style{n}；金额为演示值）
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { myWalletDefaults } from "./index";
import "./Preview.css";

export default function MyWalletPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, myWalletDefaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const layoutStyle = Number(module.layoutStyle ?? 1);
    const stats = [
        { label: "余额", value: "0.00" },
        { label: "优惠券", value: "0" },
        { label: "积分", value: "0" }
    ];
    return (
        <div className="module-ad-con" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className={`dec-my-wallet dec-my-wallet--style${layoutStyle}`}>
                    <div className="dec-my-wallet__container" style={{ padding: module.frame?.innerPadding ?? 16 }}>
                        {layoutStyle === 1 ? (
                            <>
                                <div className="dec-my-wallet__header">
                                    <div className="dec-my-wallet__title">我的钱包</div>
                                    <div className="dec-my-wallet__link">
                                        <span>进入钱包</span>
                                        <span className="dec-my-wallet__arrow">›</span>
                                    </div>
                                </div>
                                <div className="dec-my-wallet__stats dec-my-wallet__stats--block">
                                    {stats.map((s) => (
                                        <div className="dec-my-wallet__stat-item" key={s.label}>
                                            <div className="dec-my-wallet__stat-value">{s.value}</div>
                                            <div className="dec-my-wallet__stat-label">{s.label}</div>
                                        </div>
                                    ))}
                                </div>
                            </>
                        ) : (
                            <div className="dec-my-wallet__style2">
                                <div className="dec-my-wallet__icon-section">
                                    <div className="dec-my-wallet__icon">
                                        {module.walletIcon?.picUrl ? <img src={imageFormat(module.walletIcon.picUrl)} alt="" /> : <span>👛</span>}
                                    </div>
                                    <span className="dec-my-wallet__icon-label">我的钱包</span>
                                </div>
                                <div className="dec-my-wallet__divider" />
                                <div className="dec-my-wallet__stats">
                                    {stats.map((s) => (
                                        <div className="dec-my-wallet__stat" key={s.label}>
                                            <span className="dec-my-wallet__value">{s.value}</span>
                                            <span className="dec-my-wallet__label">{s.label}</span>
                                        </div>
                                    ))}
                                </div>
                            </div>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}
