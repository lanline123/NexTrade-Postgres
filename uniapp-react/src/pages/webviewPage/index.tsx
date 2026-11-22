import { useState } from "react";
import { View, WebView } from "@tarojs/components";
import Taro, { useRouter } from "@tarojs/taro";

import EmptyBox from "@/components/empty-box/EmptyBox";
import { usePageGuards } from "@/utils/pageMixin";
import { t } from "@/utils/i18n";

/** 外链落地页：小程序用 web-view，H5 端 Taro 会渲染 iframe */
export default function WebviewPage() {
    usePageGuards();
    const router = useRouter();
    const url = decodeURIComponent(router.params.url || "");
    const [loadFailed, setLoadFailed] = useState(false);

    if (!url) {
        // 此前直接 return null 白屏：参数丢失/外链失效时用户无路可走，改为与 404 同构的兜底
        return (
            <View style={{ paddingTop: "120rpx" }}>
                <EmptyBox height="auto" text={t("链接无效或已失效")} actionText={t("回到首页")} onAction={() => void Taro.reLaunch({ url: "/pages/index/index" })} />
            </View>
        );
    }
    if (loadFailed) {
        return (
            <View style={{ paddingTop: "120rpx" }}>
                <EmptyBox
                    height="auto"
                    text={t("页面加载失败，请检查网络后重试")}
                    actionText={t("重新加载")}
                    onAction={() => setLoadFailed(false)}
                />
            </View>
        );
    }
    return <WebView src={url} onError={() => setLoadFailed(true)} />;
}
