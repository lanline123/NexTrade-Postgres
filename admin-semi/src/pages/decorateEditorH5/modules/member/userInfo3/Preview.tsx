/**
 * 用户信息3 预览（移植自 view/admin editor/modules/mobileMember/userInfo3/Preview.vue，四种布局）
 *
 * 样式一：bgColor+背景图、头像描边、副标题「个人资料」、统计（值上标下）、会员等级条（上圆角、#fcdabe 文案）、
 *         底部弧（SVG fill #f5f5f5，margin-top -14）；样式二：渐变（默认 #E0FCED→#FDF1F3）、手机号副标题、
 *         设置/通知图标（红点）；样式三：渐变（#F1EFE9→#E7F0E6）、右箭头、成长值条（深灰渐变、值 #ffdea0）；
 * 样式四：居中 80px 头像+编辑图标+认证徽标、顶部渐变层（止于成长卡半高 bottom 51px）、成长值卡（粉渐变+波浪图）。
 * container 仅左右 padding=frame.innerPadding；文字色按样式默认（样式一全白，其余 #333/#666）再被 textColors 覆盖。
 * 原版 memberImmersiveActive 注入（沉浸式时渐变由页面统一绘制）在本编辑器无对应 provide，按非沉浸式渲染（见摘要）。
 */
import { useMemo, type CSSProperties } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { userInfo3Defaults, USERINFO3_SETTINGS_ICON, USERINFO3_NOTIFICATION_ICON, USERINFO3_GROWTH_CARD_BG } from "./index";
import "./Preview.css";

const DEMO_AVATAR = "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202605/1779691335wtQOROIkUPPNOzuply.jpeg?x-oss-process=image/resize,m_pad,h_200,h_200";
const AUTH_BADGE = "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202606/1780880708HREkXQF9XjeKH1ddgB.png";
const EDIT_ICON = "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202606/1780880709E3AknH2icO6fSZeMly.png";
const DEMO_NAME = "我饿了了了";
const DEMO_PHONE = "156****8888";
const STATS_DEMO: Record<string, number> = { collect: 12, follow: 824, history: 5688 };
const DEFAULT_STATS = [
    { key: "collect", label: "商品收藏", show: true },
    { key: "follow", label: "门店关注", show: true },
    { key: "history", label: "浏览记录", show: true }
];
const GRADIENT_DEFAULTS: Record<number, { colorA: string; colorB: string }> = {
    2: { colorA: "#E0FCED", colorB: "#FDF1F3" },
    3: { colorA: "#F1EFE9", colorB: "#E7F0E6" },
    4: { colorA: "#FBD9E1", colorB: "#FDEDEF" }
};
const TEXT_DEFAULTS: Record<number, Record<string, string>> = {
    1: { nameColor: "#ffffff", subColor: "#ffffff", statValueColor: "#ffffff", statLabelColor: "#ffffff" },
    2: { nameColor: "#333333", subColor: "#666666", statValueColor: "#333333", statLabelColor: "#666666" },
    3: { nameColor: "#333333", subColor: "#666666", statValueColor: "#333333", statLabelColor: "#666666" },
    4: { nameColor: "#333333", subColor: "#666666", statValueColor: "#333333", statLabelColor: "#666666" }
};
const GRADIENT_DIRS: Record<string, string> = { upDown: "to bottom", leftRight: "to right", diagonal: "135deg" };

