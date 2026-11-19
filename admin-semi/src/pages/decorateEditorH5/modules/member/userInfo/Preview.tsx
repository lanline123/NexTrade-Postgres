/**
 * 用户信息 预览（移植自 view/admin editor/modules/mobileMember/userInfo/Preview.vue）
 *
 * 整块背景 = gradient 线性渐变（colorA→colorB 横向），borderRadius=frame.boxRadius，
 * container padding=frame.innerPadding；头像 64 圆、昵称「演示账号」、等级 V6会员；
 * 右上快捷入口（签到/设置，受 signIn.show / settings.show 控制）；统计行 4 项带分割线。
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { userInfoDefaults, defaultSignInIcon, defaultSettingsIcon, defaultGradient } from "./index";
import "./Preview.css";

const AVATAR = "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202605/1779691335wtQOROIkUPPNOzuply.jpeg?x-oss-process=image/resize,m_pad,h_200,h_200";
const LEVEL_ICON = "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202605/17796914549H8jHXMHCnnkf8uOXg.jpeg?x-oss-process=image/resize,m_pad,h_200,h_200";

const STATS: Array<{ label: string; value: string }> = [
    { label: "商品收藏", value: "13" },
    { label: "店铺收藏", value: "0" },
    { label: "待评价", value: "246" },
    { label: "浏览记录", value: "10" }
];

export default function UserInfoPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        // 原版 Preview 的旧字段兼容：showSignIn/showSettings → signIn/settings
        if (merged.signIn === undefined && merged.showSignIn !== undefined) merged.signIn = { show: !!merged.showSignIn };
        if (merged.settings === undefined && merged.showSettings !== undefined) merged.settings = { show: !!merged.showSettings };
        mergeDefaultModule(merged, userInfoDefaults);
        return merged;
    }, [data]);

    const frame = module.frame ?? {};
    const frameFormat = formatFrame({ ...frame, innerPadding: frame.innerPadding ?? 16 });
    const gradient = module.gradient ?? defaultGradient;
    const wrapStyle: React.CSSProperties = {
        background: `linear-gradient(90deg, ${gradient.colorA || defaultGradient.colorA}, ${gradient.colorB || defaultGradient.colorB})`,
        borderRadius: `${frame.boxRadius ?? 0}px`
    };

    return (
        <div
            className="module-ad-con module-member-user-info"
            style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}
        >
            <div className="module-ad-content">
                <div className="user-info-wrap" style={wrapStyle}>
                    <div className="user-info-container" style={{ padding: `${frame.innerPadding ?? 16}px` }}>
                        <div className="user-info-header">
                            <div className="user-info-profile">
                                <div className="user-info-avatar">
                                    <img src={AVATAR} alt="头像" />
                                </div>
                                <div className="user-info-text">
                                    <div className="user-info-name">演示账号</div>
                                    <div className="user-info-level">
                                        <img className="user-info-level-icon" src={LEVEL_ICON} alt="" />
                                        <span className="user-info-level-text">V6会员</span>
                                    </div>
                                </div>
                            </div>
                            <div className="user-info-shortcuts">
                                {module.signIn?.show !== false ? (
                                    <div className="shortcut-item">
                                        <img src={module.signIn?.icon?.picUrl || defaultSignInIcon} alt="" className="shortcut-icon" />
                                        <span className="shortcut-text">{module.signIn?.title || "签到有礼"}</span>
                                    </div>
                                ) : null}
                                {module.settings?.show !== false ? (
                                    <div className="shortcut-item shortcut-settings">
                                        <img src={module.settings?.icon?.picUrl || defaultSettingsIcon} alt="" className="shortcut-icon" />
                                    </div>
                                ) : null}
                            </div>
                        </div>
                        <div className="user-info-stats">
                            {STATS.map((item, index) => (
                                <div key={item.label} style={{ display: "contents" }}>
                                    {index > 0 ? <div className="stat-divider" /> : null}
                                    <div
                                        className="stat-item"
                                        style={{
                                            justifyContent: index === 0 ? "flex-start" : index === STATS.length - 1 ? "flex-end" : "center"
                                        }}
                                    >
                                        <span className="stat-label">{item.label}</span>
                                        <span className="stat-value">{item.value}</span>
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
