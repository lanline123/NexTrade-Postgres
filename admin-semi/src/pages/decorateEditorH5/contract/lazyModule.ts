/**
 * 模块预览/设置组件的懒加载（带部署切换自愈，2026-11-18）
 *
 * 背景：admin 每次部署都会换掉全部带 hash 的分片文件。部署前打开的编辑器页面
 * 再点选模块时，lazy() 去加载旧 hash 分片会 404 抛错，错误边界把设置面板降级为
 * 「渲染失败」且整场会话不恢复（2026-11-18 实测：v52 部署窗口内首焦轮播/快捷导航/
 * 商品分组三模块设置同时报红字，刷新后恢复）。
 *
 * 对策：
 * 1. 分片加载失败时先把该条缓存逐出，错误边界点「重试」可重新发起加载；
 * 2. 若失败是分片加载类错误（chunk 404 / 动态导入失败）且 15 秒内未自动刷新过，
 *    直接整页刷新一次拉新版入口（带 sessionStorage 防循环标记），用户无感恢复。
 */
import { lazy, type ComponentType } from "react";
import { getModuleDefinition } from "./moduleRegistry";

type ModuleKind = "preview" | "settings";

const caches: Record<ModuleKind, Map<string, ComponentType<any>>> = {
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

/** 按 type@url 取懒加载组件（预览与设置共用；失败自动逐出缓存，可重试） */
export function getLazyModuleComponent(kind: ModuleKind, type: string, url?: string): ComponentType<any> | null {
    const key = `${type}@${url ?? ""}`;
    const cache = caches[kind];
    const hit = cache.get(key);
    if (hit) return hit;
    const def = getModuleDefinition(type, url);
    if (!def) return null;
    const factory = (kind === "preview" ? def.preview : def.settings) as () => Promise<{ default: ComponentType<any> }>;
    const component = lazy(async () => {
        try {
            return await factory();
        } catch (error) {
            cache.delete(key);
            if (isChunkLoadError(error)) {
                reloadOnceForStaleChunk();
            }
            throw error;
        }
    });
    cache.set(key, component);
    return component;
}
