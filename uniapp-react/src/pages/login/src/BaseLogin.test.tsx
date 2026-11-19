import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { csrfCreate, userSignin } from "@/api/login/login";
import BaseLogin from "./BaseLogin";

vi.mock("@tarojs/components", () => ({
    View: ({ children, ...props }: { children?: ReactNode }) => <div {...props}>{children}</div>,
    Text: ({ children, ...props }: { children?: ReactNode }) => <span {...props}>{children}</span>,
    Image: () => null,
    Input: ({ value, placeholder, password, onInput }: any) => (
        <input value={value} placeholder={placeholder} type={password ? "password" : "text"} onChange={(event) => onInput({ detail: { value: event.target.value } })} />
    )
}));
vi.mock("@nutui/nutui-react-taro", () => ({
    Popup: ({ visible, children }: { visible: boolean; children: ReactNode }) => visible ? children : null,
    Divider: () => null
}));
vi.mock("@/utils", () => ({ isOverseas: () => true, isWeixinBrowser: () => false, staticResource: vi.fn(), redirect: vi.fn() }));
vi.mock("@/utils/i18n", () => ({ t: (value: string) => value }));
vi.mock("@/store/config", () => ({
    useConfigStore: () => ({ isOpenMobileAreaCode: 1, openEmailRegister: 1, shopRegClosed: 0, googleLoginOn: 0, facebookLoginOn: 0, twitterLoginOn: 0 })
}));
vi.mock("@/store/theme", () => ({ useThemeStore: (selector: (state: any) => unknown) => selector({ themeStyle: {} }) }));
vi.mock("@/store/i18n", () => ({ useI18nStore: (selector: (state: any) => unknown) => selector({ langCode: "zh" }) }));
vi.mock("@/store/user", () => ({ useUserStore: { getState: () => ({ setToken: vi.fn(), getUserInfo: vi.fn() }) } }));
vi.mock("@/store/cart", () => ({ useCartStore: { getState: () => ({ fetchCartCount: vi.fn() }) } }));
vi.mock("@/api/login/login", () => ({ csrfCreate: vi.fn(), userSignin: vi.fn(), sendMobileCode: vi.fn() }));
vi.mock("@/components/tig/tig-layout/TigLayout", () => ({ default: ({ children }: { children: ReactNode }) => <>{children}</> }));
vi.mock("@/components/tig/tig-navbar/TigNavbar", () => ({ default: () => null }));
vi.mock("@/components/tig/tig-mobile-areacode/TigMobileAreacode", () => ({ default: () => null }));
vi.mock("@/components/tig/tig-button/TigButton", () => ({
    default: ({ children, disabled, loading, onClick }: any) => <button disabled={disabled || loading} onClick={onClick}>{children}</button>
}));
vi.mock("@/components/verifition/Verify", async () => {
    const { forwardRef, useImperativeHandle, useState } = await import("react");
    return {
        default: forwardRef(function Captcha({ onOkCallback }: any, ref) {
            const [visible, setVisible] = useState(false);
            useImperativeHandle(ref, () => ({ show: () => setVisible(true) }));
            return visible ? <button onClick={() => onOkCallback({ verifyToken: "fresh-captcha" })}>完成滑块验证</button> : null;
        })
    };
});

function renderLogin(loginType: "mobile" | "password") {
    return render(<BaseLogin loginType={loginType} onLoginTypeChange={vi.fn()} csrfToken="" onCsrfTokenChange={vi.fn()} />);
}

beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(csrfCreate).mockResolvedValue("fresh-csrf");
});
afterEach(cleanup);

describe("登录页", () => {
    it("拒绝协议不会提交，同意后立即提交账号密码", async () => {
        vi.mocked(userSignin).mockRejectedValue({ code: 400, message: "密码错误" });
        const { container } = renderLogin("password");
        fireEvent.change(screen.getByPlaceholderText("用户名/手机/邮箱"), { target: { value: " alice " } });
        fireEvent.change(screen.getByPlaceholderText("请输入密码"), { target: { value: "test-password" } });
        const loginButton = screen.getByRole("button", { name: "登录" });
        fireEvent.click(loginButton);
        fireEvent.click(screen.getByText("不同意", { exact: true }));
        expect(userSignin).not.toHaveBeenCalled();
        fireEvent.click(loginButton);
        fireEvent.click(screen.getByText("同意", { exact: true }));
        await waitFor(() => expect(userSignin).toHaveBeenCalledWith(
            expect.objectContaining({ username: "alice", password: "test-password", verifyToken: "" }),
            { "X-CSRF-Token": "fresh-csrf" }
        ));
        expect(container.querySelector(".footer-row")?.textContent).toBe("✓登录即为同意《商城用户服务协议》");
    });

    it("协议弹窗同意后继续原来的发码操作", async () => {
        renderLogin("mobile");
        fireEvent.change(screen.getByPlaceholderText("请输入手机号"), { target: { value: "13800000000" } });
        fireEvent.click(await screen.findByText("获取验证码"));
        expect(screen.queryByRole("button", { name: "完成滑块验证" })).toBeNull();
        fireEvent.click(screen.getByText("同意", { exact: true }));
        expect(await screen.findByRole("button", { name: "完成滑块验证" })).toBeTruthy();
        expect(screen.queryByText("同意", { exact: true })).toBeNull();
    });

    it("滑块验证通过后用新令牌重试原登录请求", async () => {
        vi.mocked(userSignin).mockRejectedValueOnce({ code: 1002 }).mockRejectedValueOnce({ code: 400, message: "密码错误" });
        renderLogin("password");
        fireEvent.change(screen.getByPlaceholderText("用户名/手机/邮箱"), { target: { value: "alice" } });
        fireEvent.change(screen.getByPlaceholderText("请输入密码"), { target: { value: "test-password" } });
        fireEvent.click(screen.getByText("登录即为同意"));
        fireEvent.click(screen.getByRole("button", { name: "登录" }));
        fireEvent.click(await screen.findByRole("button", { name: "完成滑块验证" }));
        await waitFor(() => expect(userSignin).toHaveBeenNthCalledWith(
            2, expect.objectContaining({ verifyToken: "fresh-captcha" }), { "X-CSRF-Token": "fresh-csrf" }
        ));
    });
});
