import { useCallback, useEffect, useRef, useState } from "react";
import { View } from "@tarojs/components";
import { useRouter } from "@tarojs/taro";

import DecorateModules from "@/pages/customPage/components/DecorateModules";
import { MODULE_REGISTRY } from "@/pages/customPage/modules/registry";
import ConfigPagePreview from "./configPages";
import { configPageSlotIndices, resolveConfigPageBranch } from "./configPages/resolve";
import { getIndex } from "@/api/home/home";
import { getShopDetail } from "@/api/shop/shop";
import type { ShopDetailItem } from "@/types/shop/shop";

import "./index.scss";

/**
 * 装修编辑器嵌入预览页（2026-11-17 用户拍板：H5 编辑器画布改 iframe 嵌入真实前台）。
 *
 * 本页只在 H5 端被 admin 编辑器以 iframe 方式打开（/mobile/pages/decoratePreview/index），
 * 不进任何导航/Tabbar，不从接口取装修数据：
 * - 父页面（编辑器）通过 postMessage 发 `decorate-data` { moduleList, pageModule }，本页用与
 *   线上首页/自定义页完全相同的 DecorateModules 分发器渲染，数据变了就重渲染（不刷新）。
 * - 本页回 `decorate-ready`（挂载完成）与 `decorate-layout` { items: [{index, top, height}], totalHeight }，
 *   父页面据此在 iframe 上盖覆盖层做点选/拖拽（iframe 本身纯展示，页内点击一律拦截不跳转）。
 * - 另回 `decorate-rendered` { seq, count, totalHeight }（2026-11-19 管理端「加载门」配合项）：收到数据并
 *   完成首帧渲染（React 已提交 + 双 rAF 已过 + 槽位 DOM 已存在）后才发，每批新数据发一次；
 *   `decorate-ready` 只是挂载信号、内容可能还没渲染，加载门应以本信号为准。seq 为所应用那份
 *   `decorate-data` 的下发序号（管理端递增下发、本页原样回带，精确对齐用）；`decorate-layout` 同样回带 seq。
 * - 兜底/自检：带 `?id=<decorateId>` 打开时走与 customPage 相同的 getIndex 拉已发布数据渲染
 *   （postMessage 数据一旦到达即覆盖并锁定，编辑器实时数据优先）。
 * - 店铺上下文【原版有·我方改口径】（2026-11-19）：原版店铺页由页面自身拉店铺详情后
 *   经分发器传 shopInfo/shopId；本嵌入页没有页面级店铺上下文，改为按编辑器在
 *   decorate-data 里下发的 shopId（或 ?shopId= 参数）拉 getShopDetail 后透传给
 *   DecorateModules，口径与 ShopModules 一致。此前不传曾让 shopInfo 模块无守卫
 *   直读崩溃、含 shopInfo 的页面（如 10079 店铺模板）真实渲染整页空白。
 * - 整页配置型页型【原版有·我方改口径】（2026-11-19）：登录(5)/搜索(6)/商品详情(8)
 *   的模块是整页配置（前台页面本体消费 module 作配置、无积木渲染器），通用分发
 *   器无从渲染。此前 decorate-data 不带 pageType、本页无从分支，三类页真实渲染
 *   整页空白（10103/10104/10105 实证）。现编辑器下发 pageType，本页经
 *   configPages/resolve 判定命中后改走整页分支（configPages/）：以编辑器实时
 *   数据作配置渲染真实页面组件；整页占 1 个槽位（配置模块下标），布局/就绪
 *   回报口径不变。不命中（其它页型/配置模块缺失）回退通用分发，行为不变。
 *
 * 通信安全：只接受 ALLOWED_ORIGINS 的消息；本机 localhost 调试时放行任意来源（仅按 hostname 判断）。
 * 方案：~/workspace/editor-unify/iframe-canvas-design.md
 */

const ALLOWED_ORIGINS = ["https://admin.recy.cc", "https://battery.recy.cc"];

