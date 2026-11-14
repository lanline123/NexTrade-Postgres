/**
 * 登录页 预览（按 BaseLogin 样式：标题/副文案/输入框/协议/按钮，全部由 defaultMobileLogin 颜色与边距字段驱动；
 * linkColor/buttonColor 留空时跟随主题色 --main-bg，与渲染端兜底口径一致）
 */
import { useMemo } from "react";
import { mergeDefaultModule } from "../../../contract/sharedDefaults";
import { mobileLoginDefaults } from "./index";
import "./Preview.css";

export default function MobileLoginPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, mobileLoginDefaults);
        return merged;
    }, [data]);
    const linkColor = module.linkColor || "var(--main-bg, #333333)";
    const buttonColor = module.buttonColor || "var(--main-bg, #333333)";
    return (
        <div className="module-ad-con dec-login">
            <div
                className="dec-login__container"
                style={{ paddingLeft: module.contentPaddingX ?? 20, paddingRight: module.contentPaddingX ?? 20, paddingTop: module.contentPaddingTop ?? 0 }}
            >
                <div className="dec-login__title" style={{ color: module.titleColor }}>欢迎登录</div>
                <div className="dec-login__sub" style={{ color: module.subTextColor }}>登录后享受会员权益</div>
                <div className="dec-login__field" style={{ borderBottomColor: module.dividerColor }}>
                    <span className="dec-login__input" style={{ color: module.inputTextColor }}>138****0000</span>
                </div>
                <div className="dec-login__field" style={{ borderBottomColor: module.dividerColor }}>
                    <span className="dec-login__placeholder" style={{ color: module.placeholderColor }}>请输入密码</span>
                    <span className="dec-login__link" style={{ color: linkColor }}>忘记密码？</span>
                </div>
                <div className="dec-login__agree">
                    <span className="dec-login__checkbox" style={{ borderColor: module.dividerColor }} />
                    <span style={{ color: module.subTextColor }}>
                        我已阅读并同意<span style={{ color: linkColor }}>《用户协议》</span>与<span style={{ color: linkColor }}>《隐私政策》</span>
                    </span>
                </div>
                <div className="dec-login__button" style={{ background: buttonColor }}>登录</div>
                <div className="dec-login__foot" style={{ color: module.subTextColor }}>
                    还没有账号？<span style={{ color: linkColor }}>立即注册</span>
                </div>
            </div>
        </div>
    );
}
