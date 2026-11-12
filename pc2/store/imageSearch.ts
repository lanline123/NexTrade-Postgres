import { getImageSearchCapabilities } from "~/api/search/imageSearch";
import { useUserStore } from "~/store/user";
import { loadImageSearchFile, removeImageSearchFile, saveImageSearchFile } from "~/utils/imageSearchFile";

/** 最近一次图片查询：编号与缩略图存在浏览器会话里，原图另存在本地，刷新后仍能展示并重新搜索 */
interface ImageSearchQueryState {
    /** 图搜编号；空表示图片已选定、还没有完成检索 */
    searchToken: string;
    /** 本地暂存的原图标识，用于确认暂存的图片属于这次查询 */
    fileId: string;
    /** 缩小后的预览图，原图无法在浏览器内解码时为空 */
    thumb: string;
    /** 发起查询时的身份标记，登录状态变化后旧查询作废 */
    identity: string;
}

const QUERY_STORAGE_KEY = "imageSearchQuery";
const THUMB_MAX_SIDE = 320;
// 暂存的原图不属于当前页面的查询时，超过这个时间才清理，避免清掉其它标签页还在用的图片
const STALE_FILE_MS = 2 * 60 * 60 * 1000;

/** 浏览器内无法解码预览、但接口支持的格式 */
const NO_PREVIEW_TYPES = ["image/tiff"];

const FORMAT_MIME_MAP: Record<string, string[]> = {
    JPEG: ["image/jpeg"],
    JPG: ["image/jpeg"],
    PNG: ["image/png"],
    WEBP: ["image/webp"],
    BMP: ["image/bmp", "image/x-ms-bmp"],
    GIF: ["image/gif"],
    TIFF: ["image/tiff"]
};

const FORMAT_EXT_MAP: Record<string, string[]> = {
    JPEG: [".jpg", ".jpeg"],
    JPG: [".jpg", ".jpeg"],
    PNG: [".png"],
    WEBP: [".webp"],
    BMP: [".bmp"],
    GIF: [".gif"],
    TIFF: [".tif", ".tiff"]
};

/** 身份标记：只用于比较前后是否同一身份，不保存登录令牌本身 */
const buildIdentity = (token: string | null | undefined): string => {
    if (!token) return "guest";
    let hash = 5381;
    for (let i = 0; i < token.length; i++) {
        hash = ((hash << 5) + hash + token.charCodeAt(i)) | 0;
    }
    return `user:${hash}`;
};

const createFileId = (): string => `${Date.now().toString(36)}${Math.random().toString(36).slice(2, 10)}`;

/** 生成用于恢复展示的缩略图，保持完整画面等比缩小 */
const createThumb = (file: File): Promise<string> => {
    return new Promise((resolve) => {
        if (NO_PREVIEW_TYPES.includes(file.type)) return resolve("");
        const url = URL.createObjectURL(file);
        const image = new Image();
        image.onload = () => {
            try {
                const scale = Math.min(1, THUMB_MAX_SIDE / Math.max(image.naturalWidth, image.naturalHeight));
                const canvas = document.createElement("canvas");
                canvas.width = Math.max(1, Math.round(image.naturalWidth * scale));
                canvas.height = Math.max(1, Math.round(image.naturalHeight * scale));
                const context = canvas.getContext("2d");
                if (!context) return resolve("");
                context.fillStyle = "#fff";
                context.fillRect(0, 0, canvas.width, canvas.height);
                context.drawImage(image, 0, 0, canvas.width, canvas.height);
                resolve(canvas.toDataURL("image/jpeg", 0.82));
            } catch {
                resolve("");
            } finally {
                URL.revokeObjectURL(url);
            }
        };
        image.onerror = () => {
            URL.revokeObjectURL(url);
            resolve("");
        };
        image.src = url;
    });
};

