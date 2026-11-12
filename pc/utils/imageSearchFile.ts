/**
 * 以图搜图的原图暂存（IndexedDB）
 * 只保留最近一张，用于刷新页面后继续搜索或在图搜编号失效后重新上传；
 * 存储不可用、被禁用或读写失败时一律静默跳过，调用方按“没有原图”处理
 */

const DB_NAME = "imageSearch";
const STORE_NAME = "file";
const RECORD_KEY = "current";

export interface StoredImageSearchFile {
    /** 与浏览器会话里的查询记录对应，不一致说明这张图属于别的查询 */
    id: string;
    file: File;
    savedAt: number;
}

const openDb = (): Promise<IDBDatabase> => {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open(DB_NAME, 1);
        request.onupgradeneeded = () => {
            request.result.createObjectStore(STORE_NAME);
        };
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
        request.onblocked = () => reject(new Error("blocked"));
    });
};

const run = async <T>(mode: IDBTransactionMode, action: (store: IDBObjectStore) => IDBRequest<T>): Promise<T | undefined> => {
    try {
        const db = await openDb();
        try {
            return await new Promise<T>((resolve, reject) => {
                const transaction = db.transaction(STORE_NAME, mode);
                const request = action(transaction.objectStore(STORE_NAME));
                transaction.oncomplete = () => resolve(request.result);
                transaction.onerror = () => reject(transaction.error);
                transaction.onabort = () => reject(transaction.error);
            });
        } finally {
            db.close();
        }
    } catch {
        return undefined;
    }
};

export const saveImageSearchFile = async (id: string, file: File): Promise<void> => {
    await run("readwrite", (store) => store.put({ id, file, savedAt: Date.now() } as StoredImageSearchFile, RECORD_KEY));
};

export const loadImageSearchFile = async (): Promise<StoredImageSearchFile | null> => {
    const record = await run<StoredImageSearchFile | undefined>("readonly", (store) => store.get(RECORD_KEY));
    return record && record.file instanceof Blob && typeof record.id === "string" ? record : null;
};

export const removeImageSearchFile = async (): Promise<void> => {
    await run("readwrite", (store) => store.delete(RECORD_KEY));
};
