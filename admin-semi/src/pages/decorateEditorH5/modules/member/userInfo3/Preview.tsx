/**
 * 会员信息3 预览（类名对齐渲染端 dec-user-info--style{n}；四种布局共用头部结构，统计数为演示值）
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { userInfo3Defaults } from "./index";
import "./Preview.css";

const GRADIENT_DIR: Record<string, string> = { upDown: "to bottom", leftRight: "to right", diagonal: "135deg" };
const STYLE_GRADIENT: Record<number, { colorA: string; colorB: string }> = {
    1: { colorA: "#57D45C", colorB: "#34C759" },
    2: { colorA: "#E0FCED", colorB: "#FDF1F3" },
    3: { colorA: "#F1EFE9", colorB: "#E7F0E6" },
    4: { colorA: "#FBD9E1", colorB: "#FDEDEF" }
};

export default function UserInfo3Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, userInfo3Defaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const layoutStyle = Number(module.layoutStyle ?? 1);
    const g = module.gradient ?? {};
    const d = STYLE_GRADIENT[layoutStyle] || STYLE_GRADIENT[1];
    const colorA = module.bgColor || g.colorA || d.colorA;
    const colorB = module.bgColor || g.colorB || d.colorB;
    const dir = GRADIENT_DIR[g.gradientType || "leftRight"] || "to right";
    const bgStyle: React.CSSProperties =
        Number(module.backgroundImageShow ?? 1) !== 0 && module.backgroundImage?.picUrl
            ? { backgroundImage: `url(${imageFormat(module.backgroundImage.picUrl)}), linear-gradient(${dir}, ${colorA}, ${colorB})`, backgroundSize: "cover", backgroundPosition: "center" }
            : { background: `linear-gradient(${dir}, ${colorA}, ${colorB})` };
    const tc = module.textColors ?? {};
    const statsItems: any[] = (module.stats?.items || []).filter((it: any) => Number(it.show ?? 1) !== 0);
    const bar = module.memberBar ?? {};
    return (
        <div className="module-ad-con" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className={`dec-user-info dec-user-info--style${layoutStyle}`} style={bgStyle}>
                    <div className="dec-user-info3__container" style={{ padding: module.frame?.innerPadding ?? 16 }}>
                        <div className="dec-user-info__header">
                            <div className="dec-user-info__profile">
                                <div className="dec-user-info__avatar">
                                    <div className="dec-user-info__avatar-img" />
                                </div>
                                <div className="dec-user-info__info">
                                    <div className="dec-user-info__name-row">
                                        <span className="dec-user-info__name" style={{ color: tc.nameColor || undefined }}>{module.nickname}</span>
                                        {Number(module.showEditIcon ?? 1) !== 0 ? <span className="dec-user-info3__edit">✎</span> : null}
                                    </div>
                                    <div className="dec-user-info__level">
                                        <span className="dec-user-info__level-text" style={{ color: tc.subColor || undefined }}>{module.description}</span>
                                    </div>
                                </div>
                            </div>
                            <div className="dec-user-info__shortcuts">
                                {Number(module.settings?.show ?? 1) !== 0 ? (
                                    <div className="dec-user-info__shortcut-item">
                                        {module.settings?.icon?.picUrl ? <img className="dec-user-info__shortcut-icon" src={imageFormat(module.settings.icon.picUrl)} alt="" /> : <div className="dec-user-info__shortcut-icon dec-user-info__shortcut-ph">⚙</div>}
                                    </div>
                                ) : null}
                                {Number(module.notification?.show ?? 1) !== 0 ? (
                                    <div className="dec-user-info__shortcut-item">
                                        {module.notification?.icon?.picUrl ? <img className="dec-user-info__shortcut-icon" src={imageFormat(module.notification.icon.picUrl)} alt="" /> : <div className="dec-user-info__shortcut-icon dec-user-info__shortcut-ph">🔔</div>}
                                    </div>
                                ) : null}
                            </div>
                        </div>
                        {Number(module.stats?.show ?? 1) !== 0 ? (
                            <div className="dec-user-info__stats">
                                {statsItems.map((it) => (
                                    <div className="dec-user-info__stat-item" key={it.key}>
                                        <span className="dec-user-info__stat-value" style={{ color: tc.statValueColor || undefined }}>0</span>
                                        <span className="dec-user-info__stat-label" style={{ color: tc.statLabelColor || undefined }}>{it.label}</span>
                                    </div>
                                ))}
                            </div>
                        ) : null}
                        {Number(bar.show ?? 1) !== 0 ? (
                            <div
                                className="dec-user-info3__member-bar"
                                style={{
                                    borderRadius: `${bar.borderRadius ?? 8}px ${bar.borderRadius ?? 8}px 0 0`,
                                    background: layoutStyle === 3 ? `linear-gradient(to right, ${bar.backgroundColor || "#292929"}, ${bar.backgroundColorB || "#525252"})` : bar.backgroundColor || "#333333"
                                }}
                            >
                                <span className="dec-user-info3__member-bar-name">{bar.levelName || "会员成长值"}</span>
                                <span className="dec-user-info3__member-bar-desc">{bar.description || ""}</span>
                                <span className="dec-user-info3__member-bar-btn">{bar.buttonText || "查看"}</span>
                            </div>
                        ) : null}
                    </div>
                    {layoutStyle === 1 && Number(module.bottomArc?.show ?? 1) !== 0 ? <div className="dec-user-info3__arc" style={{ height: module.bottomArc?.height ?? 24 }} /> : null}
                </div>
            </div>
        </div>
    );
}
