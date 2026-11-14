/**
 * 移动端登录页 预览（移植自 view/admin editor/modules/mobileLogin/mobileLogin/Preview.vue）
 *
 * 整页登录 mock：国内 ml-* 与海外 ol-* 双变体，由 isOverseas() 判定（与原版一致，
 * 非模块数据字段）。原版社交图标为 google.png/facebook.png/twitter.svg 图片，
 * admin-semi 无对应静态资源，此处以内联品牌 SVG 等尺寸（45×45）复刻。
 */
import { useMemo } from "react";
import { mergeDefaultModule, defaultMobileLogin } from "../../../contract/sharedDefaults";
import { isOverseas } from "../../../contract/version";
import { mobileLoginDefaults } from "./index";
import "./Preview.css";

function EyeIcon({ size = 20 }: { size?: number }) {
    return (
        <svg viewBox="0 0 1024 1024" width={size} height={size} aria-hidden="true">
            <path
                fill="#666"
                d="M512 256C264 256 64 512 64 512s200 256 448 256 448-256 448-256-200-256-448-256zm0 416a160 160 0 1 1 0-320 160 160 0 0 1 0 320zm0-256a96 96 0 1 0 0 192 96 96 0 0 0-96-192z"
            />
        </svg>
    );
}

function GoogleIcon() {
    return (
        <svg viewBox="0 0 48 48" width="45" height="45" aria-label="Google">
            <path fill="#FFC107" d="M43.6 20.1H42V20H24v8h11.3C33.7 32.7 29.2 36 24 36c-6.6 0-12-5.4-12-12s5.4-12 12-12c3.1 0 5.9 1.2 8 3l5.7-5.7C34.3 6.1 29.3 4 24 4 13 4 4 13 4 24s9 20 20 20 20-9 20-20c0-1.3-.1-2.6-.4-3.9z" />
            <path fill="#FF3D00" d="M6.3 14.7l6.6 4.8C14.7 15.1 19 12 24 12c3.1 0 5.9 1.2 8 3l5.7-5.7C34.3 6.1 29.3 4 24 4 16.3 4 9.7 8.3 6.3 14.7z" />
            <path fill="#4CAF50" d="M24 44c5.2 0 9.9-2 13.4-5.2l-6.2-5.2C29.2 35.1 26.7 36 24 36c-5.2 0-9.6-3.3-11.3-8l-6.5 5C9.5 39.6 16.2 44 24 44z" />
            <path fill="#1976D2" d="M43.6 20.1H42V20H24v8h11.3c-.8 2.3-2.3 4.3-4.1 5.6l6.2 5.2C36.9 39.2 44 34 44 24c0-1.3-.1-2.6-.4-3.9z" />
        </svg>
    );
}

function FacebookIcon() {
    return (
        <svg viewBox="0 0 48 48" width="45" height="45" aria-label="Facebook">
            <circle cx="24" cy="24" r="22" fill="#1877F2" />
            <path fill="#fff" d="M29.5 24h-3.8v13h-5.4V24h-2.6v-4.4h2.6v-2.9c0-2.2.6-5.7 5.8-5.7h3.2v4.8h-2c-.4 0-1.6 0-1.6 1.5v2.3h3.9L29.5 24z" />
        </svg>
    );
}

function TwitterIcon() {
    return (
        <svg viewBox="0 0 24 24" width="25" height="25" fill="#000" aria-label="Twitter/X">
            <path d="M18.9 2.1h3.7l-8.1 9.3L24 23.9h-7.5l-5.9-7.7-6.7 7.7H.2l8.7-9.9L0 2.1h7.7l5.3 7 5.9-7zm-1.3 19.5h2L6.6 4.2H4.4l13.2 17.4z" />
        </svg>
    );
}

function PhoneLoginIcon() {
    return (
        <svg viewBox="0 0 1024 1024" width="45" height="45" aria-hidden="true">
            <path
                fill="#333"
                d="M704 64H320a32 32 0 0 0-32 32v832a32 32 0 0 0 32 32h384a32 32 0 0 0 32-32V96a32 32 0 0 0-32-32zm-192 800a32 32 0 1 1 0-64 32 32 0 0 1 0 64zm160-128H352V160h320v576z"
            />
        </svg>
    );
}

