import { useState } from "react";
import { Text, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import { requestDeliveryDownload, type DeliveryFileItem } from "@/api/product/product";
import { copy } from "@/utils";
import { t } from "@/utils/i18n";
import { baseUrl } from "@/utils/requestBaseUrl";

import "./index.scss";

/** 字节数转可读大小（B / KB / MB，保留一位小数）。 */
export function formatFileSize(bytes: number): string {
    const size = Number(bytes || 0);
    if (size < 1024) return `${size} B`;
    if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
    if (size < 1024 * 1024 * 1024) return `${(size / 1024 / 1024).toFixed(1)} MB`;
    return `${(size / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

/**
 * 交付附件行（文件型交付，我方新增）：与 DeliveryLinks 同款行样式，
 * 呈现附件名 + 大小 + 「下载」。点击先向服务端申领短时效令牌（接口层
 * 强校验订单归属与已购/已发货门），再打开一次性下载链接；链接
 * 30 分钟有效、最多下载 5 次，过期重新点击即可。非 H5 端（未来
 * weapp）降级为复制完整链接 + 提示到浏览器打开。无附件时不渲染。
 */
export default function DeliveryFiles({
    files,
    orderId
}: {
    files?: DeliveryFileItem[] | null;
    orderId?: number;
}) {
    const [busyId, setBusyId] = useState(0);
    const list = Array.isArray(files) ? files.filter((f) => f && Number(f.fileId) > 0) : [];
    if (!list.length) return null;

    const download = async (file: DeliveryFileItem) => {
        if (busyId) return;
        setBusyId(file.fileId);
        try {
            const grant: any = await requestDeliveryDownload({
                fileId: file.fileId,
                productId: file.productId,
                orderId
            });
            const path = String(grant?.downloadUrl || "");
            if (!path) throw new Error(t("获取下载链接失败，请重试"));
            if (process.env.TARO_ENV === "h5") {
                window.open(path, "_blank");
            } else {
                const full = `${String(baseUrl || "").replace(/\/$/, "")}${path}`;
                copy(full, () => {
                    Taro.showToast({ title: t("链接已复制，请在浏览器中打开下载"), icon: "none" });
                });
            }
        } catch (error: any) {
            Taro.showToast({
                title: error?.message || error?.msg || t("获取下载链接失败，请重试"),
                icon: "none"
            });
        } finally {
            setBusyId(0);
        }
    };

    return (
        <View className="delivery-files">
            {list.map((file) => (
                <View className="delivery-file-row" key={file.fileId}>
                    <Text className="delivery-file-name">{file.fileName}</Text>
                    <Text className="delivery-file-size">{formatFileSize(file.fileSize)}</Text>
                    <View className="delivery-file-btn" onClick={() => download(file)}>
                        {busyId === file.fileId ? t("获取中…") : t("下载")}
                    </View>
                </View>
            ))}
            <View className="delivery-file-tip">{t("下载链接 30 分钟内有效，最多可下载 5 次，过期后重新点击下载即可。")}</View>
        </View>
    );
}
