import { useRef, useState } from "react";
import { Image, Text, Video, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import { baseUrl, getSecret } from "@/utils/request";
import { imageFormat, isVideoPic } from "@/utils/format";
import { t } from "@/utils/i18n";

import "./TigMediaUpload.scss";

export interface MediaUploadItem {
    name?: string;
    extname?: string;
    url: string;
    picName?: string;
    picThumb?: string;
    picUrl?: string;
    /** 媒体类型：视频条目落库带 fileType="video"，图片不带（与历史 pics 数据一致） */
    fileType?: "image" | "video";
}

interface Props {
    /** 最多可上传数量（图片与视频合计） */
    limit?: number;
    /** 已上传列表（v-model 同形） */
    modelValue?: MediaUploadItem[];
    onModelValueChange?: (list: MediaUploadItem[]) => void;
    /** 单个上传成功回调 */
    onUploaded?: (item: MediaUploadItem) => void;
    /** 删除某个 */
    onDelete?: (index: number) => void;
    /** 上传按钮图标：加号 / 相机（与 TigImageUpload 同口径） */
    trigger?: "camera" | "plus";
}

/** 视频大小上限（MB），与后端 uploadVideo 服务层校验一致 */
export const VIDEO_MAX_SIZE_MB = 50;
/** 允许的视频扩展名（小写），与后端白名单一致 */
export const VIDEO_EXTENSIONS = ["mp4", "webm", "mov"];

const videoExtOf = (name: string) => {
    const clean = (name || "").split(/[?#]/)[0].toLowerCase();
    const dot = clean.lastIndexOf(".");
    return dot >= 0 ? clean.slice(dot + 1) : "";
};

/**
 * 图片/视频混合上传（售后凭证专用，2026-11-19 新增）：
 * - 结构与 TigImageUpload 同形（九宫格、删除、图片预览），条目可直接进售后 pics
 * - 图片走 user/user/uploadImg（与 TigImageUpload 完全一致），视频走 user/user/uploadVideo
 * - 视频落库条目带 fileType="video"，picUrl 为视频地址、picThumb 为封面（后端暂不产封面则为空）
 * - H5 用原生 file input + fetch 直传（保住真实文件名，后端按扩展名校验）；非 H5 走 chooseImage/chooseVideo
 * - 视频客户端先行校验：格式 mp4/webm/mov、大小 ≤50MB，超限明确 toast（后端同口径兜底）
 */
export default function TigMediaUpload({
    limit = 5,
    modelValue = [],
    onModelValueChange,
    onUploaded,
    onDelete,
    trigger = "plus"
}: Props) {
    const listRef = useRef<MediaUploadItem[]>(modelValue);
    listRef.current = modelValue;
    const uploadingRef = useRef(0);
    const [, setUploadingTick] = useState(0);

    const beginUpload = () => {
        uploadingRef.current += 1;
        if (uploadingRef.current === 1) {
            Taro.showLoading({ title: t("上传中") });
        }
        setUploadingTick((v) => v + 1);
    };
    const endUpload = () => {
        uploadingRef.current = Math.max(0, uploadingRef.current - 1);
        if (uploadingRef.current === 0) {
            Taro.hideLoading();
        }
        setUploadingTick((v) => v + 1);
    };

    const appendItem = (item: MediaUploadItem) => {
        const next = [...listRef.current, item];
        listRef.current = next;
        onModelValueChange?.(next);
        onUploaded?.(item);
    };

    /** 图片上传（与 TigImageUpload.uploadFile 同链路同回写形状） */
    const uploadImage = (filePath: string) => {
        beginUpload();
        Taro.uploadFile({
            url: baseUrl + process.env.TARO_APP_API_PREFIX + "user/user/uploadImg",
            filePath,
            name: "file",
            header: {
                Authorization: Taro.getStorageSync("token"),
                Secret: getSecret()
            },
            success: (uploadFileRes) => {
                let result: { code?: number; message?: string; data?: any } | null = null;
                try {
                    result = JSON.parse(uploadFileRes.data);
                } catch (error) {
                    console.error("上传响应解析失败:", error, uploadFileRes.data);
                }
                if (uploadFileRes.statusCode < 200 || uploadFileRes.statusCode >= 300 || result?.code !== 0 || !result?.data) {
                    Taro.showToast({
                        title: result?.message || t("图片上传失败"),
                        icon: "none"
                    });
                    return;
                }
                const data = result.data;
                appendItem({
                    name: data.picName,
                    extname: String(data.picUrl || "").split(".")[1],
                    url: imageFormat(data.picThumb),
                    picName: data.picName,
                    picThumb: data.picThumb,
                    picUrl: data.picUrl
                });
                Taro.showToast({ title: t("图片上传成功") });
            },
            fail: (error) => {
                console.error(error);
                Taro.showToast({ title: t("图片上传失败"), icon: "none" });
            },
            complete: () => endUpload()
        });
    };

    /** 视频上传成功回写（uploadVideo 返回 {videoUrl, videoName, videoCover}） */
    const handleVideoResult = (result: { code?: number; message?: string; data?: any } | null, fallbackName: string) => {
        if (!result || result.code !== 0 || !result.data?.videoUrl) {
            Taro.showToast({
                title: result?.message || t("视频上传失败"),
                icon: "none"
            });
            return;
        }
        const data = result.data;
        appendItem({
            name: data.videoName || fallbackName,
            extname: videoExtOf(data.videoUrl),
            url: imageFormat(data.videoUrl),
            picName: data.videoName || fallbackName,
            picThumb: data.videoCover || "",
            picUrl: data.videoUrl,
            fileType: "video"
        });
        Taro.showToast({ title: t("视频上传成功") });
    };

    /** 视频上传（非 H5：Taro.uploadFile 临时文件链路） */
    const uploadVideo = (filePath: string, fallbackName: string) => {
        beginUpload();
        Taro.uploadFile({
            url: baseUrl + process.env.TARO_APP_API_PREFIX + "user/user/uploadVideo",
            filePath,
            name: "file",
            header: {
                Authorization: Taro.getStorageSync("token"),
                Secret: getSecret()
            },
            success: (uploadFileRes) => {
                let result: { code?: number; message?: string; data?: any } | null = null;
                try {
                    result = JSON.parse(uploadFileRes.data);
                } catch (error) {
                    console.error("上传响应解析失败:", error, uploadFileRes.data);
                }
                if (uploadFileRes.statusCode < 200 || uploadFileRes.statusCode >= 300) {
                    Taro.showToast({ title: t("视频上传失败"), icon: "none" });
                    return;
                }
                handleVideoResult(result, fallbackName);
            },
            fail: (error) => {
                console.error(error);
                Taro.showToast({ title: t("视频上传失败"), icon: "none" });
            },
            complete: () => endUpload()
        });
    };

    /** H5 直传（图片/视频共用）：fetch + FormData，文件名取真实 File 名 */
    const uploadByFetch = async (file: File, kind: "image" | "video") => {
        beginUpload();
        try {
            const body = new FormData();
            body.append("file", file, file.name);
            const response = await fetch(
                baseUrl + process.env.TARO_APP_API_PREFIX + (kind === "image" ? "user/user/uploadImg" : "user/user/uploadVideo"),
                {
                    method: "POST",
                    headers: {
                        Authorization: Taro.getStorageSync("token"),
                        Secret: getSecret()
                    },
                    body
                }
            );
            const result = await response.json().catch(() => null);
            if (!response.ok || !result || result.code !== 0 || !result.data) {
                Taro.showToast({
                    title: result?.message || (kind === "image" ? t("图片上传失败") : t("视频上传失败")),
                    icon: "none"
                });
                return;
            }
            if (kind === "image") {
                const data = result.data;
                appendItem({
                    name: data.picName,
                    extname: String(data.picUrl || "").split(".")[1],
                    url: imageFormat(data.picThumb),
                    picName: data.picName,
                    picThumb: data.picThumb,
                    picUrl: data.picUrl
                });
                Taro.showToast({ title: t("图片上传成功") });
            } else {
                handleVideoResult(result, file.name);
            }
        } catch (error) {
            console.error(error);
            Taro.showToast({ title: kind === "image" ? t("图片上传失败") : t("视频上传失败"), icon: "none" });
        } finally {
            endUpload();
        }
    };

    /** 校验视频（格式 + 大小），不通过时 toast 并返回 false */
    const validateVideo = (name: string, size?: number) => {
        if (!VIDEO_EXTENSIONS.includes(videoExtOf(name))) {
            Taro.showToast({ title: t("仅支持 mp4、webm、mov 格式的视频"), icon: "none" });
            return false;
        }
        if (typeof size === "number" && size > VIDEO_MAX_SIZE_MB * 1024 * 1024) {
            Taro.showToast({ title: t("视频大小不能超过{0}MB", { 0: VIDEO_MAX_SIZE_MB }), icon: "none" });
            return false;
        }
        return true;
    };

    /** H5：动态 input 选文件（图片/视频混合，按 MIME 分流） */
    const chooseByInput = (remain: number) => {
        if (typeof document === "undefined") return;
        const input = document.createElement("input");
        input.type = "file";
        input.accept = "image/*,video/*";
        input.multiple = remain > 1;
        input.style.display = "none";
        input.onchange = () => {
            const files = Array.from(input.files || []).slice(0, remain);
            files.forEach((file) => {
                if (file.type.startsWith("video/")) {
                    if (validateVideo(file.name, file.size)) void uploadByFetch(file, "video");
                } else if (file.type.startsWith("image/")) {
                    void uploadByFetch(file, "image");
                } else {
                    Taro.showToast({ title: t("不支持的文件类型"), icon: "none" });
                }
            });
            input.remove();
        };
        document.body.appendChild(input);
        input.click();
    };

    /** 非 H5：chooseImage / chooseVideo 分流 */
    const chooseNative = (remain: number) => {
        Taro.showActionSheet({
            itemList: [t("选择图片"), t("选择视频")],
            success: (res) => {
                if (res.tapIndex === 0) {
                    Taro.chooseImage({
                        count: remain,
                        sizeType: ["original", "compressed"],
                        success: (chooseRes) => {
                            chooseRes.tempFilePaths.forEach((filePath) => uploadImage(filePath));
                        },
                        fail: (error) => console.error(error)
                    });
                } else {
                    Taro.chooseVideo({
                        sourceType: ["album", "camera"],
                        compressed: true,
                        success: (chooseRes) => {
                            const name = chooseRes.tempFilePath || "video.mp4";
                            if (validateVideo(name, (chooseRes as any).size)) {
                                uploadVideo(chooseRes.tempFilePath, name.split("/").pop() || "video.mp4");
                            }
                        },
                        fail: (error) => console.error(error)
                    });
                }
            },
            fail: (error) => console.error(error)
        });
    };

    const handleChoose = () => {
        const remain = limit - listRef.current.length;
        if (remain <= 0) {
            Taro.showToast({
                title: t("最多上传{0}个文件", { 0: limit }),
                icon: "none"
            });
            return;
        }
        if (process.env.TARO_ENV === "h5") {
            chooseByInput(remain);
        } else {
            chooseNative(remain);
        }
    };

    const handleDelete = (index: number) => {
        const next = listRef.current.filter((_item, i) => i !== index);
        listRef.current = next;
        onModelValueChange?.(next);
        onDelete?.(index);
    };

    const handlePreviewImage = (current: MediaUploadItem) => {
        const urls = listRef.current.filter((item) => !isVideoPic(item)).map((item) => imageFormat(item.picUrl || item.url));
        if (!urls.length) return;
        Taro.previewImage({
            urls,
            current: imageFormat(current.picUrl || current.url)
        });
    };

    return (
        <View className="tig-media-upload">
            {modelValue.map((item, index) => {
                const video = isVideoPic(item);
                return (
                    <View className="tig-media-upload__item" key={`${item.picName || item.url}-${index}`}>
                        {video ? (
                            <Video
                                className="tig-media-upload__video"
                                src={item.url}
                                controls
                                showCenterPlayBtn
                                objectFit="cover"
                            />
                        ) : (
                            <Image
                                className="tig-media-upload__img"
                                src={item.url}
                                mode="aspectFill"
                                onClick={() => handlePreviewImage(item)}
                            />
                        )}
                        <View className="tig-media-upload__delete" onClick={() => handleDelete(index)}>
                            <Text className="tig-media-upload__delete-icon">×</Text>
                        </View>
                    </View>
                );
            })}
            {modelValue.length < limit && (
                <View className="tig-media-upload__trigger" onClick={handleChoose}>
                    {trigger === "camera" ? (
                        <Text className="iconfont-h5 icon-camera tig-media-upload__camera" />
                    ) : (
                        <Text className="tig-media-upload__plus">+</Text>
                    )}
                </View>
            )}
        </View>
    );
}
