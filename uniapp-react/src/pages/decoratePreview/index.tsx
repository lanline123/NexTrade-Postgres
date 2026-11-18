import { useCallback, useEffect, useRef, useState } from "react";
import { View } from "@tarojs/components";
import { useRouter } from "@tarojs/taro";

import DecorateModules from "@/pages/customPage/components/DecorateModules";
import { MODULE_REGISTRY } from "@/pages/customPage/modules/registry";
import { getIndex } from "@/api/home/home";

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
 * - 兜底/自检：带 `?id=<decorateId>` 打开时走与 customPage 相同的 getIndex 拉已发布数据渲染
 *   （postMessage 数据一旦到达即覆盖并锁定，编辑器实时数据优先）。
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

export default function DecoratePreview() {
    const router = useRouter();
    const [modulesData, setModulesData] = useState<any[]>([]);
    const [pageModule, setPageModule] = useState<any>({});
    const [received, setReceived] = useState(false);
    const rootRef = useRef<any>(null);
    const parentOriginRef = useRef<string>("*");
    const parentDataRef = useRef(false);
    const stateRef = useRef({ modulesData, pageModule, received });
    stateRef.current = { modulesData, pageModule, received };

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
            if (Array.isArray(data.moduleList)) setModulesData(data.moduleList);
            if (data.pageModule && typeof data.pageModule === "object") setPageModule(data.pageModule);
            setReceived(true);
        };
        window.addEventListener("message", onMessage);
        postToParent({ type: "decorate-ready" });
        return () => window.removeEventListener("message", onMessage);
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // ?id= 兜底：拉已发布页数据（与 customPage 同一接口）；编辑器 postMessage 到达后不再覆盖
    useEffect(() => {
        const id = Number(router.params.id);
        if (!id) return;
        void (async () => {
            try {
                const res: any = await getIndex(id);
                if (parentDataRef.current || !res) return;
                if (Array.isArray(res.moduleList)) setModulesData(res.moduleList);
                if (res.pageModule) setPageModule(res.pageModule);
                setReceived(true);
            } catch (e) {
                console.error("[decoratePreview] getIndex failed", e);
            }
        })();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [router.params.id]);

    // 嵌入态纯展示：拦截一切点击/双击，避免模块内链接在 iframe 里真的跳转
    useEffect(() => {
        const block = (e: Event) => {
            e.preventDefault();
            e.stopPropagation();
        };
        document.addEventListener("click", block, true);
        document.addEventListener("dblclick", block, true);
        return () => {
            document.removeEventListener("click", block, true);
            document.removeEventListener("dblclick", block, true);
        };
    }, []);

    // 布局上报：DecorateModules 根节点的子节点与可见模块槽位一一对应（pageModule 标题 navbar 多占一个首子节点）
    const reportLayout = useCallback(() => {
        const { modulesData: mods, pageModule: pm, received: got } = stateRef.current;
        if (!got) return;
        const root = rootRef.current as HTMLElement | undefined;
        const container = (root?.firstElementChild as HTMLElement | undefined) ?? undefined;
        if (!container) return;
        const containerRect = container.getBoundingClientRect();
        const slots = visibleSlotIndices(mods);
        const offset = pm && pm.title ? 1 : 0;
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
            items,
            totalHeight: Math.round(containerRect.height)
        });
    }, [postToParent]);

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

    return (
        <View
            ref={rootRef as any}
            className="decorate-preview"
            style={{ backgroundColor: pageModule?.backgroundColor || "#ffffff" }}
        >
            {received ? (
                <DecorateModules modules={modulesData} pageModles={pageModule} scrollTop={0} />
            ) : (
                <View className="decorate-preview__waiting">等待编辑器数据…</View>
            )}
        </View>
    );
}
