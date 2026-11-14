import { useEffect, useState } from "react";
import { Image, View } from "@tarojs/components";

import { getQrCode } from "@/api/common";

interface Props {
    /** 二维码内容（旧 value） */
    value?: string;
    /** 兼容旧 modelValue（生成图片 URL 回传走 onUpdateModelValue/onSuccess） */
    modelValue?: string;
    id?: string;
    size?: number;
    fileType?: string;
    show?: boolean;
    margin?: number;
    immediatelyCreate?: boolean;
    onUpdateModelValue?: (url: string) => void;
    onSuccess?: (url: string) => void;
}

/**
 * 二维码（自旧 tig-qrcode.vue 对位移植，生成方式按本项目既定决策变更）。
 * 旧版用 uqrcodejs 在本地 canvas 生成；重构版未引入该依赖，项目既有
 * 范式（pages/salesman/src/qrcode.ts）已改为后端 common/util/qrCode
 * 生成图片。本组件沿用该范式承接旧 props（value/size/show/margin），
 * success 事件在图片就绪后以图片 URL 回调；离线本地生成不在此恢复
 * （原因见 salesman/src/qrcode.ts 文件头 TODO 记录）。
 */
export default function TigQrcode({
    value = "https://www.tigshop.com/",
    size = 200,
    show = true,
    onUpdateModelValue,
    onSuccess
}: Props) {
    const [url, setUrl] = useState("");

    useEffect(() => {
        let cancelled = false;
        (async () => {
            try {
                const img: any = await getQrCode(value);
                const src = typeof img === "string" ? img : img?.url || img?.data || "";
                if (cancelled || !src) return;
                setUrl(src);
                onUpdateModelValue?.(src);
                onSuccess?.(src);
            } catch {
                // 静默降级：二维码拉取失败不阻塞页面（与现行海报链路一致）
            }
        })();
        return () => {
            cancelled = true;
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [value]);

    if (!show) return null;
    return (
        <View style={{ width: `${size}px`, height: `${size}px` }}>
            {url ? <Image src={url} style={{ width: `${size}px`, height: `${size}px` }} /> : null}
        </View>
    );
}
