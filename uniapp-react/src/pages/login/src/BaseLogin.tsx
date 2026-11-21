import { useEffect, useMemo, useRef, useState } from "react";
import { View, Text, Input, Image } from "@tarojs/components";
import Taro from "@tarojs/taro";

import VerificationCode, { type VerificationCodeRef } from "@/components/verificationCode/VerificationCode";
import Verify from "@/components/verifition/Verify";
import TigLayout from "@/components/tig/tig-layout/TigLayout";
import { Divider, Popup } from "@nutui/nutui-react-taro";
import TigNavbar from "@/components/tig/tig-navbar/TigNavbar";
import TigButton from "@/components/tig/tig-button/TigButton";
import TigCheckbox from "@/components/tig/tig-checkbox/TigCheckbox";
import TigMobileAreacode from "@/components/tig/tig-mobile-areacode/TigMobileAreacode";
import { sendMobileCode, setMiniProgramOpenid, userSignin, csrfCreate, getOauth, getOauthUrl, getQuickLoginSetting, getWechartMobile } from "@/api/login/login";
import { useUserStore } from "@/store/user";
import { currentMiniAppId } from "@/store/miniShop";
import { redirect, isOverseas, isWeixinBrowser, staticResource } from "@/utils";
import { useI18nStore } from "@/store/i18n";
import { t } from "@/utils/i18n";
import { useConfigStore } from "@/store/config";
import { useThemeStore } from "@/store/theme";
import { useCartStore } from "@/store/cart";
import { rpxToStyleLength } from "@/utils/styleLength";
import { loginReturnUrl, redirectAfterLogin } from "@/utils/loginReturn";

import "./BaseLogin.scss";

interface Props {
    /** 原 v-model:loginType */
    loginType: string;
    onLoginTypeChange: (value: string) => void;
    /** 原 v-model:csrf-token */
    csrfToken: string;
    onCsrfTokenChange: (value: string) => void;
    /** 登录页装修配置（仅颜色 + 边距） */
    decorateConfig?: Record<string, string>;
}

// 装修面板以 375px 为基准，渲染时换成 750rpx 设计稿单位。
const px2rpx = (px: number) => rpxToStyleLength(px * 2);
const toNumber = (val: any, fallback: number): number => {
    const n = Number(val);
    return Number.isNaN(n) ? fallback : n;
};

type AgreementModalPart =
    | {
          type: "text";
          text: string;
      }
    | {
          type: "link";
          text: string;
          url: string;
      };

const AGREEMENT_PLACEHOLDERS = {
    service: "__LOGIN_SERVICE_AGREEMENT__",
    privacy: "__LOGIN_PRIVACY_POLICY__"
} as const;

/**
 * 登录核心组件（自旧 pages/login/src/baseLogin.vue 移植）
 * - 跨境分支：Tab 布局 + Google/Facebook/Twitter 第三方登录（旧 #ifdef WEB）
 * - 微信小程序一键登录（旧 #ifdef MP-WEIXIN）、图形验证码防刷、用户协议勾选
 * - 账号、验证码与各端快捷登录共用协议确认流程
 */
