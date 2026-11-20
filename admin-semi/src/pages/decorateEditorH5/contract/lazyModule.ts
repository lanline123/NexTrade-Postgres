/**
 * 模块预览/设置组件的懒加载（带部署切换自愈，2026-11-18；2026-11-20 改自管理加载器）
 *
 * 背景：admin 每次部署都会换掉全部带 hash 的分片文件。部署前打开的编辑器页面
 * 再点选模块时，去加载旧 hash 分片会 404 抛错，错误边界把设置面板降级为
 * 「渲染失败」（2026-11-18 实测：v52 部署窗口内首焦轮播/快捷导航/商品分组三
 * 模块设置同时报红字；2026-11-20 复发：图文导航设置分片 Settings-*.js 404，
 * 访问日志实证 01:19:02/01:19:58 两次 404、dist 于此前数分钟内被换版）。
 *
 * 旧实现用 React.lazy + 失败逐出缓存，但 React.lazy 会把加载拒绝永久缓存在
 * 组件类型上：错误边界点「重试」只是以新 key 重挂同一组件类型，工厂函数不会
 * 再跑——逐出缓存形同虚设，分片错误判定与整页刷新自愈都只剩首次失败那一次
 * 机会（又被 15 秒防刷新标记挡掉时），面板整场会话卡在红字上（2026-11-20 定案）。
 *
 * 现实现：每个 key 一个稳定的包装组件 + 一份加载记录（pending/done/error）。
 * - pending：渲染时抛 promise，交给 Suspense 显示加载态；
 * - done：直接渲染已加载组件；
 * - error：渲染时先删掉加载记录再把错误抛给错误边界——边界点「重试」重挂
 *   组件时必然重新发起工厂加载：新分片已随新版上线则当场恢复；仍是旧会话
 *   地址、文件已被换版删除时，工厂失败会再次走分片错误判定与整页刷新自愈
 *   （15 秒防循环标记只挡自动刷新，手动重试每次都会重新判定）。
 */
import { createElement, type ComponentType } from "react";
import { getModuleDefinition } from "./moduleRegistry";

type ModuleKind = "preview" | "settings";

interface LoadRecord {
    status: "pending" | "done" | "error";
    component?: ComponentType<any>;
    error?: unknown;
    promise?: Promise<void>;
}

/** 包装组件身份按 key 稳定（调用方 memo/渲染引用不变），加载记录独立存放、失败即删 */
const components: Record<ModuleKind, Map<string, ComponentType<any>>> = {
    preview: new Map(),
    settings: new Map()
};
const records: Record<ModuleKind, Map<string, LoadRecord>> = {
    preview: new Map(),
    settings: new Map()
};

const RELOAD_GUARD_KEY = "h5-editor-chunk-reload-at";

function isChunkLoadError(error: unknown): boolean {
    const message = error instanceof Error ? error.message : String(error ?? "");
    return /loading chunk|chunkloaderror|failed to fetch dynamically imported module|importing a module script failed|unable to preload css/i.test(message);
}

function reloadOnceForStaleChunk(): boolean {
    try {
        const last = Number(sessionStorage.getItem(RELOAD_GUARD_KEY) || 0);
        if (Date.now() - last < 15000) return false;
        sessionStorage.setItem(RELOAD_GUARD_KEY, String(Date.now()));
    } catch {
        return false;
    }
    window.location.reload();
    return true;
}

/** 按 type@url 取懒加载组件（预览与设置共用；加载失败可由错误边界重试真正恢复） */
export function getLazyModuleComponent(kind: ModuleKind, type: string, url?: string): ComponentType<any> | null {
    const key = `${type}@${url ?? ""}`;
    const cache = components[kind];
    const hit = cache.get(key);
    if (hit) return hit;
    const def = getModuleDefinition(type, url);
    if (!def) return null;
    const factory = (kind === "preview" ? def.preview : def.settings) as () => Promise<{ default: ComponentType<any> }>;

    const LazyModuleComponent = (props: Record<string, any>) => {
        let rec = records[kind].get(key);
        if (!rec) {
            const fresh: LoadRecord = { status: "pending" };
            records[kind].set(key, fresh);
            fresh.promise = Promise.resolve()
                .then(factory)
                .then(
                    (mod) => {
                        fresh.status = "done";
                        fresh.component = mod.default;
                    },
                    (error) => {
                        fresh.status = "error";
                        fresh.error = error;
                        if (isChunkLoadError(error)) {
                            reloadOnceForStaleChunk();
                        }
                    }
                );
            rec = fresh;
        }
        if (rec.status === "done") {
            return createElement(rec.component as ComponentType<any>, props);
        }
        if (rec.status === "error") {
            const error = rec.error;
            // 先删记录再抛错：错误边界「重试」重挂本组件时将重新发起工厂加载
            records[kind].delete(key);
            throw error;
        }
        // pending：抛 promise 给 Suspense（与 React.lazy 相同的挂起协议）
        throw rec.promise;
    };
    cache.set(key, LazyModuleComponent);
    return LazyModuleComponent;
}
