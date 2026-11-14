import Taro from "@tarojs/taro";

import { imageFormat } from "@/utils/format";
import { t } from "@/utils/i18n";
import { baseUrl, requestHeader } from "@/utils/request";

/**
 * 图片上传工具（自旧 utils/upload.ts 等价移植，恢复共享入口）。
 * 此前上传逻辑分散在 components/upload、tig-upload、tig-image-upload
 * 与若干页面内联副本中；本文件按旧签名收口：
 * uploadImage(filePath, onProgress?, apiUrl?, hidden?) => Promise<string>
 * 底层走 Taro.uploadFile，鉴权头（Authorization token / Secret）与
 * 现有组件实现保持一致。
 */
export function uploadImage(
    filePath: string,
    onProgress?: (progress: number) => void,
    apiUrl: string = "user/user/uploadImg",
    hidden: boolean = false
): Promise<string> {
    return new Promise((resolve, reject) => {
        if (!hidden) {
            Taro.showLoading({ title: t("上传中") });
        }
        const task = Taro.uploadFile({
            url: baseUrl + (process.env.TARO_APP_API_PREFIX || "") + apiUrl,
            filePath,
            name: "file",
            // 鉴权头与普通请求共用 requestHeader()（token + Secret 等，与各上传组件一致）
            header: requestHeader(),
            success: (res) => {
                let result: { code?: number; message?: string; data?: any } | null = null;
                try {
                    result = JSON.parse(res.data);
                } catch {
                    result = null;
                }
                if (res.statusCode < 200 || res.statusCode >= 300 || !result || result.code !== 0 || !result.data) {
                    Taro.showToast({ title: result?.message || t("图片上传失败"), icon: "none" });
                    reject(new Error(result?.message || "upload failed"));
                    return;
                }
                // 旧版返回经 imageFormat 处理后的图片 URL 字符串
                const url = typeof result.data === "string" ? result.data : result.data.url || result.data.picUrl || "";
                resolve(imageFormat(url));
            },
            fail: (err) => {
                Taro.showToast({ title: t("图片上传失败"), icon: "none" });
                reject(err);
            },
            complete: () => {
                if (!hidden) Taro.hideLoading();
            }
        });
        if (onProgress && task && typeof (task as any).onProgressUpdate === "function") {
            (task as any).onProgressUpdate((res: any) => onProgress(res.progress));
        }
    });
}

/** 批量上传（旧 uploadImages）：顺序上传，任一失败整体 reject。 */
export async function uploadImages(
    filePaths: string[],
    onProgress?: (progress: number) => void,
    apiUrl: string = "user/user/uploadImg"
): Promise<string[]> {
    const urls: string[] = [];
    for (let i = 0; i < filePaths.length; i++) {
        const url = await uploadImage(filePaths[i], onProgress, apiUrl, i > 0);
        urls.push(url);
    }
    return urls;
}