/**
 * 以图搜图状态
 * - 本端是否可用：由后台开关决定，开关未取到或读取失败时按不可用处理，入口不显示
 * - 店铺内入口还要看店铺开关，关闭后只影响店铺内搜索，全站入口不受影响
 * - 最近一次图片查询对应的图片：编号由结果页地址携带，图片用于展示、发起检索和失效后重新搜索
 */
export const useImageSearchStore = defineStore("imageSearch", () => {
    const userStore = useUserStore();

    // ===================== 能力与开关 =====================
    // 全站入口是否可用：总开关与 PC 开关
    const available = ref(false);
    // 店铺开关；旧接口没有这个字段时按开启处理
    const shopEnabled = ref(false);
    // 店铺内入口是否可用
    const shopAvailable = computed(() => available.value && shopEnabled.value);
    // 结果页当前是否为店铺内搜索，图搜接口返回功能关闭时据此判断关闭的范围
    const resultInShop = ref(false);
    const formats = ref<string[]>([]);
    const capabilitiesLoaded = ref(false);
    let pending: Promise<void> | null = null;

    /**
     * 读取后台开关
     * @param force - 忽略已读取的结果重新读取，接口返回功能关闭后用来同步最新配置
     */
    const fetchCapabilities = (force = false): Promise<void> => {
        if (!import.meta.client) {
            capabilitiesLoaded.value = true;
            return Promise.resolve();
        }
        if (capabilitiesLoaded.value && !force) return Promise.resolve();
        if (pending) return pending;
        pending = getImageSearchCapabilities()
            .then((result) => {
                available.value = !!result?.pcEnabled;
                shopEnabled.value = result?.shopEnabled !== false;
                formats.value = Array.isArray(result?.formats) ? result.formats.map((item) => String(item).toUpperCase()) : [];
                capabilitiesLoaded.value = true;
            })
            .catch(() => {
                // 首次读取失败按不可用处理，下次进入有入口的页面会重试；之后重新读取失败时保留当前结果
                if (!capabilitiesLoaded.value) {
                    available.value = false;
                    shopEnabled.value = false;
                }
            })
            .finally(() => {
                pending = null;
            });
        return pending;
    };

    /** 按搜索范围判断入口是否可用 */
    const isAvailable = (inShop: boolean): boolean => (inShop ? shopAvailable.value : available.value);

    /**
     * 接口返回功能已关闭：先按关闭处理，入口隐藏、结果页退出；再重新读取开关，以最新配置刷新界面
     * 开关只在页面加载时读取一次，不在点击入口或进入结果页时重复读取
     * @param inShop - 出错的是店铺内搜索时只关闭店铺内入口，全站入口不闪烁，重新读取失败也不受影响
     */
    const markUnavailable = (inShop = false): void => {
        if (inShop) shopEnabled.value = false;
        else available.value = false;
        void fetchCapabilities(true);
    };

    /** 文件选择框的 accept */
    const accept = computed(() => {
        const list = formats.value.flatMap((format) => [...(FORMAT_MIME_MAP[format] ?? []), ...(FORMAT_EXT_MAP[format] ?? [])]);
        return list.length > 0 ? Array.from(new Set(list)).join(",") : "image/*";
    });

    /** 是否为接口支持的图片格式；接口未返回格式清单时只要求是图片 */
    const isSupportedFile = (file: File): boolean => {
        const name = file.name.toLowerCase();
        if (formats.value.length === 0) return file.type.startsWith("image/");
        return formats.value.some((format) => {
            if (file.type) return (FORMAT_MIME_MAP[format] ?? []).includes(file.type);
            return (FORMAT_EXT_MAP[format] ?? []).some((ext) => name.endsWith(ext));
        });
    };

    const canPreview = (file: File): boolean => !NO_PREVIEW_TYPES.includes(file.type);

    // ===================== 当前图片查询 =====================
    const query = ref<ImageSearchQueryState | null>(null);
    /** 本次查询的原图，用于发起检索和编号失效后重新搜索 */
    const file = shallowRef<File | null>(null);
    /** 原图的本地预览地址 */
    const previewUrl = ref("");
    let restoring: Promise<void> | null = null;

    const currentIdentity = computed(() => buildIdentity(userStore.token));

    const persist = (): void => {
        try {
            if (query.value) {
                sessionStorage.setItem(QUERY_STORAGE_KEY, JSON.stringify(query.value));
            } else {
                sessionStorage.removeItem(QUERY_STORAGE_KEY);
            }
        } catch {
            // 存储不可用时只影响刷新后的缩略图恢复
        }
    };

    const setFile = (source: File): void => {
        if (file.value === source) return;
        if (previewUrl.value) URL.revokeObjectURL(previewUrl.value);
        previewUrl.value = canPreview(source) ? URL.createObjectURL(source) : "";
        file.value = source;
    };

    const readSavedQuery = (): ImageSearchQueryState | null => {
        try {
            const saved = JSON.parse(sessionStorage.getItem(QUERY_STORAGE_KEY) || "null");
            if (!saved || !/^([a-f0-9]{32})?$/.test(String(saved.searchToken ?? ""))) return null;
            return {
                searchToken: String(saved.searchToken ?? ""),
                fileId: String(saved.fileId ?? ""),
                thumb: typeof saved.thumb === "string" ? saved.thumb : "",
                identity: String(saved.identity ?? "")
            };
        } catch {
            return null;
        }
    };

    /** 刷新页面后恢复最近一次图片查询：查询记录来自浏览器会话，原图来自本地暂存 */
    const restore = (): Promise<void> => {
        if (!import.meta.client) return Promise.resolve();
        if (restoring) return restoring;
        restoring = (async () => {
            // 本次访问已经选过图时，内存里的状态比暂存的新
            if (query.value) return;
            const saved = readSavedQuery();
            const stored = await loadImageSearchFile();
            if (query.value) return;
            if (stored && saved?.fileId && stored.id === saved.fileId) {
                setFile(stored.file);
            } else if (stored && Date.now() - stored.savedAt > STALE_FILE_MS) {
                void removeImageSearchFile();
            }
            // 还没检索的图片丢了原图就无法继续，不恢复
            query.value = saved && (saved.searchToken || file.value) ? saved : null;
        })();
        return restoring;
    };

    /** 选定一张待搜索的图片：此时不发起检索，由结果页上传并取得编号 */
    const setPending = (source: File): void => {
        setFile(source);
        const fileId = createFileId();
        query.value = { searchToken: "", fileId, thumb: "", identity: currentIdentity.value };
        persist();
        void saveImageSearchFile(fileId, source);
    };

    /**
     * 记录一次检索成功的图片查询：编号由结果页地址携带，这里保存对应的图片
     * isCurrent 在缩小图生成后再核对一次，检索已被取代或页面已离开时不写入，返回 false
     */
    const commit = async (searchToken: string, source: File, isCurrent: () => boolean = () => true): Promise<boolean> => {
        const thumb = await createThumb(source);
        if (!isCurrent()) return false;
        // 这张图在选定时已经暂存过，沿用同一个标识
        const pendingId = file.value === source ? query.value?.fileId : "";
        const fileId = pendingId || createFileId();
        setFile(source);
        query.value = { searchToken, fileId, thumb, identity: currentIdentity.value };
        persist();
        if (!pendingId) void saveImageSearchFile(fileId, source);
        return true;
    };

    /** 指定编号是否由当前登录身份发起；不是本页记录的编号时交给接口判断 */
    const isOwnedByCurrentIdentity = (searchToken: string): boolean => {
        if (!query.value || query.value.searchToken !== searchToken) return true;
        return query.value.identity === currentIdentity.value;
    };

    return {
        available,
        shopAvailable,
        resultInShop,
        isAvailable,
        formats,
        accept,
        capabilitiesLoaded,
        fetchCapabilities,
        markUnavailable,
        isSupportedFile,
        canPreview,
        query,
        file,
        previewUrl,
        restore,
        setPending,
        commit,
        isOwnedByCurrentIdentity
    };
});
