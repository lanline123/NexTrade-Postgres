import { computed, onBeforeUnmount, onMounted, provide, ref, watch, type InjectionKey, type Ref } from "vue";
import { useCommonStore } from "~/store/common";
import { useUserStore } from "~/store/user";
import { getBaseConfig } from "~/api/common";
import { requestHeader, requestPrefix } from "~/utils/request";
import { message } from "ant-design-vue";
import {
    canDownloadRawImages, fetchRawImageArchive, normalizeRawImageConfig, rawImageContextKey, rawImageDownloadUrl,
    RawImageDownloadError, saveRawImageBlob, validRawImageIntent, type RawImageContext
} from "~/utils/productRawImageDownload";

const intentKey = "product-raw-image-download-intent";
export const rawImageDownloadKey: InjectionKey<{
    visible: Ref<boolean>;
    busy: Ref<boolean>;
    download: () => Promise<void>;
}> = Symbol("productRawImageDownload");

export function useProductRawImageDownload(context: Ref<RawImageContext | null>, reloadDetail: () => Promise<unknown>) {
    const common = useCommonStore();
    const user = useUserStore();
    const route = useRoute();
    const router = useRouter();
    const { t } = useI18n();
    const busy = ref(false);
    const visible = computed(() => !!context.value && canDownloadRawImages(common, "pc"));
    let controller: AbortController | undefined;
    let generation = 0;
    let disposed = false;
    const releases = new Set<() => void>();
    const cancel = () => {
        generation++;
        controller?.abort();
        controller = undefined;
        busy.value = false;
    };
    const refreshConfig = async () => {
        try {
            Object.assign(common, normalizeRawImageConfig(await getBaseConfig()));
        } catch {
            Object.assign(common, normalizeRawImageConfig());
        }
    };
    const login = async (snapshot: RawImageContext) => {
        try {
            sessionStorage.setItem(intentKey, JSON.stringify({
                route: route.fullPath, context: rawImageContextKey(snapshot), expires: Date.now() + 600000
            }));
        } catch { /* Login remains available without session storage. */ }
        user.clearToken();
        await router.push(`/member/login?returnUrl=${encodeURIComponent(route.fullPath)}`);
    };
    const download = async (resumed = false) => {
        if (disposed || busy.value || !visible.value || !context.value) return;
        const snapshot = { ...context.value };
        if (!user.isAuthenticated) {
            if (!resumed) await login(snapshot);
            return;
        }
        busy.value = true;
        const current = ++generation;
        const abort = new AbortController();
        controller = abort;
        let timedOut = false;
        const timeout = setTimeout(() => { timedOut = true; abort.abort(); }, 120000);
        try {
            const result = await fetchRawImageArchive(
                rawImageDownloadUrl(requestPrefix, snapshot), requestHeader(), abort.signal, snapshot.id
            );
            if (current !== generation || disposed) return;
            const revoke = saveRawImageBlob(result.blob, result.filename);
            const timer = setTimeout(release, 30000);
            function release() { clearTimeout(timer); revoke(); releases.delete(release); }
            releases.add(release);
            message.success(t("已发起下载"));
        } catch (error) {
            if (current !== generation || disposed) return;
            if (error instanceof RawImageDownloadError && error.code === 401) {
                user.clearToken();
                if (!resumed) await login(snapshot);
                else message.error(t("登录已失效，请重新登录"));
            } else {
                message.error(timedOut ? t("素材下载超时，请重试") : error instanceof RawImageDownloadError
                    ? error.serverMessage ? error.message : t(error.message) : t("素材下载失败，请重试"));
                if (error instanceof RawImageDownloadError) {
                    await refreshConfig();
                    if (current === generation && error.code === 1001) await reloadDetail();
                }
            }
        } finally {
            clearTimeout(timeout);
            if (current === generation) { busy.value = false; controller = undefined; }
        }
    };
    watch(() => rawImageContextKey(context.value), cancel, { flush: "sync" });
    watch(() => route.fullPath, cancel, { flush: "sync" });
    onMounted(async () => {
        let intent: unknown;
        try {
            intent = JSON.parse(sessionStorage.getItem(intentKey) || "null");
            sessionStorage.removeItem(intentKey);
        } catch { return; }
        if (!intent || !user.isAuthenticated) return;
        const startingRoute = route.fullPath;
        await refreshConfig();
        if (disposed || startingRoute !== route.fullPath) return;
        await reloadDetail();
        if (!disposed && context.value && validRawImageIntent(intent, route.fullPath, context.value)) await download(true);
    });
    onBeforeUnmount(() => {
        disposed = true;
        cancel();
        releases.forEach((release) => release());
    });
    const state = { visible, busy, download: () => download(false) };
    provide(rawImageDownloadKey, state);
    return state;
}