function isAllowedOrigin(origin: string): boolean {
    if (ALLOWED_ORIGINS.includes(origin)) return true;
    const host = window.location.hostname;
    return host === "localhost" || host === "127.0.0.1";
}

/** 与 DecorateModules 的分发条件保持一致：这些模块会产生一个画布槽位（catNav 在嵌入态永不切分类，恒为初始态） */
const SPECIAL_SLOT_TYPES = new Set(["searchBar", "searchBar2", "catNav"]);

function visibleSlotIndices(modules: any[]): number[] {
    const indices: number[] = [];
    modules.forEach((mod, i) => {
        if (!mod?.isShow) return;
        if (SPECIAL_SLOT_TYPES.has(mod.type) || MODULE_REGISTRY[mod.type]) indices.push(i);
    });
    return indices;
}

/**
 * 当前数据的槽位下标：整页配置型分支命中时只有配置模块 1 个槽位，
 * 否则走常规口径。布局测量与就绪计数共用，保证两处一致。
 */
function slotIndicesFor(pageType: number, modules: any[]): number[] {
    return configPageSlotIndices(pageType, modules) ?? visibleSlotIndices(modules);
}

export default function DecoratePreview() {
    const router = useRouter();
    const [modulesData, setModulesData] = useState<any[]>([]);
    const [pageModule, setPageModule] = useState<any>({});
    const [received, setReceived] = useState(false);
    /** 编辑器下发的页面类型（整页配置型分支判定用；旧编辑器不下发时为 0，恒走通用分发） */
    const [pageType, setPageType] = useState(0);
    const [shopId, setShopId] = useState(0);
    const [shopInfo, setShopInfo] = useState<ShopDetailItem | undefined>(undefined);
    const rootRef = useRef<any>(null);
    const parentOriginRef = useRef<string>("*");
    const parentDataRef = useRef(false);
    const shopIdRef = useRef(0);
    const shopFetchTokenRef = useRef(0);
    const renderedForRef = useRef<any[] | null>(null);
    const seqRef = useRef<number | null>(null);
    const stateRef = useRef({ modulesData, pageModule, received, pageType });
    stateRef.current = { modulesData, pageModule, received, pageType };

    const postToParent = useCallback((msg: Record<string, any>) => {
        try {
            window.parent?.postMessage(msg, parentOriginRef.current);
        } catch {
            /* 父页面已关闭等场景忽略 */
        }
    }, []);

    // 接收编辑器数据 + 挂载后通知 ready
    useEffect(() => {
        const onMessage = (e: MessageEvent) => {
            if (!isAllowedOrigin(e.origin)) return;
            const data = e.data as any;
            if (!data || data.type !== "decorate-data") return;
            parentOriginRef.current = e.origin;
            parentDataRef.current = true;
            if (typeof data.seq === "number") seqRef.current = data.seq;
            if (Array.isArray(data.moduleList)) setModulesData(data.moduleList);
            if (data.pageModule && typeof data.pageModule === "object") setPageModule(data.pageModule);
            // 页面类型（整页配置型分支判定；旧编辑器不下发时保持 0，向后兼容）
            if (typeof data.pageType === "number") setPageType(data.pageType);
            // 店铺上下文（向后兼容：旧编辑器不下发 shopId 时保持 0，模块走守卫空态）
            if (typeof data.shopId === "number" && data.shopId !== shopIdRef.current) {
                shopIdRef.current = data.shopId;
                setShopId(data.shopId);
            }
            setReceived(true);
        };
        window.addEventListener("message", onMessage);
        postToParent({ type: "decorate-ready" });
        return () => window.removeEventListener("message", onMessage);
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // 按 shopId 拉店铺详情：令牌防竞态（后发起的请求优先，迟到的旧结果丢弃）；
    // 失败只记日志不崩页（shopInfo 模块有守卫，缺失时渲染结构空态）
    const loadShopDetail = useCallback(async (id: number) => {
        if (!id || id <= 0) return;
        const token = ++shopFetchTokenRef.current;
        try {
            const result: any = await getShopDetail(id);
            if (shopFetchTokenRef.current !== token) return;
            if (result) setShopInfo(result as ShopDetailItem);
        } catch (e) {
            console.error("[decoratePreview] getShopDetail failed", e);
        }
    }, []);

    useEffect(() => {
        if (shopId > 0) void loadShopDetail(shopId);
    }, [shopId, loadShopDetail]);

    /** shopInfo 模块收藏后回调：重拉当前店铺详情（与店铺页 refreshShopDetail 同口径） */
    const handleRefreshShopDetail = useCallback(
        (id?: number) => {
            const target = id && id > 0 ? id : shopIdRef.current;
            if (target > 0) void loadShopDetail(target);
        },
        [loadShopDetail]
    );

    // ?id= 兜底：拉已发布页数据（与 customPage 同一接口）；编辑器 postMessage 到达后不再覆盖
    useEffect(() => {
        const id = Number(router.params.id);
        // ?shopId= 自检参数：直接打开预览页时也能拉真实店铺详情
        const paramShopId = Number(router.params.shopId);
        if (paramShopId > 0 && !parentDataRef.current) {
            shopIdRef.current = paramShopId;
            setShopId(paramShopId);
        }
        // ?pageType= 自检参数：直接打开预览页时也能命中整页配置型分支
        const paramPageType = Number(router.params.pageType);
        if (paramPageType > 0 && !parentDataRef.current) setPageType(paramPageType);
        if (!id) return;
        void (async () => {
            try {
                const res: any = await getIndex(id);
                if (parentDataRef.current || !res) return;
                if (Array.isArray(res.moduleList)) setModulesData(res.moduleList);
                if (res.pageModule) setPageModule(res.pageModule);
                if (Number(res.pageType) > 0) setPageType(Number(res.pageType));
                if (Number(res.shopId) > 0) {
                    shopIdRef.current = Number(res.shopId);
                    setShopId(Number(res.shopId));
                }
                setReceived(true);
            } catch (e) {
                console.error("[decoratePreview] getIndex failed", e);
            }
        })();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [router.params.id]);

    // 嵌入态隐藏全部滚动条（用户 2026-11-19：编辑器中间 iframe 里那条滚动栏不需要）：
    // embed=1 或身处 iframe 时给 <html> 打标，index.scss 据此隐藏条、保留滚动（同分类页口径）；
    // 直接开标签页（?id= 自检）不打标，滚动条保留便于调试。
    useEffect(() => {
        const embedded = router.params.embed === "1" || window.self !== window.top;
        if (!embedded) return;
        document.documentElement.classList.add("decorate-preview-embed");
        return () => {
            document.documentElement.classList.remove("decorate-preview-embed");
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // 嵌入态纯展示：拦截一切点击/双击，避免模块内链接在 iframe 里真的跳转；
    // submit 一并拦截（整页分支的登录页含真实表单，Enter 提交不能真的发登录请求）
    useEffect(() => {
        const block = (e: Event) => {
            e.preventDefault();
            e.stopPropagation();
        };
        document.addEventListener("click", block, true);
        document.addEventListener("dblclick", block, true);
        document.addEventListener("submit", block, true);
        return () => {
            document.removeEventListener("click", block, true);
            document.removeEventListener("dblclick", block, true);
            document.removeEventListener("submit", block, true);
        };
    }, []);

    // 布局上报：DecorateModules 根节点的子节点与可见模块槽位一一对应（pageModule 标题 navbar 多占一个首子节点）；
    // 整页配置型分支时容器是分支包裹节点、其唯一子节点即整页内容，槽位只有配置模块一个、无 navbar 偏移
    const reportLayout = useCallback(() => {
        const { modulesData: mods, pageModule: pm, received: got, pageType: pt } = stateRef.current;
        if (!got) return;
        const root = rootRef.current as HTMLElement | undefined;
        const container = (root?.firstElementChild as HTMLElement | undefined) ?? undefined;
        if (!container) return;
        const containerRect = container.getBoundingClientRect();
        const branch = resolveConfigPageBranch(pt, mods);
        const slots = slotIndicesFor(pt, mods);
        const offset = branch ? 0 : pm && pm.title ? 1 : 0;
        const items = slots
            .map((moduleIndex, i) => {
                const child = container.children[i + offset] as HTMLElement | undefined;
                if (!child) return null;
                const r = child.getBoundingClientRect();
                return {
                    index: moduleIndex,
                    top: Math.round(r.top - containerRect.top),
                    height: Math.round(r.height)
                };
            })
            .filter(Boolean);
        postToParent({
            type: "decorate-layout",
            seq: seqRef.current,
            items,
            totalHeight: Math.round(containerRect.height)
        });
    }, [postToParent]);

    // 首帧就绪信号 decorate-rendered：数据到达 + React 提交 + 双 rAF（首帧已绘制）+ 槽位 DOM 已存在，
    // 四个条件齐了才发；同一批数据只发一次，编辑器后续推新数据（新引用）会再发一次。
    // 槽位 DOM 迟迟未出现时限次重试（整页分支组件懒加载 + 异步取数，首帧检查时
    // 子节点可能尚未挂载；常规路径模块同为 lazy，此重试对两条路径一致生效）。
    useEffect(() => {
        if (!received) return;
        if (renderedForRef.current === modulesData) return;
        let cancelled = false;
        let raf1 = 0;
        let raf2 = 0;
        let retryTimer: ReturnType<typeof setTimeout> | undefined;
        let attempts = 0;
        const attempt = () => {
            raf1 = requestAnimationFrame(() => {
                raf2 = requestAnimationFrame(() => {
                    if (cancelled) return;
                    const root = rootRef.current as HTMLElement | undefined;
                    const container = (root?.firstElementChild as HTMLElement | undefined) ?? undefined;
                    if (!container || container.children.length === 0) {
                        if (++attempts <= 10) retryTimer = setTimeout(attempt, 300);
                        return;
                    }
                    renderedForRef.current = modulesData;
                    postToParent({
                        type: "decorate-rendered",
                        seq: seqRef.current,
                        count: slotIndicesFor(pageType, modulesData).length,
                        totalHeight: Math.round(container.getBoundingClientRect().height)
                    });
                });
            });
        };
        attempt();
        return () => {
            cancelled = true;
            cancelAnimationFrame(raf1);
            cancelAnimationFrame(raf2);
            if (retryTimer) clearTimeout(retryTimer);
        };
    }, [received, modulesData, pageModule, pageType, postToParent]);

    useEffect(() => {
        if (!received) return;
        const timers = [setTimeout(reportLayout, 100), setTimeout(reportLayout, 600), setTimeout(reportLayout, 1500)];
        const interval = setInterval(reportLayout, 2000);
        window.addEventListener("resize", reportLayout);
        return () => {
            timers.forEach(clearTimeout);
            clearInterval(interval);
            window.removeEventListener("resize", reportLayout);
        };
    }, [received, modulesData, pageModule, reportLayout]);

    // 整页配置型分支（登录/搜索/商品详情）：命中时整页改走真实页面分支，
    // 不经通用分发器；不命中与此前完全一致。
    const configBranch = received ? resolveConfigPageBranch(pageType, modulesData) : null;

    return (
        <View
            ref={rootRef as any}
            className="decorate-preview"
            style={{ backgroundColor: pageModule?.backgroundColor || "#ffffff" }}
        >
            {received ? (
                configBranch ? (
                    <ConfigPagePreview branch={configBranch} />
                ) : (
                    <DecorateModules
                        modules={modulesData}
                        pageModles={pageModule}
                        scrollTop={0}
                        shopInfo={shopInfo}
                        shopId={shopId}
                        onRefreshShopDetail={handleRefreshShopDetail}
                        moduleErrorPlaceholder
                    />
                )
            ) : (
                <View className="decorate-preview__waiting">等待编辑器数据…</View>
            )}
        </View>
    );
}
