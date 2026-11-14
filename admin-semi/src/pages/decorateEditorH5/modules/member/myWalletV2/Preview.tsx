/**
 * 我的钱包 V2 预览（类名对齐渲染端 dec-wallet-v2__*；数值为演示值）
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { myWalletV2Defaults } from "./index";
import "./Preview.css";

const DEMO_VALUE: Record<string, string> = { balance: "0.00", coupon: "0", collect: "0", history: "" };

export default function MyWalletV2Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, myWalletV2Defaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const items: any[] = module.items?.length ? module.items : myWalletV2Defaults.items;
    return (
        <div className="module-ad-con" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className="dec-wallet-v2">
                    <div className="dec-wallet-v2__container" style={{ padding: module.frame?.innerPadding ?? 12 }}>
                        <div className="dec-wallet-v2__grid">
                            {items.map((item, index) => (
                                <div className={`dec-wallet-v2__cell${index < 2 ? " dec-wallet-v2__cell--border-bottom" : ""}${index % 2 === 0 ? " dec-wallet-v2__cell--border-right" : ""}`} key={item.key || index}>
                                    {item.icon?.picUrl ? (
                                        <div className="dec-wallet-v2__icon">
                                            <img src={imageFormat(item.icon.picUrl)} alt="" />
                                        </div>
                                    ) : null}
                                    <span className="dec-wallet-v2__label">{item.label}</span>
                                    <span className="dec-wallet-v2__value">{DEMO_VALUE[item.key] ?? ""}</span>
                                    <span className="dec-wallet-v2__arrow">›</span>
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