export default function MobileLoginPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        // mergeDefaultModule 已把空值回填为默认（左右 20 / 顶部 0），此处按 px 直接使用
        mergeDefaultModule(merged, mobileLoginDefaults);
        return merged;
    }, [data]);

    const overseas = isOverseas();
    const m = module || {};
    // 颜色变量挂在根节点，子元素继承（与原版 colorVars 逐字段一致）
    const colorVars = {
        "--login-title": m.titleColor || "#333333",
        "--login-subtext": m.subTextColor || "#999999",
        "--login-input": m.inputTextColor || "#252525",
        "--login-placeholder": m.placeholderColor || "#252525",
        "--login-link": m.linkColor || "var(--tig-primary, #e93b3d)",
        "--login-button": m.buttonColor || "var(--tig-primary, #e93b3d)",
        "--login-divider": m.dividerColor || "#eeeeee"
    } as React.CSSProperties;
    // 左右/顶部间距只作用于标题+表单包裹层（原版 warpStyle）
    const px = m.contentPaddingX ?? defaultMobileLogin.contentPaddingX;
    const pt = m.contentPaddingTop ?? defaultMobileLogin.contentPaddingTop;
    const warpStyle: React.CSSProperties = {
        paddingTop: `${pt}px`,
        paddingLeft: `${px}px`,
        paddingRight: `${px}px`
    };

    if (overseas) {
        return (
            <div className="module-ad-con dec-mobile-login" style={colorVars}>
                <div className="overseas-login-page" style={warpStyle}>
                    <div className="ol-tabs">
                        <span className="ol-tab ol-tab-active">账号登录</span>
                        <span className="ol-tab">验证码登录</span>
                    </div>
                    <div className="ol-form">
                        <div className="ol-item">
                            <span>用户名/手机号</span>
                        </div>
                        <div className="ol-item ol-password">
                            <span>请输入密码</span>
                            <span className="ml-eye">
                                <EyeIcon size={18} />
                            </span>
                        </div>
                    </div>
                    <div className="ol-btn">登录</div>
                    <div className="ol-other-row">
                        <span>忘记密码</span>
                        <span>
                            新用户<b>注册</b>
                        </span>
                    </div>
                    <div className="ol-footer">
                        <div className="ol-divider">
                            <span>其他登录方式</span>
                        </div>
                        <div className="ol-socials">
                            <div>
                                <GoogleIcon />
                                <span>Google登录</span>
                            </div>
                            <div>
                                <FacebookIcon />
                                <span>Facebook登录</span>
                            </div>
                            <div>
                                <span className="ol-twitter-icon">
                                    <TwitterIcon />
                                </span>
                                <span>Twitter/X登录</span>
                            </div>
                        </div>
                        <div className="ol-agreement">
                            <span className="ol-checkbox"></span>登录即为同意<b>《商城用户服务协议》</b>
                        </div>
                    </div>
                </div>
            </div>
        );
    }

    return (
        <div className="module-ad-con dec-mobile-login" style={colorVars}>
            <div className="ml-warp" style={warpStyle}>
                <div className="ml-title">账号登录</div>
                <div className="ml-form">
                    <div className="ml-item">
                        <span className="ml-text ml-placeholder">用户名/手机号</span>
                    </div>
                    <div className="ml-item ml-item-pwd">
                        <span className="ml-text ml-placeholder">请输入密码</span>
                        <span className="ml-eye">
                            <EyeIcon />
                        </span>
                    </div>
                    <div className="ml-btn">确 认</div>
                    <div className="ml-forgot">忘记密码?</div>
                    <div className="ml-register">
                        没有账号？<span className="ml-link">立即注册</span>
                    </div>
                </div>
            </div>

            <div className="ml-bottom">
                <div className="ml-other-tit">
                    <span className="ml-line ml-line1"></span>
                    <span className="ml-other-text">其他登录方式</span>
                    <span className="ml-line ml-line2"></span>
                </div>
                <div className="ml-quick">
                    <span className="ml-quick-ico">
                        <PhoneLoginIcon />
                    </span>
                    <span className="ml-quick-text">手机号登录</span>
                </div>
                <div className="ml-rule">
                    <span className="ml-checkbox"></span>
                    <span className="ml-rule-text">
                        我已阅读并同意<span className="ml-link">《服务协议》</span>和<span className="ml-link">《隐私协议》</span>
                    </span>
                </div>
            </div>
        </div>
    );
}
