import { cleanup, render } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import DecorateModules from "./DecorateModules";

/**
 * 会员中心模块分发集成自检（2026-11-19 个人中心页 10012 真实渲染空白修复）：
 * 用假注册表替身记录会员模块实际收到的 props，断言——
 * - 10012 的 6 个模块按序经通用分发器渲染（与首页 10020 同一方案），不再被跳过；
 * - userInfo3 的 immersive 按 MemberDecorate 口径下发（pageModule.immersive +
 *   首个可见模块才为 true，第二个 userInfo3 为 false）。
 */
const received = vi.hoisted(() => ({ props: [] as any[] }));

vi.mock("@tarojs/components", () => ({
    View: ({ children, ...props }: { children?: ReactNode }) => <div {...props}>{children}</div>,
    Image: () => null,
    Swiper: ({ children }: { children?: ReactNode }) => <div>{children}</div>,
    SwiperItem: ({ children }: { children?: ReactNode }) => <div>{children}</div>
}));
vi.mock("@/utils", () => ({ redirect: vi.fn(), getElementRect: vi.fn(async () => ({ height: 0 })) }));
vi.mock("@/utils/format", () => ({ imageFormat: (value: string) => value }));

function fakeMemberModule(type: string) {
    return (props: any) => {
        received.props.push({ type, props });
        return <div data-testid={`member-${type}`}>{type}-ok</div>;
    };
}

vi.mock("@/pages/customPage/modules/registry", () => ({
    MODULE_REGISTRY: {
        navbar: () => <div data-testid="navbar" />,
        userInfo: fakeMemberModule("userInfo"),
        orderEntry: fakeMemberModule("orderEntry"),
        fixedFunctionalArea: fakeMemberModule("fixedFunctionalArea"),
        adImage: fakeMemberModule("adImage"),
        myService: fakeMemberModule("myService"),
        account: fakeMemberModule("account"),
        userInfo3: fakeMemberModule("userInfo3")
    },
    STICKY_REGISTRY: {}
}));

/** 10012 个人中心页已发布 moduleList 的类型与顺序（MCP decorate_get_page 实取，module 内容略省） */
const MODULE_LIST_10012 = [
    { isShow: true, label: "用户信息", type: "userInfo", moduleIndex: 1779777344728, module: { gradient: { colorA: "#fcf6fc", colorB: "#f3f3fe" } } },
    { isShow: true, label: "我的订单", type: "orderEntry", moduleIndex: 1779777347586, module: { title: "我的订单", layoutStyle: 1 } },
    { fixed: true, isShow: true, label: "固定功能区", type: "fixedFunctionalArea", moduleIndex: 1792652265843, module: {} },
    { isShow: true, label: "广告图", type: "adImage", moduleIndex: 1779777372576, module: { pic: { picUrl: "x.png" } } },
    { isShow: true, label: "我的服务", type: "myService", moduleIndex: 1779777374138, module: { moduleTitle: "服务管理", serviceList: [] } },
    { isShow: true, label: "账号", type: "account", moduleIndex: 1779777380846, module: { moduleTitle: "账号信息", accountList: [] } }
];

beforeEach(() => {
    received.props = [];
    vi.spyOn(console, "error").mockImplementation(() => {});
    vi.spyOn(console, "warn").mockImplementation(() => {});
});
afterEach(cleanup);

describe("DecorateModules 会员中心模块分发", () => {
    it("10012 的 6 个模块全部渲染，module 数据原样到达各模块", () => {
        const { getByTestId } = render(<DecorateModules modules={MODULE_LIST_10012} pageModles={{ title: "" }} scrollTop={0} />);
        for (const type of ["userInfo", "orderEntry", "fixedFunctionalArea", "adImage", "myService", "account"]) {
            expect(getByTestId(`member-${type}`)).toBeTruthy();
        }
        const byType = Object.fromEntries(received.props.map((r) => [r.type, r.props]));
        expect(byType.orderEntry.module.title).toBe("我的订单");
        expect(byType.myService.module.moduleTitle).toBe("服务管理");
        expect(byType.account.module.moduleTitle).toBe("账号信息");
    });

    it("userInfo3 immersive：沉浸式开 + 首个可见模块为 true，后续 userInfo3 为 false", () => {
        const modules = [
            { isShow: false, type: "userInfo", moduleIndex: 1, module: {} },
            { isShow: true, type: "userInfo3", moduleIndex: 2, module: {} },
            { isShow: true, type: "userInfo3", moduleIndex: 3, module: {} }
        ];
        render(<DecorateModules modules={modules} pageModles={{ title: "", immersive: true }} scrollTop={0} />);
        const v3 = received.props.filter((r) => r.type === "userInfo3");
        expect(v3).toHaveLength(2);
        expect(v3[0].props.immersive).toBe(true);
        expect(v3[1].props.immersive).toBe(false);
    });

    it("userInfo3 immersive：pageModule.immersive 关时为 false", () => {
        const modules = [{ isShow: true, type: "userInfo3", moduleIndex: 2, module: {} }];
        render(<DecorateModules modules={modules} pageModles={{ title: "", immersive: false }} scrollTop={0} />);
        const v3 = received.props.filter((r) => r.type === "userInfo3");
        expect(v3).toHaveLength(1);
        expect(v3[0].props.immersive).toBe(false);
    });
});
