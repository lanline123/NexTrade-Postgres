import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import ProductGroupon from "./index";

vi.mock("@tarojs/components", () => ({ View: "div", Text: "span" }));
vi.mock("@/utils/i18n", () => ({ t: (value: string) => value }));
vi.mock("@/utils", () => ({ redirect: vi.fn() }));
vi.mock("@/utils/format", () => ({
    imageFormat: (path: string) => path || "",
    hideUserName: (username: string) => {
        if (!username) return "";
        if (username.length === 1) return username;
        if (username.length === 2) return username[0] + "*";
        return username.replace(/^(.).*?(.)$/, "$1**$2");
    }
}));
vi.mock("@/components/tig/tig-image/TigImage", () => ({ default: () => null }));
vi.mock("@/store/config", () => ({
    useConfigStore: () => ({ defaultAvatar: "default-avatar.png" })
}));
vi.mock("@/store/user", () => ({
    useUserStore: () => ({ userInfo: { userId: 999 } })
}));

afterEach(cleanup);

const formatDateTime = (d: Date) => {
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
};
const futureEndTime = () => formatDateTime(new Date(Date.now() + 2 * 60 * 60 * 1000));

const baseGroup: any = {
    grouponPromotionRecordId: 1,
    grouponPromotionItemId: 10,
    productName: "测试商品",
    joinStatus: 1,
    minGroupNum: 3,
    joinNumCount: 1,
    endTime: futureEndTime(),
    joinUser: { userId: 123, username: "张三丰", avatar: "a.png", leader: true }
};

const joinBtnOpacity = (container: HTMLElement) => (container.querySelector(".join-btn") as HTMLElement | null)?.style.opacity;

describe("ProductGroupon 空态健壮性（异常数据降级渲染，不抛错）", () => {
    it("joinUser 为 null（后端 Option 查不到用户）时按非我的团渲染，倒计时照常", () => {
        const { container } = render(<ProductGroupon groups={[{ ...baseGroup, joinUser: null }]} />);
        expect(container.textContent).toContain("去参团");
        expect(container.textContent).toContain("剩余");
        expect(container.textContent).not.toContain("已结束");
        // 非我的团且未结束：按钮不置灰
        expect(joinBtnOpacity(container)).toBe("1");
    });

    it("joinUser 缺失（undefined）时不抛错", () => {
        const group = { ...baseGroup };
        delete group.joinUser;
        const { container } = render(<ProductGroupon groups={[group]} />);
        expect(container.textContent).toContain("去参团");
        expect(joinBtnOpacity(container)).toBe("1");
    });

    it("endTime 缺失时按已结束降级渲染，无倒计时", () => {
        const group = { ...baseGroup };
        delete group.endTime;
        const { container } = render(<ProductGroupon groups={[group]} />);
        expect(container.textContent).toContain("已结束");
        expect(container.textContent).not.toContain("剩余");
        // 已结束：按钮置灰
        expect(joinBtnOpacity(container)).toBe("0.5");
    });

    it("endTime 为空字符串时同样降级为已结束", () => {
        const { container } = render(<ProductGroupon groups={[{ ...baseGroup, endTime: "" }]} />);
        expect(container.textContent).toContain("已结束");
    });
});

describe("ProductGroupon 正常数据口径不变", () => {
    it("团长是当前用户时按钮置灰（我的团）", () => {
        const group = { ...baseGroup, joinUser: { userId: 999, username: "我自己", avatar: "", leader: true } };
        const { container } = render(<ProductGroupon groups={[group]} />);
        expect(joinBtnOpacity(container)).toBe("0.5");
        expect(container.textContent).toContain("剩余");
    });

    it("joinUser 为数组且含当前用户团长时同样判为我的团", () => {
        const group = { ...baseGroup, joinUser: [{ userId: 999, username: "我自己", leader: true }] };
        const { container } = render(<ProductGroupon groups={[group]} />);
        expect(joinBtnOpacity(container)).toBe("0.5");
    });

    it("普通团（他人团长、未结束）显示还差人数与倒计时，按钮可用", () => {
        const { container } = render(<ProductGroupon groups={[baseGroup]} />);
        expect(container.textContent).toContain("还差");
        expect(container.textContent).toContain("剩余");
        expect(joinBtnOpacity(container)).toBe("1");
    });
});
