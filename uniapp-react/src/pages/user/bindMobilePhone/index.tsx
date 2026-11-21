import { useRef, useState } from "react";
import { Input, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import TigButton2 from "@/components/tig/tig-button2/TigButton2";
import TigLayout from "@/components/tig/tig-layout/TigLayout";
import VerificationCode from "@/components/verificationCode/VerificationCode";
import Verify from "@/components/verifition/Verify";
import { sendMobileCode } from "@/api/login/login";
import { bindMobilePhone } from "@/api/user/bindMobilePhone";
import { currentMiniAppId } from "@/store/miniShop";
import { useI18nStore } from "@/store/i18n";
import { useUserStore } from "@/store/user";
import { loginReturnUrl, redirectAfterLogin } from "@/utils/loginReturn";
import { t } from "@/utils/i18n";
import { usePageGuards } from "@/utils/pageMixin";

import "./index.scss";

/**
 * 第三方登录后绑定手机号页（自旧 pages/user/bindMobilePhone/index.vue 逐行移植）
 * - 输入手机号、验证码、密码完成绑定
 * - 绑定成功后设置 Token 并回到登录前页面
 * - 支持滑动验证码二次校验
 */
export default function BindMobilePhone() {
    usePageGuards();

    // 订阅语言，切换语言时重算 t() 文案
    const langCode = useI18nStore((s) => s.langCode);
    void langCode;

    const setToken = useUserStore((s) => s.setToken);
    const getUserInfo = useUserStore((s) => s.getUserInfo);
    const setAuthType = useUserStore((s) => s.setAuthType);

    const [form, setForm] = useState({
        mobile: "",
        mobileCode: "",
        password: "",
        openData: {} as any
    });
    const [verifyToken, setVerifyToken] = useState("");
    const [isChecked, setIsChecked] = useState(true);
    const [isLoading, setIsLoading] = useState(false);

    const verifyRef = useRef<{ show: () => void; refresh: () => void } | null>(null);

    const mobileErrorCallback = (msg: string) => {
        Taro.showToast({
            title: msg,
            duration: 1500,
            icon: "none"
        });
    };

    const isloginDisabled = !(form.mobile && form.mobileCode && form.password);

    // 旧 up-form rules：手机号正则
    const validateMobile = () => {
        const regex = /^(?:(?:\+|00)86)?1[3-9]\d{9}$/;
        return regex.test(form.mobile);
    };

    const handleSubmit = async () => {
        setIsLoading(true);
        try {
            if (!validateMobile()) {
                mobileErrorCallback(t("请输入正确的手机号码"));
                return;
            }
            // 店铺小程序：openData 附带自身 appId，服务端据此把授权按
            // appid 分键（bindTicket 路径以凭证内作用域为准，此处仅旧
            // openData 路径需要）；平台/公众号场景 appId 为空不附带
            const storedOpenData = Taro.getStorageSync("openData");
            const miniAppId = currentMiniAppId();
            const openData =
                miniAppId && storedOpenData && typeof storedOpenData === "object"
                    ? { ...storedOpenData, appId: miniAppId }
                    : storedOpenData;
            const payload = { ...form, openData };
            const result: any = await bindMobilePhone(payload);
            setToken(result);
            void getUserInfo();
            setAuthType("");
            Taro.showToast({
                title: t("绑定成功"),
                duration: 1500,
                icon: "success"
            });
            setTimeout(() => {
                Taro.setStorageSync("openData", "");
                redirectAfterLogin(loginReturnUrl(Taro.getStorageSync("URL")) || "/pages/member/index");
            }, 1500);
        } catch (error: any) {
            console.error(error);
            if (error.code == 1002 && verifyRef.current) {
                verifyRef.current.show();
            } else if (error.code > 0) {
                Taro.showToast({
                    title: error.message,
                    duration: 1500,
                    icon: "none"
                });
                setVerifyToken("");
            }
        } finally {
            setIsLoading(false);
        }
    };

    const okCallback = (e: any) => {
        setVerifyToken(e.verifyToken);
        void handleSubmit();
    };

    return (
        <TigLayout title={t("绑定")}>
            <View className="bindMobilePhone">
                <View className="bindMobilePhone-title">{t("手机号绑定")}</View>
                <View className="form-item" data-prop="mobile">
                    <Input
                        className="form-input"
                        value={form.mobile}
                        placeholder={t("请输入手机号")}
                        placeholderStyle="font-size:28rpx;color:#c0c4cc"
                        onInput={(e) => setForm((prev) => ({ ...prev, mobile: e.detail.value }))}
                    />
                </View>
                <View className="form-item" data-prop="mobileCode">
                    <View className="form-row">
                        <Input
                            className="form-input flex1"
                            value={form.mobileCode}
                            placeholder={t("请输入验证码")}
                            placeholderStyle="font-size:28rpx;color:#c0c4cc"
                            onInput={(e) => setForm((prev) => ({ ...prev, mobileCode: e.detail.value }))}
                        />
                        <View className="verify-wrap">
                            <VerificationCode
                                mobile={form.mobile}
                                isChecked={isChecked}
                                verifyTokenData={verifyToken}
                                requestApi={sendMobileCode}
                                onMobileErrorCallback={mobileErrorCallback}
                                onVerifyTokenDataChange={(token) => setVerifyToken(token)}
                            />
                        </View>
                    </View>
                </View>
                <View className="form-item" data-prop="password">
                    <Input
                        className="form-input"
                        value={form.password}
                        password
                        placeholder={t("请输入密码")}
                        placeholderStyle="font-size:28rpx;color:#c0c4cc"
                        onInput={(e) => setForm((prev) => ({ ...prev, password: e.detail.value }))}
                    />
                </View>
                <TigButton2 theme loading={isLoading} disabled={isloginDisabled} onClick={handleSubmit}>
                    {t("确 认")}
                </TigButton2>
            </View>
            {/* #ifdef H5 */}
            {process.env.TARO_ENV === "h5" && (
                <Verify
                    ref={verifyRef}
                    mode="pop"
                    captchaType="blockPuzzle"
                    imgSize={{ width: "310px", height: "155px" }}
                    onOkCallback={okCallback}
                />
            )}
            {/* #endif */}
        </TigLayout>
    );
}
