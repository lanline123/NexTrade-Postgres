/**
 * 会员信息 预览（类名对齐渲染端 dec-user-info__*；统计数为演示值，真实值来自 user store 的 orderNum）
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { userInfoDefaults } from "./index";
import "./Preview.css";

export default function UserInfoPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, userInfoDefaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const gradient = module.gradient ?? {};
    const stats = [
        { label: "商品收藏", value: 0 },
        { label: "店铺收藏", value: 0 },
        { label: "待评价", value: 0 },
        { label: "浏览记录", value: 0 }
    ];
    return (
        <div className="module-ad-con" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className="dec-user-info">
                    <div
                        className="dec-user-info__container"
                        style={{ background: `linear-gradient(to right, ${gradient.colorA || "#fcf6fc"}, ${gradient.colorB || "#f3f3fe"})`, padding: module.frame?.innerPadding ?? 16 }}
                    >
                        <div className="dec-user-info__header">
                            <div className="dec-user-info__profile">
                                <div className="dec-user-info__avatar">
                                    <div className="dec-user-info__avatar-img" />
                                </div>
                                <div className="dec-user-info__info">
                                    <div className="dec-user-info__name-row">
                                        <span className="dec-user-info__name">{module.nickname}</span>
                                    </div>
                                    <div className="dec-user-info__level">
                                        <span className="dec-user-info__level-text">{module.description}</span>
                                    </div>
                                </div>
                            </div>
                            <div className="dec-user-info__shortcuts">
                                {Number(module.signIn?.show ?? 1) !== 0 ? (
                                    <div className="dec-user-info__shortcut-item">
                                        {module.signIn?.icon?.picUrl ? <img className="dec-user-info__shortcut-icon" src={imageFormat(module.signIn.icon.picUrl)} alt="" /> : <div className="dec-user-info__shortcut-icon dec-user-info__shortcut-ph">签</div>}
                                        <span className="dec-user-info__shortcut-text">{module.signIn?.title || "签到有礼"}</span>
                                    </div>
                                ) : null}
                                {Number(module.settings?.show ?? 1) !== 0 ? (
                                    <div className="dec-user-info__shortcut-item dec-user-info__settings">
                                        {module.settings?.icon?.picUrl ? <img className="dec-user-info__shortcut-icon" src={imageFormat(module.settings.icon.picUrl)} alt="" /> : <div className="dec-user-info__shortcut-icon dec-user-info__shortcut-ph">⚙</div>}
                                    </div>
                                ) : null}
                            </div>
                        </div>
                        <div className="dec-user-info__stats">
                            {stats.map((s) => (
                                <div className="dec-user-info__stat-item" key={s.label}>
                                    <span className="dec-user-info__stat-label">{s.label}</span>
                                    <span className="dec-user-info__stat-value">{s.value}</span>
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
