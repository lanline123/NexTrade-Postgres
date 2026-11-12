/**
 * 商品详情页「规格 ↔ 地址栏 skuId」同步
 *
 * - 用户点选规格时以 replaceState 写入 ?skuId=，不刷新、不新增后退记录；默认选中不写参数
 * - 带 skuId 打开/刷新/分享时按该 SKU 预选规格；skuId 不属于本商品时回落默认规格并移除参数
 * - 不用 router.replace：商详页监听 route.query.skuId 变化会整页重拉详情
 */
import type { InjectionKey, Ref } from "vue";
import type { RouteLocationNormalizedLoaded } from "vue-router";

interface SkuUrlContext {
    /** 是否同步：拼团模式下购买按钮直接读取 route.query.skuId，不接入同步 */
    enabled: Ref<boolean>;
    /** 地址栏中属于本商品的 skuId，规格区挂载（含批发页签切回后重挂）时据此预选；未带或未命中为空串 */
    urlSkuId: Ref<string>;
}

const skuUrlKey = Symbol("productSkuUrl") as InjectionKey<SkuUrlContext>;

/** 地址栏 skuId：客户端读实时地址（选规格经 replaceState 写入，route.query 不跟随），服务端读请求路由 */
export const readUrlSkuId = (route: RouteLocationNormalizedLoaded): string => {
    if (import.meta.client) return new URL(window.location.href).searchParams.get("skuId")?.trim() ?? "";
    const raw = route.query.skuId;
    return String((Array.isArray(raw) ? raw[0] : raw) ?? "").trim();
};

/**
 * 改写地址栏 skuId，空值即移除。
 * 同步改 history.state.current，否则 vue-router 下次跳转前会按旧地址回写当前记录，后退时丢失 skuId。
 */
export const replaceUrlSkuId = (skuId: string | number | null | undefined): void => {
    if (!import.meta.client) return;
    const value = skuId === undefined || skuId === null ? "" : String(skuId);
    const url = new URL(window.location.href);
    if (value) {
        url.searchParams.set("skuId", value);
    } else {
        url.searchParams.delete("skuId");
    }
    if (url.href === window.location.href) return;
    // state.current 不含 baseURL，只替换其中的查询串
    const prevCurrent = String(window.history.state?.current ?? "");
    const currentPath = prevCurrent ? prevCurrent.replace(/[?#].*$/, "") : url.pathname;
    window.history.replaceState({ ...window.history.state, current: `${currentPath}${url.search}${url.hash}` }, "", url.href);
};

/** 商详页：下发地址栏规格状态（仅普通商品详情调用，积分兑换等不接入） */
export const useProvideSkuUrl = () => {
    // 在 setup 同步阶段取路由：applyUrlSkuId 在异步请求之后调用，服务端届时已拿不到 Nuxt 上下文
    const route = useRoute();
    const context: SkuUrlContext = {
        enabled: ref(false),
        urlSkuId: ref("")
    };
    provide(skuUrlKey, context);

    /** 详情数据就绪后调用：地址栏 skuId 命中则用于预选，否则从地址栏移除；enabled 为 false 时不预选也不改地址栏 */
    const applyUrlSkuId = (skuList: { skuId?: number | string | null }[] | undefined, enabled: boolean) => {
        context.enabled.value = enabled;
        if (!enabled) {
            context.urlSkuId.value = "";
            return;
        }
        const urlSkuId = readUrlSkuId(route);
        const matched = !!urlSkuId && (skuList || []).some((item) => String(item.skuId ?? "") === urlSkuId);
        context.urlSkuId.value = matched ? urlSkuId : "";
        if (urlSkuId && !matched) replaceUrlSkuId("");
    };

    return { applyUrlSkuId };
};

/** 规格区：读取预选 skuId、在用户点选后写回地址栏；商详页外的复用场景下均为空操作 */
export const useSkuUrlSync = () => {
    const context = inject(skuUrlKey, null);
    return {
        urlSkuId: () => context?.urlSkuId.value ?? "",
        syncSkuId: (skuId: string | number | null | undefined) => {
            if (!context?.enabled.value) return;
            context.urlSkuId.value = skuId === undefined || skuId === null ? "" : String(skuId);
            replaceUrlSkuId(context.urlSkuId.value);
        }
    };
};