function Arrow({ size = 12, color }: { size?: number; color?: string }) {
    return (
        <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true" style={{ flexShrink: 0, color }}>
            <polyline points="9 4.5 16.5 12 9 19.5" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

export default function UserInfo3Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        // 旧数据迁移（同原版）：showSignIn/showSettings → signIn/settings.show
        if (merged.signIn === undefined && merged.showSignIn !== undefined) merged.signIn = { show: !!merged.showSignIn };
        if (merged.settings === undefined && merged.showSettings !== undefined) merged.settings = { show: !!merged.showSettings };
        mergeDefaultModule(merged, userInfo3Defaults);
        return merged;
    }, [data]);

    const layoutStyle = module.layoutStyle ?? 1;
    const frame = module.frame ?? {};
    const frameFormat = formatFrame(frame);
    const tcDefaults = TEXT_DEFAULTS[layoutStyle] || TEXT_DEFAULTS[1];
    const tc = module.textColors ?? {};
    const colors = {
        nameColor: tc.nameColor || tcDefaults.nameColor,
        subColor: tc.subColor || tcDefaults.subColor,
        statValueColor: tc.statValueColor || tcDefaults.statValueColor,
        statLabelColor: tc.statLabelColor || tcDefaults.statLabelColor
    };
    const gDefaults = GRADIENT_DEFAULTS[layoutStyle] || GRADIENT_DEFAULTS[2];
    const gradient = {
        colorA: module.gradient?.colorA || gDefaults.colorA,
        colorB: module.gradient?.colorB || gDefaults.colorB,
        gradientType: module.gradient?.gradientType || "leftRight"
    };
    const gradientDir = GRADIENT_DIRS[gradient.gradientType] || "to right";
    const containerStyle: CSSProperties = { paddingLeft: `${frame.innerPadding ?? 16}px`, paddingRight: `${frame.innerPadding ?? 16}px` };
    const visibleStats = (module.stats?.items?.length ? module.stats.items : DEFAULT_STATS).filter((i: any) => i.show !== false);
    const bar = module.memberBar ?? {};
    const barRadius = bar.borderRadius ?? 25;
    const memberBarStyle: CSSProperties =
        layoutStyle === 3
            ? {
                  borderRadius: `${barRadius}px ${barRadius}px 0 0`,
                  background: `linear-gradient(to right, ${bar.backgroundColor || "#292929"}, ${bar.backgroundColorB || "#525252"})`
              }
            : { borderRadius: `${barRadius}px ${barRadius}px 0 0`, backgroundColor: bar.backgroundColor || "#333" };

    const statsBlock = module.stats?.show !== false && (
        <div className="user-info-stats">
            {visibleStats.map((item: any) => (
                <div className="stat-item" key={item.key}>
                    <span className="stat-value" style={{ color: colors.statValueColor }}>
                        {STATS_DEMO[item.key] ?? 0}
                    </span>
                    <span className="stat-label" style={{ color: colors.statLabelColor }}>
                        {item.label}
                    </span>
                </div>
            ))}
        </div>
    );

    return (
        <div className="module-ad-con module-member-user-info-v3" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content">
                {layoutStyle === 1 ? (
                    <>
                        <div
                            className="user-info-wrap user-info-wrap--style1"
                            style={{
                                ...(module.bgColor ? { backgroundColor: module.bgColor } : {}),
                                ...(module.backgroundImage?.picUrl
                                    ? { backgroundImage: `url(${module.backgroundImage.picUrl})`, backgroundSize: "cover", backgroundPosition: "center" }
                                    : {})
                            }}
                        >
                            <div className="user-info-container" style={containerStyle}>
                                <div className="user-info-header">
                                    <div className="user-info-profile">
                                        <div className="user-info-avatar user-info-avatar--bordered">
                                            <img src={DEMO_AVATAR} alt="头像" />
                                        </div>
                                        <div className="user-info-text">
                                            <div className="user-info-name" style={{ color: colors.nameColor }}>
                                                {DEMO_NAME}
                                            </div>
                                            <div className="user-info-sub" style={{ color: colors.subColor }}>
                                                个人资料
                                                <Arrow size={12} />
                                            </div>
                                        </div>
                                    </div>
                                </div>
                                {statsBlock}
                                {module.memberBar?.show !== false ? (
                                    <div className="member-bar member-bar--level" style={memberBarStyle}>
                                        <div className="member-bar-info">
                                            <span className="member-bar-level-name">{bar.levelName || "付费会员"}</span>
                                            <span className="member-bar-divider-line" />
                                            <span className="member-bar-upgrade-text">{bar.description || "购买会员，畅享更多专属权益"}</span>
                                        </div>
                                        <span className="member-bar-btn">
                                            {bar.buttonText || "查看"}
                                            <Arrow size={6} />
                                        </span>
                                    </div>
                                ) : (
                                    <div className="member-bar-placeholder" style={{ height: 22 }} />
                                )}
                            </div>
                        </div>
                        {module.bottomArc?.show !== false ? (
                            <div className="user-info-arc" style={{ height: `${module.bottomArc?.height ?? 14}px` }}>
                                <svg
                                    className="user-info-arc-svg"
                                    viewBox={`0 0 375 ${module.bottomArc?.height ?? 24}`}
                                    preserveAspectRatio="none"
                                    xmlns="http://www.w3.org/2000/svg"
                                >
                                    <path
                                        d={`M 0 0 Q 187.5 ${(module.bottomArc?.height ?? 14) * 1.4} 375 0 L 375 ${module.bottomArc?.height ?? 14} L 0 ${module.bottomArc?.height ?? 14} Z`}
                                        fill="#f5f5f5"
                                    />
                                </svg>
                            </div>
                        ) : null}
                    </>
                ) : layoutStyle === 2 ? (
                    <div
                        className="user-info-wrap user-info-wrap--style2"
                        style={{ background: `linear-gradient(${gradientDir}, ${gradient.colorA}, ${gradient.colorB})`, borderRadius: `${frame.boxRadius ?? 0}px` }}
                    >
                        <div className="user-info-container" style={containerStyle}>
                            <div className="user-info-header">
                                <div className="user-info-avatar">
                                    <img src={DEMO_AVATAR} alt="头像" />
                                </div>
                                <div className="user-info-text">
                                    <div className="user-info-name-row">
                                        <div className="user-info-name" style={{ color: colors.nameColor }}>
                                            {DEMO_NAME}
                                        </div>
                                        <div className="user-info-action-icons">
                                            {module.settings?.show !== false ? (
                                                <img src={module.settings?.icon?.picUrl || USERINFO3_SETTINGS_ICON} alt="设置" className="action-icon" />
                                            ) : null}
                                            {module.notification?.show !== false ? (
                                                <div className="action-icon-wrap">
                                                    <img
                                                        src={module.notification?.icon?.picUrl || USERINFO3_NOTIFICATION_ICON}
                                                        alt="消息"
                                                        className="action-icon"
                                                    />
                                                    <span className="notification-dot" />
                                                </div>
                                            ) : null}
                                        </div>
                                    </div>
                                    <div className="user-info-sub" style={{ color: colors.subColor }}>
                                        {DEMO_PHONE}
                                    </div>
                                </div>
                            </div>
                            {statsBlock}
                            <div className="member-bar-placeholder" style={{ height: 12 }} />
                        </div>
                    </div>
                ) : layoutStyle === 3 ? (
                    <div
                        className="user-info-wrap user-info-wrap--style3"
                        style={{ background: `linear-gradient(${gradientDir}, ${gradient.colorA}, ${gradient.colorB})`, borderRadius: `${frame.boxRadius ?? 0}px` }}
                    >
                        <div className="user-info-container" style={containerStyle}>
                            <div className="user-info-header user-info-header--arrow">
                                <div className="user-info-profile">
                                    <div className="user-info-avatar">
                                        <img src={DEMO_AVATAR} alt="头像" />
                                    </div>
                                    <div className="user-info-text">
                                        <div className="user-info-name" style={{ color: colors.nameColor }}>
                                            {DEMO_NAME}
                                        </div>
                                        <div className="user-info-sub" style={{ color: colors.subColor }}>
                                            {DEMO_PHONE}
                                        </div>
                                    </div>
                                </div>
                                <Arrow size={16} color="#333" />
                            </div>
                            {module.memberBar?.show !== false ? (
                                <div className="member-bar member-bar--growth" style={memberBarStyle}>
                                    <span className="member-bar-growth-label">成长值</span>
                                    <span className="member-bar-growth-right">
                                        <span className="member-bar-growth-value">56515</span>
                                        <Arrow size={12} color="#ffe3b1" />
                                    </span>
                                </div>
                            ) : null}
                        </div>
                    </div>
                ) : (
                    <div className="user-info-wrap user-info-wrap--style4">
                        <div
                            className="user-info-style4-grad"
                            style={{
                                left: 0,
                                right: 0,
                                top: 0,
                                bottom: module.memberBar?.show !== false ? "51px" : "0",
                                background: `linear-gradient(${gradientDir}, ${gradient.colorA}, ${gradient.colorB})`,
                                zIndex: 0
                            }}
                        />
                        <div className="user-info-container user-info-container--style4" style={containerStyle}>
                            <div className="user-info-center-header">
                                <div className="user-info-avatar-wrap">
                                    <div className="user-info-avatar user-info-avatar--lg">
                                        <img src={DEMO_AVATAR} alt="头像" />
                                    </div>
                                    {module.showEditIcon !== false ? <img className="user-info-edit-icon" src={EDIT_ICON} alt="编辑" /> : null}
                                </div>
                                <div className="user-info-center-name-row">
                                    <span className="user-info-center-name" style={{ color: colors.nameColor }}>
                                        {DEMO_NAME}
                                    </span>
                                    <img className="user-info-auth-badge" src={AUTH_BADGE} alt="已认证" />
                                </div>
                            </div>
                            {module.memberBar?.show !== false ? (
                                <div
                                    className="growth-card"
                                    style={{
                                        borderRadius: `${bar.borderRadius ?? 10}px`,
                                        background: `linear-gradient(135deg, ${bar.backgroundColor || "#FF9EB5"}, ${bar.backgroundColorB || "#FF6E97"})`
                                    }}
                                >
                                    <img className="growth-card-wave" src={bar.waveImage?.picUrl || USERINFO3_GROWTH_CARD_BG} alt="" />
                                    <div className="growth-card-left">
                                        <div className="growth-card-label-row">
                                            <span className="growth-card-label">成长值</span>
                                            <Arrow size={10} color="rgba(255,255,255,0.9)" />
                                        </div>
                                        <span className="growth-card-value">56515</span>
                                    </div>
                                    <span className="growth-card-btn">{bar.buttonText || "查看"}</span>
                                </div>
                            ) : null}
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}