export default function BaseLogin({ loginType, onLoginTypeChange, csrfToken, onCsrfTokenChange, decorateConfig }: Props) {
    const configStore = useConfigStore();
    const themeStyle = useThemeStore((s) => s.themeStyle);
    const langCode = useI18nStore((s) => s.langCode);

    const isOverseasVer = isOverseas();
    const isH5 = process.env.TARO_ENV === "h5";
    const isWeapp = process.env.TARO_ENV === "weapp";
    const showWechatLogin = (isH5 && Number(configStore.openWechatOauth) === 1) ||
        (isWeapp && Number(configStore.openWechatRegister) === 1);
    const oauthProviders = [
        { id: "google", enabled: configStore.googleLoginOn, label: "Google登录", image: "login/google.png" },
        { id: "facebook", enabled: configStore.facebookLoginOn, label: "Facebook登录", image: "login/facebook.png" },
        { id: "twitter", enabled: configStore.twitterLoginOn, label: "Twitter/X登录", image: "login/twitter.svg" }
    ].filter((provider) => isH5 && Number(provider.enabled) === 1);

    const [mobile, setMobile] = useState("");
    const [mobileCode, setMobileCode] = useState("");
    const [username, setUsername] = useState((process.env.TARO_APP_DEFAULT_USER_NAME as string) || "");
    const [password, setPassword] = useState((process.env.TARO_APP_DEFAULT_USER_PASSWORD as string) || "");
    const [isChecked, setIsChecked] = useState(false);
    const [verifyToken, setVerifyToken] = useState("");
    const [loginLoading, setLoginLoading] = useState(false);
    const signinPending = useRef(false);
    const verifyRef = useRef<{ show: () => void; refresh: () => void } | null>(null);
    const verificationCodeRef = useRef<VerificationCodeRef | null>(null);
    const [mobileAreaCode, setMobileAreaCode] = useState("86");
    const phone = mobile.trim();
    const signinMobile = phone && (configStore.isOpenMobileAreaCode ? mobileAreaCode + phone : phone);

    const [agreementAction, setAgreementAction] = useState<"get-code" | (() => void) | null>(null);

    useEffect(() => {
        if (!isChecked || agreementAction !== "get-code") return;
        setAgreementAction(null);
        // 发码组件需要先收到更新后的协议勾选状态。
        void verificationCodeRef.current?.handBtn();
    }, [agreementAction, isChecked]);

    const agreementModalParts = useMemo<AgreementModalPart[]>(() => {
        const template = t("为了更好地保障您的合法权益，请您阅读并同意{service}和{privacy}。", {
            service: AGREEMENT_PLACEHOLDERS.service,
            privacy: AGREEMENT_PLACEHOLDERS.privacy
        });
        const linkParts = [
            {
                token: AGREEMENT_PLACEHOLDERS.service,
                text: t("《服务协议》"),
                url: "/pages/login/mallAgreement?articleSn=fwxy"
            },
            {
                token: AGREEMENT_PLACEHOLDERS.privacy,
                text: t("《隐私协议》"),
                url: "/pages/login/mallAgreement?articleSn=ysxy"
            }
        ];
        const parts: AgreementModalPart[] = [];
        let rest = template;

        while (rest) {
            let nextIndex = -1;
            let nextLinkPart: (typeof linkParts)[number] | undefined;

            for (const part of linkParts) {
                const index = rest.indexOf(part.token);
                if (index !== -1 && (nextIndex === -1 || index < nextIndex)) {
                    nextIndex = index;
                    nextLinkPart = part;
                }
            }

            if (!nextLinkPart || nextIndex === -1) {
                parts.push({ type: "text", text: rest });
                break;
            }

            if (nextIndex > 0) {
                parts.push({ type: "text", text: rest.slice(0, nextIndex) });
            }

            parts.push({
                type: "link",
                text: nextLinkPart.text,
                url: nextLinkPart.url
            });
            rest = rest.slice(nextIndex + nextLinkPart.token.length);
        }

        return parts;
        // langCode：语言切换时重算文案
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [langCode]);

    // 微信内 H5 登录方式开关（后台「登录设置」wechatH5MobileLogin /
    // wechatH5PasswordLogin，默认全开）。仅微信内置浏览器内拉取并生效，
    // 普通浏览器不请求、不受影响；拉取失败按全开处理。
    const [wechatLoginFlags, setWechatLoginFlags] = useState({ mobile: 1, password: 1 });
    useEffect(() => {
        if (!isWeixinBrowser()) return;
        let alive = true;
        getQuickLoginSetting()
            .then((res: any) => {
                if (!alive || !res) return;
                setWechatLoginFlags({
                    mobile: res.wechatH5MobileLogin === 0 ? 0 : 1,
                    password: res.wechatH5PasswordLogin === 0 ? 0 : 1
                });
            })
            .catch(() => {});
        return () => {
            alive = false;
        };
    }, []);

    const loginTypeTabs = useMemo(() => {
        const all = [
            {
                name: t("账号登录"),
                value: "password"
            },
            {
                name: t("验证码登录"),
                value: "mobile"
            }
        ];
        if (!isWeixinBrowser()) return all;
        const filtered = all.filter((tab) => (tab.value === "password" ? wechatLoginFlags.password === 1 : wechatLoginFlags.mobile === 1));
        // 双关兜底：两个开关都关闭时保留账号密码登录，避免出现无登录方式可用
        return filtered.length ? filtered : all.filter((tab) => tab.value === "password");
        // langCode：语言切换时重算文案
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [wechatLoginFlags, langCode]);

    // 当前登录方式被配置过滤掉时，切到剩余的第一个方式
    useEffect(() => {
        if (!loginTypeTabs.some((tab) => tab.value === loginType)) {
            onLoginTypeChange(loginTypeTabs[0].value);
        }
    }, [loginTypeTabs, loginType, onLoginTypeChange]);

    // 仅当配置项有值时才注入对应 CSS 变量；未注入的变量在样式里带 fallback 回退原值，保证零变化
    const loginStyleVars = useMemo<Record<string, string>>(() => {
        const c = decorateConfig || {};
        const s: Record<string, string> = {};
        const set = (varName: string, val?: string) => {
            if (val && String(val).trim()) s[varName] = val;
        };
        set("--login-title", c.titleColor);
        set("--login-subtext", c.subTextColor);
        set("--login-input", c.inputTextColor);
        set("--login-placeholder", c.placeholderColor);
        set("--login-link", c.linkColor);
        set("--login-divider", c.dividerColor);
        // 仅当装修数据显式配置了才注入变量；未配置时不注入，交给各布局 CSS 自己的原始默认 fallback
        const hasVal = (v?: string | number) => v !== undefined && v !== null && String(v).trim() !== "";
        if (hasVal(c.contentPaddingX)) s["--login-padding-x"] = px2rpx(toNumber(c.contentPaddingX, 20));
        if (hasVal(c.contentPaddingTop)) s["--login-padding-top"] = px2rpx(toNumber(c.contentPaddingTop, 0));
        return s;
    }, [decorateConfig]);

    // 主按钮色：配置有值则用配置，否则回退 tig-button 默认（var(--main-bg)），保证未配置零变化
    const buttonColorFinal = useMemo(() => {
        const v = decorateConfig?.buttonColor;
        return v && String(v).trim() ? v : "var(--main-bg, var(--general, #ff4444))";
    }, [decorateConfig]);

    const handleLink = (url: string) => {
        if (!url) return;
        redirect({
            url: url
        });
    };

    const mobileErrorCallback = (msg: string) => {
        // 验证码组件未勾选协议时抛出的提示改为弹窗，勾选后自动补发验证码
        if (msg === t("请阅读并同意协议！")) {
            setAgreementAction("get-code");
            return;
        }
        Taro.showToast({
            title: msg,
            duration: 1500,
            icon: "none"
        });
    };

    const runAfterAgreement = (action: () => void) => {
        if (loginLoading) return;
        if (isChecked) action();
        else setAgreementAction(() => action);
    };

    const handleAgreementAgree = () => {
        setIsChecked(true);
        if (typeof agreementAction === "function") {
            setAgreementAction(null);
            agreementAction();
        }
    };

    const submitLogin = () => {
        if (loginType === "password" && (!username.trim() || !password)) {
            void Taro.showToast({ title: t(!username.trim() ? "请输入账号" : "请输入密码"), icon: "none" });
            return;
        }
        if (loginType === "mobile" && (!signinMobile || !mobileCode.trim() || !verifyToken)) {
            const message = !signinMobile ? "请输入手机号" : "请输入短信验证码";
            void Taro.showToast({ title: t(message), icon: "none" });
            return;
        }
        void signin();
    };

    // 跨境分支：Tab 切换登录方式
    const loginTypeChange = (value: string) => {
        onLoginTypeChange(value);
    };

    // 跨境分支：登录前刷新 CSRF（保持迁移前逐次刷新行为）；React setState 异步，返回值供 signin 同步取用
    const refreshCsrf = async (): Promise<string> => {
        try {
            const token = await csrfCreate();
            onCsrfTokenChange(token);
            return token;
        } catch (error) {
            console.error("获取CSRF Token失败:", error);
            return csrfToken;
        }
    };

    const completeLogin = (token: string) => {
        const userStore = useUserStore.getState();
        userStore.setToken(token);
        void userStore.getUserInfo();
        void useCartStore.getState().fetchCartCount(true);
        void Taro.showToast({ title: t("登录成功"), icon: "success" });
        if (isWeapp) void updateUserOpenId();
        redirectAfterLogin();
    };

    const handleOtherLogin = async (provider: string) => {
        if (signinPending.current) return;
        signinPending.current = true;
        setLoginLoading(true);
        try {
            if (provider === "wechat") {
                if (!isWeixinBrowser()) {
                    void Taro.showToast({ title: t("请在微信中打开后使用微信登录"), icon: "none" });
                    return;
                }
                const callback = new URL("../oauth/callback", window.location.href);
                callback.search = "?provider=wechat";
                const result = await getOauthUrl({ url: callback.href });
                const state = new URL(result.url).searchParams.get("state");
                if (!state) throw new Error(t("缺少授权参数"));
                Taro.setStorageSync("wechatLoginState", state);
                window.location.href = result.url;
            } else {
                const url = await getOauth(provider, loginReturnUrl(Taro.getStorageSync("URL")));
                Taro.setStorageSync("oauthPath", provider);
                window.location.href = url;
            }
        } catch (error: any) {
            void Taro.showToast({ title: error?.message || t("授权失败"), icon: "none" });
        } finally {
            signinPending.current = false;
            setLoginLoading(false);
        }
    };

    const handleWechatPhone = async (event: { detail: { code?: string } }) => {
        if (!isChecked || !event.detail.code || signinPending.current) return;
        signinPending.current = true;
        setLoginLoading(true);
        try {
            // 店铺小程序带自身 appId：服务端按绑定 Secret 解密手机号
            const token: string = await getWechartMobile({ code: event.detail.code, appId: currentMiniAppId() || undefined });
            completeLogin(token);
        } catch (error: any) {
            void Taro.showToast({ title: error?.message || t("登录失败"), icon: "none" });
        } finally {
            signinPending.current = false;
            setLoginLoading(false);
        }
    };

    const signin = async (captchaToken?: string) => {
        if (signinPending.current) return;
        signinPending.current = true;
        setLoginLoading(true);
        try {
            // 跨境版登录前刷新 CSRF（标准 / o2o 由 index.vue 预取透传，保持原行为）
            const csrfTokenToUse = isOverseasVer ? await refreshCsrf() : csrfToken;
            const data: any = {
                loginType: loginType,
                username: loginType === "password" ? username.trim() : "",
                password: loginType === "password" ? password : "",
                mobile: "",
                mobileCode: "",
                verifyToken: captchaToken ?? (loginType === "mobile" ? verifyToken : "")
            };
            if (loginType === "mobile") {
                data.mobile = signinMobile;
                data.mobileCode = mobileCode.trim();
            }
            const result = await userSignin(data, {
                "X-CSRF-Token": csrfTokenToUse
            });
            completeLogin(result.token);
        } catch (error: any) {
            if (error?.code == 1002 && verifyRef.current) {
                verifyRef.current.show();
            } else {
                Taro.showToast({
                    title: error?.message || t("登录失败"),
                    duration: 1500,
                    icon: "none"
                });
                if (error?.code > 0) setMobileCode("");
            }
        } finally {
            signinPending.current = false;
            setLoginLoading(false);
        }
    };

    const okCallback = ({ verifyToken: captchaToken }: { verifyToken: string }) => {
        void signin(captchaToken);
    };

    const [passwordType, setPasswordType] = useState("password");
    const passwordTypeChange = () => {
        setPasswordType(passwordType === "password" ? "text" : "password");
    };

    const handleReset = () => {
        Taro.navigateTo({
            url: "/pages/reset/index"
        });
    };

    const updateUserOpenId = async () => {
        Taro.login({
            success: async (res: any) => {
                await setMiniProgramOpenid({
                    code: res.code,
                    // 店铺小程序带自身 appId：openid 按 appid 分键落库
                    appId: currentMiniAppId() || undefined
                });
            },
            fail: (err: any) => {
                console.log(err);
            }
        });
    };

    // 旧 .bottom-box 使用 v-bind("configStore.windowInfo.windowHeight + 'px'")，模板未引用该类（见 BaseLogin.scss 注释）
    void configStore.windowInfo;

    return (
        <>
            <TigLayout needSafeBottom={false} overflow={false}>
                <TigNavbar fixed={false} />
                <View className="login-page login-panel-MjI4Mmdxbnc" style={loginStyleVars as any}>
                    <View className="tab-row">
                        {loginTypeTabs.map((item) => (
                            <View key={item.value} className={loginType === item.value ? "active" : ""} onClick={() => loginTypeChange(item.value)}>
                                {item.name}
                            </View>
                        ))}
                    </View>
                    <View className="form-box">
                        {loginType === "mobile" && (
                            <>
                                <View className="item">
                                    {!!configStore.isOpenMobileAreaCode && (
                                        // 原 tig-mobile-areacode v-model="mobileAreaCode" style="width: 90rpx"
                                        <View className="login-area-code">
                                            <TigMobileAreacode value={mobileAreaCode} onChange={setMobileAreaCode} />
                                        </View>
                                    )}
                                    <Input
                                        value={mobile}
                                        type="text"
                                        placeholder={t("请输入手机号")}
                                        placeholderClass="placeholder"
                                        className="mobile-text text"
                                        onInput={(e) => setMobile(e.detail.value)}
                                    />
                                </View>
                                <View className="item">
                                    <View className="login-code-input">
                                        <Input
                                            type="text"
                                            placeholder={t("手机短信验证码")}
                                            placeholderClass="placeholder"
                                            name="mobile_code"
                                            className="text"
                                            value={mobileCode}
                                            onInput={(e) => setMobileCode(e.detail.value)}
                                        />
                                    </View>
                                    <VerificationCode
                                        ref={verificationCodeRef}
                                        isChecked={isChecked}
                                        onVerifyTokenDataChange={setVerifyToken}
                                        mobile={signinMobile}
                                        btnType="text"
                                        requestApi={sendMobileCode}
                                        onMobileErrorCallback={mobileErrorCallback}
                                    />
                                </View>
                            </>
                        )}
                        {loginType === "password" && (
                            <>
                                <View className="item">
                                    <Input
                                        value={username}
                                        type="text"
                                        placeholder={t(configStore.openEmailRegister === 1 ? t("用户名/手机/邮箱") : t("用户名/手机号"))}
                                        placeholderClass="placeholder"
                                        className="text"
                                        onInput={(e) => setUsername(e.detail.value)}
                                    />
                                </View>
                                <View className="item">
                                    <Input
                                        type="text"
                                        password={passwordType === "password"}
                                        placeholder={t("请输入密码")}
                                        placeholderClass="placeholder"
                                        className="text password-text"
                                        value={password}
                                        onInput={(e) => setPassword(e.detail.value)}
                                    />
                                    <View
                                        className={`password_show_hide ${passwordType === "password" ? "iconfont icon-yincang" : "iconfont-h5 icon-xianshi"}`}
                                        onClick={passwordTypeChange}
                                    />
                                </View>
                            </>
                        )}
                    </View>
                    <View className="btn2-css3">
                        {/* 旧 tig-button :loading :disabled :color size=normal（已收敛到公共组件；禁用态灰色对齐旧 type="info"） */}
                        <TigButton
                            loading={loginLoading}
                            loadingText={t("登录中")}
                            color={buttonColorFinal}
                            className="login-btn"
                            onClick={() => runAfterAgreement(submitLogin)}
                        >
                            {t("登录")}
                        </TigButton>
                    </View>
                    {loginType === "password" && (
                        <View className="other-row">
                            <View className="item" onClick={handleReset}>
                                {t("忘记密码")}
                            </View>
                            {configStore.shopRegClosed === 0 && (
                                <View
                                    className="item flex"
                                    onClick={() =>
                                        redirect({
                                            url: "/pages/register/index"
                                        })
                                    }
                                >
                                    {t("新用户")}
                                    <Text className="special-color">{t("注册")}</Text>
                                </View>
                            )}
                        </View>
                    )}
                    {loginType === "mobile" && configStore.shopRegClosed === 0 && (
                        <View className="other-row">
                            <View className="tips">{t("未注册过的账号将自动注册账号")}</View>
                        </View>
                    )}

                    <View className="footer-row">
                        {(showWechatLogin || oauthProviders.length > 0) && (
                            <>
                                <Divider>{t("其他登录方式")}</Divider>
                                <View className="login-other">
                                    {showWechatLogin && (
                                        <TigButton
                                            plain
                                            size="small"
                                            disabled={loginLoading}
                                            className="wechat-login-button"
                                            openType={isWeapp && isChecked ? "getPhoneNumber" : undefined}
                                            onGetPhoneNumber={handleWechatPhone}
                                            onClick={() => runAfterAgreement(() => {
                                                if (isH5) void handleOtherLogin("wechat");
                                            })}
                                        >
                                            {t(isWeapp ? "手机号快捷登入" : "微信登录")}
                                        </TigButton>
                                    )}
                                    {oauthProviders.map((provider) => (
                                        <View key={provider.id} className="item" onClick={() => runAfterAgreement(() => { void handleOtherLogin(provider.id); })}>
                                            <Image src={staticResource(provider.image)} className={provider.id === "twitter" ? "icon twitter-icon" : "icon"} mode="widthFix" />
                                            <View>{t(provider.label)}</View>
                                        </View>
                                    ))}
                                </View>
                            </>
                        )}
                        <View className="desc-box">
                            <View className="rule-text" onClick={() => setIsChecked((checked) => !checked)}>
                                <TigCheckbox checked={isChecked} />
                                <View className="rule-xieyi">
                                    <Text>{t("我已阅读并同意")}</Text>
                                    <Text
                                        className="special-color"
                                        onClick={(e) => {
                                            e.stopPropagation();
                                            handleLink("/pages/login/mallAgreement?articleSn=fwxy");
                                        }}
                                    >
                                        {t("《服务协议》")}
                                    </Text>
                                    <Text>{t("和")}</Text>
                                    <Text
                                        className="special-color"
                                        onClick={(e) => {
                                            e.stopPropagation();
                                            handleLink("/pages/login/mallAgreement?articleSn=ysxy");
                                        }}
                                    >
                                        {t("《隐私协议》")}
                                    </Text>
                                </View>
                            </View>
                        </View>
                    </View>
                </View>
                <Verify ref={verifyRef} mode="pop" captchaType="blockPuzzle" imgSize={{ width: "310px", height: "155px" }} onOkCallback={okCallback} />
            </TigLayout>

            {/* 协议确认弹窗：未勾选服务协议时以弹窗代替 toast 提示 */}
            {/* 弹窗在 tig-layout 外，需自行注入 themeStyle + 登录装修变量，否则主题色/--login-link 无效 */}
            <Popup
                visible={agreementAction !== null && !isChecked}
                position="center"
                closeOnOverlayClick={false}
                closeable={false}
                style={{ backgroundColor: "#fff", borderRadius: rpxToStyleLength(24) }}
                zIndex={10081}
            >
                <View className="agreement-modal" style={{ ...(themeStyle as any), ...(loginStyleVars as any) } as any}>
                    <View className="agreement-modal__title">{t("温馨提示")}</View>
                    <View className="agreement-modal__content">
                        {agreementModalParts.map((part, index) => (
                            <Text key={`${part.type}-${index}`} className={part.type === "link" ? "agreement-modal__link" : ""} onClick={part.type === "link" ? () => handleLink(part.url) : undefined}>
                                {part.text}
                            </Text>
                        ))}
                    </View>
                    <View className="agreement-modal__actions">
                        <View className="agreement-modal__btn agreement-modal__btn--cancel" onClick={() => setAgreementAction(null)}>
                            {t("不同意")}
                        </View>
                        <View
                            className="agreement-modal__btn agreement-modal__btn--confirm"
                            style={{ background: buttonColorFinal, borderColor: buttonColorFinal }}
                            onClick={handleAgreementAgree}
                        >
                            {t("同意")}
                        </View>
                    </View>
                </View>
            </Popup>
        </>
    );
}
